//! The shared parts of reordering by pointer or key: where a drop lands between items, where a moved item ends up,
//! and the Alt+arrow intent. A collection's row drag and a section stack both use them.

use crate::runtime::drag::DropMark;
use crate::runtime::keys::Intents;
use craft_behavior::{Axis, Intent};
use egui::{Pos2, Rect};

/// The gap a drop would land in.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Gap {
    /// The insert position among the rects: the gap before rect `insert`, `rects.len()` after the last.
    pub insert: usize,
    /// The insertion line across the first rect's extent.
    pub mark: DropMark,
}

/// The gap nearest `at` among `rects`, which lie in order along `axis`: before the first rect whose middle is past
/// the pointer, else after the last. `None` without rects. With one rect, `insert` is 0 above its middle and 1 below.
pub fn gap_at(rects: &[Rect], axis: Axis, at: Pos2) -> Option<Gap> {
    let (first, last) = (rects.first()?, rects.last()?);
    let along = |point: Pos2| match axis {
        Axis::X => point.x,
        _ => point.y,
    };
    let pointer = along(at);
    let insert = rects.iter().position(|rect| pointer < along(rect.center())).unwrap_or(rects.len());
    let mark = match axis {
        Axis::X => {
            let x = rects.get(insert).map_or(last.max.x, |rect| rect.min.x);
            DropMark::line(Pos2::new(x, first.min.y), Pos2::new(x, first.max.y))
        }
        _ => {
            let y = rects.get(insert).map_or(last.max.y, |rect| rect.min.y);
            DropMark::line(Pos2::new(first.min.x, y), Pos2::new(first.max.x, y))
        }
    };
    Some(Gap { insert, mark })
}

/// The index item `from` has once it moves to the gap before item `insert` of the list it is in: it leaves its place
/// first, so a gap below it counts one less.
pub fn move_to(from: usize, insert: usize) -> usize {
    move_all_to(&[from], insert)
}

/// The index the first of `places` has once all of them move to the gap before item `insert`: every moved item above
/// the gap leaves first.
pub fn move_all_to(places: &[usize], insert: usize) -> usize {
    insert - places.iter().filter(|&&place| place < insert).count()
}

/// The index an Alt+arrow nudge moves item `at` of `len` to: one step toward the end when `forward`, else toward the
/// start, stopping at the ends.
pub fn nudge_to(at: usize, len: usize, forward: bool) -> usize {
    if forward { (at + 1).min(len.saturating_sub(1)) } else { at.saturating_sub(1) }
}

/// The Alt+arrow move of the focused item this pass: `Some(true)` toward the end, `Some(false)` toward the start.
pub fn take_move_item(intents: &mut Intents) -> Option<bool> {
    if intents.take(Intent::move_item(true)) {
        Some(true)
    } else if intents.take(Intent::move_item(false)) {
        Some(false)
    } else {
        None
    }
}
