//! Tests of the Warp options-bar group (task TR-T5).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A one-patch mesh reads Default, even 3 × 3 to 5 × 5 read their entries, a split mesh Custom;
// picking 4 x 4 reports Grid(4).
#[test]
#[ignore = "skeleton"]
fn the_grid_menu_reads_default_even_and_custom() {
    todo!()
}

// On a type layer Split and Grid are disabled and the style select offers no Custom.
#[test]
#[ignore = "skeleton"]
fn a_type_layer_greys_split_and_grid_and_hides_custom() {
    todo!()
}

// On a preset style the orientation and Bend, H, V report edits; on Custom they are disabled.
#[test]
#[ignore = "skeleton"]
fn bend_and_distortion_fields_act_only_on_a_preset_style() {
    todo!()
}

// A press on a Split icon reports it armed; a press on the armed one reports None.
#[test]
#[ignore = "skeleton"]
fn an_armed_split_tool_is_reported_and_toggles_off() {
    todo!()
}
