//! The body step moves and resizes an OS window (16.3, 21.2): a move band and grips act only where the app draws
//! the frame and the window is neither maximized nor full screen; a `Window` mount puts its grips on the window's
//! edges; a double click on the move band is reported once.

use crate::contract::kit::{PlainStyle, Probe, Seen};
use crate::windows_kit::Windows;
use craft_host::target::{ShellInfo, WindowChrome, WindowInfo, WindowKind};
use craft_ui::contract::overlay::Resize;
use craft_ui::contract::{
    BodyCx, Common, Component, Container, ContainerProps, Cx, DragBand, HeaderRegion, Mount, Overlay, Props as _, Role, Show as _,
    ShowContainer as _, Sizing,
};
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::windows::WindowSpec;
use craft_ui::style::StyleCx;
use craft_ui::testing::layout::allocate;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::trial;
use craft_ui::testing::windows::FakeSystem;
use egui::viewport::ResizeDirection;
use egui::{Event, Pos2, Rect, Response, Ui, Vec2, ViewportCommand, ViewportInfo, pos2, vec2};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

const HEADER: Vec2 = Vec2::new(100.0, 24.0);
const DRAG: Vec2 = Vec2::new(40.0, 30.0);
/// The size of a child window in the windows kit, and a point just inside its bottom right corner.
const CHILD: Vec2 = Vec2::new(400.0, 300.0);
/// The size of the root window in the windows kit.
const ROOT: Vec2 = Vec2::new(800.0, 600.0);

/// A container whose header is a block of a fixed size; it counts the passes in which its move band reported a
/// double click.
#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Bar<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    header: &'a Cell<Rect>,
    doubles: &'a Cell<u32>,
}

impl Component for Bar<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: BarStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Bar<'_> {
    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        self.header.set(allocate(ui, HEADER));
        if cx.band_double_pressed() {
            self.doubles.set(self.doubles.get() + 1);
        }
    }
}

/// A frame source for one container: it keeps the header rect and the double click count.
struct Rig {
    header: Cell<Rect>,
    doubles: Cell<u32>,
    seen: RefCell<Seen>,
    props: ContainerProps,
}

impl Rig {
    fn new(mount: Mount) -> Self {
        let props = ContainerProps::new().mount(mount).sizing(Sizing::Content).header(HeaderRegion::new().band(DragBand::Move));
        Rig { header: Cell::new(Rect::NOTHING), doubles: Cell::new(0), seen: RefCell::new(Seen::default()), props }
    }

    fn show(&self, ui: &mut Ui) {
        Bar { common: Common::new(), props: self.props.clone(), header: &self.header, doubles: &self.doubles }.id_salt("win").show(ui, |ui, _| {
            Probe::new(&self.seen).size(vec2(40.0, 20.0)).show(ui);
        });
    }

    fn frame(&self, windows: &Windows, root: Vec<Event>, child: Vec<Event>) -> Vec<ViewportCommand> {
        windows.frame(root, child, |ui| self.show(ui)).commands.into_iter().map(|(_, command)| command).collect()
    }
}

/// Presses at `at`, drags by `by` in four steps and releases. `events` wraps each step for the viewport under test.
fn drag(at: Pos2, by: Vec2, mut frame: impl FnMut(Vec<Event>) -> Vec<ViewportCommand>) -> Vec<ViewportCommand> {
    let mut sent = frame(vec![moved(at)]);
    sent.extend(frame(vec![press(at)]));
    for step in 1..=4 {
        sent.extend(frame(vec![moved(at + by * (step as f32 / 4.0))]));
    }
    sent.extend(frame(vec![release(at + by)]));
    sent.extend(frame(vec![]));
    sent
}

fn starts(sent: &[ViewportCommand]) -> usize {
    sent.iter().filter(|command| matches!(command, ViewportCommand::StartDrag)).count()
}

fn resizes(sent: &[ViewportCommand]) -> Vec<ResizeDirection> {
    sent.iter()
        .filter_map(|command| match command {
            ViewportCommand::BeginResize(direction) => Some(*direction),
            _ => None,
        })
        .collect()
}

/// The commands a drag on the move band of a viewport container sends; `info` is what the child window reports.
fn band_drag(mount: Mount, info: ViewportInfo) -> Vec<ViewportCommand> {
    let windows = Windows::new();
    windows.child_info(info);
    let rig = Rig::new(mount);
    for _ in 0..4 {
        rig.frame(&windows, vec![], vec![]);
    }
    drag(rig.header.get().center(), DRAG, |events| rig.frame(&windows, vec![], events))
}

/// The commands a drag at `at` sends in a child window of a container with `overlay`, opened as a tool window.
fn grip_drag(overlay: Overlay, at: Pos2) -> Vec<ViewportCommand> {
    let windows = Windows::new();
    let rig = Rig::new(Mount::viewport_with(overlay, WindowSpec::tool().chrome(WindowChrome::Integrated)));
    for _ in 0..4 {
        rig.frame(&windows, vec![], vec![]);
    }
    drag(at, -DRAG, |events| rig.frame(&windows, vec![], events))
}

#[trial]
fn a_viewport_containers_move_band_sends_one_os_move_unless_the_app_does_not_own_the_window() {
    let movable = || Mount::viewport_with(Overlay::floating().movable(true), WindowSpec::tool().chrome(WindowChrome::Integrated));
    assert_eq!(starts(&band_drag(movable(), ViewportInfo::default())), 1, "an undecorated window moves once");
    let maximized = ViewportInfo { maximized: Some(true), ..ViewportInfo::default() };
    assert_eq!(starts(&band_drag(movable(), maximized)), 0, "a maximized window does not move");
    let full_screen = ViewportInfo { fullscreen: Some(true), ..ViewportInfo::default() };
    assert_eq!(starts(&band_drag(movable(), full_screen)), 0, "a full-screen window does not move");
    let framed_by_os = Mount::viewport(Overlay::floating().movable(true));
    assert_eq!(starts(&band_drag(framed_by_os, ViewportInfo::default())), 0, "the OS moves a window it draws the frame of");
}

#[trial]
fn a_grip_drag_resizes_only_a_container_that_asks_for_grips() {
    let corner = Pos2::new(CHILD.x - 1.0, CHILD.y - 1.0);
    let sent = grip_drag(Overlay::floating().movable(true).resizable(true), corner);
    assert_eq!(resizes(&sent), [ResizeDirection::SouthEast], "the corner grip resizes from the south east: {sent:?}");
    let sent = grip_drag(Overlay::floating().movable(true), corner);
    assert!(resizes(&sent).is_empty(), "a container that is not resizable has no grips: {sent:?}");
}

/// The root window with the frame drawn by the app, or by the OS.
fn root(chrome: WindowChrome) -> Windows {
    let shell = ShellInfo::default().window(WindowInfo::new(WindowKind::Main).chrome(chrome));
    Windows::with(InstallSpec::new(shell, Arc::new(FakeSystem::new())))
}

fn root_drag(chrome: WindowChrome, at: Pos2, by: Vec2) -> (Vec<ViewportCommand>, Rig) {
    let windows = root(chrome);
    let rig = Rig::new(Mount::window(Resize::GRIPS));
    for _ in 0..4 {
        rig.frame(&windows, vec![], vec![]);
    }
    (drag(at, by, |events| rig.frame(&windows, events, vec![])), rig)
}

#[trial]
fn a_window_mounts_edges_resize_the_window_and_the_childs_press_keeps_its_click() {
    let (sent, _) = root_drag(WindowChrome::Integrated, pos2(1.0, ROOT.y / 2.0), vec2(20.0, 0.0));
    assert_eq!(resizes(&sent), [ResizeDirection::West], "a drag on the left edge resizes from the west: {sent:?}");
    let (sent, _) = root_drag(WindowChrome::Native, pos2(1.0, ROOT.y / 2.0), vec2(20.0, 0.0));
    assert!(resizes(&sent).is_empty(), "the OS resizes a window it draws the frame of: {sent:?}");

    // The probe is the first thing in the body, under the left edge: a click on it is the probe's, not a resize.
    let windows = root(WindowChrome::Integrated);
    let rig = Rig::new(Mount::window(Resize::GRIPS));
    for _ in 0..4 {
        rig.frame(&windows, vec![], vec![]);
    }
    let on_probe = pos2(1.0, rig.header.get().bottom() + 5.0);
    let mut sent = rig.frame(&windows, vec![moved(on_probe)], vec![]);
    sent.extend(rig.frame(&windows, vec![press(on_probe)], vec![]));
    sent.extend(rig.frame(&windows, vec![release(on_probe)], vec![]));
    sent.extend(rig.frame(&windows, vec![], vec![]));
    assert_eq!(rig.seen.borrow().activations, 1, "the child under the edge keeps its click");
    assert!(resizes(&sent).is_empty(), "and the click starts no resize: {sent:?}");
}

#[trial]
fn a_double_click_on_the_move_band_is_reported_once() {
    let windows = root(WindowChrome::Integrated);
    let rig = Rig::new(Mount::window(Resize::NONE));
    for _ in 0..4 {
        rig.frame(&windows, vec![], vec![]);
    }
    let at = rig.header.get().center();
    let mut sent = rig.frame(&windows, vec![moved(at)], vec![]);
    for events in [vec![press(at)], vec![release(at)], vec![press(at)]] {
        sent.extend(rig.frame(&windows, events, vec![]));
    }
    for _ in 0..4 {
        sent.extend(rig.frame(&windows, vec![], vec![]));
    }
    assert_eq!(rig.doubles.get(), 1, "the second press is one pass, however long the button is held");
    assert_eq!(starts(&sent), 0, "a double click without a drag moves nothing");
    rig.frame(&windows, vec![release(at)], vec![]);
}
