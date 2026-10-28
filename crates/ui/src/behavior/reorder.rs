//! [`Reorder`]: drag-to-reorder rules for a list or a strip of items, without drawing. The list
//! reads the pointer, draws the marker and the ghost, and feeds the rules.

use std::ops::Range;

use egui::{Id, Key, Modifiers, Pos2, Rect, Ui};

use super::list_selection::{DisplayOrder, first_child, parent};
use crate::edit::{CancelReason, EditEvent, EditPolicy, Gesture, GestureId};

/// The axis a list runs along.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    /// Rows stacked top to bottom.
    #[default]
    Vertical,
    /// Items side by side, in the reading direction (a tab strip).
    Horizontal,
}

/// Where a drop lands relative to its target row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropPosition {
    /// Before the row, in reading order.
    Before,
    /// After the row, in reading order.
    After,
    /// Into the row, which is a group: as its first children (PhotoCraft).
    Into,
}

/// A reorder or copy: the rows `items` go to `position` of the row `target`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Move<K> {
    /// The rows moved, in display order, each with the rows inside it: a row inside another
    /// moved row is not listed.
    pub items: Vec<K>,
    /// The row the drop is relative to. It may be one of `items`: rows apart gather at the edge
    /// of one of them, so a host finds the slot before it takes the rows out.
    pub target: K,
    /// Where the drop lands relative to `target`.
    pub position: DropPosition,
    /// Whether the rows are copied rather than moved.
    pub copy: bool,
}

/// The drag payload of a list's rows (egui's `DragAndDrop`), so other widgets can take drops
/// from it: footer buttons, a canvas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListDrag<K> {
    /// The list the rows come from.
    pub list: Id,
    /// The rows dragged, in display order: the selection when the grabbed row is in it, else the
    /// grabbed row. A row inside another dragged row is not listed; it goes with that row.
    pub items: Vec<K>,
    /// The grabbed row's label, for the ghost.
    pub label: String,
}

impl<K: Clone + PartialEq> ListDrag<K> {
    /// The payload for dragging `grabbed` from the list `list`: all of `selection` (in display
    /// order) when it holds `grabbed`, else `grabbed` alone. The selection itself doesn't change.
    pub fn new(list: Id, order: &(impl DisplayOrder<K> + ?Sized), selection: &[K], grabbed: K, label: impl Into<String>) -> Self {
        let items = if selection.contains(&grabbed) {
            dragged_positions(order, selection).into_iter().map(|at| order.key(at).clone()).collect()
        } else {
            vec![grabbed]
        };
        Self { list, items, label: label.into() }
    }
}

/// The extents of the rows along the list's axis, in the same coordinates as the pointer.
/// Spans of consecutive rows don't overlap and follow each other in row order.
pub trait RowSpans {
    /// How many rows there are.
    fn len(&self) -> usize;

    /// Whether there are no rows.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The extent of row `at`, below [`len`](RowSpans::len).
    fn span(&self, at: usize) -> Range<f32>;
}

impl RowSpans for [Range<f32>] {
    fn len(&self) -> usize {
        <[Range<f32>]>::len(self)
    }

    fn span(&self, at: usize) -> Range<f32> {
        self[at].clone()
    }
}

/// Rows of one extent `extent`, `gap` apart, the first starting at `origin`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UniformSpans {
    /// Where the first row starts.
    pub origin: f32,
    /// A row's extent along the axis.
    pub extent: f32,
    /// The space between rows.
    pub gap: f32,
    /// How many rows there are.
    pub rows: usize,
}

impl RowSpans for UniformSpans {
    fn len(&self) -> usize {
        self.rows
    }

    fn span(&self, at: usize) -> Range<f32> {
        let start = self.origin + at as f32 * (self.extent + self.gap);
        start..start + self.extent
    }
}

/// Spans in reading order of a right-to-left horizontal list: negated, so that they run forward.
struct Mirrored<'a, S: ?Sized>(&'a S);

impl<S: RowSpans + ?Sized> RowSpans for Mirrored<'_, S> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn span(&self, at: usize) -> Range<f32> {
        let span = self.0.span(at);
        -span.end..-span.start
    }
}

/// A drop target under the pointer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved<K> {
    /// The target row's position in the display order.
    pub at: usize,
    /// What the drop does.
    pub mv: Move<K>,
}

/// The keys that move the selected rows in the list order: Cmd+] / Cmd+[ on macOS, Ctrl+] /
/// Ctrl+[ elsewhere (PhotoCraft's bring forward and send backward), Shift to the ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackKey {
    /// One row toward the start of the list (`]`).
    TowardStart,
    /// One row toward the end (`[`).
    TowardEnd,
    /// To the start (Shift+`]`).
    ToStart,
    /// To the end (Shift+`[`).
    ToEnd,
}

impl StackKey {
    /// The move `key` pressed with `modifiers` asks for. `mac` says whether Cmd and Control are
    /// different keys (macOS), where Cmd is the command key.
    pub fn from_key(key: Key, modifiers: Modifiers, mac: bool) -> Option<Self> {
        let Modifiers { alt, ctrl, shift, mac_cmd, .. } = modifiers;
        let command = if mac { mac_cmd && !ctrl } else { ctrl && !mac_cmd };
        if alt || !command {
            return None;
        }
        match (key, shift) {
            (Key::CloseBracket, false) => Some(Self::TowardStart),
            (Key::OpenBracket, false) => Some(Self::TowardEnd),
            (Key::CloseBracket | Key::CloseCurlyBracket, true) => Some(Self::ToStart),
            (Key::OpenBracket | Key::OpenCurlyBracket, true) => Some(Self::ToEnd),
            _ => None,
        }
    }
}

/// How a drag ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DragEnd<K> {
    /// The pointer was released over this accepted target, or over none.
    Released(Option<Move<K>>),
    /// Escape was pressed.
    Escape,
    /// The window lost focus.
    FocusLost,
    /// The app cleared egui's drag payload, or another drag replaced it.
    PayloadCleared,
    /// The list was not drawn in a pass.
    Removed,
}

/// The edit lifecycle of one drag of rows, on an [`edit::Gesture`](Gesture): `Begin` at the
/// start, then `Commit` with the move on a drop on an accepted target, or `Cancel` (`Escape`,
/// `FocusLost`, `Removed`, or `Dismissed` for a release where nothing accepts the rows, which
/// includes a no-op, and for a payload the app cleared); a stacking key's move is a `Set`. The gesture is live while its control
/// loads it ([`edit::is_live`](crate::edit::is_live)), and Esc and
/// [`edit::cancel_all`](crate::edit::cancel_all) end it, as for every edit gesture.
///
/// The target under the pointer is not an edit: the control reports it as its own state each
/// frame (a list's [`ListOutput::drop_target`](crate::list::ListOutput::drop_target)), so a host
/// may preview it, and nothing reaches the document before the drop.
///
/// The control loads the gesture in every pass it is drawn, with a drag under way or not, and
/// finishes it in the same pass:
///
/// ```
/// use craft_ui::behavior::{DragEnd, ReorderGesture};
/// # egui::__run_test_ui(|ui| {
/// let mut gesture = ReorderGesture::<usize>::load(ui, ui.id().with("tabs"));
/// let (started, released) = (false, false);
/// if started {
///     gesture.begin();
/// }
/// if gesture.is_active() && released {
///     gesture.end(DragEnd::Released(None), ui.input(|i| i.modifiers));
/// }
/// let event = gesture.finish();
/// # assert!(event.is_none());
/// # });
/// ```
#[derive(Debug)]
#[must_use = "finish the gesture to keep it and get the frame's event"]
pub struct ReorderGesture<K> {
    gesture: Gesture<Option<Move<K>>>,
}

impl<K: Clone + PartialEq + Send + Sync + 'static> ReorderGesture<K> {
    /// The drag gesture of the control with id `id` (a list, a tab strip), as its last pass left
    /// it. Applies Esc, [`cancel_all`](crate::edit::cancel_all) and a pass that didn't draw the
    /// control, each of which ends a drag under way.
    pub fn load(ui: &Ui, id: Id) -> Self {
        let policy = EditPolicy::default().on_release_only(true);
        let gesture = Gesture::load(ui, id.with("craft_ui::reorder"), &None, policy).focus_owners(&[]).enabled(ui.is_enabled());
        Self { gesture }
    }

    /// Whether a drag is under way.
    pub fn is_active(&self) -> bool {
        self.gesture.is_active()
    }

    /// The drag's id, while one is under way.
    pub fn id(&self) -> Option<GestureId> {
        self.gesture.id()
    }

    /// Starts a drag; nothing while one is under way.
    pub fn begin(&mut self) {
        self.gesture.begin();
    }

    /// Ends the drag under way: a commit for a release over an accepted target, otherwise a
    /// cancel for the reason.
    pub fn end(&mut self, end: DragEnd<K>, modifiers: Modifiers) {
        match end {
            DragEnd::Released(Some(mv)) => {
                self.gesture.update(Some(mv), modifiers);
                self.gesture.commit();
            }
            DragEnd::Released(None) => self.gesture.cancel(CancelReason::Dismissed),
            DragEnd::Escape => self.gesture.cancel(CancelReason::Escape),
            DragEnd::FocusLost => self.gesture.cancel(CancelReason::FocusLost),
            DragEnd::PayloadCleared => self.gesture.cancel(CancelReason::Dismissed),
            DragEnd::Removed => self.gesture.cancel(CancelReason::Removed),
        }
    }

    /// A move made at once, by a stacking key: a `Set`.
    pub fn set(&mut self, mv: Move<K>) {
        self.gesture.set(Some(mv));
    }

    /// Keeps the gesture and returns this frame's event.
    pub fn finish(self) -> Option<EditEvent<Move<K>>> {
        Some(match self.gesture.finish()? {
            EditEvent::Begin { id } => EditEvent::Begin { id },
            EditEvent::Cancel { id, reason } => EditEvent::Cancel { id, reason },
            EditEvent::Commit { id, value: Some(value), changed, modifiers } => EditEvent::Commit { id, value, changed, modifiers },
            EditEvent::Set { value: Some(value), modifiers } => EditEvent::Set { value, modifiers },
            // Inside an edit scope, the end of a drag and a key's move.
            EditEvent::Update { id, value: Some(value), modifiers } => EditEvent::Update { id, value, modifiers },
            EditEvent::Commit { .. } | EditEvent::Set { .. } | EditEvent::Update { .. } => return None,
        })
    }
}

type Accepts<'a, K> = Box<dyn Fn(&Move<K>, Modifiers) -> bool + 'a>;

/// Where a drag over a list or strip drops, which keys move rows, and how fast it scrolls.
///
/// Targets, as in PhotoCraft's Layers panel:
/// - A group (a row with children): the middle 30 to 70 percent of the row is `Into`, above that
///   `Before`, below it `After`. Other rows: halves.
/// - The gap between two rows drops before the lower row; the space past the last row drops
///   after it.
/// - `Into` puts the rows at the top of the group, as its first children.
/// - A drop that changes nothing has no target: onto the dragged rows, onto the slot next to
///   them at the same depth, or into their own group when they are its first children already.
///   Neither has a drop into a dragged row or its descendants. Copying allows the dragged rows'
///   own edges and their own group.
/// - The dragged rows are taken without the rows inside another dragged row, which go with it.
/// - [`accepts`](Reorder::accepts) refuses targets for the host's own rules.
///
/// Horizontal lists follow the reading direction: before and after are left and right, or right
/// and left with [`right_to_left`](Reorder::right_to_left).
pub struct Reorder<'a, K> {
    axis: Axis,
    right_to_left: bool,
    copy_with_alt: bool,
    accepts: Option<Accepts<'a, K>>,
}

impl<K: Clone + PartialEq> Default for Reorder<'_, K> {
    fn default() -> Self {
        Self::new(Axis::Vertical)
    }
}

/// The share of a group row that is above the `Into` zone, and below it.
const INTO_START: f32 = 0.3;
const INTO_END: f32 = 0.7;

/// The widest edge zone that scrolls, in points.
const SCROLL_ZONE: f32 = 32.0;
/// The scroll speed at the inner edge of the zone and its growth to the outer edge, points a second.
const SCROLL_MIN_SPEED: f32 = 80.0;
const SCROLL_SPEED_RANGE: f32 = 520.0;
/// The longest frame time that scrolls, in seconds.
const SCROLL_MAX_DT: f32 = 0.05;

impl<'a, K: Clone + PartialEq> Reorder<'a, K> {
    /// Rules for a list along `axis`.
    pub fn new(axis: Axis) -> Self {
        Self { axis, right_to_left: false, copy_with_alt: false, accepts: None }
    }

    /// Whether a horizontal list reads right to left (default false).
    pub fn right_to_left(mut self, right_to_left: bool) -> Self {
        self.right_to_left = right_to_left;
        self
    }

    /// Whether holding Alt makes the drop a copy (default false).
    pub fn copy_with_alt(mut self, copy_with_alt: bool) -> Self {
        self.copy_with_alt = copy_with_alt;
        self
    }

    /// Refuses the moves for which `accepts` is false; a refused target has no marker and takes
    /// no drop. It gets the modifiers held (a locked layer takes drops with Cmd/Ctrl).
    pub fn accepts(mut self, accepts: impl Fn(&Move<K>, Modifiers) -> bool + 'a) -> Self {
        self.accepts = Some(Box::new(accepts));
        self
    }

    /// The axis.
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// Whether a drop with `modifiers` held is a copy (Alt, with
    /// [`copy_with_alt`](Reorder::copy_with_alt)).
    pub fn copies(&self, modifiers: Modifiers) -> bool {
        self.copy_with_alt && modifiers.alt
    }

    /// The target of a drag of `dragged` with the pointer at `pointer` along the axis, in the
    /// coordinates of `spans`. `None` when nothing there takes a drop.
    pub fn resolve(
        &self,
        order: &(impl DisplayOrder<K> + ?Sized),
        spans: &(impl RowSpans + ?Sized),
        pointer: f32,
        dragged: &[K],
        modifiers: Modifiers,
    ) -> Option<Resolved<K>> {
        if self.axis == Axis::Horizontal && self.right_to_left {
            self.resolve_in_reading_order(order, &Mirrored(spans), -pointer, dragged, modifiers)
        } else {
            self.resolve_in_reading_order(order, spans, pointer, dragged, modifiers)
        }
    }

    fn resolve_in_reading_order(
        &self,
        order: &(impl DisplayOrder<K> + ?Sized),
        spans: &(impl RowSpans + ?Sized),
        pointer: f32,
        dragged: &[K],
        modifiers: Modifiers,
    ) -> Option<Resolved<K>> {
        let len = order.len().min(spans.len());
        if len == 0 {
            return None;
        }
        let (at, position) = {
            let first_after = partition_point(len, |at| spans.span(at).end <= pointer);
            if first_after == len {
                (len - 1, DropPosition::After)
            } else {
                let span = spans.span(first_after);
                if pointer < span.start {
                    (first_after, DropPosition::Before)
                } else {
                    let share = (pointer - span.start) / (span.end - span.start).max(f32::EPSILON);
                    let position = if order.has_children(first_after) {
                        if share < INTO_START {
                            DropPosition::Before
                        } else if share > INTO_END {
                            DropPosition::After
                        } else {
                            DropPosition::Into
                        }
                    } else if share < 0.5 {
                        DropPosition::Before
                    } else {
                        DropPosition::After
                    };
                    (first_after, position)
                }
            }
        };
        let copy = self.copies(modifiers);
        let positions = dragged_positions(order, dragged);
        let items: Vec<K> = positions.iter().map(|&p| order.key(p).clone()).collect();
        if items.is_empty() || !drop_changes_something(order, &positions, at, position, copy) {
            return None;
        }
        let mv = Move { items, target: order.key(at).clone(), position, copy };
        self.accepts.as_ref().is_none_or(|accepts| accepts(&mv, modifiers)).then_some(Resolved { at, mv })
    }

    /// The move for a stacking key over the `selected` rows, or `None` when they can't move that
    /// way or the host refuses it. The rows move as one block: from the nearest row beyond the
    /// block's first (or last) row, so a selection with gaps closes up there. A selected row
    /// inside another selected row moves with it, so the block never lands inside itself.
    ///
    /// A step past a closed group goes past it; a step onto the children of an open group
    /// enters it as their sibling, and a step from a group's first (last) child leaves the group.
    pub fn key_move(&self, order: &(impl DisplayOrder<K> + ?Sized), selected: &[K], key: StackKey, modifiers: Modifiers) -> Option<Move<K>> {
        let len = order.len();
        let positions = dragged_positions(order, selected);
        let (&first, &last) = (positions.first()?, positions.last()?);
        // The positions are disjoint subtrees in order, so the last one's ends the block.
        let block_end = subtree_end(order, last);
        let packed = is_block(order, &positions);
        let (at, position) = match key {
            StackKey::TowardStart if first > 0 => (first - 1, DropPosition::Before),
            StackKey::ToStart if !(first == 0 && packed) => (0, DropPosition::Before),
            StackKey::TowardEnd if block_end + 1 < len => {
                let next = block_end + 1;
                if order.depth(next) < order.depth(last) {
                    (next, DropPosition::Before)
                } else if let Some(child) = first_child(order, next) {
                    (child, DropPosition::Before)
                } else {
                    (next, DropPosition::After)
                }
            }
            StackKey::ToEnd if !(block_end + 1 == len && packed) => {
                let mut top = len - 1;
                while let Some(up) = parent(order, top) {
                    top = up;
                }
                (top, DropPosition::After)
            }
            _ => return None,
        };
        if inside_dragged(order, &positions, at) {
            return None;
        }
        let items = positions.iter().map(|&p| order.key(p).clone()).collect();
        let mv = Move { items, target: order.key(at).clone(), position, copy: false };
        self.accepts.as_ref().is_none_or(|accepts| accepts(&mv, modifiers)).then_some(mv)
    }

    /// How far to move the scroll offset this frame while a drag is held at `pointer` over a
    /// list occupying `viewport`: negative toward the start edge (top, or left), positive toward
    /// the end edge, zero away from them. Only while the pointer is within the list's extent
    /// across the axis. `dt` is the frame time in seconds.
    pub fn auto_scroll(&self, pointer: Pos2, viewport: Rect, dt: f32) -> f32 {
        if dt.is_nan() || dt <= 0.0 {
            return 0.0;
        }
        let (along, start, end, across, across_range) = match self.axis {
            Axis::Vertical => (pointer.y, viewport.top(), viewport.bottom(), pointer.x, viewport.x_range()),
            Axis::Horizontal => (pointer.x, viewport.left(), viewport.right(), pointer.y, viewport.y_range()),
        };
        if !across.is_finite() || !along.is_finite() || !across_range.contains(across) {
            return 0.0;
        }
        edge_scroll_speed(along, start, end) * dt.min(SCROLL_MAX_DT)
    }
}

/// The scroll speed in points a second for a drag held at `pointer` between the edges `start`
/// and `end` of a list: negative toward `start`, positive toward `end`, zero outside the edge
/// zones. The zone is the smaller of 32 points and a quarter of the list; the speed is 80 at the
/// zone's inner edge up to 600 at the list's edge, and 600 beyond it.
pub fn edge_scroll_speed(pointer: f32, start: f32, end: f32) -> f32 {
    let extent = end - start;
    if extent.is_nan() || extent <= 0.0 {
        return 0.0;
    }
    let zone = SCROLL_ZONE.min(extent * 0.25);
    let near_start = ((zone - (pointer - start)) / zone).clamp(0.0, 1.0);
    let near_end = ((zone - (end - pointer)) / zone).clamp(0.0, 1.0);
    let direction = near_end - near_start;
    if direction == 0.0 {
        return 0.0;
    }
    direction.signum() * (SCROLL_MIN_SPEED + SCROLL_SPEED_RANGE * direction.abs())
}

/// The first of `len` rows for which `before` is false, given that it is true then false.
fn partition_point(len: usize, before: impl Fn(usize) -> bool) -> usize {
    let (mut low, mut high) = (0, len);
    while low < high {
        let mid = low + (high - low) / 2;
        if before(mid) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

/// The display positions of `keys` that are shown, ascending, without those inside another one's
/// subtree: each moves with its subtree.
fn dragged_positions<K: PartialEq>(order: &(impl DisplayOrder<K> + ?Sized), keys: &[K]) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut covered = None;
    for at in 0..order.len() {
        if covered.is_some_and(|end| at <= end) || !keys.contains(order.key(at)) {
            continue;
        }
        positions.push(at);
        covered = Some(subtree_end(order, at));
    }
    positions
}

/// The position of the last row below `at` that belongs to its subtree (`at` for a leaf).
fn subtree_end<K>(order: &(impl DisplayOrder<K> + ?Sized), at: usize) -> usize {
    let depth = order.depth(at);
    let mut end = at;
    while end + 1 < order.len() && order.depth(end + 1) > depth {
        end += 1;
    }
    end
}

/// Whether the dragged rows, with their subtrees, form one run of rows.
fn is_block<K>(order: &(impl DisplayOrder<K> + ?Sized), positions: &[usize]) -> bool {
    positions.windows(2).all(|pair| pair[1] == subtree_end(order, pair[0]) + 1)
}

/// Whether the row at `at` is a descendant of a dragged row: rows can't go into their own
/// subtree.
fn inside_dragged<K>(order: &(impl DisplayOrder<K> + ?Sized), positions: &[usize], at: usize) -> bool {
    positions.iter().any(|&p| p < at && at <= subtree_end(order, p))
}

/// Whether dropping `positions` at `position` of row `at` is a drop that takes effect.
fn drop_changes_something<K>(order: &(impl DisplayOrder<K> + ?Sized), positions: &[usize], at: usize, position: DropPosition, copy: bool) -> bool {
    let dragged = positions.contains(&at);
    if inside_dragged(order, positions, at) || (dragged && position == DropPosition::Into) {
        return false;
    }
    if copy || !is_block(order, positions) {
        return true;
    }
    if position == DropPosition::Into {
        // Into puts the rows first in the group: a no-op for its first children.
        let depth = order.depth(at) + 1;
        return !(first_child(order, at) == Some(positions[0]) && positions.iter().all(|&p| order.depth(p) == depth));
    }
    let first = positions[0];
    let after_block = subtree_end(order, positions[positions.len() - 1]) + 1;
    let slot = if position == DropPosition::After { at + 1 } else { at };
    let adjacent = (first..=after_block).contains(&slot);
    let same_depth = positions.iter().all(|&p| order.depth(p) == order.depth(at));
    !(adjacent && same_depth)
}
