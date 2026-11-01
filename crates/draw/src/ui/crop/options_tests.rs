//! Tests of the crop settings model (task TO-C1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Choosing each of `CROP_RATIOS` sets the ratio and its fields; each of the eight `CROP_SIZES` fills W, H and the resolution in px/in and turns W x H x Resolution on; `choice` shows them back.
#[test]
#[ignore = "skeleton"]
fn ratio_presets_and_size_presets_fill_the_fields() {
    todo!()
}

// "4 in", "4in", "10,5 cm" and "1024" parse; a bare number keeps the field's unit (pixels when empty); zero, negative, percent and unknown units do not; normalising keeps the previous text for junk.
#[test]
#[ignore = "skeleton"]
fn lengths_parse_with_units_and_a_bare_number_keeps_the_field_unit() {
    todo!()
}

// `target` converts physical lengths at the typed resolution (px/cm × 2.54), else at the document's; points at the document's points per inch.
#[test]
#[ignore = "skeleton"]
fn target_uses_the_typed_resolution_else_the_documents() {
    todo!()
}

// W or H empty: no target and no ratio; one ratio side typed: a free ratio that keeps the side.
#[test]
#[ignore = "skeleton"]
fn empty_w_or_h_is_a_free_crop() {
    todo!()
}

// X swaps W and H in W x H x Resolution mode and the ratio's sides otherwise (Original turns the document's size); Front Image fills the document's size in px and its resolution in the field's unit.
#[test]
#[ignore = "skeleton"]
fn swap_and_front_image() {
    todo!()
}

// "4 x 5 in 300 ppi", "2 : 3", mixed units, a px/cm resolution; `None` for Original and a free ratio.
#[test]
#[ignore = "skeleton"]
fn preset_name_is_made_from_the_values() {
    todo!()
}

// An empty or partial serialised `CropSettings` loads with every other field at its default (Delete Cropped Pixels on, Rule of Thirds, Auto).
#[test]
#[ignore = "skeleton"]
fn older_settings_load_with_defaults() {
    todo!()
}
