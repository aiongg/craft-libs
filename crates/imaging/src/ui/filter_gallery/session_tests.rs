//! Tests of Filter Gallery's catalog and stack (task WS-FG1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Picking a filter for the selected layer resets its fields to the filter's defaults and keeps its
// eye; the same filter changes nothing.
#[test]
#[ignore = "skeleton"]
fn gallery_thumbnail_click_sets_the_selected_effects_filter() {
    todo!()
}

// New copies the selected layer above it and selects it; Delete keeps the last layer; Move and the
// eye change one layer; the top row is applied last.
#[test]
#[ignore = "skeleton"]
fn gallery_stack_new_delete_reorder_and_eye() {
    todo!()
}

// `GalleryFilter::defaults` takes each Range's default, each Choice's default or first option,
// each Bool, and the colour.
#[test]
#[ignore = "skeleton"]
fn gallery_settings_come_from_the_catalog_params() {
    todo!()
}

// Undo of a Delete gives the layer back and clamps the selection.
#[test]
#[ignore = "skeleton"]
fn gallery_undo_restores_the_stack_and_clamps_the_selection() {
    todo!()
}
