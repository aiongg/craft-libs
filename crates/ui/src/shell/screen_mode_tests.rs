use egui::{
    Context, RawInput,
    ViewportCommand::{Fullscreen, Maximized},
    ViewportId, ViewportInfo,
};

use super::screen_mode::plan;
use super::*;

#[test]
fn next_and_previous_cycle_and_are_inverse() {
    let mut mode = ScreenMode::Standard;
    let mut seen = vec![];
    for _ in 0..3 {
        mode = mode.next();
        seen.push(mode);
    }
    assert_eq!(seen, [ScreenMode::FullScreenWithMenuBar, ScreenMode::FullScreen, ScreenMode::Standard]);
    for mode in [ScreenMode::Standard, ScreenMode::FullScreenWithMenuBar, ScreenMode::FullScreen] {
        assert_eq!(mode.next().previous(), mode);
        assert_eq!(mode.previous().next(), mode);
    }
}

#[test]
fn modes_hide_what_photoshop_hides() {
    // (body_only, tabs_visible, scrollbars_visible)
    let table = |m: ScreenMode| (m.body_only(), m.tabs_visible(), m.scrollbars_visible());
    assert_eq!(table(ScreenMode::Standard), (false, true, true));
    assert_eq!(table(ScreenMode::FullScreenWithMenuBar), (false, false, false));
    assert_eq!(table(ScreenMode::FullScreen), (true, false, false));
}

// Port of PhotoCraft's `full_screen_and_maximized_never_overlap_on_windows`.
#[test]
fn full_screen_and_maximized_never_overlap_on_windows() {
    // Maximized to full screen: un-maximize first, and remember it.
    assert_eq!(plan(true, false, true, false, true), (vec![Maximized(false), Fullscreen(true)], true));
    // Full screen to the other full-screen mode: nothing about maximized changes.
    assert_eq!(plan(true, true, false, true, true), (vec![Fullscreen(true)], true));
    // Full screen to Standard: maximize again after leaving, then forget.
    assert_eq!(plan(true, true, false, true, false), (vec![Fullscreen(false), Maximized(true)], false));
    // A window that was not maximized goes in and out untouched.
    assert_eq!(plan(true, false, false, false, true), (vec![Fullscreen(true)], false));
    assert_eq!(plan(true, true, false, false, false), (vec![Fullscreen(false)], false));
    // Standard while already standard never maximizes.
    assert_eq!(plan(true, false, false, true, false).0, vec![Fullscreen(false)]);
    // Elsewhere the commands are as before.
    assert_eq!(plan(false, false, true, false, true), (vec![Fullscreen(true)], false));
}

fn run(ctx: &Context, fullscreen: bool, maximized: bool, f: impl FnOnce(&Context)) {
    let info = ViewportInfo { fullscreen: Some(fullscreen), maximized: Some(maximized), ..Default::default() };
    let mut raw = RawInput::default();
    raw.viewports.insert(ViewportId::ROOT, info);
    let mut f = Some(f);
    let mut out = ctx.run_ui(raw, |ui| (f.take().expect("once"))(ui.ctx()));
    out.textures_delta.clear();
}

#[test]
fn the_was_maximized_memory_is_per_window() {
    let ctx = Context::default();
    let (a, b) = (WindowKey::new("a"), WindowKey::new("b"));
    let mut cmds = vec![];
    run(&ctx, false, true, |ctx| {
        cmds = full_screen_commands(ctx, &a, ScreenMode::FullScreen, true);
    });
    assert_eq!(cmds, [Maximized(false), Fullscreen(true)]);
    // Window b remembers nothing: leaving full screen does not maximize it.
    run(&ctx, true, false, |ctx| {
        cmds = full_screen_commands(ctx, &b, ScreenMode::Standard, true);
    });
    assert_eq!(cmds, [Fullscreen(false)]);
    // Window a does.
    run(&ctx, true, false, |ctx| {
        cmds = full_screen_commands(ctx, &a, ScreenMode::Standard, true);
    });
    assert_eq!(cmds, [Fullscreen(false), Maximized(true)]);
    // And forgets afterwards.
    run(&ctx, true, false, |ctx| {
        cmds = full_screen_commands(ctx, &a, ScreenMode::Standard, true);
    });
    assert_eq!(cmds, [Fullscreen(false)]);
}

#[test]
fn screen_mode_command_ids_round_trip() {
    for mode in [ScreenMode::Standard, ScreenMode::FullScreenWithMenuBar, ScreenMode::FullScreen] {
        assert_eq!(ScreenMode::from_command_id(mode.command_id()), Some(mode));
    }
    assert_eq!(ScreenMode::FullScreenWithMenuBar.command_id(), "view.screenMode.fullScreenWithMenuBar");
    assert_eq!(ScreenMode::from_command_id(CYCLE_COMMAND), None);
    assert_eq!(ScreenMode::from_command_id("view.zoomIn"), None);
}

#[test]
fn screen_mode_menu_marks_the_current_mode() {
    use crate::menu::ItemKind;
    let labels = ScreenModeLabels::default();
    let entries = ScreenMode::FullScreen.menu_entries(&labels);
    let rows: Vec<_> = entries.iter().filter_map(|e| e.item()).map(|i| (i.label().to_owned(), i.command().copied(), i.item_kind())).collect();
    assert_eq!(
        rows,
        [
            ("Standard Screen Mode".to_owned(), Some(ScreenMode::Standard), ItemKind::Radio(false)),
            ("Full Screen Mode With Menu Bar".to_owned(), Some(ScreenMode::FullScreenWithMenuBar), ItemKind::Radio(false)),
            ("Full Screen Mode".to_owned(), Some(ScreenMode::FullScreen), ItemKind::Radio(true)),
        ]
    );
    assert_eq!(entries.len(), 3);
}
