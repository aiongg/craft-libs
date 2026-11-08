//! A container in an OS window: its placement reports follow the window's facts, and a modal's strips over the title
//! bar stand for the platform's hit test.

use super::kit::PlainStyle;
use crate::windows_kit::Windows;
use craft_host::target::{Caps, ShellInfo};
use craft_ui::contract::overlay::{PlacementChange, PlacementKind};
use craft_ui::contract::{
    Bind, Common, Component, Container, ContainerProps, Cx, Mount, Overlay, Props as _, Role, Show as _, ShowContainer as _, Sizing,
};
use craft_ui::dialog::Dialog;
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::windows::{WindowHit, WindowHost, WindowSpec};
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::shell::{Caption, CaptionFamily, Captions, Frame, Title, TitleBar};
use craft_ui::style::StyleCx;
use craft_ui::testing::layout::Block;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::trial;
use craft_ui::testing::windows::NoWindows;
use craft_window::policy::{Decision, Desktop, DesktopKind};
use egui::{Pos2, Rect, Response, Vec2, ViewportCommand, ViewportId, ViewportInfo, vec2};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

/// A container that logs the placement changes it is given.
#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Logged<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    log: &'a RefCell<Vec<PlacementChange>>,
}

impl Component for Logged<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: LoggedStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Logged<'_> {
    fn placement_changed(&mut self, change: &PlacementChange) {
        self.log.borrow_mut().push(*change);
    }
}

/// A container torn off into an OS window learns of the window's move once, as a layer learns of its own.
#[trial]
fn a_torn_off_container_gets_one_change_when_its_window_moves() {
    let windows = Windows::new();
    let log = RefCell::new(Vec::new());
    let props = ContainerProps::new().mount(Mount::viewport_with(Overlay::floating(), WindowSpec::tool())).sizing(Sizing::Content);
    let frame = || {
        windows.frame(vec![], vec![], |ui| {
            Logged { common: Common::new(), props: props.clone(), log: &log }.label("Tool").id_salt("tool").show(ui, |ui, _| {
                Block::new(Vec2::new(120.0, 60.0)).show(ui);
            });
        })
    };
    let at = Rect::from_min_size(Pos2::new(200.0, 150.0), Vec2::new(400.0, 300.0));
    windows.child_info(ViewportInfo { inner_rect: Some(at), outer_rect: Some(at), ..ViewportInfo::default() });
    for _ in 0..3 {
        frame();
    }
    log.borrow_mut().clear();
    let moved_to = at.translate(vec2(60.0, 40.0));
    windows.child_info(ViewportInfo { inner_rect: Some(moved_to), outer_rect: Some(moved_to), ..ViewportInfo::default() });
    for _ in 0..3 {
        frame();
    }
    let log = log.into_inner();
    assert_eq!(log.len(), 1, "one change for one move: {log:?}");
    assert_eq!(log[0].kind, PlacementKind::Move, "the window moved, its size stayed");
    assert_eq!(log[0].rect, moved_to, "the change carries the window's outer rect");
}

/// With the platform answering the hit test (a sink installed), the strips a modal shows over the title bar join the
/// window's exclusive list as drag areas, as the bar's band does: the bar beside the dialog still moves the window,
/// through the platform.
#[trial]
fn a_modals_strip_stands_for_the_hit_sink() {
    let shell = ShellInfo::default();
    let shell = shell.clone().caps(shell.caps.union(Caps::OS_RESIZES_UNDECORATED));
    let layout = craft_window::Layout { left: vec![], right: vec![Caption::Minimize, Caption::Maximize, Caption::Close] };
    let frame = Frame::Integrated(Captions { family: CaptionFamily::Windows, layout });
    let desktop = Desktop { kind: DesktopKind::Other, ssd: None, gtk_theme: None, icon_theme: None };
    let windows = Windows::with(InstallSpec::new(shell, Arc::new(NoWindows)).frame(Decision { frame, overridden: None, desktop }));
    let host = WindowHost::of(&windows.ctx);
    let _sink = host.install_hit_sink(ViewportId::ROOT);
    let (bar, dialog) = (Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
    let at = Pos2::new(300.0, 4.0);
    let frame = |events: Vec<egui::Event>| {
        windows
            .frame(events, vec![], |ui| {
                bar.set(TitleBar::new().show_titled(ui, Title::new("Landscape.psd"), |_, _| {}, |_, _| {}).response.rect);
                let mut open = true;
                let shown = Dialog::new("Grid options")
                    .first_place(at)
                    .opened(Bind::bound(&mut open))
                    .buttons([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)])
                    .show(ui, |ui, _| {
                        Block::new(Vec2::new(120.0, 40.0)).show(ui);
                    });
                dialog.set(shown.output.response.rect);
            })
            .commands
    };
    // The strips read the last complete pass: the first publishes the bar's drag areas, the second shows the strips.
    frame(vec![]);
    frame(vec![]);
    assert!(dialog.get().intersects(bar.get()) && dialog.get().left() > bar.get().left(), "the dialog lies over the bar");
    let beside = Pos2::new((bar.get().left() + dialog.get().left()) / 2.0, bar.get().center().y);
    let regions = host.caption_regions(ViewportId::ROOT).expect("the bar registered its parts");
    assert_eq!(regions.hit(beside), Some(WindowHit::Move), "the strip beside the dialog is a drag area for the sink: {regions:?}");
    let mut sent = Vec::new();
    for events in [vec![moved(beside)], vec![press(beside)], vec![moved(beside + vec2(20.0, 0.0))], vec![release(beside + vec2(20.0, 0.0))]] {
        sent.extend(frame(events));
    }
    assert!(sent.iter().all(|(_, command)| *command != ViewportCommand::StartDrag), "the platform moves the window, not the app: {sent:?}");
}
