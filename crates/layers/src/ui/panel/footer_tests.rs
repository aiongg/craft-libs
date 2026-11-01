//! Screen tests of the footer and the panel menu (L7): `footer::show_footer` shown alone, and
//! `panel_menu` with `run_command`.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Link, fx, Mask, Adjustment, Group, New, Delete from the reading start.
#[test]
#[ignore = "skeleton"]
fn footer_order_is_link_fx_mask_adjustment_group_new_delete() {
    todo!()
}

// New Layer: Alt asks, Cmd below; New Group: Shift or two selected from the selection, Alt asks,
// Cmd below; Delete: Alt doesn't confirm; Mask: Alt inverts.
#[test]
#[ignore = "skeleton"]
fn footer_modifiers_report_ask_below_and_from_selection() {
    todo!()
}

// The adjustment menu opens on the press and a release over an item reports `Adjustment(command)`.
#[test]
#[ignore = "skeleton"]
fn adjustment_menu_opens_on_press_and_release_chooses() {
    todo!()
}

// Show Filters, Clip Thumbnails, Panel Options… and Collapse All Groups: the entries and what
// `run_command` does with each.
#[test]
#[ignore = "skeleton"]
fn panel_menu_entries_and_commands_change_the_settings() {
    todo!()
}

// The footer buttons are named by their words (Delete and Mask by the app's labels), the menu
// buttons have `HasPopup`.
#[test]
#[ignore = "skeleton"]
fn footer_and_toggle_cells_are_named() {
    todo!()
}
