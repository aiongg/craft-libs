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
use craft_ui::runtime::windows::{WindowAction, WindowHit, WindowHost};
use craft_ui::shell::{Caption, CaptionFamily, Captions, Frame, MacInset, Title, TitleBar, double_click_action, title_bar};
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

/// Maximize sends the window command, Close only reports.
#[trial]
fn maximize_acts_on_the_window_and_close_only_reports() {
    let windows = integrated();
    let host = WindowHost::of(&windows.ctx);
    let shown = Shown::default();
    run(&windows, vec![], &shown);
    let regions = host.caption_regions(ViewportId::ROOT).expect("captions");
    let button = |wanted: Caption| regions.buttons.iter().find(|(caption, _)| *caption == wanted).expect("a button").1.center();
    let click = |at| {
        let mut sent = Vec::new();
        for events in [vec![moved(at)], vec![press(at)], vec![release(at)], vec![]] {
            sent.extend(run(&windows, events, &shown).commands);
        }
        sent
    };
    let sent = click(button(Caption::Close));
    assert!(shown.closed.get(), "the bar reports Close");
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
            below.set(Text::new("Below").show(ui).rect);
        });
    };
    frames.frame(vec![], &mut show);
    assert!(room.get().contains_rect(below.get()), "the next child has room under the bar");
    assert!(below.get().top() >= bar.get().bottom(), "the next child lies under the bar");
    assert_eq!(bar.get().width(), room.get().width(), "the bar fills the width");
}

/// The bar's node is named by the document title and described by the unsaved label; without a title it keeps the
/// bar's own name.
#[trial]
fn the_bar_is_named_by_the_document_title() {
    let mut frames = Frames::new();
    let id = Cell::new(None);
    frames.frame(vec![], |ui| {
        id.set(Some(TitleBar::new().show_titled(ui, Title::unsaved("Landscape.psd"), |_, _| {}, |_, _| {}).response.id));
    });
    let tree = frames.tree();
    let id = id.get().expect("the bar was shown");
    assert_eq!(tree.name(id), Some("Landscape.psd *"));
    assert_eq!(tree.description(id), Some("Unsaved changes"));
}
