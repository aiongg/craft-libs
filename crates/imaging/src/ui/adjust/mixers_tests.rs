//! Tests of Black & White, Channel Mixer and Selective Color (task AD-5), and the AccessKit test over every
//! adjustment editor, which needs the bodies of tasks AD-1 to AD-5.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Default reports one `Set` with the weights 40, 60, 40, 60, 20, 80 and the tint unchanged.
#[test]
#[ignore = "skeleton"]
fn black_white_default_button_restores_the_six_defaults() {
    todo!()
}

// A click on the tint swatch with Tint on reports a `ColorRequest` keyed `blackWhite.tint`; with Tint off the swatch is disabled.
#[test]
#[ignore = "skeleton"]
fn black_white_tint_swatch_asks_for_a_colour() {
    todo!()
}

// Monochrome on reports a `Set` with `gray` = [40, 40, 20, 0] and the select shows Gray; off reports `gray: None`.
#[test]
#[ignore = "skeleton"]
fn channel_mixer_monochrome_seeds_the_gray_row_and_off_removes_it() {
    todo!()
}

// A total of 120 % shows "+120%" in `status.warn.fg` and the accessible value adds "over 100 percent"; 100 % shows the normal colour.
#[test]
#[ignore = "skeleton"]
fn channel_mixer_total_turns_warn_above_one_hundred_percent() {
    todo!()
}

// Picking Neutrals and moving Black reports a value whose only change is range 7, slot 3.
#[test]
#[ignore = "skeleton"]
fn selective_color_edits_the_shown_range_only() {
    todo!()
}

// Every editor's controls have AccessKit names and values (Levels and Curves groups, picker toggles, slider rows, spectrum images, range-thumb sliders, the Tone radio group, the Total label).
#[test]
#[ignore = "skeleton"]
fn adjust_editors_have_accesskit_names_and_values() {
    todo!()
}
