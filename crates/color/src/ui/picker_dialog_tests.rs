//! Behaviour tests of `picker_dialog`, to be written by the fill task (names only in the skeleton).

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

/// The hex field has the focus with its text selected when the dialog opens.
#[test]
#[ignore = "skeleton"]
fn opening_focuses_the_hex_field() {
    todo!()
}

/// Only Web Colors and the field's component are recalled on open and stored on OK, not on Cancel.
#[test]
#[ignore = "skeleton"]
fn mode_and_web_colors_are_remembered_on_ok_only() {
    todo!()
}

/// With Only Web Colors on, the opening colour is snapped and OK without an edit returns it unchanged as the compare current.
#[test]
#[ignore = "skeleton"]
fn web_only_snaps_the_opening_colour_and_current_follows() {
    todo!()
}

/// Alt turns Cancel into Reset, which restores the opening colour and keeps the dialog open.
#[test]
#[ignore = "skeleton"]
fn alt_cancel_restores_the_opening_colour() {
    todo!()
}

/// Add to Swatches keeps the dialog open and hands the colour to take_add_to_swatches once.
#[test]
#[ignore = "skeleton"]
fn add_to_swatches_keeps_the_dialog_open() {
    todo!()
}

/// A colour delivered by the canvas dropper while the dialog is on top becomes the new colour.
#[test]
#[ignore = "skeleton"]
fn a_canvas_sample_sets_the_new_colour_while_on_top() {
    todo!()
}

/// Typed CMYK inks come back in the result's model values while they still give the colour.
#[test]
#[ignore = "skeleton"]
fn typed_cmyk_inks_come_back_in_the_result() {
    todo!()
}

/// OK answers the key the dialog was opened with.
#[test]
#[ignore = "skeleton"]
fn ok_answers_the_opening_key() {
    todo!()
}
