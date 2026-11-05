//! A [`List`](super::List)'s row drag: what it keeps between frames, the ghost that follows the
//! pointer, and the reorder stories.

use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;

use egui::mutex::Mutex;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Context, CursorIcon, DragAndDrop, Id, LayerId, Modifiers, Order, Painter, Pos2, Rect, Stroke, StrokeKind, Ui, pos2, vec2};

use super::row::{DropMark, ListRow, ListRowStyle, RowEvent};
use super::spans::Spans;
use super::toggle::ToggleColumn;
use super::view::{List, ListEvent};
use crate::behavior::{Axis, DisplayOrder, DragEnd, DropPosition, ListDrag, Move, Reorder, ReorderGesture, Resolved, RowInfo, Selection, StackKey};
use crate::controls::button::Button;
use crate::direction;
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::localized::Labels as _;
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use crate::story::NoArgs;
use crate::story::{Level, Story, StoryLabels};
use crate::theme::{Tokens, TokensOf as _, corner_radius, role_format};
use crate::typed_story;

/// A drag of rows under way.
#[derive(Clone)]
pub(super) struct Dragging<K> {
    /// The rows dragged, in display order.
    pub(super) items: Vec<K>,
    /// The row the pointer grabbed, kept drawn while the drag lasts.
    pub(super) grabbed: K,
    pub(super) label: String,
}

/// Whether every key of `items` is a row of `rows`.
pub(super) fn all_shown<K: Eq + Hash>(rows: &(impl DisplayOrder<K> + ?Sized), items: &[K]) -> bool {
    let mut missing: HashSet<&K> = items.iter().collect();
    for at in 0..rows.len() {
        if missing.is_empty() {
            break;
        }
        missing.remove(rows.key(at));
    }
    missing.is_empty()
}

/// One pass of the row drag of a list or a table that [reorders](Reorder): the drag under way,
/// which its owner keeps between passes, the drag's edit gesture, and where it drops this pass.
///
/// The owner loads it at the start of the pass, calls [`end_for_gone_rows`](Self::end_for_gone_rows)
/// before reading input, [`start`](Self::start) when a row reports a primary drag start,
/// [`resolve`](Self::resolve) before drawing the rows (in the coordinates of their spans), draws
/// [`mark`](Self::mark) on the target row, calls [`frame`](Self::frame) after the rows, and
/// [`finish`](Self::finish)es it in the same pass.
pub(super) struct RowDrag<'p, K> {
    ctx: Context,
    /// The id of the list or table, which the payload names.
    list: Id,
    reorder: Option<&'p Reorder<'p, K>>,
    gesture: Option<ReorderGesture<K>>,
    dragging: Option<Dragging<K>>,
    target: Option<Resolved<K>>,
    /// The lists whose rows this one takes drops from.
    accepts: &'p [Id],
    /// Rows of an accepted list are held over this one.
    foreign: Option<Foreign<K>>,
    /// The drop of such rows, when they were released over this list this pass.
    dropped: Option<Dropped<K>>,
}

/// Rows of another list dragged over this one: their payload and the target under the pointer
/// (`None` over a list without rows, or where the host refuses).
struct Foreign<K> {
    payload: ListDrag<K>,
    target: Option<Resolved<K>>,
}

/// Rows of another list dropped on this one.
pub(super) struct Dropped<K> {
    pub(super) payload: ListDrag<K>,
    pub(super) target: Option<Move<K>>,
}

impl<'p, K: Clone + Eq + Hash + Send + Sync + 'static> RowDrag<'p, K> {
    /// The drag of the list `list` in this pass, with the drag its last pass kept.
    pub(super) fn load(ui: &Ui, list: Id, reorder: Option<&'p Reorder<'p, K>>, dragging: Option<Dragging<K>>) -> Self {
        let gesture = reorder.is_some().then(|| ReorderGesture::load(ui, list));
        Self { ctx: ui.ctx().clone(), list, reorder, gesture, dragging, target: None, accepts: &[], foreign: None, dropped: None }
    }

    /// The lists whose rows this list takes drops from.
    pub(super) fn accept_from(&mut self, lists: &'p [Id]) {
        self.accepts = lists;
    }

    /// Forgets a drag that ended without the list (Esc, [`cancel_all`](crate::edit::cancel_all),
    /// a pass that didn't draw it), and cancels one whose rows left `rows` with `Removed`.
    pub(super) fn end_for_gone_rows(&mut self, rows: &(impl DisplayOrder<K> + ?Sized)) {
        if !self.gesture.as_ref().is_some_and(ReorderGesture::is_active) && self.dragging.take().is_some() && self.has_payload() {
            DragAndDrop::clear_payload(&self.ctx);
        }
        if self.dragging.take_if(|drag| !all_shown(rows, &drag.items)).is_some() {
            self.end(DragEnd::Removed, Modifiers::NONE);
        }
    }

    /// Whether a row may start a drag: the list reorders and no drag is under way.
    pub(super) fn can_start(&self) -> bool {
        self.gesture.is_some() && self.dragging.is_none()
    }

    /// The row the drag under way grabbed, which stays drawn while the drag lasts (egui's drag
    /// state is its response's).
    pub(super) fn grabbed(&self) -> Option<&K> {
        self.dragging.as_ref().map(|drag| &drag.grabbed)
    }

    /// Starts a drag of the row `grabbed`, named `label`: the selection when it holds the row,
    /// else the row ([`ListDrag::new`]).
    pub(super) fn start(&mut self, rows: &(impl DisplayOrder<K> + ?Sized), selection: &[K], grabbed: K, label: String) {
        let payload = ListDrag::new(self.list, rows, selection, grabbed.clone(), label);
        let Some(gesture) = self.gesture.as_mut() else { return };
        gesture.begin();
        if !gesture.is_active() {
            return;
        }
        self.dragging = Some(Dragging { items: payload.items.clone(), grabbed, label: payload.label.clone() });
        DragAndDrop::set_payload(&self.ctx, payload);
    }

    /// Finds the target of the drag under way while the pointer is over `ui`'s clip rect (the
    /// rows' viewport); `top` is where the rows start on the screen.
    pub(super) fn resolve(&mut self, ui: &Ui, rows: &(impl DisplayOrder<K> + ?Sized), spans: &Spans, top: f32) {
        self.target = None;
        self.foreign = None;
        if self.dragging.is_none() {
            self.resolve_foreign(ui, rows, spans, top);
            return;
        }
        let (Some(drag), Some(reorder)) = (self.dragging.as_ref(), self.reorder) else { return };
        let (pointer, modifiers) = ui.input(|i| (i.pointer.hover_pos(), i.modifiers));
        let Some(pointer) = pointer.filter(|_| ui.rect_contains_pointer(ui.clip_rect())) else { return };
        self.target = reorder.resolve(rows, spans, pointer.y - top, &drag.items, modifiers);
    }

    /// Finds where rows of an accepted list held over this one drop.
    fn resolve_foreign(&mut self, ui: &Ui, rows: &(impl DisplayOrder<K> + ?Sized), spans: &Spans, top: f32) {
        let Some(payload) = DragAndDrop::payload::<ListDrag<K>>(&self.ctx).filter(|p| p.list != self.list && self.accepts.contains(&p.list)) else {
            return;
        };
        let (pointer, modifiers) = ui.input(|i| (i.pointer.hover_pos(), i.modifiers));
        let Some(pointer) = pointer.filter(|_| ui.rect_contains_pointer(ui.clip_rect())) else { return };
        let plain = Reorder::new(Axis::Vertical);
        let reorder = self.reorder.unwrap_or(&plain);
        let target = reorder.resolve_foreign(rows, spans, pointer.y - top, &payload.items, modifiers);
        self.foreign = Some(Foreign { payload: (*payload).clone(), target });
    }

    /// The position in the display order of the row the drag drops on this pass.
    pub(super) fn target_at(&self) -> Option<usize> {
        self.resolved().map(|t| t.at)
    }

    /// The marker of the row at `at`, when the drag drops on it.
    pub(super) fn mark(&self, at: usize) -> Option<DropMark> {
        self.resolved().filter(|t| t.at == at).map(|t| drop_mark(t.mv.position))
    }

    /// The target of the drag of this list, or of rows held over it from an accepted list.
    fn resolved(&self) -> Option<&Resolved<K>> {
        self.target.as_ref().or_else(|| self.foreign.as_ref().and_then(|f| f.target.as_ref()))
    }

    /// The drop of rows from an accepted list that was released over this list this pass.
    pub(super) fn take_dropped(&mut self) -> Option<Dropped<K>> {
        self.dropped.take()
    }

    /// The scroll offset while the drag is held near the top or bottom edge of `view`, the rows'
    /// viewport on the screen, scrolled to `offset`.
    pub(super) fn scroll(&self, spans: &Spans, offset: f32, view: Option<Rect>) -> Option<f32> {
        let (reorder, view) = (self.reorder?, view?);
        self.dragging.as_ref()?;
        let pointer = self.ctx.pointer_latest_pos()?;
        let delta = reorder.auto_scroll(pointer, view, self.ctx.input(|i| i.stable_dt));
        let to = (offset + delta).clamp(0.0, (spans.total() - view.height()).max(0.0));
        if to == offset {
            return None;
        }
        self.ctx.request_repaint();
        Some(to)
    }

    /// Whether the list reorders: its rows sense drags.
    pub(super) fn reorders(&self) -> bool {
        self.gesture.is_some()
    }

    /// A stacking key on the `selected` rows of `rows`, while no drag is under way: reports the
    /// move as a `Set`. Returns whether there was one.
    pub(super) fn stack(&mut self, rows: &(impl DisplayOrder<K> + ?Sized), selected: &[K], key: StackKey, modifiers: Modifiers) -> bool {
        let Some(mv) = self.reorder.and_then(|reorder| reorder.key_move(rows, selected, key, modifiers)) else { return false };
        let Some(gesture) = self.gesture.as_mut().filter(|g| !g.is_active()) else { return false };
        gesture.set(mv);
        true
    }

    /// Ends the drag under way when it was released, Esc was pressed or the window lost the
    /// focus; otherwise draws the ghost (`width` wide at most) and the copy cursor.
    pub(super) fn frame(&mut self, ui: &Ui, style: &ListRowStyle, width: f32) {
        let Some(drag) = self.dragging.take() else {
            self.drop_foreign(ui);
            return;
        };
        let (released, down, modifiers, pointer) =
            ui.input(|i| (i.pointer.any_released(), i.pointer.any_down(), i.modifiers, i.pointer.latest_pos()));
        let end = if crate::focus::lost(&self.ctx) {
            Some(DragEnd::FocusLost)
        } else if released || !down {
            Some(DragEnd::Released(self.target.as_ref().map(|t| t.mv.clone())))
        } else if !self.has_payload() {
            // egui's drag and drop takes Esc and drops the payload before the gesture sees the key.
            Some(if DragAndDrop::aborted_by_escape(&self.ctx) { DragEnd::Escape } else { DragEnd::PayloadCleared })
        } else {
            None
        };
        if let Some(end) = end {
            self.end(end, modifiers);
            return;
        }
        if self.reorder.is_some_and(|reorder| reorder.copies(modifiers)) {
            self.ctx.set_cursor_icon(CursorIcon::Copy);
        }
        if let Some(pointer) = pointer {
            paint_ghost(ui, self.list, style, &drag.label, pointer, width, None);
        }
        self.dragging = Some(drag);
    }

    /// Takes the rows of an accepted list when the pointer was released over this list: the
    /// payload is cleared so no other widget takes it. The source list ends its own drag in the
    /// same pass, whichever list is drawn first, as a cancel (its own order is unchanged).
    fn drop_foreign(&mut self, ui: &Ui) {
        let Some(foreign) = self.foreign.take() else { return };
        let (released, down) = ui.input(|i| (i.pointer.any_released(), i.pointer.any_down()));
        if !(released || !down) {
            self.foreign = Some(foreign);
            return;
        }
        DragAndDrop::clear_payload(&self.ctx);
        self.dropped = Some(Dropped { payload: foreign.payload, target: foreign.target.map(|t| t.mv) });
    }

    /// What the pass leaves.
    pub(super) fn finish(self) -> DragOutcome<K> {
        let drop_target = self.dragging.as_ref().and(self.target).or_else(|| self.foreign.and_then(|f| f.target)).map(|t| t.mv);
        DragOutcome { dragging: self.dragging, event: self.gesture.and_then(ReorderGesture::finish), drop_target }
    }

    /// Whether egui's drag and drop holds this list's rows.
    fn has_payload(&self) -> bool {
        DragAndDrop::payload::<ListDrag<K>>(&self.ctx).is_some_and(|payload| payload.list == self.list)
    }

    /// Ends the drag. A drop on the list or a cancel clears the payload, so no other widget takes
    /// it; a release elsewhere leaves it to the widget under the pointer.
    fn end(&mut self, end: DragEnd<K>, modifiers: Modifiers) {
        if !matches!(end, DragEnd::Released(None)) && self.has_payload() {
            DragAndDrop::clear_payload(&self.ctx);
        }
        if let Some(gesture) = self.gesture.as_mut() {
            gesture.end(end, modifiers);
        }
    }
}

/// What a pass of a [`RowDrag`] leaves.
pub(super) struct DragOutcome<K> {
    /// The drag to keep for the next pass.
    pub(super) dragging: Option<Dragging<K>>,
    /// The pass's reorder event.
    pub(super) event: Option<EditEvent<Move<K>>>,
    /// Where a drag under way drops this pass.
    pub(super) drop_target: Option<Move<K>>,
}

/// The marker of a drop at `position` of its target row.
pub(super) fn drop_mark(position: DropPosition) -> DropMark {
    match position {
        DropPosition::Before => DropMark::Before,
        DropPosition::After => DropMark::After,
        DropPosition::Into => DropMark::Into,
    }
}

/// Paints the thumbnail of a ghost into the rect, a square of the given side.
pub type GhostThumb<'a> = (f32, &'a dyn Fn(&Painter, Rect));

/// The ghost: the grabbed row's label in a box beside the pointer on the tooltip layer, cut to
/// `max_width`; with `thumb` (its side and painter), a thumbnail before the label.
///
/// A [`List`](super::List) paints its own. A host that builds its panel from the list parts calls
/// this once per pass while its drag is under way, with the list's `list` id, the row `style`
/// ([`ListRow::style`]), the dragged row's `label` and the latest `pointer` position.
pub fn paint_ghost(ui: &Ui, list: Id, style: &ListRowStyle, label: &str, pointer: Pos2, max_width: f32, thumb: Option<GhostThumb<'_>>) {
    let tokens = Tokens::of(ui);
    let pad = vec2(style.pad_x, style.gap);
    let mut job = LayoutJob::single_section(label.to_owned(), role_format(&tokens, style.label_role, style.text));
    let side = thumb.map_or(0.0, |(side, _)| side);
    let lead = if side > 0.0 { side + style.gap } else { 0.0 };
    job.wrap = TextWrapping::truncate_at_width((max_width - 2.0 * pad.x - lead).max(style.min_label));
    let painter = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, list.with("drag-ghost")));
    let galley = painter.layout_job(job);
    let size = vec2(galley.size().x + lead, galley.size().y.max(side)) + 2.0 * pad;
    let rtl = direction::is_rtl(ui);
    let left = if rtl { pointer.x - style.ghost_offset.x - size.x } else { pointer.x + style.ghost_offset.x };
    let rect = Rect::from_min_size(pos2(left, pointer.y + style.ghost_offset.y), size);
    painter.rect(rect, corner_radius(style.ghost_radius), style.ghost_fill, Stroke::new(1.0, style.ghost_border), StrokeKind::Inside);
    let inner = rect.shrink2(pad);
    let text_x = if rtl { inner.right() - lead - galley.size().x } else { inner.left() + lead };
    if let Some((side, paint)) = thumb.filter(|(side, _)| *side > 0.0) {
        let x = if rtl { inner.right() - side } else { inner.left() };
        let at = Rect::from_min_size(pos2(x, inner.center().y - side / 2.0), vec2(side, side));
        paint(&painter.with_clip_rect(at), at);
    }
    painter.galley(pos2(text_x, inner.center().y - galley.size().y / 2.0), galley, style.text);
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/list/reorder", Level::Composites, "List", "Reorder", NoArgs {}, show_flat),
        typed_story!("composites/list/reorder-tree", Level::Composites, "List", "Reorder a tree", NoArgs {}, show_tree),
        typed_story!("composites/list/reorder-refused", Level::Composites, "List", "Refused targets", NoArgs {}, show_refused),
        typed_story!("composites/list/reorder-copy", Level::Composites, "List", "Copy with Alt", NoArgs {}, show_copy),
    ]
}

/// Drag a layer or the selection; the edges scroll; Delete takes a dragged layer. Cmd+] and
/// Cmd+[ (Ctrl off macOS) move the selection, Shift to the ends.
pub(super) fn show_flat(ui: &mut Ui, _: &mut NoArgs) {
    show_story(ui, Preset::Flat);
}

/// Groups take drops into their middle; their triangles and → and ← open and close them.
pub(super) fn show_tree(ui: &mut Ui, _: &mut NoArgs) {
    show_story(ui, Preset::Tree);
}

/// Background stays at the bottom; locked layers take no drops beside or into them unless
/// Cmd/Ctrl is held.
pub(super) fn show_refused(ui: &mut Ui, _: &mut NoArgs) {
    show_story(ui, Preset::Refused);
}

/// Alt held while dropping copies the layers, also onto their own edges.
pub(super) fn show_copy(ui: &mut Ui, _: &mut NoArgs) {
    show_story(ui, Preset::Copy);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Preset {
    Flat,
    Tree,
    Refused,
    Copy,
}

/// A layer of a story's document, in the document's order: a group directly above its children.
#[derive(Clone)]
struct Layer {
    key: usize,
    name: String,
    depth: usize,
    group: bool,
    open: bool,
    locked: bool,
}

/// A story's document, selection and last event.
struct Layers {
    layers: Vec<Layer>,
    next: usize,
    background: Option<usize>,
    selection: Selection<usize>,
    last: String,
    /// The layers the open context menu acts on.
    menu_rows: Vec<usize>,
    /// A rename the menu asked for, taken by the next list.
    rename: Option<usize>,
    /// A drag of layers is under way.
    dragging: bool,
}

crate::words! {
    /// The sample text of the reorder stories, English by default.
    #[non_exhaustive]
    pub struct DragStoryLabels {
        delete: "dragStory.delete" = "Delete";
    }
}

impl Layers {
    fn new(preset: Preset, words: &StoryLabels) -> Self {
        let flat: Vec<(String, usize, bool)> = (1..=24).map(|n| (format!("Layer {n}"), 0, false)).collect();
        let tree = [
            ("Title", 0, false),
            ("Header", 0, true),
            ("Logo", 1, false),
            ("Tagline", 1, false),
            ("Photos", 0, true),
            ("Beach", 1, false),
            ("Archive", 1, true),
            ("Harbour", 2, false),
            ("Market", 2, false),
            ("Caption", 0, false),
        ];
        let rows = match preset {
            Preset::Flat => flat,
            Preset::Copy => flat.into_iter().take(8).collect(),
            Preset::Tree => tree.iter().map(|&(name, depth, group)| (name.to_owned(), depth, group)).collect(),
            Preset::Refused => [words.text.as_str(), "Frame", words.photo.as_str(), "Badge", "Shape", "Background"]
                .iter()
                .map(|n| ((*n).to_owned(), 0, false))
                .collect(),
        };
        let layers: Vec<Layer> = rows
            .into_iter()
            .enumerate()
            .map(|(key, (name, depth, group))| Layer {
                locked: preset == Preset::Refused && matches!(name.as_str(), "Frame" | "Badge"),
                open: name != "Archive",
                key,
                name,
                depth,
                group,
            })
            .collect();
        let background = (preset == Preset::Refused).then(|| layers.len() - 1);
        let last = match preset {
            Preset::Flat => "Drag a layer, or press Cmd+] or Cmd+[",
            Preset::Tree => "Drop into the middle of a group",
            Preset::Refused => "Background and locked layers refuse drops",
            Preset::Copy => "Hold Alt while dropping to copy",
        };
        Layers {
            next: layers.len(),
            layers,
            background,
            selection: Selection::only(0),
            last: last.to_owned(),
            menu_rows: Vec::new(),
            rename: None,
            dragging: false,
        }
    }

    fn at(&self, key: usize) -> Option<usize> {
        self.layers.iter().position(|l| l.key == key)
    }

    fn name(&self, key: usize) -> &str {
        self.at(key).map_or("?", |at| self.layers[at].name.as_str())
    }

    /// The layer at `at` and the layers inside it.
    fn block(&self, at: usize) -> std::ops::Range<usize> {
        let depth = self.layers[at].depth;
        let end = self.layers[at + 1..].iter().position(|l| l.depth <= depth).map_or(self.layers.len(), |n| at + 1 + n);
        at..end
    }

    /// The layers shown: closed groups' children left out.
    fn display(&self) -> Vec<RowInfo<usize>> {
        let mut closed_at: Option<usize> = None;
        let mut rows = Vec::new();
        for layer in &self.layers {
            if closed_at.is_some_and(|depth| layer.depth > depth) {
                continue;
            }
            closed_at = (layer.group && !layer.open).then_some(layer.depth);
            let row = if layer.group { RowInfo::group(layer.key, layer.open) } else { RowInfo::leaf(layer.key) };
            rows.push(row.at_depth(layer.depth));
        }
        rows
    }

    /// The blocks of `keys`, without those inside another one, in document order.
    fn blocks(&self, keys: &[usize]) -> Vec<std::ops::Range<usize>> {
        let mut blocks: Vec<std::ops::Range<usize>> = Vec::new();
        let mut starts: Vec<usize> = keys.iter().filter_map(|&k| self.at(k)).collect();
        starts.sort_unstable();
        for at in starts {
            if !blocks.iter().any(|b| b.contains(&at)) {
                blocks.push(self.block(at));
            }
        }
        blocks
    }

    /// Removes the layers of `keys` with their contents, and returns them as blocks in document
    /// order.
    fn take(&mut self, keys: &[usize]) -> Vec<Vec<Layer>> {
        let mut taken: Vec<Vec<Layer>> = self.blocks(keys).into_iter().rev().map(|block| self.layers.drain(block).collect()).collect();
        taken.reverse();
        taken
    }

    fn apply(&mut self, mv: &Move<usize>) {
        let Some(target) = self.at(mv.target) else { return };
        // The slot as the document stands, found before the rows leave it: the target may be one
        // of them (rows apart gather at one of their edges).
        let (slot, depth) = match mv.position {
            DropPosition::Before => (target, self.layers[target].depth),
            DropPosition::After => (self.block(target).end, self.layers[target].depth),
            DropPosition::Into => {
                self.layers[target].open = true;
                (target + 1, self.layers[target].depth + 1)
            }
        };
        let blocks = self.blocks(&mv.items);
        let (moved, insert): (Vec<Vec<Layer>>, usize) = if mv.copy {
            let copies = blocks
                .into_iter()
                .map(|block| {
                    self.layers[block]
                        .iter()
                        .cloned()
                        .map(|mut layer| {
                            layer.key = self.next;
                            self.next += 1;
                            layer.name.push_str(" copy");
                            layer
                        })
                        .collect()
                })
                .collect();
            (copies, slot)
        } else {
            let above: usize = blocks.iter().map(|b| b.end.min(slot).saturating_sub(b.start)).sum();
            (self.take(&mv.items), slot - above)
        };
        // Each block's top layer lands at `depth`; the layers inside it keep their depth below it.
        let moved = moved.into_iter().flat_map(|block| {
            let top = block.first().map_or(0, |l| l.depth);
            block.into_iter().map(move |mut layer| {
                layer.depth = layer.depth - top + depth;
                layer
            })
        });
        self.layers.splice(insert..insert, moved.collect::<Vec<_>>());
    }

    /// Copies each block of `keys` right below itself.
    fn duplicate(&mut self, keys: &[usize]) {
        for block in self.blocks(keys).into_iter().rev() {
            let copies: Vec<Layer> = self.layers[block.clone()]
                .iter()
                .map(|layer| {
                    let key = self.next;
                    self.next += 1;
                    Layer { key, name: format!("{} copy", layer.name), ..layer.clone() }
                })
                .collect();
            self.layers.splice(block.end..block.end, copies);
        }
    }

    fn describe(&self, mv: &Move<usize>) -> String {
        let names: Vec<&str> = mv.items.iter().map(|&k| self.name(k)).collect();
        let position = match mv.position {
            DropPosition::Before => "before",
            DropPosition::After => "after",
            DropPosition::Into => "into",
        };
        let verb = if mv.copy { "copy" } else { "move" };
        format!("{verb} {} {position} {}", names.join(", "), self.name(mv.target))
    }

    /// The rules of the refused-targets story: Background doesn't move and nothing goes below
    /// it (PhotoCraft); a locked layer takes no drop beside or into it unless Cmd/Ctrl is held
    /// (DesignCraft).
    fn rules(&self) -> impl Fn(&Move<usize>, egui::Modifiers) -> bool + 'static {
        let background = self.background;
        let locked: HashSet<usize> = self.layers.iter().filter(|l| l.locked).map(|l| l.key).collect();
        move |mv, modifiers| {
            let background_ok = background.is_none_or(|bg| !mv.items.contains(&bg) && !(mv.target == bg && mv.position != DropPosition::Before));
            background_ok && (modifiers.command || !locked.contains(&mv.target))
        }
    }
}

/// What the Reorder story's context menu does to the layers it was opened on.
#[derive(Clone, Copy, Debug, PartialEq)]
enum LayerCommand {
    Rename,
    Duplicate,
    Lock,
    Delete,
}

fn show_story(ui: &mut Ui, preset: Preset) {
    let state_id = ui.scope_id().with(("reorder-story", preset));
    let words = StoryLabels::of(ui);
    let shared = ui.data_mut(|d| d.get_temp_mut_or_insert_with(state_id, || Arc::new(Mutex::new(Layers::new(preset, &words)))).clone());
    let mut doc = shared.lock();
    let order = doc.display();
    let mut reorder = Reorder::new(Axis::Vertical).copy_with_alt(preset == Preset::Copy);
    if preset == Preset::Refused {
        reorder = reorder.accepts(doc.rules());
    }
    let mut selection = doc.selection.clone();
    let rename = doc.rename.take();
    let (response, events, drop_target) = ui
        .allocate_ui(vec2(300.0, 240.0), |ui| {
            let mut list = List::new("layers", "Layers", order.as_slice())
                .selection(&mut selection)
                .reorder(reorder)
                .renamable(preset == Preset::Flat)
                .max_height(240.0);
            if let Some(key) = rename {
                list = list.start_rename(key);
            }
            let out = list.tree(preset == Preset::Tree).show(ui, |_, &key| {
                let layer = doc.at(key).map(|at| &doc.layers[at]);
                let (name, group, locked) = layer.map_or(("", false, false), |l| (l.name.as_str(), l.group, l.locked));
                let mut row = ListRow::new(key, name).layer(true);
                if group {
                    row = row.icon(Icon::FolderOpen);
                }
                if preset == Preset::Refused {
                    row = row.toggle(ToggleColumn::lock("Lock {row}", locked));
                }
                row
            });
            (out.response, out.events, out.drop_target)
        })
        .inner;
    doc.selection = selection;
    let dragging = events.iter().fold(doc.dragging, |dragging, event| match event {
        ListEvent::Reorder(EditEvent::Begin { .. }) => true,
        ListEvent::Reorder(EditEvent::Commit { .. } | EditEvent::Cancel { .. }) => false,
        _ => dragging,
    });
    if dragging {
        doc.last = drop_target.map_or_else(|| "No drop here".to_owned(), |mv| format!("Drop to {}", doc.describe(&mv)));
    }
    doc.dragging = dragging;
    let mut open_menu = None;
    for event in events {
        match event {
            ListEvent::ContextMenu(request) if preset == Preset::Flat => {
                doc.menu_rows = match request.row {
                    Some(_) if request.selected => doc.selection.items.clone(),
                    Some(row) => vec![row],
                    None => Vec::new(),
                };
                open_menu = Some(request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer));
            }
            ListEvent::Row(RowEvent::Renamed { row, text }) if !text.is_empty() => {
                if let Some(at) = doc.at(row) {
                    doc.layers[at].name = text;
                }
            }
            ListEvent::Reorder(EditEvent::Commit { value: mv, .. } | EditEvent::Set { value: mv, .. }) => {
                doc.last = format!("Did: {}", doc.describe(&mv));
                doc.apply(&mv);
            }
            ListEvent::Reorder(EditEvent::Cancel { reason, .. }) => doc.last = format!("Drag cancelled: {reason:?}"),
            ListEvent::Row(RowEvent::Toggle(EditEvent::Update { value, .. } | EditEvent::Set { value, .. })) => {
                if let Some(at) = doc.at(value.row) {
                    doc.layers[at].locked = value.on;
                }
            }
            ListEvent::Open(change) => {
                for layer in doc.layers.iter_mut().filter(|l| l.group && (change.all || l.key == change.row)) {
                    layer.open = change.open;
                }
            }
            _ => {}
        }
    }
    if preset == Preset::Flat {
        let rows = doc.menu_rows.clone();
        let locked = !rows.is_empty() && rows.iter().all(|&k| doc.at(k).is_some_and(|at| doc.layers[at].locked));
        let entries: Vec<MenuEntry<LayerCommand>> = vec![
            MenuItem::new("Rename", LayerCommand::Rename).enabled(rows.len() == 1).into(),
            MenuItem::new("Duplicate", LayerCommand::Duplicate).enabled(!rows.is_empty()).into(),
            MenuItem::new("Locked", LayerCommand::Lock).check(locked).enabled(!rows.is_empty()).into(),
            MenuEntry::Separator,
            MenuItem::new("Delete", LayerCommand::Delete).icon(Icon::Trash).enabled(!rows.is_empty()).into(),
        ];
        let menu = ContextMenu::new(&entries).open_at(open_menu).own_gestures(false).show(ui, &response);
        if let Some(command) = menu.chosen {
            match command {
                LayerCommand::Rename => doc.rename = rows.first().copied(),
                LayerCommand::Duplicate => {
                    doc.duplicate(&rows);
                    doc.last = format!("Duplicated {} layers", rows.len());
                }
                LayerCommand::Lock => {
                    for &key in &rows {
                        if let Some(at) = doc.at(key) {
                            doc.layers[at].locked = !locked;
                        }
                    }
                }
                LayerCommand::Delete => {
                    doc.take(&rows);
                    doc.selection = Selection::default();
                    doc.last = format!("Deleted {} layers", rows.len());
                }
            }
        }
        let response = ui.add(Button::new(&DragStoryLabels::of(ui).delete).icon(Icon::Trash));
        if let Some(drag) = response.dnd_release_payload::<ListDrag<usize>>() {
            doc.take(&drag.items);
            doc.selection = Selection::default();
            doc.last = format!("Deleted {} layers", drag.items.len());
        }
    }
    ui.label(doc.last.as_str());
}
