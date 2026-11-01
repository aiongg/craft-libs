//! Tests of the Gradient Map editor (task AD-6).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Picking a preset with a 50 % opacity stop reports one `Set` whose stops are the preset's colour
// stops with alpha 1 and no opacity stops.
#[test]
#[ignore = "skeleton"]
fn gradient_map_preset_pick_sets_opaque_colour_stops() {
    todo!()
}

// A stop drag on the bar reports `Begin`, `Update` and `Commit` of the map with the new stops and
// Reverse and Dither unchanged.
#[test]
#[ignore = "skeleton"]
fn gradient_map_stop_edits_come_out_as_map_edits() {
    todo!()
}

// The bar keeps two to 64 stops: Delete is disabled at two, a click on the bar adds none at 64.
#[test]
#[ignore = "skeleton"]
fn gradient_map_keeps_two_to_sixty_four_stops() {
    todo!()
}

// Reverse and Dither each report one `Set` with the flag toggled; Import .GRD sets `import_presets`
// and no edit.
#[test]
#[ignore = "skeleton"]
fn gradient_map_reverse_dither_and_import() {
    todo!()
}
