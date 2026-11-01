//! `MediaBin` behaviour through egui_kittest (tasks B11, B12).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A search lists matching items of sub-bins, flat; clearing it shows the bin again (B11).
#[test]
#[ignore = "skeleton"]
fn a_search_shows_matches_from_sub_bins() {
    todo!()
}

// A click on the selected item's name, then nothing for the double-click time: the rename editor opens (B11).
#[test]
#[ignore = "skeleton"]
fn a_click_on_the_selected_name_renames_after_the_double_click_time() {
    todo!()
}

// Double click plain, with Shift, with Cmd: OpenBin InPlace, NewTab, NewWindow (B11).
#[test]
#[ignore = "skeleton"]
fn a_double_click_on_a_bin_reports_how_to_open_it() {
    todo!()
}

// A drag released over a bin row reports Drop into it; over the background, into None (B11).
#[test]
#[ignore = "skeleton"]
fn dropping_on_a_bin_row_reports_the_target() {
    todo!()
}

// A drag released over a footer action that accepts drops reports FooterDrop with the items (B11).
#[test]
#[ignore = "skeleton"]
fn a_drop_on_a_footer_action_reports_it() {
    todo!()
}

// FilmCraft #580: Esc during the zoom slider drag gives Cancel and the size it started from (B11).
#[test]
#[ignore = "skeleton"]
fn escape_during_the_zoom_drag_restores_the_size() {
    todo!()
}

// A Freeform drag moves the selection and the dragged card's stack; Esc puts them back; release commits once (B12).
#[test]
#[ignore = "skeleton"]
fn freeform_drag_moves_the_stack_and_escape_puts_it_back() {
    todo!()
}

// Icon view: hover scrub reports the fraction and draws the playhead; bins are cards before items (B12).
#[test]
#[ignore = "skeleton"]
fn the_icon_view_shows_bins_first_and_scrubs_items() {
    todo!()
}
