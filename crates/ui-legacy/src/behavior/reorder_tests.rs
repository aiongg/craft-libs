use egui::{Id, Key, Modifiers, Rect, pos2};

use super::list_selection::RowInfo;
use super::reorder::{DragEnd, DropPosition, ListDrag, Move, StackKey, UniformSpans, edge_scroll_speed};
use super::{Axis, Reorder, ReorderGesture};
use crate::edit::{CancelReason, EditEvent};

use DropPosition::{After, Before, Into};

fn flat(n: u32) -> Vec<RowInfo<u32>> {
    (0..n).map(RowInfo::leaf).collect()
}

/// 0; group 1 (open) with 2 and group 3 (closed); 4; group 5 (open) with 6 and 7.
fn tree() -> Vec<RowInfo<u32>> {
    vec![
        RowInfo::leaf(0),
        RowInfo::group(1, true),
        RowInfo::leaf(2).at_depth(1),
        RowInfo::group(3, false).at_depth(1),
        RowInfo::leaf(4),
        RowInfo::group(5, true),
        RowInfo::leaf(6).at_depth(1),
        RowInfo::leaf(7).at_depth(1),
    ]
}

/// Rows 20 high from 0, no gaps.
fn spans(rows: usize) -> UniformSpans {
    UniformSpans { origin: 0.0, extent: 20.0, gap: 0.0, rows }
}

fn mv(items: &[u32], target: u32, position: DropPosition) -> Move<u32> {
    Move { items: items.to_vec(), target, position, copy: false }
}

fn drop_at(order: &[RowInfo<u32>], pointer: f32, dragged: &[u32]) -> Option<Move<u32>> {
    Reorder::default().resolve(order, &spans(order.len()), pointer, dragged, Modifiers::NONE).map(|r| r.mv)
}

#[test]
fn a_leaf_row_drops_before_in_its_top_half_and_after_in_its_bottom_half() {
    let rows = flat(4);
    assert_eq!(drop_at(&rows, 45.0, &[0]), Some(mv(&[0], 2, Before)));
    assert_eq!(drop_at(&rows, 55.0, &[0]), Some(mv(&[0], 2, After)));
}

#[test]
fn a_group_row_has_zones_above_into_and_below() {
    let rows = tree();
    // Row 1 spans 20..40: before up to 26, into to 34, after beyond.
    assert_eq!(drop_at(&rows, 24.0, &[4]), Some(mv(&[4], 1, Before)));
    assert_eq!(drop_at(&rows, 30.0, &[4]), Some(mv(&[4], 1, Into)));
    assert_eq!(drop_at(&rows, 36.0, &[4]), Some(mv(&[4], 1, After)));
}

#[test]
fn the_gap_between_rows_drops_before_the_lower_row_and_the_space_past_the_last_drops_after_it() {
    let rows = flat(4);
    let gapped = UniformSpans { origin: 10.0, extent: 20.0, gap: 4.0, rows: 4 };
    let reorder = Reorder::default();
    let at = |pointer| reorder.resolve(&rows[..], &gapped, pointer, &[3], Modifiers::NONE).map(|r| (r.at, r.mv.position));
    // Row 1 spans 34..54, so 32 is in the gap above it.
    assert_eq!(at(32.0), Some((1, Before)));
    // Above the first row, at its top edge.
    assert_eq!(at(5.0).map(|(row, _)| row), Some(0));
    let past = |pointer| reorder.resolve(&rows[..], &gapped, pointer, &[0], Modifiers::NONE).map(|r| (r.at, r.mv.position));
    assert_eq!(past(500.0), Some((3, After)));
    assert_eq!(past(100.0), Some((3, After)));
}

#[test]
fn a_drop_that_changes_nothing_has_no_target() {
    let rows = flat(5);
    // Dragging row 2: the slots at its own top and bottom edge, and onto itself.
    for pointer in [45.0, 55.0, 35.0, 65.0] {
        assert_eq!(drop_at(&rows, pointer, &[2]), None, "pointer {pointer}");
    }
    assert_eq!(drop_at(&rows, 25.0, &[2]), Some(mv(&[2], 1, Before)));
    assert_eq!(drop_at(&rows, 75.0, &[2]), Some(mv(&[2], 3, After)));
    // Dragging 2 and 3 together: the slots from 2 to 4 are no-ops.
    for pointer in [45.0, 59.0, 65.0, 75.0, 85.0] {
        assert_eq!(drop_at(&rows, pointer, &[3, 2]), None, "pointer {pointer}");
    }
    assert_eq!(drop_at(&rows, 95.0, &[3, 2]), Some(mv(&[2, 3], 4, After)));
}

#[test]
fn dragged_rows_that_are_apart_gather_even_onto_an_edge_of_one() {
    let rows = flat(5);
    assert_eq!(drop_at(&rows, 65.0, &[1, 3]), Some(mv(&[1, 3], 3, Before)));
}

#[test]
fn a_copy_may_drop_on_the_dragged_rows_own_edges() {
    let rows = flat(4);
    let reorder = Reorder::default().copy_with_alt(true);
    let alt = Modifiers::ALT;
    let target = reorder.resolve(&rows[..], &spans(4), 45.0, &[2], alt).unwrap();
    assert_eq!(target.mv, Move { copy: true, ..mv(&[2], 2, Before) });
    assert_eq!(reorder.resolve(&rows[..], &spans(4), 45.0, &[2], Modifiers::NONE), None);
    let off = Reorder::default();
    assert_eq!(off.resolve(&rows[..], &spans(4), 65.0, &[0], alt).map(|r| r.mv.copy), Some(false));
}

#[test]
fn accepts_refuses_a_target_and_sees_the_modifiers() {
    let rows = flat(4);
    let reorder = Reorder::default().accepts(|m: &Move<u32>, mods| m.target != 2 || mods.command);
    assert_eq!(reorder.resolve(&rows[..], &spans(4), 45.0, &[0], Modifiers::NONE), None);
    assert!(reorder.resolve(&rows[..], &spans(4), 45.0, &[0], Modifiers::COMMAND).is_some());
    assert!(reorder.resolve(&rows[..], &spans(4), 25.0, &[3], Modifiers::NONE).is_some());
}

#[test]
fn a_row_does_not_go_into_itself_or_its_descendants() {
    let rows = tree();
    // Dragging group 1: into itself, before and after its own children.
    assert_eq!(drop_at(&rows, 30.0, &[1]), None);
    assert_eq!(drop_at(&rows, 45.0, &[1]), None);
    assert_eq!(drop_at(&rows, 65.0, &[1]), None);
    // Dragging group 5 into its own child 6 (row 6 spans 120..140).
    assert_eq!(drop_at(&rows, 125.0, &[5]), None);
    // A leaf has no Into zone.
    assert_eq!(drop_at(&rows, 90.0, &[0]), Some(mv(&[0], 4, After)));
}

#[test]
fn the_slot_after_a_dragged_group_is_a_no_op_but_leaving_a_group_is_not() {
    let rows = tree();
    // Group 1 with its children ends at row 3; the gap above row 4 is its own slot.
    assert_eq!(drop_at(&rows, 81.0, &[1]), None);
    assert_eq!(drop_at(&rows, 79.0, &[1]), None);
    // Child 3 dragged before the top-level row 4: the same slot, a different parent.
    assert_eq!(drop_at(&rows, 81.0, &[3]), Some(mv(&[3], 4, Before)));
    // Child 3 dropped after itself or before itself stays.
    assert_eq!(drop_at(&rows, 69.0, &[3]), None);
}

#[test]
fn items_come_in_display_order() {
    let rows = flat(6);
    assert_eq!(drop_at(&rows, 5.0, &[4, 2]), Some(mv(&[2, 4], 0, Before)));
    assert_eq!(drop_at(&rows, 5.0, &[9]), None);
    assert_eq!(drop_at(&flat(0), 5.0, &[1]), None);
}

#[test]
fn a_horizontal_list_follows_the_reading_direction() {
    let rows = flat(3);
    // Physical spans left to right: 0..20, 20..40, 40..60.
    let left_to_right = Reorder::new(Axis::Horizontal);
    let at = |r: &Reorder<u32>, x| r.resolve(&rows[..], &spans(3), x, &[0], Modifiers::NONE).map(|t| (t.mv.target, t.mv.position));
    assert_eq!(at(&left_to_right, 55.0), Some((2, After)));
    assert_eq!(at(&left_to_right, 45.0), Some((2, Before)));
    // Right to left: row 0 is at the right (40..60), row 2 at the left (0..20).
    let mirrored: Vec<_> = (0..3).map(|i| (60.0 - 20.0 * (i + 1) as f32)..(60.0 - 20.0 * i as f32)).collect();
    let rtl = Reorder::new(Axis::Horizontal).right_to_left(true);
    let resolve = |x| rtl.resolve(&rows[..], &mirrored[..], x, &[0], Modifiers::NONE).map(|t| (t.mv.target, t.mv.position));
    assert_eq!(resolve(5.0), Some((2, After)));
    assert_eq!(resolve(15.0), Some((2, Before)));
    assert_eq!(resolve(25.0), Some((1, After)));
    // Before row 1 is the slot next to the dragged row 0.
    assert_eq!(resolve(35.0), None);
    assert_eq!(resolve(55.0), None);
    assert_eq!(resolve(-30.0), Some((2, After)));
    assert_eq!(resolve(90.0), None);
    // Right to left changes nothing in a vertical list.
    let vertical = Reorder::new(Axis::Vertical).right_to_left(true);
    assert_eq!(vertical.resolve(&rows[..], &spans(3), 45.0, &[0], Modifiers::NONE).map(|t| t.mv.position), Some(Before));
}

fn key_move(rows: &[RowInfo<u32>], selected: &[u32], key: StackKey) -> Option<Move<u32>> {
    Reorder::default().key_move(rows, selected, key, Modifiers::NONE)
}

#[test]
fn the_stacking_keys_map_to_moves_with_the_platform_command() {
    let mac = |k, m| StackKey::from_key(k, m, true);
    let other = |k, m| StackKey::from_key(k, m, false);
    assert_eq!(mac(Key::CloseBracket, Modifiers::MAC_CMD), Some(StackKey::TowardStart));
    assert_eq!(mac(Key::OpenBracket, Modifiers::MAC_CMD), Some(StackKey::TowardEnd));
    assert_eq!(other(Key::CloseBracket, Modifiers::CTRL), Some(StackKey::TowardStart));
    assert_eq!(other(Key::OpenBracket, Modifiers::CTRL), Some(StackKey::TowardEnd));
    let shift_cmd = Modifiers { mac_cmd: true, shift: true, ..Modifiers::NONE };
    assert_eq!(mac(Key::CloseBracket, shift_cmd), Some(StackKey::ToStart));
    assert_eq!(mac(Key::OpenCurlyBracket, shift_cmd), Some(StackKey::ToEnd));
    assert_eq!(mac(Key::CloseBracket, Modifiers::CTRL), None);
    assert_eq!(other(Key::CloseBracket, Modifiers::MAC_CMD), None);
    assert_eq!(other(Key::CloseBracket, Modifiers::NONE), None);
    assert_eq!(other(Key::CloseBracket, Modifiers { ctrl: true, alt: true, ..Modifiers::NONE }), None);
}

#[test]
fn a_key_move_steps_one_row_and_stops_at_the_ends() {
    let rows = flat(5);
    assert_eq!(key_move(&rows, &[2], StackKey::TowardStart), Some(mv(&[2], 1, Before)));
    assert_eq!(key_move(&rows, &[2], StackKey::TowardEnd), Some(mv(&[2], 3, After)));
    assert_eq!(key_move(&rows, &[0], StackKey::TowardStart), None);
    assert_eq!(key_move(&rows, &[4], StackKey::TowardEnd), None);
    assert_eq!(key_move(&rows, &[9], StackKey::TowardEnd), None);
}

#[test]
fn a_key_move_with_shift_goes_to_the_ends_unless_already_there() {
    let rows = flat(5);
    assert_eq!(key_move(&rows, &[2, 3], StackKey::ToStart), Some(mv(&[2, 3], 0, Before)));
    assert_eq!(key_move(&rows, &[1, 2], StackKey::ToEnd), Some(mv(&[1, 2], 4, After)));
    assert_eq!(key_move(&rows, &[0, 1], StackKey::ToStart), None);
    assert_eq!(key_move(&rows, &[3, 4], StackKey::ToEnd), None);
    assert_eq!(key_move(&rows, &[0, 2], StackKey::ToStart), Some(mv(&[0, 2], 0, Before)));
}

#[test]
fn a_key_move_takes_a_selection_with_gaps_as_one_block() {
    let rows = flat(6);
    assert_eq!(key_move(&rows, &[4, 2], StackKey::TowardStart), Some(mv(&[2, 4], 1, Before)));
    assert_eq!(key_move(&rows, &[2, 4], StackKey::TowardEnd), Some(mv(&[2, 4], 5, After)));
}

#[test]
fn a_key_move_goes_past_closed_groups_and_into_open_ones() {
    let rows = tree();
    // Past closed group 3: row 2 steps after it.
    assert_eq!(key_move(&rows, &[2], StackKey::TowardEnd), Some(mv(&[2], 3, After)));
    // Row 4 steps up onto the last child of open group 1, which is closed group 3: a sibling.
    assert_eq!(key_move(&rows, &[4], StackKey::TowardStart), Some(mv(&[4], 3, Before)));
    // Row 4 steps down onto open group 5's children: before child 6.
    assert_eq!(key_move(&rows, &[4], StackKey::TowardEnd), Some(mv(&[4], 6, Before)));
    // A group moves past a leaf as one block with its children.
    assert_eq!(key_move(&rows, &[1], StackKey::TowardEnd), Some(mv(&[1], 4, After)));
}

#[test]
fn a_key_move_leaves_a_group_from_its_first_or_last_child() {
    let rows = tree();
    assert_eq!(key_move(&rows, &[2], StackKey::TowardStart), Some(mv(&[2], 1, Before)));
    assert_eq!(key_move(&rows, &[3], StackKey::TowardEnd), Some(mv(&[3], 4, Before)));
    assert_eq!(key_move(&rows, &[7], StackKey::TowardEnd), None);
    assert_eq!(key_move(&rows, &[0], StackKey::ToEnd), Some(mv(&[0], 5, After)));
    assert_eq!(key_move(&rows, &[4], StackKey::ToStart), Some(mv(&[4], 0, Before)));
}

#[test]
fn a_key_move_asks_accepts() {
    let rows = flat(4);
    let reorder = Reorder::default().accepts(|m: &Move<u32>, _| m.target != 1);
    assert_eq!(reorder.key_move(&rows[..], &[2], StackKey::TowardStart, Modifiers::NONE), None);
    assert!(reorder.key_move(&rows[..], &[2], StackKey::TowardEnd, Modifiers::NONE).is_some());
}

#[test]
fn the_edge_scroll_speed_ramps_from_80_to_600_in_the_zone() {
    // A list from 0 to 400: zone 32.
    assert_eq!(edge_scroll_speed(200.0, 0.0, 400.0), 0.0);
    assert_eq!(edge_scroll_speed(32.0, 0.0, 400.0), 0.0);
    assert!((edge_scroll_speed(16.0, 0.0, 400.0) + 340.0).abs() < 1e-3);
    assert_eq!(edge_scroll_speed(0.0, 0.0, 400.0), -600.0);
    assert_eq!(edge_scroll_speed(-50.0, 0.0, 400.0), -600.0);
    assert!((edge_scroll_speed(384.0, 0.0, 400.0) - 340.0).abs() < 1e-3);
    assert_eq!(edge_scroll_speed(450.0, 0.0, 400.0), 600.0);
    // A short list: the zone is a quarter of it.
    assert_eq!(edge_scroll_speed(30.0, 0.0, 80.0), 0.0);
    assert!(edge_scroll_speed(15.0, 0.0, 80.0) < 0.0);
    assert_eq!(edge_scroll_speed(1.0, 0.0, 0.0), 0.0);
}

#[test]
fn auto_scroll_clamps_the_frame_time_and_needs_the_pointer_within_the_list_across_the_axis() {
    let viewport = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 400.0));
    let vertical = Reorder::<u32>::new(Axis::Vertical);
    assert!((vertical.auto_scroll(pos2(50.0, 399.0), viewport, 0.01) - 6.0).abs() < 0.2);
    assert!((vertical.auto_scroll(pos2(50.0, 0.0), viewport, 1.0) + 30.0).abs() < 1e-3);
    assert_eq!(vertical.auto_scroll(pos2(150.0, 0.0), viewport, 0.01), 0.0);
    assert_eq!(vertical.auto_scroll(pos2(50.0, 0.0), viewport, 0.0), 0.0);
    assert_eq!(vertical.auto_scroll(pos2(50.0, 0.0), viewport, f32::NAN), 0.0);
    let wide = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 30.0));
    let horizontal = Reorder::<u32>::new(Axis::Horizontal);
    assert!(horizontal.auto_scroll(pos2(399.0, 10.0), wide, 0.01) > 0.0);
    assert!(horizontal.auto_scroll(pos2(1.0, 10.0), wide, 0.01) < 0.0);
    assert_eq!(horizontal.auto_scroll(pos2(399.0, 40.0), wide, 0.01), 0.0);
    assert_eq!(horizontal.auto_scroll(pos2(200.0, 10.0), wide, 0.01), 0.0);
}

#[test]
fn a_drag_of_a_selected_row_takes_the_selection_in_display_order() {
    let rows = flat(6);
    let list = Id::unique("list");
    let drag = ListDrag::new(list, &rows[..], &[4, 1], 4, "Four");
    assert_eq!((drag.items, drag.label.as_str(), drag.list), (vec![1, 4], "Four", list));
    let other = ListDrag::new(list, &rows[..], &[4, 1], 2, "Two");
    assert_eq!(other.items, vec![2]);
}

/// Runs one pass in which the control loads its drag gesture, lets `drive` act on it, and
/// returns the pass's event.
fn pass(ctx: &egui::Context, drive: impl FnOnce(&mut ReorderGesture<u32>)) -> Option<EditEvent<Move<u32>>> {
    let (mut event, mut drive) = (None, Some(drive));
    ctx.run_ui(egui::RawInput::default(), |ui| {
        let mut gesture = ReorderGesture::load(ui, Id::unique("strip"));
        if let Some(drive) = drive.take() {
            drive(&mut gesture);
        }
        event = gesture.finish();
    })
    .drop_without_applying_deltas();
    event
}

#[test]
fn a_drag_is_one_gesture_from_begin_to_commit_or_cancel() {
    let ctx = egui::Context::default();
    let begin = pass(&ctx, ReorderGesture::begin);
    let Some(EditEvent::Begin { id, .. }) = begin else { panic!("a begin: {begin:?}") };
    assert_eq!(pass(&ctx, |_| {}), None, "the target under the pointer is not an edit");
    let drop = mv(&[0], 2, After);
    let end = pass(&ctx, |g| g.end(DragEnd::Released(Some(drop.clone())), Modifiers::ALT));
    assert!(matches!(end, Some(EditEvent::Commit { id: ended, ref value, changed: true, .. }) if ended == id && *value == drop), "{end:?}");

    for (drag_end, reason) in [
        (DragEnd::Escape, CancelReason::Escape),
        (DragEnd::FocusLost, CancelReason::FocusLost),
        (DragEnd::Removed, CancelReason::Removed),
        (DragEnd::Released(None), CancelReason::Dismissed),
        (DragEnd::PayloadCleared, CancelReason::Dismissed),
    ] {
        let Some(EditEvent::Begin { id, .. }) = pass(&ctx, ReorderGesture::begin) else { panic!("a begin") };
        assert_eq!(pass(&ctx, |g| g.end(drag_end, Modifiers::NONE)), Some(EditEvent::cancel(id, reason)));
    }

    let set = pass(&ctx, |g| g.set(drop.clone()));
    assert!(matches!(set, Some(EditEvent::Set { ref value, .. }) if *value == drop), "a key's move: {set:?}");
}

#[test]
fn a_drag_ends_on_cancel_all_and_on_a_pass_that_skips_its_control() {
    let ctx = egui::Context::default();
    let Some(EditEvent::Begin { id, .. }) = pass(&ctx, ReorderGesture::begin) else { panic!("a begin") };
    crate::edit::cancel_all(&ctx, CancelReason::External);
    assert_eq!(pass(&ctx, |g| assert!(!g.is_active())), Some(EditEvent::cancel(id, CancelReason::External)));

    let Some(EditEvent::Begin { id, .. }) = pass(&ctx, ReorderGesture::begin) else { panic!("a begin") };
    let mut live = true;
    ctx.run_ui(egui::RawInput::default(), |ui| live = crate::edit::is_live(ui.ctx(), id)).drop_without_applying_deltas();
    assert!(!live, "a pass without the control ends the drag");
    assert_eq!(pass(&ctx, |g| assert!(!g.is_active())), None);
}

/// A document of row orders with an undo history of whole orders.
#[derive(Default)]
struct Orders {
    rows: Vec<u32>,
    undo: Vec<Vec<u32>>,
}

impl crate::edit::UndoMarks for Orders {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(rows) = self.undo.get(mark) {
            self.rows.clone_from(rows);
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

fn apply_order(doc: &mut Orders, mv: Move<u32>) {
    doc.undo.push(doc.rows.clone());
    doc.rows.retain(|r| !mv.items.contains(r));
    let at = doc.rows.iter().position(|&r| r == mv.target).map_or(doc.rows.len(), |at| at + usize::from(mv.position == After));
    doc.rows.splice(at..at, mv.items);
}

type Drive = dyn Fn(&mut ReorderGesture<u32>);

/// One pass of a host: the control (when `drive` is given) and the host's end of frame, which it
/// returns.
fn host_pass(ctx: &egui::Context, host: &mut crate::edit::GestureHost<Orders>, doc: &mut Orders, drive: Option<&Drive>) -> Option<EditEvent<()>> {
    let mut ended = None;
    ctx.run_ui(egui::RawInput::default(), |ui| {
        if let Some(drive) = drive {
            let mut gesture = ReorderGesture::load(ui, Id::unique("strip"));
            drive(&mut gesture);
            if let Some(event) = gesture.finish() {
                host.handle(ui.ctx(), doc, event, apply_order);
            }
        }
        ended = host.end_frame(ui.ctx(), doc);
    })
    .drop_without_applying_deltas();
    ended
}

#[test]
fn a_gesture_host_makes_a_drop_one_undo_step_and_ends_a_drag_whose_control_went_away() {
    let ctx = egui::Context::default();
    let mut host = crate::edit::GestureHost::new();
    let mut doc = Orders { rows: vec![0, 1, 2], undo: Vec::new() };
    host_pass(&ctx, &mut host, &mut doc, Some(&ReorderGesture::begin));
    for _ in 0..3 {
        assert_eq!(host_pass(&ctx, &mut host, &mut doc, Some(&|_| {})), None, "the drag stays live while its control is drawn");
    }
    assert!(host.is_active());
    host_pass(&ctx, &mut host, &mut doc, Some(&|g| g.end(DragEnd::Released(Some(mv(&[0], 2, After))), Modifiers::NONE)));
    assert_eq!((doc.rows.as_slice(), doc.undo.len(), host.is_active()), ([1, 2, 0].as_slice(), 1, false), "one undo step");

    host_pass(&ctx, &mut host, &mut doc, Some(&ReorderGesture::begin));
    let ended = host_pass(&ctx, &mut host, &mut doc, None);
    assert!(matches!(ended, Some(EditEvent::Cancel { reason: CancelReason::Removed, .. })), "{ended:?}");
    assert_eq!((doc.rows.as_slice(), doc.undo.len(), host.is_active()), ([1, 2, 0].as_slice(), 1, false));
}

#[test]
fn a_group_and_its_child_selected_move_as_the_group() {
    let rows = tree();
    // Group 1 and its child 2: the group moves past the next top-level row, not past its child.
    assert_eq!(key_move(&rows, &[1, 2], StackKey::TowardEnd), Some(mv(&[1], 4, After)));
    // Group 5 with child 6 is already at the end.
    assert_eq!(key_move(&rows, &[5, 6], StackKey::ToEnd), None);
    assert_eq!(key_move(&rows, &[5, 6], StackKey::TowardStart), Some(mv(&[5], 4, Before)));
    // A drag of both: the same block, so its own slots are no-ops.
    assert_eq!(drop_at(&rows, 85.0, &[1, 2]), None);
    assert_eq!(drop_at(&rows, 95.0, &[1, 2]), Some(mv(&[1], 4, After)));
    let drag = ListDrag::new(Id::unique("tree"), &rows[..], &[2, 1], 1, "One");
    assert_eq!(drag.items, [1], "the payload holds the group, its child goes with it");
}

#[test]
fn into_the_group_the_rows_already_start_is_a_no_op() {
    let rows = tree();
    // Row 2 is group 1's first child: into group 1 (its middle, 26..34) changes nothing.
    assert_eq!(drop_at(&rows, 30.0, &[2]), None);
    // Its second child goes to the top: a move.
    assert_eq!(drop_at(&rows, 30.0, &[3]), Some(mv(&[3], 1, Into)));
    // Both children, first already first: a no-op; a copy still drops.
    assert_eq!(drop_at(&rows, 30.0, &[2, 3]), None);
    let copy = Reorder::default().copy_with_alt(true).resolve(&rows[..], &spans(rows.len()), 30.0, &[2], Modifiers::ALT).map(|r| r.mv);
    assert_eq!(copy, Some(Move { copy: true, ..mv(&[2], 1, Into) }));
}
