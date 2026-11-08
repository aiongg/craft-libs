//! The title bar: where its title goes, what a double click asks of each frame, and the window parts the bar
//! registers (its captions and its move band).

use crate::windows_kit::Windows;
use craft_host::target::DoubleClick;
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::windows::{WindowAction, WindowHit, WindowHost};
use craft_ui::shell::{Caption, CaptionFamily, Captions, Frame, MacInset, Title, TitleBar, double_click_action};
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::windows::NoWindows;
use craft_ui::testing::{Frames, trial};
use craft_window::policy::{Decision, Desktop, DesktopKind};
use egui::{Rect, ViewportCommand, ViewportId};
use std::cell::Cell;
use std::sync::Arc;

const GAP: f32 = 400.0;
const WIDE: f32 = 100.0;

fn decision(frame: Frame) -> Decision {
    let desktop = Desktop { kind: DesktopKind::Other, ssd: None, gtk_theme: None, icon_theme: None };
    Decision { frame, overridden: None, desktop }
}

fn windows_captions() -> Frame {
    let layout = craft_window::Layout { left: vec![], right: vec![Caption::Minimize, Caption::Maximize, Caption::Close] };
    Frame::Integrated(Captions { family: CaptionFamily::Windows, layout })
}

/// The title stays on the window's centre line while the gap holds it, slides to the nearer end when the centre line
/// would put it over an end, and takes the whole gap when it does not fit.
#[trial]
fn the_title_is_centred_slides_and_then_fills_the_gap() {
    let centre = GAP / 2.0;
    assert_eq!(Title::span(centre, GAP, WIDE), (centre - WIDE / 2.0, WIDE));
    let (from, width) = Title::span(30.0, GAP, WIDE);
    assert_eq!((from, width), (0.0, WIDE), "the leading content is in the way of the centre line");
    let (from, width) = Title::span(GAP - 30.0, GAP, WIDE);
    assert_eq!((from + width, width), (GAP, WIDE), "the trailing end is in the way of the centre line");
    assert_eq!(Title::span(centre, WIDE, GAP), (0.0, WIDE), "a title wider than the gap takes the gap and is cut");
}

/// A double click maximizes or restores an integrated bar, follows the macOS setting on an inset bar, and is the
/// OS's on a native frame.
#[trial]
fn a_double_click_acts_by_the_frame() {
    let integrated = windows_captions();
    assert!(matches!(double_click_action(&integrated, DoubleClick::Zoom, false), Some(WindowAction::SetMaximized(true))));
    assert!(matches!(double_click_action(&integrated, DoubleClick::Nothing, true), Some(WindowAction::SetMaximized(false))));
    let inset = Frame::Inset(MacInset::default());
    assert!(matches!(double_click_action(&inset, DoubleClick::Zoom, true), Some(WindowAction::SetMaximized(false))));
    assert!(matches!(double_click_action(&inset, DoubleClick::Minimize, false), Some(WindowAction::Minimize)));
    assert!(double_click_action(&inset, DoubleClick::Nothing, false).is_none());
    assert!(double_click_action(&Frame::Native, DoubleClick::Zoom, false).is_none());
}

fn run(windows: &Windows, events: Vec<egui::Event>, bar: &Cell<Rect>, closed: &Cell<bool>) -> crate::windows_kit::Out {
    windows.frame(events, vec![], |ui| {
        let shown = TitleBar::new().show_titled(ui, Title::new("Landscape.psd"), |_, _| {}, |_, _| {});
        bar.set(shown.response.rect);
        closed.set(closed.get() || shown.close_requested);
    })
}

/// An integrated bar registers each shown caption and its move band as the window's exclusive parts; a native frame
/// registers none.
#[trial]
fn an_integrated_bar_registers_its_captions_and_move_band() {
    let windows = Windows::with(InstallSpec::new(craft_host::target::ShellInfo::default(), Arc::new(NoWindows)).frame(decision(windows_captions())));
    let host = WindowHost::of(&windows.ctx);
    let (bar, closed) = (Cell::new(Rect::NOTHING), Cell::new(false));
    for _ in 0..4 {
        run(&windows, vec![], &bar, &closed);
    }
    let regions = host.caption_regions(ViewportId::ROOT).expect("the bar registered its parts");
    assert_eq!(regions.buttons.iter().map(|(caption, _)| *caption).collect::<Vec<_>>(), [Caption::Minimize, Caption::Maximize, Caption::Close]);
    assert_eq!(regions.moves.len(), 1, "the free bar is the move band");
    let close = regions.buttons.iter().find(|(caption, _)| *caption == Caption::Close).expect("a close button").1;
    assert!(bar.get().contains_rect(close), "the buttons lie in the bar");
    assert_eq!(regions.hit(close.center()), Some(WindowHit::Caption(Caption::Close)));
    assert_eq!(regions.hit(bar.get().left_center() + egui::vec2(8.0, 0.0)), Some(WindowHit::Move));

    let native = Windows::with(InstallSpec::new(craft_host::target::ShellInfo::default(), Arc::new(NoWindows)).frame(decision(Frame::Native)));
    for _ in 0..4 {
        run(&native, vec![], &bar, &closed);
    }
    assert!(WindowHost::of(&native.ctx).caption_regions(ViewportId::ROOT).is_none(), "a native frame registers no bar parts");
}

/// Maximize sends the window command, Close only reports.
#[trial]
fn maximize_acts_on_the_window_and_close_only_reports() {
    let windows = Windows::with(InstallSpec::new(craft_host::target::ShellInfo::default(), Arc::new(NoWindows)).frame(decision(windows_captions())));
    let host = WindowHost::of(&windows.ctx);
    let (bar, closed) = (Cell::new(Rect::NOTHING), Cell::new(false));
    for _ in 0..4 {
        run(&windows, vec![], &bar, &closed);
    }
    let regions = host.caption_regions(ViewportId::ROOT).expect("captions");
    let button = |wanted: Caption| regions.buttons.iter().find(|(caption, _)| *caption == wanted).expect("a button").1.center();
    let click = |at| {
        let mut sent = Vec::new();
        for events in [vec![moved(at)], vec![press(at)], vec![release(at)], vec![]] {
            sent.extend(run(&windows, events, &bar, &closed).commands);
        }
        sent
    };
    let sent = click(button(Caption::Close));
    assert!(closed.get(), "the bar reports Close");
    assert!(sent.iter().all(|(_, command)| !matches!(command, ViewportCommand::Close)), "the bar never closes the window itself");
    let sent = click(button(Caption::Maximize));
    assert!(sent.iter().any(|(_, command)| matches!(command, ViewportCommand::Maximized(true))));
}

// Height and name of the bar.

/// A bar at the top of a column takes the width and its content's height, so the next child lies under it.
#[trial]
fn a_bar_in_a_column_leaves_the_room_below_it_to_the_next_child() {
    let mut frames = Frames::new();
    let (bar, below, room) = (Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
    let mut show = |ui: &mut egui::Ui| {
        room.set(ui.max_rect());
        ui.vertical(|ui| {
            bar.set(TitleBar::new().show_titled(ui, Title::new("Landscape.psd"), |_, _| {}, |_, _| {}).response.rect);
            below.set(ui.allocate_space(egui::vec2(10.0, 10.0)).1);
        });
    };
    for _ in 0..4 {
        frames.frame(vec![], &mut show);
    }
    assert!(bar.get().height() < room.get().height() / 2.0, "the bar does not take the room");
    assert!(below.get().top() >= bar.get().bottom(), "the next child lies under the bar");
    assert_eq!(bar.get().width(), room.get().width(), "the bar fills the width");
}

/// The bar's node is named by the document title and described by the unsaved label; without a title it keeps the
/// bar's own name.
#[trial]
fn the_bar_is_named_by_the_document_title() {
    let mut frames = Frames::new();
    let id = Cell::new(None);
    for _ in 0..3 {
        frames.frame(vec![], |ui| {
            id.set(Some(TitleBar::new().show_titled(ui, Title::unsaved("Landscape.psd"), |_, _| {}, |_, _| {}).response.id));
        });
    }
    let tree = frames.tree();
    let id = id.get().expect("the bar was shown");
    assert_eq!(tree.name(id), Some("Landscape.psd *"));
    assert_eq!(tree.description(id), Some("Unsaved changes"));
}
