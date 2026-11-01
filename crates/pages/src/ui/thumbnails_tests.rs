//! The thumbnails panel in a harness.

// A click reports Picked then Go and resets the picked set; Cmd/Ctrl-click toggles a page; Shift-click extends from the last plain click; neither reports Go.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn clicks_go_and_modifier_clicks_pick() {
    todo!()
}

// Dragging page 6 of picked 6–8 to the gap before page 2 reports Reorder Begin then Commit with Move { items: [5, 6, 7], target: 1, position: Before }; dropping where they are reports Cancel; Esc reports Cancel; moved_page agrees.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn dragging_picked_pages_moves_them_together() {
    todo!()
}

// Delete with the panel focused deletes the picked pages; with every page picked it does nothing; read-only it does nothing.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn delete_needs_the_panel_and_keeps_a_page() {
    todo!()
}

// The minus button steps ZOOM_STEP and stops at 0.5; the slider writes the zoom; thumbnails resize.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn size_row_steps_and_clamps_the_zoom() {
    todo!()
}

// Down moves the lead and reports Go; Shift+Down extends the picked set; a secondary click on an unpicked page reports Picked then ContextMenu.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn keys_and_context_menu_follow_the_list_rules() {
    todo!()
}
