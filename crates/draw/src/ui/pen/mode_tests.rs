//! Tests of the path mode table and the Path Options popover (task PE-B5).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `PathMode::resolve` over every row of its table: each tool, drawing or not, Auto Add/Delete on
// and off, with no key, ⇧, ⌥ and ⌘.
#[test]
#[ignore = "skeleton"]
fn path_mode_resolves_the_pen_tools_and_modifiers() {
    todo!()
}

// `PathOptionsSettings::default()` has the rubber band off and no colour or thickness; JSON
// round-trips and `{}` deserialises to the default.
#[test]
#[ignore = "skeleton"]
fn path_options_settings_default_to_photoshops() {
    todo!()
}

// The gear opens the popover; the checkbox reports a Set with the rubber band on; the swatch
// reports `pick_color` keyed `pathOptions.color`; Esc closes and returns focus to the gear.
#[test]
#[ignore = "skeleton"]
fn path_options_popover_reports_changes_and_asks_for_the_colour() {
    todo!()
}
