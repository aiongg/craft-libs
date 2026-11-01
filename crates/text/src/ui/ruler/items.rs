//! The things a ruler edits (the indent markers and the stops) and the edits on [`RulerValues`].

use super::{RulerValues, SAME_POSITION, TabStop};

/// A marker or stop on the ruler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Item {
    FirstLine,
    /// The triangle under the first-line one: moves the left indent, the first line stays.
    Hanging,
    /// The box under the hanging triangle: moves first line and left indent together.
    LeftBox,
    Right,
    /// The stop at this index of [`RulerValues::stops`].
    Stop(usize),
}

impl Item {
    /// Every item in the order Page Down visits them: the markers, then the stops.
    pub(super) fn all(values: &RulerValues) -> Vec<Item> {
        let markers = [Item::FirstLine, Item::Hanging, Item::LeftBox, Item::Right];
        markers.into_iter().chain((0..values.stops.len()).map(Item::Stop)).collect()
    }

    /// Where the item sits, points from the column's start edge.
    pub(super) fn position(self, values: &RulerValues, width: f32) -> f32 {
        match self {
            Item::FirstLine => values.left_indent + values.first_line_indent,
            Item::Hanging | Item::LeftBox => values.left_indent,
            Item::Right => width - values.right_indent,
            Item::Stop(at) => values.stops.get(at).map_or(0.0, |stop| stop.position),
        }
    }
}

/// `values` with `stop` set: stops within [`SAME_POSITION`] of it go, the rest stay sorted. The
/// index of `stop` comes with it.
pub(super) fn place(values: &RulerValues, stop: TabStop) -> (RulerValues, usize) {
    let mut placed = values.clone();
    placed.stops.retain(|other| (other.position - stop.position).abs() >= SAME_POSITION);
    placed.stops.push(stop);
    placed.stops.sort_by(|a, b| a.position.total_cmp(&b.position));
    let at = placed.stops.iter().position(|other| *other == stop).unwrap_or(0);
    (placed, at)
}

/// `values` without the stop at `at`.
pub(super) fn without(values: &RulerValues, at: usize) -> RulerValues {
    let mut cleared = values.clone();
    if at < cleared.stops.len() {
        cleared.stops.remove(at);
    }
    cleared
}

/// `values` with `item` moved to `to` points (kept within the column), and where the item is
/// afterwards (a stop moved past another has a new index).
pub(super) fn moved(values: &RulerValues, item: Item, to: f32, width: f32) -> (RulerValues, Item) {
    if !to.is_finite() {
        return (values.clone(), item);
    }
    let to = to.clamp(0.0, width.max(0.0));
    let mut next = values.clone();
    match item {
        Item::FirstLine => next.first_line_indent = to - values.left_indent,
        Item::LeftBox => next.left_indent = to,
        Item::Hanging => {
            next.left_indent = to;
            next.first_line_indent = values.left_indent + values.first_line_indent - to;
        }
        Item::Right => next.right_indent = (width - to).clamp(0.0, (width - values.left_indent).max(0.0)),
        Item::Stop(at) => {
            let Some(stop) = values.stops.get(at).copied() else { return (next, item) };
            let (placed, now) = place(&without(values, at), TabStop { position: to, ..stop });
            return (placed, Item::Stop(now));
        }
    }
    (next, item)
}
