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
