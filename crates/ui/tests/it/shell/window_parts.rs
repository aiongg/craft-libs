//! `WindowEdges`, `WindowDrag` and the caption record: which window commands a press on each sends.

use crate::windows_kit::Windows;
use craft_host::target::{ShellInfo, WindowChrome, WindowInfo, WindowKind};
use craft_ui::contract::Show as _;
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::windows::{ResizeEdge, WindowHost};
use craft_ui::shell::{Caption, CaptionHit, CaptionRegions, WindowDrag, WindowEdges};
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::trial;
use craft_ui::testing::windows::FakeSystem;
use egui::{Pos2, Rect, Ui, Vec2, ViewportCommand, ViewportId, ViewportInfo, pos2, vec2};
use std::sync::Arc;

/// A window whose frame the app draws.
fn undecorated() -> Windows {
    let shell = ShellInfo::default().window(WindowInfo::new(WindowKind::Main).chrome(WindowChrome::Integrated));
    Windows::with(InstallSpec::new(shell, Arc::new(FakeSystem::new())))
}

fn settle(windows: &Windows, show: &mut impl FnMut(&mut Ui)) {
    for _ in 0..3 {
        windows.frame(vec![], vec![], &mut *show);
    }
}

/// Presses at `at`, drags by `by` and releases. Returns the window commands sent.
fn drag(windows: &Windows, at: Pos2, by: Vec2, show: &mut impl FnMut(&mut Ui)) -> Vec<ViewportCommand> {
    let mut sent = Vec::new();
    let mut frame = |events: Vec<egui::Event>, show: &mut dyn FnMut(&mut Ui)| {
        sent.extend(windows.frame(events, vec![], show).commands.into_iter().map(|(_, command)| command));
    };
    frame(vec![moved(at)], &mut *show);
    frame(vec![press(at)], &mut *show);
    for step in 1..=4 {
        frame(vec![moved(at + by * (step as f32 / 4.0))], &mut *show);
    }
    frame(vec![release(at + by)], &mut *show);
    frame(vec![], &mut *show);
    sent
}

fn resizes(sent: &[ViewportCommand]) -> usize {
    sent.iter().filter(|command| matches!(command, ViewportCommand::BeginResize(_))).count()
}

fn moves(sent: &[ViewportCommand]) -> usize {
    sent.iter().filter(|command| matches!(command, ViewportCommand::StartDrag)).count()
}

/// Shows the zones with `holes` each frame and keeps the edges they report.
fn zones<'a>(holes: &'a [Rect], started: &'a mut Vec<ResizeEdge>) -> impl FnMut(&mut Ui) + 'a {
    move |ui| started.extend(WindowEdges::new(ViewportId::ROOT).holes(holes).place(ui).started)
}

#[trial]
fn a_drag_on_each_edge_zone_starts_the_resize_from_that_edge() {
    let inside = 1.0;
    let cases = [
        (pos2(400.0, inside), vec2(0.0, 20.0), ResizeEdge::Top),
        (pos2(400.0, 600.0 - inside), vec2(0.0, -20.0), ResizeEdge::Bottom),
        (pos2(inside, 300.0), vec2(20.0, 0.0), ResizeEdge::Left),
        (pos2(800.0 - inside, 300.0), vec2(-20.0, 0.0), ResizeEdge::Right),
        (pos2(inside, inside), vec2(20.0, 20.0), ResizeEdge::TopLeft),
        (pos2(800.0 - inside, inside), vec2(-20.0, 20.0), ResizeEdge::TopRight),
        (pos2(inside, 600.0 - inside), vec2(20.0, -20.0), ResizeEdge::BottomLeft),
        (pos2(800.0 - inside, 600.0 - inside), vec2(-20.0, -20.0), ResizeEdge::BottomRight),
    ];
    for (at, by, edge) in cases {
        let windows = undecorated();
        let mut started = Vec::new();
        let mut show = zones(&[], &mut started);
        settle(&windows, &mut show);
        let sent = drag(&windows, at, by, &mut show);
        drop(show);
        assert_eq!(started, vec![edge], "a drag at {at:?} starts the resize from {edge:?}");
        assert_eq!(resizes(&sent), 1, "one resize command for {edge:?}: {sent:?}");
    }
}

#[trial]
fn a_press_on_a_hole_sends_nothing() {
    let hole = Rect::from_min_max(pos2(380.0, 0.0), pos2(420.0, 20.0));
    let windows = undecorated();
    let mut started = Vec::new();
    let mut show = zones(std::slice::from_ref(&hole), &mut started);
    settle(&windows, &mut show);
    let on_hole = drag(&windows, pos2(400.0, 1.0), vec2(0.0, 20.0), &mut show);
    assert_eq!(resizes(&on_hole), 0, "the hole keeps its press: {on_hole:?}");
    let beside = drag(&windows, pos2(200.0, 1.0), vec2(0.0, 20.0), &mut show);
    drop(show);
    assert_eq!(resizes(&beside), 1, "the zone beside the hole still resizes");
    assert_eq!(started, vec![ResizeEdge::Top]);
}

#[trial]
fn a_maximized_or_full_screen_window_has_no_zones() {
    for (name, info) in [
        ("maximized", ViewportInfo { maximized: Some(true), ..ViewportInfo::default() }),
        ("full screen", ViewportInfo { fullscreen: Some(true), ..ViewportInfo::default() }),
    ] {
        let windows = undecorated();
        windows.root_info(info);
        let mut started = Vec::new();
        let mut show = zones(&[], &mut started);
        settle(&windows, &mut show);
        let sent = drag(&windows, pos2(400.0, 1.0), vec2(0.0, 20.0), &mut show);
        drop(show);
        assert!(started.is_empty() && resizes(&sent) == 0, "a {name} window has no zones: {sent:?}");
    }
}

#[trial]
fn a_window_the_system_frames_has_no_zones() {
    let windows = Windows::new();
    let mut started = Vec::new();
    let mut show = zones(&[], &mut started);
    settle(&windows, &mut show);
    let sent = drag(&windows, pos2(400.0, 1.0), vec2(0.0, 20.0), &mut show);
    drop(show);
    assert!(started.is_empty() && resizes(&sent) == 0, "the system frame resizes: {sent:?}");
}

const STRIP: Rect = Rect { min: Pos2::new(0.0, 0.0), max: Pos2::new(800.0, 40.0) };

#[trial]
fn a_press_on_the_strip_that_passes_the_click_distance_starts_one_move() {
    let windows = undecorated();
    let mut show = |ui: &mut Ui| {
        WindowDrag::new(ViewportId::ROOT).show_at(ui, STRIP);
    };
    settle(&windows, &mut show);
    let sent = drag(&windows, pos2(300.0, 20.0), vec2(60.0, 0.0), &mut show);
    assert_eq!(moves(&sent), 1, "one move for one press: {sent:?}");
}

#[trial]
fn a_click_on_the_strip_moves_nothing_and_a_double_click_is_reported() {
    let windows = undecorated();
    let double = std::cell::Cell::new(false);
    let mut show = |ui: &mut Ui| {
        double.set(double.get() | WindowDrag::new(ViewportId::ROOT).show_at(ui, STRIP).double_clicked);
    };
    settle(&windows, &mut show);
    let at = pos2(300.0, 20.0);
    let mut sent = Vec::new();
    let mut frame = |events: Vec<egui::Event>, show: &mut dyn FnMut(&mut Ui)| {
        sent.extend(windows.frame(events, vec![], show).commands.into_iter().map(|(_, command)| command));
    };
    frame(vec![moved(at)], &mut show);
    frame(vec![press(at)], &mut show);
    frame(vec![release(at)], &mut show);
    frame(vec![], &mut show);
    assert!(!double.get(), "one click is no double click");
    frame(vec![press(at)], &mut show);
    frame(vec![release(at)], &mut show);
    frame(vec![], &mut show);
    assert!(double.get(), "the second press is a double click");
    assert_eq!(moves(&sent), 0, "no click moves the window: {sent:?}");
}

#[trial]
fn a_control_inside_the_strip_takes_the_press() {
    use craft_ui::buttons::Button;
    let windows = undecorated();
    let button = Rect::from_min_size(pos2(100.0, 4.0), vec2(120.0, 32.0));
    let mut show = |ui: &mut Ui| {
        WindowDrag::new(ViewportId::ROOT).show_at(ui, STRIP);
        Button::new("Control").show_at(ui, button);
    };
    settle(&windows, &mut show);
    let sent = drag(&windows, button.center(), vec2(60.0, 0.0), &mut show);
    assert_eq!(moves(&sent), 0, "a drag that starts on the control does not move the window: {sent:?}");
}

#[trial]
fn caption_regions_return_what_the_bar_published_and_nothing_for_other_windows() {
    let windows = undecorated();
    let close = Rect::from_min_max(pos2(750.0, 0.0), pos2(800.0, 40.0));
    let regions = CaptionRegions::new(STRIP).caption(Caption::Close, close);
    let other = ViewportId::from_hash_of("other");
    let host = WindowHost::of(&windows.ctx);
    let mut seen = Vec::new();
    for _ in 0..2 {
        windows.frame(vec![], vec![], |ui| {
            regions.publish(ui, ViewportId::ROOT);
            seen.push((host.caption_regions(ViewportId::ROOT), host.caption_regions(other)));
        });
    }
    let (root, none) = seen.last().cloned().expect("a frame ran");
    assert_eq!(root.as_ref(), Some(&regions), "the bar's record is read back");
    assert_eq!(none, None, "a window whose bar published nothing has no record");
    assert_eq!(regions.hit(close.center()), Some(CaptionHit::Button(Caption::Close)), "a button wins over the drag area");
    assert_eq!(regions.hit(pos2(10.0, 10.0)), Some(CaptionHit::Drag));
    assert_eq!(regions.hit(pos2(10.0, 300.0)), None);
    for _ in 0..3 {
        windows.frame(vec![], vec![], |_| {});
    }
    assert_eq!(host.caption_regions(ViewportId::ROOT), None, "a bar that stops publishing leaves no record");
}
