//! [`Reorder`]: drag-to-reorder rules for a list or a strip of items, without drawing. The list
//! reads the pointer, draws the marker and the ghost, and feeds the rules.

use std::ops::Range;

use egui::{Id, Key, Modifiers, Pos2, Rect};

use super::list_selection::{DisplayOrder, first_child, parent};
use crate::edit::{CancelReason, EditEvent, GestureId};

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
    /// Into the row, which is a group.
    Into,
}

/// A reorder or copy: the rows `items` go to `position` of the row `target`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Move<K> {
    /// The rows moved, in display order.
    pub items: Vec<K>,
    /// The row the drop is relative to.
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
    /// grabbed row.
    pub items: Vec<K>,
    /// The grabbed row's label, for the ghost.
    pub label: String,
}

impl<K: Clone + PartialEq> ListDrag<K> {
    /// The payload for dragging `grabbed` from the list `list`: all of `selection` (in display
    /// order) when it holds `grabbed`, else `grabbed` alone. The selection itself doesn't change.
    pub fn new(list: Id, order: &(impl DisplayOrder<K> + ?Sized), selection: &[K], grabbed: K, label: impl Into<String>) -> Self {
        let items = if selection.contains(&grabbed) {
            (0..order.len()).map(|at| order.key(at)).filter(|k| selection.contains(k)).cloned().collect()
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
    /// The list was not drawn in a pass.
    Removed,
}

/// The [`EditEvent`] sequence of one drag: `Begin` at the start, `Update` when the target changes,
/// then `Commit` on a drop on an accepted target or `Cancel` (`Escape`, `FocusLost`, `Removed`, or
/// `Dismissed` for a release where nothing accepts the rows, which includes a no-op).
///
/// The value is `None` while the pointer is over no valid target, so a host's live preview
/// clears; a `Commit` always carries `Some`.
#[derive(Clone, Debug)]
pub struct ReorderGesture<K> {
    id: GestureId,
    last: Option<Move<K>>,
}

impl<K: Clone + PartialEq> ReorderGesture<K> {
    /// A drag that starts now, and its `Begin` event.
    pub fn begin() -> (Self, EditEvent<Option<Move<K>>>) {
        let id = GestureId::next();
        (Self { id, last: None }, EditEvent::begin(id))
    }

    /// The gesture's id.
    pub fn id(&self) -> GestureId {
        self.id
    }

    /// The target under the pointer this frame, if any accepts the rows: an `Update` when it is
    /// not the same as the last one, with `None` when the pointer left every valid target.
    pub fn update(&mut self, target: Option<Move<K>>, modifiers: Modifiers) -> Option<EditEvent<Option<Move<K>>>> {
        if target == self.last {
            return None;
        }
        self.last.clone_from(&target);
        Some(EditEvent::update(self.id, target, modifiers))
    }

    /// The event that ends the drag.
    pub fn finish(self, end: DragEnd<K>, modifiers: Modifiers) -> EditEvent<Option<Move<K>>> {
        match end {
            DragEnd::Released(Some(mv)) => EditEvent::commit(self.id, Some(mv), true, modifiers),
            DragEnd::Released(None) => EditEvent::cancel(self.id, CancelReason::Dismissed),
            DragEnd::Escape => EditEvent::cancel(self.id, CancelReason::Escape),
            DragEnd::FocusLost => EditEvent::cancel(self.id, CancelReason::FocusLost),
            DragEnd::Removed => EditEvent::cancel(self.id, CancelReason::Removed),
        }
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
/// - A drop that changes nothing has no target: onto the dragged rows, or onto the slot next to
///   them at the same depth. Neither has a drop into a dragged row or its descendants. Copying
///   allows the dragged rows' own edges.
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
        let copy = self.copy_with_alt && modifiers.alt;
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
    /// block's first (or last) row, so a selection with gaps closes up there.
    ///
    /// A step past a closed group goes past it; a step onto the children of an open group
    /// enters it as their sibling, and a step from a group's first (last) child leaves the group.
    pub fn key_move(&self, order: &(impl DisplayOrder<K> + ?Sized), selected: &[K], key: StackKey, modifiers: Modifiers) -> Option<Move<K>> {
        let len = order.len();
        let positions = dragged_positions(order, selected);
        let (&first, &last) = (positions.first()?, positions.last()?);
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

/// The display positions of `keys` that are shown, ascending.
fn dragged_positions<K: PartialEq>(order: &(impl DisplayOrder<K> + ?Sized), keys: &[K]) -> Vec<usize> {
    (0..order.len()).filter(|&at| keys.contains(order.key(at))).collect()
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

/// Whether dropping `positions` at `position` of row `at` is a drop that takes effect.
fn drop_changes_something<K>(order: &(impl DisplayOrder<K> + ?Sized), positions: &[usize], at: usize, position: DropPosition, copy: bool) -> bool {
    let dragged = positions.contains(&at);
    // A row can't go into itself or its own descendants.
    let inside = positions.iter().any(|&p| p < at && at <= subtree_end(order, p));
    if inside || (dragged && position == DropPosition::Into) {
        return false;
    }
    if copy || position == DropPosition::Into || !is_block(order, positions) {
        return true;
    }
    let first = positions[0];
    let after_block = subtree_end(order, positions[positions.len() - 1]) + 1;
    let slot = if position == DropPosition::After { at + 1 } else { at };
    let adjacent = (first..=after_block).contains(&slot);
    let same_depth = positions.iter().all(|&p| order.depth(p) == order.depth(at));
    !(adjacent && same_depth)
}
