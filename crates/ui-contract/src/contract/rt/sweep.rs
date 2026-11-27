//! Toggle sweeps: a primary drag that starts on a toggle sets every sibling toggle it crosses to the
//! value the first one took (docs/contract-rt.md section 5).
//!
//! The window root keeps one record per window. The press owner writes it at `DragStart`, then
//! hit-tests each move's segment itself and marks each toggle of its group it crosses, queuing no
//! move of its own; the release clears the record. A marked toggle's run takes itself off the
//! record and sets the swept value, so each toggle runs once per sweep.

use craft_behavior::PointerButton;
use craft_ui_rt::{BuildCx, EventCx, Gesture, NodeId, PointerInput, Pos2, State};

use super::node::Kept;

/// The distance between the points a sweep tests along a move, in logical points.
const STEP: f32 = 2.0;

/// One window's running sweep.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Sweep {
    owner: Option<NodeId>,
    group: u64,
    on: bool,
    /// The pointer's last position, in window coordinates.
    last: Pos2,
    /// Every toggle the sweep has set, the owner included.
    crossed: Vec<NodeId>,
    /// Crossed toggles whose run has not yet taken the value.
    waiting: Vec<NodeId>,
}

/// Runs the sweep part of a phase of the press `kept`'s node owns, for a sweeping toggle of group
/// `group`. Returns true when the sweep took the phase, so nothing is queued.
pub(crate) fn on_pointer(group: u64, kept: &mut Kept, cx: &mut EventCx<'_>, p: &PointerInput) -> bool {
    let (Some(cell), Some(shown)) = (kept.sweep, kept.toggle) else {
        return false;
    };
    if group == 0 || p.button != PointerButton::Primary {
        return false;
    }
    let me = cx.node();
    match p.gesture {
        Gesture::DragStart { origin, .. } => {
            let mut record = Sweep { owner: Some(me), group, on: !shown, last: origin, crossed: vec![me], waiting: Vec::new() };
            cross(&mut record, cx, p.pos);
            cell.set(cx, record);
            // The owner's own DragStart is queued: its run sets its own value.
            false
        }
        Gesture::Drag { .. } => {
            let mut record = cell.with(cx, Clone::clone);
            if record.owner != Some(me) {
                return false;
            }
            cross(&mut record, cx, p.pos);
            cell.set(cx, record);
            true
        }
        Gesture::Up { .. } | Gesture::Cancel(_) => {
            if cell.with(cx, |r| r.owner != Some(me)) {
                return false;
            }
            cell.set(cx, Sweep::default());
            kept.press = None;
            kept.dragging = false;
            true
        }
        Gesture::Down { .. } | Gesture::LongPress => false,
    }
}

/// Marks each toggle of the record's group under the segment from the last position to `to`.
fn cross(record: &mut Sweep, cx: &mut EventCx<'_>, to: Pos2) {
    let from = record.last;
    let d = to - from;
    let n = (d.length() / STEP).ceil().max(1.0) as usize;
    for i in 1..=n {
        let at = from + d * (i as f32 / n as f32);
        let Some(hit) = cx.hit(at, &|s| s.takes_press()) else { continue };
        if !record.crossed.contains(&hit.node) && cx.tag_of(hit.node) == record.group {
            record.crossed.push(hit.node);
            record.waiting.push(hit.node);
            cx.request_build_of(hit.node);
        }
    }
    record.last = to;
}

/// The value the running sweep sets on `node`, taking the node off the record so it runs once.
pub(crate) fn take(cx: &mut BuildCx<'_>, cell: State<Sweep>, node: NodeId) -> Option<bool> {
    let record = cx.peek_state(cell);
    if record.owner == Some(node) {
        return Some(record.on);
    }
    if !record.waiting.contains(&node) {
        return None;
    }
    let on = record.on;
    cell.update(cx, |r| r.waiting.retain(|&n| n != node));
    Some(on)
}
