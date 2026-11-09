//! Screen mode: the shell brings its OS window to the bound mode in one pass and follows the OS out of full screen.

use crate::windows_kit::{Out, Windows};
use craft_host::target::{Os, ShellInfo};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::display::Text;
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::env::WindowKey;
use craft_ui::shell::{AppShell, ScreenMode};
use craft_ui::testing::layout::in_region;
use craft_ui::testing::trial;
use craft_ui::testing::windows::FakeSystem;
use egui::{Pos2, Rect, ViewportCommand, ViewportId, ViewportInfo};
use std::sync::Arc;

const WINDOW: Rect = Rect { min: Pos2::ZERO, max: Pos2::new(800.0, 600.0) };
const SETTLED_FRAMES: usize = 3;

fn root(fullscreen: bool, maximized: bool) -> ViewportInfo {
    ViewportInfo {
        focused: Some(true),
        inner_rect: Some(WINDOW),
        outer_rect: Some(WINDOW),
        native_pixels_per_point: Some(1.0),
        fullscreen: Some(fullscreen),
        maximized: Some(maximized),
        ..ViewportInfo::default()
    }
}

/// One frame of a shell whose screen mode is bound to `mode`.
fn frame(windows: &Windows, mode: &mut ScreenMode) -> Out {
    windows.frame(vec![], vec![], |ui| {
        AppShell::new(WindowKey::main())
            .screen_mode(Bind::bound(mode))
            .body(|ui, _, _| {
                Text::new("Document").show(ui);
            })
            .show(ui, &mut ());
    })
}

fn settle(windows: &Windows, mode: &mut ScreenMode) {
    for _ in 0..SETTLED_FRAMES {
        frame(windows, mode);
    }
}

/// The full screen and maximize commands the frame sent to the root window, in order.
fn screen_commands(out: &Out) -> Vec<ViewportCommand> {
    out.commands
        .iter()
        .filter(|(viewport, command)| {
            *viewport == ViewportId::ROOT && matches!(command, ViewportCommand::Fullscreen(_) | ViewportCommand::Maximized(_))
        })
        .map(|(_, command)| command.clone())
        .collect()
}

#[trial]
fn entering_full_screen_sends_its_command_once_and_waits_for_nothing() {
    let windows = Windows::new();
    windows.root_info(root(false, false));
    let mut mode = ScreenMode::Standard;
    settle(&windows, &mut mode);
    mode = ScreenMode::FullScreen;
    assert_eq!(screen_commands(&frame(&windows, &mut mode)), [ViewportCommand::Fullscreen(true)]);
    for _ in 0..SETTLED_FRAMES {
        assert!(screen_commands(&frame(&windows, &mut mode)).is_empty(), "the command is sent once, however long the OS takes");
    }
    assert_eq!(mode, ScreenMode::FullScreen, "a window that has not reported full screen yet keeps the mode");
}

#[trial]
fn writing_standard_into_the_bind_leaves_os_full_screen() {
    let windows = Windows::new();
    windows.root_info(root(false, false));
    let mut mode = ScreenMode::Standard;
    settle(&windows, &mut mode);
    mode = ScreenMode::FullScreen;
    frame(&windows, &mut mode);
    windows.root_info(root(true, false));
    settle(&windows, &mut mode);
    assert_eq!(mode, ScreenMode::FullScreen);
    mode = ScreenMode::Standard;
    assert_eq!(screen_commands(&frame(&windows, &mut mode)), [ViewportCommand::Fullscreen(false)], "the app's own write is carried out");
}

#[trial]
fn an_os_exit_from_full_screen_writes_standard() {
    let windows = Windows::new();
    windows.root_info(root(false, false));
    let mut mode = ScreenMode::Standard;
    settle(&windows, &mut mode);
    mode = ScreenMode::FullScreen;
    frame(&windows, &mut mode);
    windows.root_info(root(true, false));
    settle(&windows, &mut mode);
    assert_eq!(mode, ScreenMode::FullScreen);
    windows.root_info(root(false, false));
    let mut sent = Vec::new();
    for _ in 0..SETTLED_FRAMES {
        sent.extend(screen_commands(&frame(&windows, &mut mode)));
    }
    assert_eq!(mode, ScreenMode::Standard, "the window left full screen on its own, as the macOS green button does");
    assert!(!sent.contains(&ViewportCommand::Fullscreen(true)), "the shell does not send the window back: {sent:?}");
}

#[trial]
fn on_windows_a_maximized_window_leaves_maximized_before_full_screen_and_returns_to_it() {
    let windows = Windows::with(InstallSpec::new(ShellInfo::default().os(Os::Windows), Arc::new(FakeSystem::new())));
    windows.root_info(root(false, true));
    let mut mode = ScreenMode::Standard;
    settle(&windows, &mut mode);
    mode = ScreenMode::FullScreen;
    assert_eq!(
        screen_commands(&frame(&windows, &mut mode)),
        [ViewportCommand::Maximized(false), ViewportCommand::Fullscreen(true)],
        "full screen and maximized never overlap, and both go out in one pass"
    );
    windows.root_info(root(true, false));
    settle(&windows, &mut mode);
    mode = ScreenMode::Standard;
    assert_eq!(screen_commands(&frame(&windows, &mut mode)), [ViewportCommand::Fullscreen(false), ViewportCommand::Maximized(true)]);
}

#[trial]
fn an_embedded_shell_in_full_screen_hides_its_slots_inside_its_region_and_asks_the_window_for_nothing() {
    let windows = Windows::new();
    windows.root_info(root(false, false));
    let mut modes = [ScreenMode::FullScreen, ScreenMode::Standard];
    let mut slots_shown = [true; 2];
    let mut sent = Vec::new();
    for _ in 0..SETTLED_FRAMES {
        let out = windows.frame(vec![], vec![], |ui| {
            let room = ui.max_rect();
            let half = egui::vec2(room.width() / 2.0, room.height());
            let [first, second] = &mut modes;
            for (at, (key, mode)) in [("first", first), ("second", second)].into_iter().enumerate() {
                let rect = Rect::from_min_size(room.min + egui::vec2(half.x * at as f32, 0.0), half);
                in_region(ui, rect, |ui| {
                    let shown = AppShell::embedded(WindowKey::new(key))
                        .screen_mode(Bind::bound(mode))
                        .title_bar(|ui, _, _| {
                            Text::new("Title").show(ui);
                        })
                        .body(|ui, _, _| {
                            Text::new("Document").show(ui);
                        })
                        .show(ui, &mut ());
                    slots_shown[at] = !shown.regions.slots.is_empty();
                });
            }
        });
        sent.extend(screen_commands(&out));
    }
    assert_eq!(slots_shown, [false, true], "full screen hides the slots of its own shell only");
    assert!(sent.is_empty(), "an embedded shell sends no window command: {sent:?}");
    assert_eq!(modes, [ScreenMode::FullScreen, ScreenMode::Standard]);
}
