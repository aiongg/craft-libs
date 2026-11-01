//! `TileGrid` behaviour through egui_kittest (task B6).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// 50 000 fake tiles: `content` is asked only for rows in view ± one viewport; thumbnails VISIBLE and NEAR accordingly.
#[test]
#[ignore = "skeleton"]
fn only_tiles_in_view_and_one_viewport_around_are_built() {
    todo!()
}

// Click replaces, Shift ranges from the anchor, Cmd toggles, background click clears.
#[test]
#[ignore = "skeleton"]
fn click_shift_and_command_pick_as_in_a_list() {
    todo!()
}

// LightCraft's `vertical_neighbor`: Down from a wide tile goes to the tile below its centre.
#[test]
#[ignore = "skeleton"]
fn arrows_move_the_lead_over_ragged_rows_by_centre() {
    todo!()
}

// Space reports Preview; with check mode or SpaceAction::Toggle it toggles; Ctrl+Space always toggles.
#[test]
#[ignore = "skeleton"]
fn space_previews_and_toggles_in_check_mode() {
    todo!()
}

// CommandDigits: Cmd+3 rates, 3 does nothing; Digits: 3 rates, 0 clears.
#[test]
#[ignore = "skeleton"]
fn rating_keys_follow_the_chosen_set() {
    todo!()
}

// A star click on a selected tile rates the whole selection; on another tile, that tile only.
#[test]
#[ignore = "skeleton"]
fn a_star_click_rates_the_whole_selection_when_the_tile_is_in_it() {
    todo!()
}

// Dragging an unselected tile replaces the selection; with Cmd it joins; Drag is reported once.
#[test]
#[ignore = "skeleton"]
fn a_drag_carries_the_selection_and_joins_the_grabbed_tile() {
    todo!()
}

// Begin, then Commit with one Move on release; Esc gives Cancel and no Commit.
#[test]
#[ignore = "skeleton"]
fn reorder_commits_one_move_and_esc_cancels() {
    todo!()
}

// The selection follows the rectangle live; Esc restores the selection from before.
#[test]
#[ignore = "skeleton"]
fn marquee_selects_live_and_escape_restores() {
    todo!()
}

// Pointer at a quarter of the picture: 0.25; in RTL: 0.75.
#[test]
#[ignore = "skeleton"]
fn hover_scrub_reports_the_fraction_from_the_reading_start() {
    todo!()
}

// LightCraft's `shift_click_sets_the_range_to_the_clicked_state`.
#[test]
#[ignore = "skeleton"]
fn check_mode_shift_click_sets_the_range_to_the_clicked_state() {
    todo!()
}

// A disabled tile is skipped by Shift ranges, Cmd/Ctrl+A and arrows, and can't be checked.
#[test]
#[ignore = "skeleton"]
fn a_disabled_tile_is_skipped_by_ranges_and_keys() {
    todo!()
}

// LightCraft's `a_badge_never_covers_the_checkbox` (`badge_rects`).
#[test]
#[ignore = "skeleton"]
fn a_badge_never_covers_the_checkbox() {
    todo!()
}

// Scrolling: the pinned header is the top group's; as the next header arrives it pushes it up.
#[test]
#[ignore = "skeleton"]
fn pinned_group_header_is_pushed_up_by_the_next() {
    todo!()
}

// Each built tile is a ListBoxOption with position_in_set and size_of_set of the full count.
#[test]
#[ignore = "skeleton"]
fn tiles_tell_their_position_in_the_whole_set() {
    todo!()
}
