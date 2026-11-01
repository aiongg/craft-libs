//! Tests of the display order and the click and reveal rules (L5).

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `display_rows` follows `children()` top first and leaves out closed groups' contents.
#[test]
#[ignore = "skeleton"]
fn display_order_lists_top_first_and_hides_closed_groups() {
    todo!()
}

// With an active filter, matches inside closed groups are listed at their depth and the groups are
// left out.
#[test]
#[ignore = "skeleton"]
fn filtered_view_keeps_matches_inside_closed_groups_at_their_depth() {
    todo!()
}

// `thumb_click_replaces_select`: Cmd on a loadable thumbnail, Shift or Alt on a mask, Alt on the
// pixel thumbnail; Cmd on the Background's thumbnail is a plain click.
#[test]
#[ignore = "skeleton"]
fn modified_thumbnail_clicks_replace_the_select_event() {
    todo!()
}

// `reveal_opens` opens the closed ancestors outermost first when the active layer changes, and not
// on the first pass.
#[test]
#[ignore = "skeleton"]
fn active_layer_change_opens_ancestors_and_scrolls_once() {
    todo!()
}
