//! The accessibility bridge (N3.05): a page view draws pages as pictures, so on its own it is one
//! opaque widget to a screen reader. [`PagesAccess`] describes the text under it as an AccessKit
//! tree, the way a browser describes a page: the view is the Document node (with the caret and
//! selection as an AccessKit text selection), each paragraph a Paragraph, Heading (with its
//! level) or ListItem (grouped in Lists, with its ListMarker), holding one TextRun per laid-out
//! line with its characters' screen positions; tables are Table › Row › Cell, pictures Images
//! named by their alt text, hyperlinks Links around their runs.
//!
//! The app describes the pages it was asked for through [`AccessDescriber`]; the bridge builds
//! nothing unless a screen reader (or a test) asked egui for AccessKit output, describes only the
//! pages on screen and one either side, caps the tree at [`MAX_NODES`], and rebuilds only when
//! the app's revision, the selection, the described pages or their place on screen changed.
//!
//! ```ignore
//! let view = PageView::new("doc", &mut pages, &mut state).name(&doc.title).show(ui);
//! PagesAccess::new(view.response.id, &view.pages).revision(doc.layout_revision()).show(ui.ctx(), &mut DocDescriber(&doc));
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use egui::accesskit::{self, Node, NodeId, Role, TextDirection, TextPosition, TextSelection};
use egui::{Context, Id, Pos2, Rect};

use super::view::PlacedPage;

/// The most nodes described at once (a dense page has a few hundred).
pub const MAX_NODES: usize = 6000;
/// AccessKit's word starts are `u8`s: a text run holds at most this many characters; a longer
/// line is split into runs.
pub const MAX_RUN: usize = 255;
/// The deepest table nesting described.
pub const MAX_DEPTH: usize = 16;

/// What a block node is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockRole {
    Paragraph,
    /// A heading at outline level 1–9.
    Heading(u8),
    /// A list item at list level 0–8, with its marker text ("1.", "•").
    ListItem {
        level: u8,
        marker: String,
    },
    Table,
    Row,
    Cell,
    /// A picture or chart named by its alt text.
    Image(String),
    /// A hyperlink to `url`.
    Link(String),
}

/// A handle to a node the describer added, for adding its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AccessNode(u32);

/// A text position the app names: (node of the text run, char offset in it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessPosition {
    pub run: AccessNode,
    pub offset: usize,
}

/// Collects one pass's description. Every method returns `None` once [`MAX_NODES`] is reached
/// (or the parent is past [`MAX_DEPTH`] tables deep), and the describer stops.
pub struct AccessBuilder {
    view: Id,
    nodes: Vec<Built>,
    /// Per parent, the List node still taking items and its items' level.
    open_lists: HashMap<Option<AccessNode>, (AccessNode, u8)>,
}

/// A node being built.
struct Built {
    parent: Option<AccessNode>,
    node: Node,
    /// Tables from the document down to this node, itself included.
    tables: usize,
    /// A table's rows, a row's cells.
    parts: usize,
    /// A table's widest row.
    columns: usize,
}

impl AccessBuilder {
    fn new(view: Id) -> Self {
        AccessBuilder { view, nodes: Vec::new(), open_lists: HashMap::new() }
    }

    fn id(&self, node: AccessNode) -> Id {
        self.view.with(("access", node.0))
    }

    fn built(&mut self, node: AccessNode) -> Option<&mut Built> {
        self.nodes.get_mut(node.0 as usize)
    }

    /// Adds `node` under `parent`, or `None` when the tree is full or `parent` isn't one of its nodes.
    fn push(&mut self, parent: Option<AccessNode>, node: Node, tables: usize) -> Option<AccessNode> {
        if self.nodes.len() >= MAX_NODES {
            return None;
        }
        let at = AccessNode(u32::try_from(self.nodes.len()).ok()?);
        if let Some(parent) = parent {
            let id = self.id(at).accesskit_id();
            self.built(parent)?.node.push_child(id);
        }
        self.nodes.push(Built { parent, node, tables, parts: 0, columns: 0 });
        Some(at)
    }

    /// A block under `parent` (`None`: under the document). Consecutive list items at one level
    /// are grouped into a List node by the builder.
    // wordcraft: crates/ui-egui/src/a11y.rs:239 `Builder::container`, :274 `para_node`.
    pub fn block(&mut self, parent: Option<AccessNode>, role: BlockRole) -> Option<AccessNode> {
        let depth = match parent {
            Some(parent) => self.built(parent)?.tables,
            None => 0,
        };
        let tables = depth + usize::from(role == BlockRole::Table);
        if tables > MAX_DEPTH {
            return None;
        }
        if !matches!(role, BlockRole::ListItem { .. }) {
            self.open_lists.remove(&parent);
        }
        let mut under = parent;
        let node = match &role {
            BlockRole::Paragraph => Node::new(Role::Paragraph),
            BlockRole::Heading(level) => {
                let mut node = Node::new(Role::Heading);
                node.set_level(usize::from((*level).clamp(1, 9)));
                node
            }
            BlockRole::ListItem { level, .. } => {
                under = match self.open_lists.get(&parent) {
                    Some((list, open)) if open == level => Some(*list),
                    _ => {
                        let list = self.push(parent, Node::new(Role::List), depth)?;
                        self.open_lists.insert(parent, (list, *level));
                        Some(list)
                    }
                };
                let mut node = Node::new(Role::ListItem);
                node.set_level(usize::from(*level) + 1);
                node
            }
            BlockRole::Table => Node::new(Role::Table),
            BlockRole::Row => {
                let mut node = Node::new(Role::Row);
                if let Some(table) = parent.and_then(|p| self.built(p)) {
                    node.set_row_index(table.parts);
                    table.parts += 1;
                }
                node
            }
            BlockRole::Cell => {
                let mut node = Node::new(Role::Cell);
                let row = parent.and_then(|p| self.built(p));
                if let Some(row) = row {
                    node.set_column_index(row.parts);
                    if let Some(at) = row.node.row_index() {
                        node.set_row_index(at);
                    }
                    row.parts += 1;
                    let (cells, table) = (row.parts, row.parent);
                    if let Some(table) = table.and_then(|t| self.built(t)) {
                        table.columns = table.columns.max(cells);
                    }
                }
                node
            }
            BlockRole::Image(alt) => {
                let mut node = Node::new(Role::Image);
                if !alt.trim().is_empty() {
                    node.set_label(alt.as_str());
                }
                node
            }
            BlockRole::Link(url) => {
                let mut node = Node::new(Role::Link);
                node.set_url(url.as_str());
                node
            }
        };
        let at = self.push(under, node, tables)?;
        if let BlockRole::ListItem { marker, .. } = role
            && !marker.trim().is_empty()
        {
            let mut mark = Node::new(Role::ListMarker);
            mark.set_value(marker);
            self.push(Some(at), mark, depth);
        }
        Some(at)
    }

    /// One laid-out line's text under `parent`, with each character's screen x and width and the
    /// line's screen rectangle; word starts are worked out from the text.
    // wordcraft: crates/ui-egui/src/a11y.rs:431 `Builder::flush`, :501 `push_char`, :515 `word_starts`.
    pub fn line(&mut self, parent: AccessNode, text: &str, chars: &[(f32, f32)], line: Rect) -> Option<AccessNode> {
        let tables = self.built(parent)?.tables;
        let text: Vec<char> = text.chars().collect();
        let mut first = None;
        let mut previous: Option<AccessNode> = None;
        let mut start = 0;
        loop {
            let end = (start + MAX_RUN).min(text.len());
            let run = &text[start..end];
            let spans: Vec<(f32, f32)> =
                (start..end).map(|i| chars.get(i).or_else(|| chars.last()).map_or((line.left(), 0.0), |&(x, w)| (finite(x), finite(w)))).collect();
            let left = spans.iter().map(|c| c.0).fold(f32::INFINITY, f32::min);
            let right = spans.iter().map(|c| c.0 + c.1).fold(f32::NEG_INFINITY, f32::max);
            let (left, right) = if left.is_finite() && right.is_finite() { (left, right) } else { (line.left(), line.left()) };
            let bounds = Rect::from_min_max(Pos2::new(left, line.top()), Pos2::new(right.max(left), line.bottom()));

            let value: String = run.iter().collect();
            let mut node = Node::new(Role::TextRun);
            node.set_bounds(bounds_of(bounds));
            node.set_text_direction(TextDirection::LeftToRight);
            node.set_character_positions(spans.iter().map(|c| c.0 - left).collect::<Vec<f32>>());
            node.set_character_widths(spans.iter().map(|c| c.1).collect::<Vec<f32>>());
            // One to four bytes a character, which fit AccessKit's `u8`.
            node.set_character_lengths(run.iter().map(|c| c.len_utf8() as u8).collect::<Vec<u8>>());
            node.set_word_starts(word_starts(run));
            node.set_value(value.as_str());
            if let Some(before) = previous {
                node.set_previous_on_line(self.id(before).accesskit_id());
            }
            let at = self.push(Some(parent), node, tables)?;
            if let Some(before) = previous {
                let next = self.id(at).accesskit_id();
                if let Some(before) = self.built(before) {
                    before.node.set_next_on_line(next);
                }
            }
            if let Some(Built { node: link, .. }) = self.built(parent).filter(|p| p.node.role() == Role::Link) {
                let label = format!("{}{value}", link.label().unwrap_or_default());
                link.set_label(label);
            }
            first.get_or_insert(at);
            previous = Some(at);
            if end >= text.len() {
                return first;
            }
            start = end;
        }
    }

    /// The finished tree: blocks span their children, tables count their rows and columns.
    fn finish(mut self, selection: Option<(AccessPosition, AccessPosition)>) -> Tree {
        for at in (0..self.nodes.len()).rev() {
            let Built { node, parts, columns, parent, .. } = &mut self.nodes[at];
            if node.role() == Role::Table {
                node.set_row_count(*parts);
                node.set_column_count(*columns);
            }
            let (bounds, parent) = (node.bounds(), *parent);
            let (Some(bounds), Some(parent)) = (bounds, parent) else { continue };
            let above = &mut self.nodes[parent.0 as usize].node;
            let union = above.bounds().map_or(bounds, |b| accesskit::Rect {
                x0: b.x0.min(bounds.x0),
                y0: b.y0.min(bounds.y0),
                x1: b.x1.max(bounds.x1),
                y1: b.y1.max(bounds.y1),
            });
            above.set_bounds(union);
        }
        let position = |position: AccessPosition| {
            ((position.run.0 as usize) < self.nodes.len())
                .then(|| TextPosition { node: self.id(position.run).accesskit_id(), character_index: position.offset })
        };
        let selection = selection.and_then(|(caret, other)| match (position(caret), position(other)) {
            (Some(focus), Some(anchor)) => Some(TextSelection { anchor, focus }),
            (Some(focus), None) => Some(TextSelection { anchor: focus, focus }),
            _ => None,
        });
        let top =
            (0..self.nodes.len()).filter(|&at| self.nodes[at].parent.is_none()).map(|at| self.id(AccessNode(at as u32)).accesskit_id()).collect();
        let nodes = (0..self.nodes.len())
            .map(|at| self.id(AccessNode(at as u32)))
            .collect::<Vec<_>>()
            .into_iter()
            .zip(self.nodes)
            .map(|(id, built)| (id, built.node))
            .collect();
        Tree { nodes, top, selection }
    }
}

fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn bounds_of(r: Rect) -> accesskit::Rect {
    accesskit::Rect { x0: r.min.x.into(), y0: r.min.y.into(), x1: r.max.x.into(), y1: r.max.y.into() }
}

/// Characters where a word starts (after one that isn't part of a word).
fn word_starts(text: &[char]) -> Vec<u8> {
    let mut starts = Vec::new();
    let mut after_gap = true;
    for (i, c) in text.iter().enumerate() {
        let word = c.is_alphanumeric() || *c == '_';
        if word && after_gap && i > 0 {
            starts.push(i as u8);
        }
        after_gap = !word;
    }
    starts
}

/// The app's side: it describes pages of its layout.
pub trait AccessDescriber {
    /// Describe pages `pages` (their placement on screen in `placed`) into `out`, in reading
    /// order.
    fn describe(&mut self, placed: &[PlacedPage], out: &mut AccessBuilder);

    /// The caret and the selection's other end, if the document has a text selection on the
    /// described pages.
    fn selection(&self) -> Option<(AccessPosition, AccessPosition)>;
}

/// What was described: nodes in creation order, the document node's children and the selection.
struct Tree {
    nodes: Vec<(Id, Node)>,
    top: Vec<NodeId>,
    selection: Option<TextSelection>,
}

/// What a tree was built for.
#[derive(Clone, Debug, PartialEq)]
struct Key {
    revision: u64,
    pages: Vec<(usize, Rect)>,
    selection: Option<(AccessPosition, AccessPosition)>,
}

#[derive(Clone)]
struct Cached {
    key: Key,
    tree: Arc<Tree>,
}

/// The bridge, built per pass after the [`PageView`](super::view::PageView).
pub struct PagesAccess<'a> {
    view: Id,
    placed: &'a [PlacedPage],
    revision: u64,
}

impl<'a> PagesAccess<'a> {
    /// Describes the pages `placed` under the view widget `view`.
    pub fn new(view: Id, placed: &'a [PlacedPage]) -> Self {
        PagesAccess { view, placed, revision: 0 }
    }

    /// The app's layout revision: a new value rebuilds the tree.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }

    /// Describe, or reuse last pass's tree. Nodes hang under the view's Document node (egui puts
    /// new nodes under the window root; the bridge moves them).
    // wordcraft: crates/ui-egui/src/a11y.rs:87 `expose`, :138 `build`.
    pub fn show(self, ctx: &Context, describer: &mut dyn AccessDescriber) {
        let PagesAccess { view, placed, revision } = self;
        let cache_id = view.with("access cache");
        if ctx.accesskit_node_builder(view, |node| node.set_role(Role::Document)).is_none() {
            // No screen reader: describe nothing, keep nothing.
            ctx.data_mut(|d| d.remove::<Cached>(cache_id));
            return;
        }
        let visible = ctx.read_response(view).map_or_else(|| ctx.content_rect(), |response| response.rect);
        let window = page_window(placed, visible);
        let key =
            Key { revision, pages: placed[window.clone()].iter().map(|page| (page.page, page.rect)).collect(), selection: describer.selection() };
        let cached = ctx.data(|d| d.get_temp::<Cached>(cache_id)).filter(|cached| cached.key == key);
        let tree = match cached {
            Some(cached) => cached.tree,
            None => {
                let mut out = AccessBuilder::new(view);
                describer.describe(&placed[window], &mut out);
                let tree = Arc::new(out.finish(key.selection));
                ctx.data_mut(|d| d.insert_temp(cache_id, Cached { key, tree: tree.clone() }));
                tree
            }
        };
        let ours: std::collections::HashSet<NodeId> = tree.nodes.iter().map(|(id, _)| id.accesskit_id()).collect();
        for (id, node) in &tree.nodes {
            ctx.accesskit_node_builder(*id, |built| *built = node.clone());
        }
        // egui hangs every new node off the window; ours belong under the document.
        ctx.accesskit_node_builder(egui::accesskit_root_id(), |root| {
            let keep: Vec<NodeId> = root.children().iter().filter(|child| !ours.contains(child)).copied().collect();
            root.set_children(keep);
        });
        ctx.accesskit_node_builder(view, |document| {
            for child in &tree.top {
                document.push_child(*child);
            }
            match &tree.selection {
                Some(selection) => document.set_text_selection(*selection),
                None => document.clear_text_selection(),
            }
        });
    }
}

/// The pages meeting `view`, and one either side (positions in `placed`).
fn page_window(placed: &[PlacedPage], view: Rect) -> std::ops::Range<usize> {
    let Some(first) = placed.iter().position(|page| page.rect.intersects(view)) else { return 0..0 };
    let last = placed.iter().rposition(|page| page.rect.intersects(view)).unwrap_or(first);
    first.saturating_sub(1)..(last + 2).min(placed.len())
}
