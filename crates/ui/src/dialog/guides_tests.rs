//! `NewGuideDialog`, `GuideLayoutDialog` and `GuideLayout::lines`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// New Guide opens with the remembered orientation, colour and position in the current ruler unit;
// "2.5 cm" answers 2.5 cm and its pixels at the document's resolution, a bare "12" gets the ruler
// unit, "abc" keeps OK disabled with the position error; Enter is OK.
#[test]
#[ignore = "skeleton"]
fn new_guide_reads_any_unit_and_opens_with_the_last_guide() {
    todo!()
}

// Edit Selected Guides opens on the guide's orientation, position and colour; color_changed is
// false until the colour list changes; Custom… sets wants_color and set_custom_color answers it.
#[test]
#[ignore = "skeleton"]
fn edit_selected_guides_reports_only_a_changed_colour() {
    todo!()
}

// 8 columns with a 20 px gutter over 1000 px give 16 vertical edges at PhotoCraft's positions; a
// fixed width with Center Columns centres them; margins add their four edges; rows off add none;
// a layout wider than the room is DoesNotFit, margins over the size MarginsTooLarge, all off
// Nothing, over 16 000 edges TooMany; Clear Existing keeps guides outside the artboards.
#[test]
#[ignore = "skeleton"]
fn layout_lines_match_photocraft() {
    todo!()
}

// Preview gives the full set while valid, keeps the last valid set while a field is invalid, and
// is None with Preview off; a typed edit turns a named preset to Custom, a target change doesn't;
// Alt+Cancel resets to the default layout.
#[test]
#[ignore = "skeleton"]
fn layout_dialog_previews_live_and_tracks_the_preset() {
    todo!()
}
