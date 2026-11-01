//! Tests of the Levels panel (task AD-1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Auto on RGB with a fake histogram reports one `Set` of the shown channel: inputs at the 0.1 % clips, gamma 1, outputs kept.
#[test]
#[ignore = "skeleton"]
fn levels_panel_auto_sets_black_and_white_inputs_from_a_tenth_of_a_percent_clip() {
    todo!()
}

// Picking Red in the Channel select and dragging a thumb reports channel 1 with Red's levels.
#[test]
#[ignore = "skeleton"]
fn levels_panel_reports_the_shown_channel_with_the_edit() {
    todo!()
}

// Esc during a thumb drag reports `Cancel`; the panel's value is unchanged (the host restores).
#[test]
#[ignore = "skeleton"]
fn levels_panel_cancel_event_does_not_change_the_value() {
    todo!()
}

// `Levels::auto` copies PhotoCraft's loops: clip = total / 1000, lo < 253, hi > lo + 2 (an all-empty histogram gives 253 and 255).
#[test]
#[ignore = "skeleton"]
fn levels_auto_follows_the_clip_rule_at_both_ends() {
    todo!()
}
