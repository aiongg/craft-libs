//! Tests of the grip set (task K12).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A grip under the pointer (within warm_pad) is Warm, a hot grip is Hot over Warm, the rest Cold.
#[test]
#[ignore = "skeleton"]
fn grip_states_cold_warm_hot() {
    todo!()
}

// The hit area is each shape's bounds plus hit_pad: square, midpoint rectangle, arrow along its
// screen direction, triangle.
#[test]
#[ignore = "skeleton"]
fn grip_hit_pad_and_shape_bounds() {
    todo!()
}

// A press and release on a grip with ⇧ reports `GripClick { shift: true }`; Enter and ⇧+Enter on the
// picked grip do the same.
#[test]
#[ignore = "skeleton"]
fn grip_shift_click_reports_modifier() {
    todo!()
}

// A drag reports Begin, Update and Commit of `GripDrag`; an arrow on the picked grip one `Set`.
#[test]
#[ignore = "skeleton"]
fn grip_drag_and_arrows_report_edit_events() {
    todo!()
}

// 250 grips: all drawn, 200 parts in the AccessKit tree and the group's value names the rest.
#[test]
#[ignore = "skeleton"]
fn grip_access_is_capped_at_200() {
    todo!()
}
