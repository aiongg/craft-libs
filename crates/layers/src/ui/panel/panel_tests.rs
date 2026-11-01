//! Behaviour tests of the whole panel over `MockLayers` (L9: they need L1 to L8's bodies).

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// An eye click is one `Toggle(Set)`, Alt-click `ShowOnly`, a sweep down the eyes one gesture.
#[test]
#[ignore = "skeleton"]
fn eye_click_alt_click_and_sweep_report_one_event_or_gesture() {
    todo!()
}

// A pick in the eye's colour menu reports `LabelColor` for every selected layer.
#[test]
#[ignore = "skeleton"]
fn eye_colour_menu_reports_the_pick_for_every_selected_layer() {
    todo!()
}

// A click on the lock indicator reports the lock `Toggle` off and no `Select`.
#[test]
#[ignore = "skeleton"]
fn lock_icon_click_unlocks_without_selecting() {
    todo!()
}

// The panel's minimum width holds a group row with one mask at each thumbnail size.
#[test]
#[ignore = "skeleton"]
fn panel_min_width_holds_a_group_row_with_one_mask() {
    todo!()
}

// A click on a mask chain reports `MaskLink` with the opposite state.
#[test]
#[ignore = "skeleton"]
fn mask_chain_click_reports_the_link_change() {
    todo!()
}

// With Cmd held over a loadable thumbnail the cursor is the pointing hand with the operation's
// badge.
#[test]
#[ignore = "skeleton"]
fn cmd_over_a_thumbnail_shows_the_hand_with_the_mode_badge() {
    todo!()
}

// A double click on a renamable name opens the rename; on the Background's name it reports
// `Activate { Name }`; on a thumbnail `Activate { Thumbnail(part) }`.
#[test]
#[ignore = "skeleton"]
fn double_click_name_renames_and_the_background_reports_activate() {
    todo!()
}

// Enter and Tab commit, Esc cancels, an empty or unchanged name reports nothing.
#[test]
#[ignore = "skeleton"]
fn rename_rules_enter_tab_esc_empty_unchanged() {
    todo!()
}

// The list keys walk rows, open groups, activate and rename.
#[test]
#[ignore = "skeleton"]
fn keys_walk_rows_open_groups_activate_and_rename() {
    todo!()
}

// Shift+→ reports `FxOpen` for the lead, Alt+Shift+→ for every layer.
#[test]
#[ignore = "skeleton"]
fn shift_right_opens_effects_and_alt_opens_all_layers() {
    todo!()
}

// Alt-click on a group triangle reports `Open` with `all`.
#[test]
#[ignore = "skeleton"]
fn group_triangle_alt_click_opens_every_group() {
    todo!()
}

// A click below the last row reports `Select` with `Clear`.
#[test]
#[ignore = "skeleton"]
fn empty_space_click_clears_the_selection() {
    todo!()
}

// A drag of a selected row moves the whole selection as one `Drop` gesture, with the marker and
// the ghost.
#[test]
#[ignore = "skeleton"]
fn layer_drag_moves_the_selection_as_one_gesture_with_marker_and_ghost() {
    todo!()
}

// A group row's 0.3 to 0.7 band drops Into, the halves Before and After, the gaps and the space
// below the last row are drop zones.
#[test]
#[ignore = "skeleton"]
fn drop_zones_into_above_below_and_between_rows() {
    todo!()
}

// Alt during a drag copies and shows the copy cursor.
#[test]
#[ignore = "skeleton"]
fn alt_drag_copies_and_shows_the_copy_cursor() {
    todo!()
}

// Esc and a release outside the panel end the drag as `Cancel`.
#[test]
#[ignore = "skeleton"]
fn esc_and_release_elsewhere_cancel_a_drag() {
    todo!()
}

// Layer rows, effects rows and a mask dropped on Delete, New Layer and New Group report
// `FooterDrop`; New Layer and New Group refuse effects and masks.
#[test]
#[ignore = "skeleton"]
fn rows_masks_and_effects_dropped_on_footer_buttons_report_footer_drop() {
    todo!()
}

// The list scrolls while a drag is near its top or bottom edge.
#[test]
#[ignore = "skeleton"]
fn edge_scroll_runs_while_a_drag_is_near_the_edge() {
    todo!()
}

// `.lock_column(true)` adds a lock column whose clicks and sweeps report `Toggle` with `Lock`.
#[test]
#[ignore = "skeleton"]
fn lock_column_reports_toggle_events() {
    todo!()
}
