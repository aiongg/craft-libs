//! Preview selects.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `LineStyle::label` gives WordCraft's and DesignCraft's names ("Single", "Dot-dash", "Thick - Thin").
#[test]
#[ignore = "skeleton"]
fn line_style_names_follow_the_apps() {
    todo!()
}

// In a harness: with `SelectText::Never` no option text is painted, each row is still named by its text, and type-ahead finds it.
#[test]
#[ignore = "skeleton"]
fn a_select_without_text_keeps_its_names_for_accesskit() {
    todo!()
}

// In a harness: with `SelectText::InList` the closed box paints the content without the text and the list rows paint both.
#[test]
#[ignore = "skeleton"]
fn the_closed_box_shows_only_the_preview_with_in_list() {
    todo!()
}
