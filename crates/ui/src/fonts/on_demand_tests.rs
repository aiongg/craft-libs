//! On-demand registration in a harness, with a fixed candidate source.

// A Japanese character with no glyph makes the loader read the first candidate that covers it and
// register one face in that pass; the next pass draws it.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_missing_character_registers_the_first_covering_candidate() {
    todo!()
}

// After MAX_SCRIPT_READS files of a script failed to cover, no further file of that script is read.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_script_stops_reading_after_the_cap() {
    todo!()
}

// A `ja` UI language prefers Japanese forms; an `en` UI with a `zh-TW` OS locale prefers Traditional.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn han_preference_follows_ui_language_then_os_locale() {
    todo!()
}

// `deflated_default_definitions` lists egui's faces in egui's family order.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn deflated_defaults_match_egui_defaults() {
    todo!()
}
