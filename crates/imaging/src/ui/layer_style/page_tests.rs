//! Tests of the effect pages and the Blending Options page (task AD-9).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click on Drop Shadow's Color swatch reports a ColorRequest keyed "fx1/color" with the current
// colour and the dialog's picker title.
#[test]
#[ignore = "skeleton"]
fn colour_field_reports_a_colour_request_with_the_effect_and_key() {
    todo!()
}

// With Use Global Light on, dragging the Angle dial changes `global_light` and every globally lit
// instance; with it off, only this instance's angle.
#[test]
#[ignore = "skeleton"]
fn use_global_light_check_changes_the_angle_field_meaning() {
    todo!()
}

// With channel names C, M, Y, K the page shows four Channels checks; unchecking M reports
// `channels` [true, false, true, true].
#[test]
#[ignore = "skeleton"]
fn blending_options_channel_checks_follow_the_document_channels() {
    todo!()
}

// The Blend If select names Gray, R, G, B for an RGB document with four entries, and only Gray for
// a one-channel document.
#[test]
#[ignore = "skeleton"]
fn blend_if_select_names_gray_first_in_colour_documents() {
    todo!()
}

// A page whose `opacity` is Text shows one invalid-value line naming Opacity and no fields; the
// draft is reported unchanged.
#[test]
#[ignore = "skeleton"]
fn invalid_values_show_one_message_and_are_kept() {
    todo!()
}

// A page shows the spec's defaults for keys the instance lacks without writing them; editing one
// field writes only that key.
#[test]
#[ignore = "skeleton"]
fn page_shows_defaults_under_the_values_and_writes_only_edits() {
    todo!()
}
