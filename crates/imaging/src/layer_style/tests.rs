//! Tests of the Layer Style model (task AD-7).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Per effect, the field keys in order with their kinds and ranges match the table in
// `EffectSpec::photoshop`'s doc (Outer Glow: Technique before Spread, Range after Contour).
#[test]
#[ignore = "skeleton"]
fn effect_tables_match_photoshop_order_and_ranges() {
    todo!()
}

// The ten tables' defaults equal PhotoCraft's engine `effect_defaults` (Drop Shadow Multiply,
// black, 75, 120°, global light, 5, 0, 5; Satin Multiply, 50, 19°, 11, 14, anti-aliased, inverted).
#[test]
#[ignore = "skeleton"]
fn effect_defaults_match_the_photocraft_engine_defaults() {
    todo!()
}

// With fx1 Stroke, fx2 Drop Shadow, fx5 Stroke: adding a Stroke inserts fx6 after fx5 and selects
// it; adding a Satin appends it.
#[test]
#[ignore = "skeleton"]
fn adding_an_instance_places_it_after_the_last_of_its_kind_and_numbers_ids() {
    todo!()
}

// Removing the selected of two Drop Shadows selects the other; removing the only one selects the
// Styles page; removing an unselected one keeps the page.
#[test]
#[ignore = "skeleton"]
fn removing_an_instance_selects_a_sibling_of_the_same_kind_else_the_styles_page() {
    todo!()
}

// `allows_several` is true for exactly Stroke, Drop Shadow, Inner Shadow, Color Overlay and
// Gradient Overlay.
#[test]
#[ignore = "skeleton"]
fn only_five_kinds_allow_several_instances() {
    todo!()
}

// Setting a globally lit Drop Shadow's angle sets `global_light` and the angle of every globally
// lit instance, a disabled Bevel too; turning Use Global Light on adopts the shared angle.
#[test]
#[ignore = "skeleton"]
fn global_light_angle_is_shared_by_effects_that_use_it() {
    todo!()
}

// From 120° and 5 px, a drag of (4.5, −2.2) lands on whole degrees and pixels by the offset rule;
// with Use Global Light the shared angle follows; a drag back under 0.5 px keeps the angle.
#[test]
#[ignore = "skeleton"]
fn shadow_drag_uses_whole_degrees_and_pixels_and_moves_the_global_angle() {
    todo!()
}

// `BlendIfRange::ordered` and `moved` always give black low ≤ black high ≤ white low ≤ white
// high, pushing the other handles.
#[test]
#[ignore = "skeleton"]
fn blend_if_edits_keep_the_four_values_ordered() {
    todo!()
}

// An unsplit move of the black pair [10, 30] by +20 gives [30, 50]; with `split` only the moved
// half changes.
#[test]
#[ignore = "skeleton"]
fn blend_if_unsplit_move_keeps_the_spread_and_alt_move_splits() {
    todo!()
}

// Two drafts with the same style on different pages, Blend If channels and Preview states hash
// equally; a changed opacity changes the hash.
#[test]
#[ignore = "skeleton"]
fn layer_style_equal_values_have_equal_render_hash_ignoring_the_selected_page() {
    todo!()
}

// A `StyleValue::Other` key no field shows survives `set_param`, `toggle` and
// `StyleParams::over` unchanged.
#[test]
#[ignore = "skeleton"]
fn the_layer_style_snapshot_is_kept_for_fields_the_dialog_does_not_model() {
    todo!()
}

// `Contour::from_points` rejects one point, 65 points and NaN, sorts and clamps, collapses equal
// inputs; `edited` renames a preset to "Custom" and keeps an imported name.
#[test]
#[ignore = "skeleton"]
fn contours_are_validated_and_an_edited_preset_becomes_custom() {
    todo!()
}
