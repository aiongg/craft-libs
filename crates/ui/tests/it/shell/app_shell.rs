//! The app shell's own rules: the screen-mode cycle, the full-screen steps, the reveal rules and the slot order.

use craft_ui::contract::{Bind, PaneEdge, Show as _};
use craft_ui::display::Text;
use craft_ui::runtime::env::{RevealEdge, WindowKey};
use craft_ui::runtime::frame::ChromeToggle;
use craft_ui::shell::{Aim, AppShell, Reveal, ScreenMode, ScreenModeCommand, Slot, Tracking};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, ViewportId, pos2};
use std::cell::RefCell;

const ZONE: f32 = 4.0;
const WINDOW: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));
const BAR: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 32.0));
const FRAME: u64 = 7;

fn aim(pointer: Pos2, frozen: bool, rtl: bool) -> Aim {
    Aim::new(Some(pointer), frozen, WINDOW, ZONE, rtl)
}

fn body_only() -> Tracking {
    Tracking::new(true, true, true)
}

fn edges(reveal: &Reveal) -> [bool; 3] {
    [RevealEdge::Top, RevealEdge::Start, RevealEdge::End].map(|edge| reveal.edge(edge))
}

#[trial]
fn the_f_key_cycles_the_modes_and_shift_f_goes_back() {
    let mut mode = ScreenMode::Standard;
    for expected in [ScreenMode::FullScreenWithMenuBar, ScreenMode::FullScreen, ScreenMode::Standard] {
        mode = mode.next();
        assert_eq!(mode, expected);
    }
    for mode in [ScreenMode::Standard, ScreenMode::FullScreenWithMenuBar, ScreenMode::FullScreen] {
        assert_eq!(mode.next().previous(), mode);
        assert_eq!(ScreenModeCommand::Cycle.apply(mode), mode.next());
        assert_eq!(ScreenModeCommand::CycleBack.apply(mode), mode.previous());
        assert_eq!(ScreenModeCommand::of(mode).apply(ScreenMode::Standard), mode);
    }
}

#[trial]
fn each_mode_hides_what_it_hides() {
    assert!(ScreenMode::Standard.tabs_visible() && ScreenMode::Standard.scrollbars_visible());
    assert!(!ScreenMode::Standard.is_full_screen() && !ScreenMode::Standard.body_only());
    let with_bars = ScreenMode::FullScreenWithMenuBar;
    assert!(!with_bars.tabs_visible() && !with_bars.scrollbars_visible() && with_bars.is_full_screen() && !with_bars.body_only());
    assert!(ScreenMode::FullScreen.body_only() && ScreenMode::FullScreen.is_full_screen());
}

#[trial]
fn the_pointer_at_the_top_start_or_end_edge_reveals_that_edge_only() {
    let cases = [(pos2(400.0, 1.0), [true, false, false]), (pos2(1.0, 300.0), [false, true, false]), (pos2(799.0, 300.0), [false, false, true])];
    for (pointer, expected) in cases {
        let mut reveal = Reveal::default();
        reveal.track(body_only(), aim(pointer, false, false));
        assert_eq!(edges(&reveal), expected, "pointer at {pointer:?}");
    }
    let mut reveal = Reveal::default();
    reveal.track(body_only(), aim(pos2(400.0, 599.0), false, false));
    assert_eq!(edges(&reveal), [false; 3], "the bottom edge never reveals");
}

#[trial]
fn start_and_end_mirror_under_right_to_left() {
    let mut reveal = Reveal::default();
    reveal.track(body_only(), aim(pos2(1.0, 300.0), false, true));
    assert_eq!(edges(&reveal), [false, false, true]);
}

#[trial]
fn a_held_button_or_open_popup_freezes_the_edge_reveal() {
    let mut reveal = Reveal::default();
    reveal.track(body_only(), aim(pos2(400.0, 1.0), false, false));
    reveal.track(body_only(), aim(pos2(400.0, 300.0), true, false));
    assert!(reveal.edge(RevealEdge::Top), "frozen: the reveal does not end");
    reveal.track(body_only(), aim(pos2(400.0, 300.0), false, false));
    assert!(!reveal.edge(RevealEdge::Top));

    let mut frozen = Reveal::default();
    frozen.track(body_only(), aim(pos2(400.0, 1.0), true, false));
    assert!(!frozen.edge(RevealEdge::Top), "frozen: the reveal does not start");
}

#[trial]
fn the_pointer_over_a_revealed_slot_keeps_its_edge_revealed() {
    let mut reveal = Reveal::default();
    reveal.track(body_only(), aim(pos2(400.0, 1.0), false, false));
    reveal.record(vec![(RevealEdge::Top, BAR)]);
    reveal.track(body_only(), aim(pos2(400.0, 20.0), false, false));
    assert!(reveal.edge(RevealEdge::Top));
    reveal.track(body_only(), aim(pos2(400.0, 100.0), false, false));
    assert!(!reveal.edge(RevealEdge::Top));
}

#[trial]
fn a_reveal_shows_the_revealing_slots_and_never_the_status_bar() {
    let options = Slot::new("options").reveal();
    let status = Slot::new("status");
    let mut reveal = Reveal::default();
    assert!(reveal.shows(&options, PaneEdge::Top, false, true) && reveal.shows(&status, PaneEdge::Bottom, false, true), "normal mode: the flags");
    assert!(!reveal.shows(&options, PaneEdge::Top, true, true));
    reveal.track(body_only(), aim(pos2(400.0, 1.0), false, false));
    assert!(reveal.shows(&options, PaneEdge::Top, true, true));
    assert!(!reveal.shows(&options, PaneEdge::Start, true, true), "only the revealed edge's slots show");
    assert!(!reveal.shows(&options, PaneEdge::Top, true, false), "a presentation ignores the reveal");

    reveal.apply(&[(ViewportId::ROOT, ChromeToggle::Panels)], ViewportId::ROOT, FRAME, true, true);
    assert!(reveal.tab());
    assert!(reveal.shows(&options, PaneEdge::Start, true, true), "the Tab reveal shows every revealing slot");
    assert!(!reveal.shows(&status, PaneEdge::Bottom, true, true));
}

#[trial]
fn a_toggle_changes_only_its_own_window_and_once_per_frame() {
    let other = ViewportId::from_hash_of("other");
    let toggles = [(other, ChromeToggle::Panels)];
    let mut reveal = Reveal::default();
    reveal.apply(&toggles, ViewportId::ROOT, FRAME, false, true);
    assert!(!reveal.only(false), "another window's toggle leaves this one alone");

    let toggles = [(ViewportId::ROOT, ChromeToggle::Panels)];
    reveal.apply(&toggles, ViewportId::ROOT, FRAME + 1, false, true);
    reveal.apply(&toggles, ViewportId::ROOT, FRAME + 1, false, true);
    assert!(reveal.only(false), "a discard pass of the same frame does not toggle again");
    reveal.apply(&toggles, ViewportId::ROOT, FRAME + 2, false, true);
    assert!(!reveal.only(false), "Tab again shows the chrome");
}

#[trial]
fn shift_tab_hides_the_panels_and_keeps_the_tools_in_normal_mode_only() {
    let toggles = [(ViewportId::ROOT, ChromeToggle::PanelsKeepTools)];
    let mut reveal = Reveal::default();
    reveal.apply(&toggles, ViewportId::ROOT, FRAME, false, true);
    assert!(reveal.panels_hidden() && !reveal.only(false));
    let mut body_only_window = Reveal::default();
    body_only_window.apply(&toggles, ViewportId::ROOT, FRAME, true, true);
    assert!(!body_only_window.panels_hidden());
}

#[trial]
fn a_screen_mode_change_ends_a_reveal() {
    let mut reveal = Reveal::default();
    reveal.track(body_only(), aim(pos2(400.0, 1.0), false, false));
    reveal.apply(&[(ViewportId::ROOT, ChromeToggle::Panels)], ViewportId::ROOT, FRAME, true, true);
    reveal.clear();
    assert!(!reveal.tab() && edges(&reveal) == [false; 3]);
}

#[trial]
fn slots_lay_out_around_the_body_in_every_mode_but_full_screen() {
    let seen = RefCell::new(None);
    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        let out = AppShell::new(WindowKey::main())
            .title_bar(|ui, _| {
                Text::new("Title").show(ui);
            })
            .bottom(Slot::new("status"), |ui, _| {
                Text::new("Status").show(ui);
            })
            .start(Slot::new("tools"), |ui, _| {
                Text::new("Tools").show(ui);
            })
            .body(|ui, _| {
                Text::new("Document").show(ui);
            })
            .show(ui);
        *seen.borrow_mut() = Some(out);
    };
    frames.idle(3, &mut show);
    let out = seen.borrow_mut().take().expect("the shell showed");
    let rect = |key: &str| out.regions.slots.iter().find(|(shown, _)| *shown == key).map(|(_, rect)| *rect).expect("the slot showed");
    let body = out.regions.body;
    assert!(rect("title").max.y <= body.min.y, "the title is above the body");
    assert!(rect("status").min.y >= body.max.y, "the status bar is below the body");
    assert!(rect("tools").max.x <= body.min.x, "the tools are at the start of the body");
    let keys: Vec<_> = out.regions.slots.iter().map(|(key, _)| *key).collect();
    assert_eq!(keys, ["title", "status", "tools"], "slots claim space in the order title, bottom, start");

    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        let mut mode = ScreenMode::FullScreen;
        let out = AppShell::new(WindowKey::main())
            .screen_mode(Bind::bound(&mut mode))
            .title_bar(|ui, _| {
                Text::new("Title").show(ui);
            })
            .body(|ui, _| {
                Text::new("Document").show(ui);
            })
            .show(ui);
        *seen.borrow_mut() = Some(out);
    };
    frames.idle(3, &mut show);
    let out = seen.borrow_mut().take().expect("the shell showed");
    assert!(out.regions.slots.is_empty() && !out.revealed, "full screen shows the body only");
    assert_eq!(out.screen_mode, ScreenMode::FullScreen);
}
