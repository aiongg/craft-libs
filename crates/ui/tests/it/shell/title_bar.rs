//! The title bar: where its title goes, what a double click asks of each frame, and the window parts the bar
//! registers (its captions and its move band).

use crate::body::kit::Slab;
use crate::windows_kit::Windows;
use craft_host::target::{Caps, DoubleClick, ShellInfo};
use craft_ui::buttons::Button;
use craft_ui::contract::overlay::Resize;
use craft_ui::contract::{ContainerProps, DragBand, HeaderRegion, Mount, Overlay, Props as _, Show as _, ShowContainer as _, Sizing};
use craft_ui::display::Text;
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::windows::{WindowAction, WindowHit, WindowHost, double_click_action};
use craft_ui::shell::{Caption, CaptionFamily, Captions, Frame, MacInset, Title, TitleBar, title_bar};
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::windows::NoWindows;
use craft_ui::testing::{Frames, trial};
use craft_window::policy::{Decision, Desktop, DesktopKind};
use egui::{Pos2, Rect, Ui, ViewportCommand, ViewportId, ViewportInfo, vec2};
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

/// What a pass of the bar showed: the bar, its leading and trailing buttons, whether the OS may move the window from
/// the pointer, and whether Close was asked for.
struct Shown {
    bar: Cell<Rect>,
    leading: Cell<Rect>,
    trailing: Cell<Rect>,
    movable: Cell<bool>,
    closed: Cell<bool>,
}

impl Default for Shown {
    fn default() -> Self {
        let none = || Cell::new(Rect::NOTHING);
        Shown { bar: none(), leading: none(), trailing: none(), movable: Cell::new(true), closed: Cell::new(false) }
    }
}

/// Shows the bar at the top of `ui`, where it takes the width and the caption strip's height (the title slot's), with a
/// leading and a trailing button.
fn show_bar(ui: &mut Ui, shown: &Shown) {
    let out = TitleBar::new().show_titled(
        ui,
        Title::new("Landscape.psd"),
        |ui, _| shown.leading.set(Button::new("Leading").show(ui).rect),
        |ui, _| shown.trailing.set(Button::new("Trailing").show(ui).rect),
    );
    shown.bar.set(out.response.rect);
    shown.closed.set(shown.closed.get() || out.close_requested);
    shown.movable.set(title_bar::window_movable(ui.ctx()));
}

fn run(windows: &Windows, events: Vec<egui::Event>, shown: &Shown) -> crate::windows_kit::Out {
    windows.frame(events, vec![], |ui| show_bar(ui, shown))
}

fn integrated() -> Windows {
    Windows::with(InstallSpec::new(ShellInfo::default(), Arc::new(NoWindows)).frame(decision(windows_captions())))
}

/// A point of the bar's free line between the leading content and the title.
fn after_leading(shown: &Shown) -> Pos2 {
    shown.leading.get().right_center().lerp(shown.bar.get().center(), 0.5)
}

/// An integrated bar registers each shown caption and its move band as the window's exclusive parts: the band spans
/// the free line, the title gap and the room between the leading content and the title, and not the bar's buttons. A
/// native frame registers none.
#[trial]
fn an_integrated_bar_registers_its_captions_and_move_band() {
    let windows = integrated();
    let host = WindowHost::of(&windows.ctx);
    let shown = Shown::default();
    run(&windows, vec![], &shown);
    let regions = host.caption_regions(ViewportId::ROOT).expect("the bar registered its parts");
    assert_eq!(regions.buttons.iter().map(|(caption, _)| *caption).collect::<Vec<_>>(), [Caption::Minimize, Caption::Maximize, Caption::Close]);
    let close = regions.buttons.iter().find(|(caption, _)| *caption == Caption::Close).expect("a close button").1;
    assert!(shown.bar.get().contains_rect(close), "the buttons lie in the bar");
    assert_eq!(regions.hit(close.center()), Some(WindowHit::Caption(Caption::Close)));
    assert_eq!(regions.hit(shown.bar.get().center()), Some(WindowHit::Move), "the title gap moves the window: {regions:?}");
    assert!(shown.leading.get().is_positive(), "the leading content shows: {:?}", shown.leading.get());
    assert_eq!(regions.hit(after_leading(&shown)), Some(WindowHit::Move), "the room after the leading content moves the window: {regions:?}");
    assert_eq!(regions.hit(shown.leading.get().center()), None, "a leading button keeps its press: {regions:?}");
    assert_eq!(regions.hit(shown.trailing.get().center()), None, "a trailing button keeps its press: {regions:?}");

    let native = Windows::with(InstallSpec::new(ShellInfo::default(), Arc::new(NoWindows)).frame(decision(Frame::Native)));
    run(&native, vec![], &shown);
    assert!(WindowHost::of(&native.ctx).caption_regions(ViewportId::ROOT).is_none(), "a native frame registers no bar parts");
}

/// The OS may move the window only from where a press would go to the bar's band: not from a button on the bar.
#[trial]
fn the_os_moves_the_window_only_from_the_free_bar() {
    let windows = integrated();
    let shown = Shown::default();
    run(&windows, vec![], &shown);
    run(&windows, vec![moved(shown.leading.get().center())], &shown);
    assert!(!shown.movable.get(), "the window does not move from a bar button");
    run(&windows, vec![moved(shown.bar.get().center())], &shown);
    assert!(shown.movable.get(), "the window moves from the title gap");
}

/// A maximized window keeps its band: a double click on the free bar restores the window, and a drag hands the move
/// to the platform, which restores and moves the window.
#[trial]
fn a_double_click_on_a_maximized_bar_restores_the_window() {
    let windows = integrated();
    windows.root_info(ViewportInfo { maximized: Some(true), ..ViewportInfo::default() });
    let shown = Shown::default();
    run(&windows, vec![], &shown);
    let gap = shown.bar.get().center();
    let mut sent = Vec::new();
    for events in [vec![moved(gap)], vec![press(gap)], vec![release(gap)], vec![press(gap)], vec![release(gap)], vec![]] {
        sent.extend(run(&windows, events, &shown).commands);
    }
    assert!(sent.iter().any(|(_, command)| matches!(command, ViewportCommand::Maximized(false))), "the double click restores: {sent:?}");
}

/// With the platform answering the hit test (an installed sink), the window's band and edges still stand as its
/// exclusive parts, so the sink reads them, and a press on the band moves nothing in the app. A dialog's band on a layer
/// of the window is never one of the window's parts.
#[trial]
fn a_hit_sink_reads_the_band_and_edges_and_a_dialogs_band_is_not_the_windows() {
    let shell = ShellInfo::default();
    let shell = shell.clone().caps(shell.caps.union(Caps::OS_RESIZES_UNDECORATED));
    let windows = Windows::with(InstallSpec::new(shell, Arc::new(NoWindows)).frame(decision(windows_captions())));
    let host = WindowHost::of(&windows.ctx);
    let _sink = host.install_hit_sink(ViewportId::ROOT);
    let shown = Shown::default();
    let dialog = Cell::new(Rect::NOTHING);
    let frame = |events: Vec<egui::Event>| {
        windows.frame(events, vec![], |ui| {
            let window = ContainerProps::new().mount(Mount::window(Resize::GRIPS));
            Slab::new(window).id_salt("window").show(ui, |ui, _| {
                show_bar(ui, &shown);
                let layer = ContainerProps::new()
                    .mount(Mount::Layer(Overlay::floating().movable(true)))
                    .sizing(Sizing::Content)
                    .header(HeaderRegion::new().band(DragBand::Move));
                let shown = Slab::new(layer).id_salt("dialog").show(ui, |ui, _| {
                    Text::new("Dialog").show(ui);
                });
                dialog.set(shown.output.rect);
            });
        })
    };
    frame(vec![]);
    let regions = host.caption_regions(ViewportId::ROOT).expect("the window registered its parts");
    assert!(!regions.edges.is_empty(), "the window's edges stand for the sink: {regions:?}");
    assert_eq!(regions.moves.len(), 1, "the bar's band is the window's only band: {regions:?}");
    assert!(shown.bar.get().contains_rect(regions.moves[0]), "the band is the bar's: {regions:?}");
    assert!(dialog.get().is_positive(), "the dialog showed");
    let gap = shown.bar.get().center();
    let mut sent = Vec::new();
    for events in [vec![moved(gap)], vec![press(gap)], vec![moved(gap + vec2(shown.bar.get().height(), 0.0))], vec![release(gap)], vec![]] {
        sent.extend(frame(events).commands);
    }
    assert!(sent.iter().all(|(_, command)| *command != ViewportCommand::StartDrag), "the platform moves the window, not the app: {sent:?}");
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
            below.set(Text::new("Below").show(ui).rect);
        });
    };
    frames.frame(vec![], &mut show);
    assert!(room.get().contains_rect(below.get()), "the next child has room under the bar");
    assert!(below.get().top() >= bar.get().bottom(), "the next child lies under the bar");
    assert_eq!(bar.get().width(), room.get().width(), "the bar fills the width");
}

