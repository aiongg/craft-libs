//! Tree rows: [`TreeRow`] (a [`ListRow`] with an indent and a disclosure triangle), the open
//! state a host without a model keeps in [`TreeState`], and what a tree reports as an
//! [`OpenChange`].

use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use egui::accesskit::Action;
use egui::{AsIdSalt, Context, Id, Rect, Response, Shape, Stroke, Ui, pos2};

use super::row::{ListRow, ListRowStyle, RowEvent, RowResponse};
use crate::behavior::DisplayOrder;
use crate::direction;
use crate::primitives::Interactive;

/// A group row opened or closed: by its triangle, the arrow keys, or a screen reader's Expand and
/// Collapse. `all` (Alt held) asks for every group, not only this one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenChange<K> {
    /// The row asked.
    pub row: K,
    /// The state wanted: true opens, false closes.
    pub open: bool,
    /// Every group takes the state (PhotoCraft's Alt-click).
    pub all: bool,
}

/// The words of a tree's disclosure triangle, for a host to translate: its name for a screen
/// reader is "{expand} {row}" or "{collapse} {row}", and its tooltip is that verb followed by
/// the hint in parentheses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeText {
    /// The verb for a closed group.
    pub expand: String,
    /// The verb for an open group.
    pub collapse: String,
    /// What Alt does, as PhotoCraft's tooltip says it ("Alt-click: all groups").
    pub all_hint: String,
}

impl Default for TreeText {
    fn default() -> Self {
        Self { expand: "Expand".to_owned(), collapse: "Collapse".to_owned(), all_hint: "Alt-click: all groups".to_owned() }
    }
}

/// Where a row sits in a tree, as the display order says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Branch {
    pub depth: usize,
    pub group: bool,
    pub open: bool,
}

impl Branch {
    /// Levels of indent. A leaf inside a group indents one more level, past its parent's
    /// triangle column, so it sits right of its parent; top-level rows don't move.
    pub fn steps(self) -> usize {
        if !self.group && self.depth > 0 { self.depth + 1 } else { self.depth }
    }
}

/// A row of a tree: a [`ListRow`] indented by `depth` levels (`list.indent` each, squeezed before
/// the label gets narrower than `list.min_label`), with a disclosure triangle in the
/// `list.disclosure` column when it has children.
///
/// A [`List`](super::List) in [tree mode](super::List::tree) wraps its rows this way itself; a
/// `TreeRow` is for rows shown outside a list.
///
/// - **Disclosure.** A filled triangle, down when open and toward the reading end when closed;
///   its hit area is the column's width by the row's height, and a click on it doesn't select the
///   row. It reports [`RowEvent::Open`], with `all` while Alt is held.
/// - **AccessKit.** The row is a `TreeItem` with `level` (its depth, from zero) and, for a group,
///   `expanded` and the `Expand` or `Collapse` action, which report [`RowEvent::Open`] too.
#[must_use = "show the row with `show`"]
pub struct TreeRow<'a, K> {
    row: ListRow<'a, K>,
    branch: Branch,
}

impl<'a, K: Hash + Debug + Clone> TreeRow<'a, K> {
    /// `row` at `depth` levels down; `has_children` shows the triangle, `open` its state.
    pub fn new(row: ListRow<'a, K>, depth: usize, has_children: bool, open: bool) -> Self {
        Self { row, branch: Branch { depth, group: has_children, open: has_children && open } }
    }

    /// The row with its tree look, to hand a list that builds [`ListRow`]s.
    pub fn into_row(self) -> ListRow<'a, K> {
        self.row.branch(self.branch)
    }

    /// The words of the disclosure triangle, translated by the host.
    pub fn text(mut self, text: TreeText) -> Self {
        self.row = self.row.tree_text(Arc::new(text));
        self
    }

    /// Shows the row across the available width.
    pub fn show(self, ui: &mut Ui) -> RowResponse<K> {
        self.into_row().show(ui)
    }
}

/// Sets `level`, `expanded`'s actions and takes the screen reader's Expand and Collapse requests
/// of the row whose response is `response`.
pub(super) fn report<K: Clone>(ui: &Ui, response: &Response, branch: &Branch, key: &K, events: &mut Vec<RowEvent<K>>) {
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_level(branch.depth);
        if branch.group {
            node.add_action(if branch.open { Action::Collapse } else { Action::Expand });
        }
    });
    if branch.group {
        let want = if ui.input(|i| i.has_accesskit_action_request(response.id, Action::Expand)) {
            Some(true)
        } else if ui.input(|i| i.has_accesskit_action_request(response.id, Action::Collapse)) {
            Some(false)
        } else {
            None
        };
        if let Some(open) = want.filter(|&open| open != branch.open) {
            events.push(RowEvent::Open(OpenChange { row: key.clone(), open, all: false }));
        }
    }
}

/// Senses and paints the disclosure triangle in `rect` (the column's width, the row's height).
#[expect(clippy::too_many_arguments, reason = "the row's pieces; a struct would only carry them")]
pub(super) fn show_disclosure<K: Clone>(
    ui: &mut Ui,
    style: &ListRowStyle,
    rect: Rect,
    branch: &Branch,
    text: &TreeText,
    row_id: Id,
    key: &K,
    label: &str,
    events: &mut Vec<RowEvent<K>>,
) {
    let verb = if branch.open { &text.collapse } else { &text.expand };
    let name = format!("{verb} {label}");
    let sensed = Interactive::with_id(row_id.with("disclosure"), egui::accesskit::Role::Button, name)
        .sense(egui::Sense::CLICK)
        .expanded(branch.open)
        .state_layer(false)
        .focus_ring(false)
        .interact_at(ui, rect);
    let response = sensed.finish(ui);
    let response = response.on_hover_text(format!("{verb}  ({})", text.all_hint));
    if response.clicked() {
        let all = ui.input(|i| i.modifiers.alt);
        events.push(RowEvent::Open(OpenChange { row: key.clone(), open: !branch.open, all }));
    }
    if !ui.is_rect_visible(rect) {
        return;
    }
    let color = if response.hovered() { style.icon_hover } else { style.icon };
    let half = 3.5 * style.disclosure / 14.0;
    let (c, s) = (rect.center(), half);
    let flip = if direction::is_rtl(ui) { -1.0 } else { 1.0 };
    let points = if branch.open {
        vec![pos2(c.x - s, c.y - s * 0.6), pos2(c.x + s, c.y - s * 0.6), pos2(c.x, c.y + s * 0.6)]
    } else {
        vec![pos2(c.x - flip * s * 0.6, c.y - s), pos2(c.x + flip * s * 0.6, c.y), pos2(c.x - flip * s * 0.6, c.y + s)]
    };
    ui.painter().add(Shape::convex_polygon(points, color, Stroke::NONE));
}

/// The open groups of a tree, for a host without a model of its own: kept in egui memory under
/// the tree's id and persisted with it. Rows are identified by the hash of their key.
///
/// ```
/// use craft_ui::list::{OpenChange, TreeState};
/// # egui::__run_test_ui(|ui| {
/// let mut open = TreeState::load(ui.ctx(), "files");
/// open.set(&"src", true);
/// assert!(open.is_open(&"src"));
/// open.apply(&OpenChange { row: "src", open: false, all: false }, ["src", "docs"].iter());
/// assert!(!open.is_open(&"src"));
/// open.store(ui.ctx());
/// # });
/// ```
#[derive(Clone, Debug)]
pub struct TreeState {
    id: Id,
    open: HashSet<Id>,
}

impl TreeState {
    /// The state kept under `id_salt`; every group is closed the first time.
    pub fn load(ctx: &Context, id_salt: impl AsIdSalt) -> Self {
        let id = Id::NULL.with(("craft_ui::tree-state", id_salt));
        let open = ctx.data_mut(|d| d.get_persisted::<HashSet<Id>>(id)).unwrap_or_default();
        Self { id, open }
    }

    /// Keeps the state in egui memory.
    pub fn store(&self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_persisted(self.id, self.open.clone()));
    }

    /// Whether the group `key` is open.
    pub fn is_open<K: Hash + Debug>(&self, key: &K) -> bool {
        self.open.contains(&Id::NULL.with(key))
    }

    /// Opens or closes the group `key`.
    pub fn set<K: Hash + Debug>(&mut self, key: &K, open: bool) {
        if open {
            self.open.insert(Id::NULL.with(key));
        } else {
            self.open.remove(&Id::NULL.with(key));
        }
    }

    /// Applies what a list reported: the row, or with `all` every group in `groups`.
    pub fn apply<'k, K: Hash + Debug + 'k>(&mut self, change: &OpenChange<K>, groups: impl IntoIterator<Item = &'k K>) {
        if change.all {
            for group in groups {
                self.set(group, change.open);
            }
        }
        self.set(&change.row, change.open);
    }
}

/// The position of the topmost ancestor of the row at `at`, or `at` itself.
pub(super) fn topmost<K>(order: &(impl DisplayOrder<K> + ?Sized), at: usize) -> usize {
    let mut top = at;
    while let Some(parent) = crate::behavior::parent(order, top) {
        top = parent;
    }
    top
}

/// Whether the row at `inner` is below the group at `outer` in the display order.
pub(super) fn is_below<K>(order: &(impl DisplayOrder<K> + ?Sized), outer: usize, inner: usize) -> bool {
    inner > outer && (outer + 1..=inner).all(|at| order.depth(at) > order.depth(outer))
}

/// The story's tree: a name and a depth, a group being a row followed by a deeper one.
pub(super) const NODES: &[(&str, usize)] = &[
    ("Poster", 0),
    ("Background", 1),
    ("Photos", 1),
    ("Hero photograph with a name too long for a narrow panel", 2),
    ("Detail", 2),
    ("Frames", 2),
    ("Thin frame", 3),
    ("Wide frame", 3),
    ("Text", 1),
    ("Headline", 2),
    ("Body", 2),
    ("Guides", 0),
    ("Margins", 1),
    ("Columns", 1),
    ("Notes", 0),
];

/// Whether the node `at` of `nodes` has children.
pub(super) fn is_group(nodes: &[(&str, usize)], at: usize) -> bool {
    nodes.get(at + 1).is_some_and(|next| next.1 > nodes[at].1)
}

/// The rows shown with the groups of `open` open: a group directly above its children, closed
/// groups' children left out. The key of a row is its index in `nodes`.
pub(super) fn display(nodes: &[(&str, usize)], open: &TreeState) -> Vec<crate::behavior::RowInfo<usize>> {
    let mut rows = Vec::new();
    let mut hidden_below: Option<usize> = None;
    for (at, &(_, depth)) in nodes.iter().enumerate() {
        if hidden_below.is_some_and(|d| depth > d) {
            continue;
        }
        hidden_below = None;
        let group = is_group(nodes, at);
        let is_open = group && open.is_open(&at);
        if group && !is_open {
            hidden_below = Some(depth);
        }
        rows.push(crate::behavior::RowInfo { key: at, depth, has_children: group, open: is_open });
    }
    rows
}

pub(crate) fn stories() -> Vec<crate::story::Story> {
    use crate::story::{Args, Level, Story};
    vec![Story {
        id: "composites/list/tree",
        level: Level::Composites,
        component: "TreeRow",
        name: "Layer groups",
        args: || Args::new().with_number("width", 320.0, 120.0, 480.0).with_number("height", 280.0, 120.0, 480.0),
        show: show_tree,
    }]
}

/// A layers tree: ↑ and ↓ move through the rows shown, → opens a closed group or enters an open
/// one, ← closes an open group or leaves it, Alt+→ and Alt+← open or close every group, and
/// Alt-click on a triangle does the same. The open state is a [`TreeState`].
pub(super) fn show_tree(ui: &mut Ui, args: &mut crate::story::Args) {
    use super::{List, ListEvent};
    use crate::behavior::Selection;
    use crate::icons::Icon;
    let (width, height) = (args.number("width") as f32, args.number("height") as f32);
    let state_id = ui.scope_id().with("tree-story");
    let mut selection = ui.data(|d| d.get_temp::<Selection<usize>>(state_id)).unwrap_or_else(|| Selection::only(0));
    let mut open = TreeState::load(ui.ctx(), state_id);
    if !ui.data(|d| d.get_temp::<bool>(state_id.with("seeded"))).unwrap_or(false) {
        open.set(&0, true);
        open.set(&2, true);
        ui.data_mut(|d| d.insert_temp(state_id.with("seeded"), true));
    }
    let rows = display(NODES, &open);
    ui.allocate_ui(egui::vec2(width, height), |ui| {
        let out = List::new("tree", "Layers", rows.as_slice())
            .tree(true)
            .selection(&mut selection)
            .max_height(height)
            .show(ui, |_, &key| ListRow::new(key, NODES[key].0).icon(if is_group(NODES, key) { Icon::Layers } else { Icon::Check }));
        for event in out.events {
            if let ListEvent::Open(change) = event {
                let groups: Vec<usize> = (0..NODES.len()).filter(|&at| is_group(NODES, at)).collect();
                open.apply(&change, groups.iter());
            }
        }
    });
    open.store(ui.ctx());
    ui.data_mut(|d| d.insert_temp(state_id, selection));
}
