//! `runtime::windows`: the input hook per viewport, window info per viewport, the press release after an OS
//! resize (H4), the raise of a modal's window, and `act`.

use crate::contract::kit::PlainStyle;
use crate::windows_kit::Windows;
use craft_behavior::{PressCancel, Sense};
use craft_host::target::{Caps, ShellInfo, WindowChrome, WindowInfo, WindowKind, WindowState};
use craft_ui::contract::{
    Common, Component, Container, ContainerProps, Cx, Mount, Overlay, Pointer as _, Props as _, Role, Show as _, ShowContainer as _,
};
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::env::Env;
use craft_ui::runtime::windows::{WindowAction, act};
use craft_ui::style::StyleCx;
use craft_ui::testing::pointer::{moved, press};
use craft_ui::testing::trial;
use craft_ui::testing::windows::{NoWindows, dragged_id, record_input_hooks};
use egui::{Event, Pos2, Rect, Response, Ui, Vec2, ViewportBuilder, ViewportCommand, ViewportId, ViewportInfo};
use std::cell::Cell;
use std::sync::{Arc, Mutex};

const HANDLE: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(90.0, 50.0) };

fn child() -> ViewportId {
    ViewportId::from_hash_of("child")
}

/// A draggable that, when asked, cancels its own press as soon as the drag starts.
#[derive(craft_ui::contract::Component)]
struct Handle {
    #[common]
    common: Common,
    cancels: bool,
}

impl Component for Handle {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HandleStyleKey) -> PlainStyle {
        PlainStyle::min(HANDLE.size())
    }

    fn sense(&self) -> Sense {
        Sense::DRAG
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if self.cancels && cx.press().phase.is_drag() {
            cx.cancel_press(PressCancel::Escape);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A modal layer with a fixed body.
#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Modal {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl Component for Modal {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ModalStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::new(120.0, 60.0))
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Modal {}

/// The input hook of a child viewport's pass runs while the parent is the current viewport, which is why the
/// runtime's hook steps read only the raw input and records keyed by `RawInput.viewport_id`.
#[trial]
fn a_child_viewports_input_hook_runs_with_the_parent_as_current_viewport() {
    let windows = Windows::new();
    let log = Arc::new(Mutex::new(Vec::new()));
    record_input_hooks(&windows.ctx, log.clone());
    for _ in 0..2 {
        windows.frame(vec![], vec![], |ui| {
            ui.ctx().show_viewport_immediate(child(), ViewportBuilder::default(), |_, _| {});
        });
    }
    let log = log.lock().unwrap().clone();
    let hooks: Vec<_> = log.iter().filter(|(input, _)| *input == child()).collect();
    assert!(!hooks.is_empty(), "the child's input hook ran: {log:?}");
    assert!(hooks.iter().all(|(_, current)| *current == ViewportId::ROOT), "during the child's input hook, the parent is current: {hooks:?}");
}

/// Escape-like cancels end the egui drag of the viewport the press was in, whatever the other window drags.
#[trial]
fn a_cancelled_press_in_a_child_viewport_ends_the_childs_drag_not_the_main_windows() {
    let windows = Windows::new();
    let (root_dragged, child_dragged) = (Cell::new(None), Cell::new(None));
    let mut frame = |root: Vec<Event>, child_events: Vec<Event>| {
        windows.frame(root, child_events, |ui| {
            Handle { common: Common::new(), cancels: false }.id_salt("root").show_at(ui, HANDLE);
            ui.ctx().show_viewport_immediate(child(), ViewportBuilder::default(), |cui, _| {
                Handle { common: Common::new(), cancels: true }.id_salt("child").show_at(cui, HANDLE);
                child_dragged.set(dragged_id(cui.ctx()));
            });
            root_dragged.set(dragged_id(ui.ctx()));
        });
    };
    for _ in 0..3 {
        frame(vec![], vec![]);
    }
    let at = HANDLE.center();
    frame(vec![moved(at)], vec![moved(at)]);
    frame(vec![press(at)], vec![press(at)]);
    for step in [30.0, 35.0, 40.0] {
        let to = at + Vec2::splat(step);
        frame(vec![moved(to)], vec![moved(to)]);
    }
    assert!(root_dragged.get().is_some(), "the main window's drag is still on");
    assert_eq!(child_dragged.get(), None, "the child's drag ended with the cancelled press");
}

/// Window info is kept per viewport: a child that lost OS focus is inactive while the main window is not.
#[trial]
fn window_info_updates_per_viewport() {
    let windows = Windows::new();
    windows.root_info(ViewportInfo { focused: Some(true), ..ViewportInfo::default() });
    windows.child_info(ViewportInfo { focused: Some(false), maximized: Some(true), ..ViewportInfo::default() });
    let (root, kid) = (Cell::new(None), Cell::new(None));
    for _ in 0..3 {
        windows.frame(vec![], vec![], |ui| {
            let env = Env::of(ui);
            root.set(Some(env.view().shell().window.clone()));
            ui.ctx().show_viewport_immediate(child(), ViewportBuilder::default(), |cui, _| {
                let env = Env::of(cui);
                kid.set(Some(env.view().shell().window.clone()));
            });
        });
    }
    let (root, kid) = (root.take().expect("the root ran"), kid.take().expect("the child ran"));
    assert!(root.active && root.state == WindowState::Normal, "the main window: {root:?}");
    assert!(!kid.active && kid.state == WindowState::Maximized, "the child: {kid:?}");
    assert_eq!(kid.kind, WindowKind::Tool, "a window the runtime opens is a tool window until a mount says otherwise");
}

/// H4: an undecorated window the OS resizes never gets the button release, so the input hook releases the press
/// when the window's size changes under it. A decorated window keeps its press through the same size change.
#[trial]
fn a_press_is_released_after_an_os_resize_of_an_undecorated_window() {
    for (chrome, released) in [(WindowChrome::Integrated, true), (WindowChrome::Native, false)] {
        let shell = ShellInfo::default().window(WindowInfo::new(WindowKind::Main).chrome(chrome));
        let windows = Windows::with(InstallSpec::new(shell, Arc::new(NoWindows)));
        let down = Cell::new(false);
        let sized =
            |width: f32| ViewportInfo { inner_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 600.0))), ..ViewportInfo::default() };
        let mut frame = |events: Vec<Event>| {
            windows.frame(events, vec![], |ui| down.set(ui.ctx().input(|input| input.pointer.any_down())));
        };
        windows.root_info(sized(800.0));
        frame(vec![]);
        frame(vec![moved(Pos2::new(20.0, 20.0))]);
        frame(vec![press(Pos2::new(20.0, 20.0))]);
        assert!(down.get(), "the press is down");
        windows.root_info(sized(812.0));
        frame(vec![]);
        frame(vec![]);
        assert_eq!(!down.get(), released, "{chrome:?}: the press after the OS resize");
    }
}

/// A modal in one window blocks presses in another, and the blocked press raises the modal's window.
#[trial]
fn a_press_blocked_by_a_modal_in_another_window_raises_the_modal_window() {
    let windows = Windows::new();
    let mut frame = |root: Vec<Event>| {
        windows.frame(root, vec![], |ui| {
            ui.ctx().show_viewport_immediate(child(), ViewportBuilder::default(), |cui, _| {
                let props = ContainerProps::new().mount(Mount::Layer(Overlay::modal()));
                Modal { common: Common::new(), props }.id_salt("modal").show(cui, |_, _| {});
            });
        })
    };
    for _ in 0..4 {
        frame(vec![]);
    }
    frame(vec![moved(Pos2::new(30.0, 30.0))]);
    let out = frame(vec![press(Pos2::new(30.0, 30.0))]);
    assert!(out.commands.contains(&(child(), ViewportCommand::Focus)), "the press raised the modal's window: {:?}", out.commands);
}

/// `act` is the one sender of window commands and checks capabilities first.
#[trial]
fn act_sends_window_commands_only_where_the_capability_is_on() {
    let native = Windows::new();
    let web = Windows::with(InstallSpec::new(ShellInfo::default().caps(Caps::web()), Arc::new(NoWindows)));
    assert!(act(&native.ctx, ViewportId::ROOT, WindowAction::Minimize), "a native build sends the command");
    assert!(!act(&web.ctx, ViewportId::ROOT, WindowAction::Minimize), "a browser build has no window commands");
    assert!(!act(&web.ctx, ViewportId::ROOT, WindowAction::ToggleFullscreen));
}
