//! The page and artboard list.

// A click reports Picked then Activate for row 2; Shift-click on row 5 picks 2–5; Cmd/Ctrl-click on 3 unpicks it.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn clicks_activate_and_pick_ranges() {
    todo!()
}

// F2 on the lead, typing "Cover" and Enter reports Rename { row, text: "Cover" } once; Esc reports nothing.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn rename_in_place_goes_to_every_picked_row() {
    todo!()
}

// Delete with rows 1–3 picked and row 2 locked reports Delete([1, 3]); a row dragged onto New reports New { duplicate: Some(row), .. }.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn delete_leaves_locked_rows() {
    todo!()
}
