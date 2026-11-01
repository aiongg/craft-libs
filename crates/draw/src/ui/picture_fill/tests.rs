//! Tests of the picture fill editor (task SH-T13).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Tile shows Offset X and Y, Scale X and Y, Alignment and Mirror type; Stretch shows the four
// offsets; turning Tile on and off keeps each mode's values.
#[test]
#[ignore = "skeleton"]
fn picture_editor_tile_fields_and_stretch_offsets() {
    todo!()
}

// Insert… with a fake `Files` answering "photo.png" reports `Source { name: "photo.png", bytes }`
// once the job reads it; a cancelled pick reports nothing.
#[test]
#[ignore = "skeleton"]
fn picture_editor_source_pick_reports_the_picked_image() {
    todo!()
}
