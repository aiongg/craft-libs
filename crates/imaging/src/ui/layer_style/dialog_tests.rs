//! Tests of the Layer Style dialog body: the effect list, the Styles page, colour answers (task
//! AD-8).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click on an instance's check box turns it off and selects its page; a click on the row only
// selects it.
#[test]
#[ignore = "skeleton"]
fn effect_list_check_box_toggles_and_selects_the_page() {
    todo!()
}

// A click on the dimmed Satin row reports a Set whose draft has a new Satin, on, selected.
#[test]
#[ignore = "skeleton"]
fn absent_effect_row_adds_the_effect_on() {
    todo!()
}

// `+` on Drop Shadow adds a second instance after the first; `−` shows only with two and removes
// one; Satin rows have neither button.
#[test]
#[ignore = "skeleton"]
fn plus_and_minus_add_and_remove_instances_of_multi_effects() {
    todo!()
}

// Tab reaches the list once; ↑ and ↓ move the lead, Space toggles, Enter selects the page; the
// AccessKit tree has a ListBox of CheckBox rows with positions.
#[test]
#[ignore = "skeleton"]
fn effect_list_is_one_tab_stop_with_arrow_navigation() {
    todo!()
}

// A click on a preset of the Styles page reports `apply_style` with its key and no edit.
#[test]
#[ignore = "skeleton"]
fn styles_page_click_reports_apply_style() {
    todo!()
}

// `set_color(draft, "fx2/color", red)` sets that parameter; an unknown id or a key without a
// slash changes nothing and returns false.
#[test]
#[ignore = "skeleton"]
fn set_color_writes_the_named_parameter_and_ignores_unknown_keys() {
    todo!()
}

// The Preview check box at the title's end flips `draft.preview` as a Set; the render hash stays.
#[test]
#[ignore = "skeleton"]
fn preview_check_box_flips_the_drafts_preview() {
    todo!()
}
