//! `runtime::windows`: the input hook per viewport, window info per viewport, the press release after an OS
//! resize (H4), the raise of a modal's window, and `act`.

use crate::contract::kit::PlainStyle;
use crate::windows_kit::Windows;
use craft_behavior::{PressCancel, Sense};
use craft_host::target::{Caps, Os, ShellInfo, WindowChrome, WindowInfo, WindowKind, WindowState};
use craft_ui::contract::{
    Bind, Common, Component, Container, ContainerProps, Cx, Mount, Overlay, Pointer as _, Props as _, Role, Show as _, ShowContainer as _,
};
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::commands::FireSource;
use craft_ui::runtime::env::Env;
use craft_ui::runtime::frame::take_output;
use craft_ui::runtime::windows::{ResizeEdge, WindowAction, WindowHost, WindowLevel, WindowSpec, act};
use craft_ui::style::StyleCx;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::windows::{FakeSystem, NoWindows, dragged_id, record_input_hooks};
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::TextField;
use craft_window::policy::{CaptionFamily, Captions, Decision, Desktop, DesktopKind, Frame};
use craft_window::restore::{MAIN, Monitor, MonitorKey, Placement, Store};
use egui::{Event, Key, Modifiers, Pos2, Rect, Response, Vec2, ViewportBuilder, ViewportCommand, ViewportId, ViewportInfo};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

const HANDLE: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(90.0, 50.0) };
/// The least size a tool window in the trials is asked to keep.
const TOOL_MIN: Vec2 = Vec2::new(160.0, 100.0);
/// Where a palette's window stands before the trial moves it, and a point inside it.
const PLACED: Rect = Rect { min: Pos2::new(100.0, 100.0), max: Pos2::new(500.0, 400.0) };
const IN_PLACED: Pos2 = Pos2::new(150.0, 150.0);
/// Where the trial moves the window to and the size it gives it, with the rect that makes and a point inside it.
const MOVE_TO: Pos2 = Pos2::new(500.0, 400.0);
const SIZE_TO: Vec2 = Vec2::new(320.0, 240.0);
const MOVED: Rect = Rect { min: MOVE_TO, max: Pos2::new(820.0, 640.0) };
const IN_MOVED: Pos2 = Pos2::new(510.0, 410.0);
const NOWHERE: Pos2 = Pos2::new(900.0, 700.0);
/// The trial's monitor: 3840 by 2160 pixels at two pixels per point.
const MONITOR_PIXELS: Rect = Rect { min: Pos2::ZERO, max: Pos2::new(3840.0, 2160.0) };
const MONITOR_SCALE: f32 = 2.0;

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
    let frame = |root: Vec<Event>, child_events: Vec<Event>| {
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

/// H4: where the OS resizes undecorated windows, such a window never gets the button release, so the input hook
/// releases the press when the window's size changes under it. A decorated window keeps its press through the same
/// size change, and so does an undecorated window on a platform that delivers the release itself.
#[trial]
fn a_press_is_released_after_an_os_resize_of_an_undecorated_window() {
    let cases =
        [(Os::Windows, WindowChrome::Integrated, true), (Os::Windows, WindowChrome::Native, false), (Os::Linux, WindowChrome::Integrated, false)];
    for (os, chrome, released) in cases {
        let shell = ShellInfo::default().os(os).caps(Caps::native(os)).window(WindowInfo::new(WindowKind::Main).chrome(chrome));
        let windows = Windows::with(InstallSpec::new(shell, Arc::new(NoWindows)));
        let down = Cell::new(false);
        let sized =
            |width: f32| ViewportInfo { inner_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 600.0))), ..ViewportInfo::default() };
        let frame = |events: Vec<Event>| {
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
        assert_eq!(!down.get(), released, "{os:?} {chrome:?}: the press after the OS resize");
    }
}

/// The placement store is written when the window's geometry changes, not on every pass.
#[trial]
fn placement_is_written_only_when_the_window_geometry_changes() {
    let store = Arc::new(Mutex::new(Store::in_memory()));
    let windows = Windows::with(InstallSpec::new(ShellInfo::default(), Arc::new(NoWindows)).placement(store.clone()));
    let sized = |width: f32| ViewportInfo {
        inner_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 600.0))),
        outer_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 630.0))),
        ..ViewportInfo::default()
    };
    let width = || store.lock().expect("the store").get(MAIN).map(|placement| placement.normal.size[0]);
    windows.root_info(sized(800.0));
    windows.frame(vec![], vec![], |_| {});
    assert_eq!(width(), Some(800.0), "the first pass records the window");
    let marker = Placement::new([1.0, 1.0], 1.0);
    store.lock().expect("the store").set(MAIN, &marker);
    windows.frame(vec![], vec![], |_| {});
    assert_eq!(width(), Some(1.0), "an unchanged window writes nothing");
    windows.root_info(sized(900.0));
    windows.frame(vec![], vec![], |_| {});
    assert_eq!(width(), Some(900.0), "a resized window is recorded");
}

/// The shell's OS, capabilities and menu placement follow the OS a fixture simulates, as the keys do.
#[trial]
fn the_shell_follows_the_fixtures_os() {
    let seen = Rc::new(Cell::new(None));
    let shown = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let shell = Env::of(ui).view().shell().clone();
        shown.set(Some((shell.os, shell.caps.contains(Caps::NATIVE_MENU_BAR), shell.caps.contains(Caps::OS_RESIZES_UNDECORATED))));
    });
    for (os, menu, resizes) in [(Os::Mac, true, false), (Os::Windows, false, true), (Os::Linux, false, false)] {
        fixture.os(os).frame(vec![]);
        assert_eq!(seen.get(), Some((os, menu, resizes)), "{os:?}");
    }
}

/// A modal in one window blocks presses in another, and the blocked press raises the modal's window.
#[trial]
fn a_press_blocked_by_a_modal_in_another_window_raises_the_modal_window() {
    let windows = Windows::new();
    let frame = |root: Vec<Event>| {
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

/// A container mounted in an OS viewport that opens as `spec`, with a body the trial gives it.
#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Palette {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl Palette {
    fn new(title: &'static str, spec: Option<WindowSpec>) -> Self {
        Self::in_overlay(title, spec, Overlay::floating())
    }

    /// A palette whose fallback layer, and so whose window's limits, position and resizability, is `overlay`.
    fn in_overlay(title: &'static str, spec: Option<WindowSpec>, overlay: Overlay) -> Self {
        let mount = match spec {
            Some(spec) => Mount::viewport_with(overlay, spec),
            None => Mount::viewport(overlay),
        };
        Palette { common: Common::new(), props: ContainerProps::new().mount(mount) }.label(title).id_salt(title)
    }
}

impl Component for Palette {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PaletteStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::new(120.0, 60.0))
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Palette {}

fn builder_titled<'a>(out: &'a crate::windows_kit::Out, title: &str) -> (ViewportId, &'a ViewportBuilder) {
    let (id, builder) = out.builders.iter().find(|(_, builder)| builder.title.as_deref() == Some(title)).expect("the window asked to open");
    (*id, builder)
}

/// A decision that has the app draw the frame, with the Windows caption layout.
fn app_drawn() -> Decision {
    let desktop = Desktop { kind: DesktopKind::Other, ssd: None, gtk_theme: None, icon_theme: None };
    let captions = Captions { family: CaptionFamily::Windows, layout: craft_window::Layout::default() };
    Decision { frame: Frame::Integrated(captions), overridden: None, desktop }
}

/// A decision that leaves the frame to the OS (the system title bar).
fn system_drawn() -> Decision {
    let desktop = Desktop { kind: DesktopKind::Other, ssd: None, gtk_theme: None, icon_theme: None };
    Decision { frame: Frame::Native, overridden: Some(false), desktop }
}

/// A tool window, with or without a spec, and a window with no spec opened by a non-modal mount take the chrome and
/// the captions of the installed decision for their kind; an explicit chrome overrides it. A tool window shows no
/// caption buttons under either decision.
#[trial]
fn a_tool_window_follows_the_installed_decision() {
    let cases = [(app_drawn(), WindowChrome::Integrated, Some(false), Some(false)), (system_drawn(), WindowChrome::Native, Some(true), None)];
    for (decision, chrome, decorations, buttons) in cases {
        let windows = Windows::with(InstallSpec::new(ShellInfo::default(), Arc::new(FakeSystem::new())).frame(decision));
        let seen = RefCell::new(None);
        let mut out = None;
        for _ in 0..3 {
            out = Some(windows.frame(vec![], vec![], |ui| {
                Palette::new("tool", Some(WindowSpec::tool())).show(ui, |ui, _| {
                    let host = WindowHost::of(ui.ctx());
                    *seen.borrow_mut() = Some((Env::of(ui).view().shell().window.clone(), host.frame()));
                });
                Palette::new("plain", None).show(ui, |_, _| {});
                Palette::new("forced", Some(WindowSpec::tool().chrome(WindowChrome::Native))).show(ui, |_, _| {});
            }));
        }
        let out = out.expect("frames ran");
        for title in ["tool", "plain"] {
            let (_, builder) = builder_titled(&out, title);
            assert_eq!((builder.decorations, builder.titlebar_buttons_shown), (decorations, buttons), "{chrome:?}: the {title} window");
        }
        assert_eq!(builder_titled(&out, "forced").1.decorations, Some(true), "{chrome:?}: a spec's own chrome wins");
        let (window, frame) = seen.take().expect("the tool window ran");
        assert_eq!((window.kind, window.chrome), (WindowKind::Tool, chrome), "the window info follows the decision");
        assert!(
            matches!(&frame, Frame::Native) || matches!(&frame, Frame::Integrated(captions) if captions.is_empty()),
            "a tool window shows no caption buttons: {frame:?}"
        );
    }
}

/// A viewport container's size limits, position and resizability are its fallback overlay's: the window opens with
/// them, and a window that is not resizable refuses a resize the OS would start.
#[trial]
fn a_viewport_window_takes_its_limits_and_position_from_the_overlay() {
    let windows = Windows::new();
    let overlay = || Overlay::floating().size_limits(TOOL_MIN, SIZE_TO).placed(PLACED);
    let mut out = None;
    for _ in 0..3 {
        out = Some(windows.frame(vec![], vec![], |ui| {
            Palette::in_overlay("fixed", Some(WindowSpec::tool()), overlay()).show(ui, |_, _| {});
            Palette::in_overlay("grips", None, overlay().resizable(true)).show(ui, |_, _| {});
        }));
    }
    let out = out.expect("frames ran");
    let (fixed_id, fixed) = builder_titled(&out, "fixed");
    let (grips_id, grips) = builder_titled(&out, "grips");
    for builder in [fixed, grips] {
        assert_eq!((builder.min_inner_size, builder.max_inner_size), (Some(TOOL_MIN), Some(SIZE_TO)));
        assert_eq!(builder.position, Some(PLACED.min));
    }
    assert_eq!((fixed.resizable, grips.resizable), (Some(false), Some(true)));
    assert!(!act(&windows.ctx, fixed_id, WindowAction::StartResize(ResizeEdge::Left)), "a window that cannot be resized starts no resize");
    assert!(act(&windows.ctx, grips_id, WindowAction::StartResize(ResizeEdge::Left)));
}

/// A palette stays above the app's windows while one has focus and drops to the normal level once none has had it
/// for a moment; focus coming back raises it again.
#[trial]
fn a_palette_drops_its_level_while_the_app_is_inactive() {
    let windows = Windows::new();
    let level = |windows: &Windows| {
        let mut out = None;
        for _ in 0..3 {
            out = Some(windows.frame(vec![], vec![], |ui| {
                Palette::new("tool", Some(WindowSpec::tool())).show(ui, |_, _| {});
            }));
        }
        builder_titled(&out.expect("frames ran"), "tool").1.window_level
    };
    let focus = |on| ViewportInfo { focused: Some(on), ..ViewportInfo::default() };
    windows.root_info(focus(true));
    windows.child_info(focus(false));
    assert_eq!(level(&windows), Some(egui::WindowLevel::AlwaysOnTop), "the main window has focus");
    windows.root_info(focus(false));
    windows.advance(1.0);
    assert_eq!(level(&windows), Some(egui::WindowLevel::Normal), "no window of the app has focus");
    windows.root_info(focus(true));
    assert_eq!(level(&windows), Some(egui::WindowLevel::AlwaysOnTop), "the app is active again");
}

/// `SetOuterPosition` and `SetInnerSize` move and size the window as the facts show; the window host lists the
/// system's monitors in points and finds the window under a desktop point.
#[trial]
fn window_actions_and_queries_follow_the_window_system() {
    let system = Arc::new(FakeSystem::new());
    system.set_monitors(vec![Monitor::new(MonitorKey::new("DP-1", [3840, 2160]), [0, 0], [3840, 2160], MONITOR_SCALE)]);
    let windows = Windows::with(InstallSpec::new(ShellInfo::default(), system));
    windows.child_info(ViewportInfo { inner_rect: Some(PLACED), outer_rect: Some(PLACED), ..ViewportInfo::default() });
    let frame = || {
        windows.frame(vec![], vec![], |ui| {
            Palette::new("tool", Some(WindowSpec::tool())).show(ui, |_, _| {});
        })
    };
    let out = (0..3).map(|_| frame()).last().expect("frames ran");
    let (viewport, _) = builder_titled(&out, "tool");
    let host = WindowHost::of(&windows.ctx);
    assert_eq!(host.facts(viewport).map(|facts| facts.inner), Some(PLACED));
    assert_eq!(host.viewport_at(IN_PLACED), Some(viewport));
    assert_eq!(host.viewport_at(NOWHERE), None, "nothing is there");
    let monitors: Vec<_> = host.monitors().into_iter().map(|monitor| (monitor.name, monitor.pixels, monitor.scale)).collect();
    assert_eq!(monitors, [("DP-1".to_owned(), MONITOR_PIXELS, MONITOR_SCALE)], "monitors in pixels");
    assert!(act(&windows.ctx, viewport, WindowAction::SetOuterPosition(MOVE_TO)));
    assert!(act(&windows.ctx, viewport, WindowAction::SetInnerSize(SIZE_TO)));
    frame();
    frame();
    let facts = host.facts(viewport).expect("the window reported");
    assert_eq!(facts.inner, MOVED);
    assert_eq!(host.viewport_at(IN_PLACED), None, "the window left that place");
    assert_eq!(host.viewport_at(IN_MOVED), Some(viewport));
    assert!(act(&windows.ctx, viewport, WindowAction::SetTitle("Tools".into())));
    assert!(act(&windows.ctx, viewport, WindowAction::SetLevel(WindowLevel::Normal)));
    let sent = [frame(), frame()];
    assert!(
        sent.iter().any(|out| out.commands.contains(&(viewport, ViewportCommand::Title("Tools".into())))),
        "the title command went out: {:?}",
        sent.iter().map(|out| &out.commands).collect::<Vec<_>>()
    );
    assert_eq!(builder_titled(&sent[1], "tool").1.window_level, Some(egui::WindowLevel::Normal), "the palette left its level");
}

/// Monitors of different scales that touch in pixels touch in the points of any one window's scale: no gap, no
/// overlap.
#[trial]
fn monitors_of_different_scales_touch_in_one_window_scale() {
    let system = Arc::new(FakeSystem::new());
    system.set_monitors(vec![
        Monitor::new(MonitorKey::new("DP-1", [3840, 2160]), [0, 0], [3840, 2160], 2.0),
        Monitor::new(MonitorKey::new("DP-2", [1920, 1080]), [3840, 0], [1920, 1080], 1.0),
    ]);
    let windows = Windows::with(InstallSpec::new(ShellInfo::default(), system));
    let monitors = WindowHost::of(&windows.ctx).monitors();
    for scale in [1.0, 1.5, 2.0] {
        let [left, right] = [&monitors[0], &monitors[1]].map(|monitor| monitor.points_at(scale));
        assert_eq!(left.max.x, right.min.x, "the monitors touch at scale {scale}");
        assert_eq!((left.min.y, right.min.y), (0.0, 0.0));
    }
}

/// `SetLevel` on a window the runtime did not open (the root) sends the level command.
#[trial]
fn set_level_on_the_root_window_sends_the_level_command() {
    let windows = Windows::new();
    windows.frame(vec![], vec![], |_| {});
    assert!(act(&windows.ctx, ViewportId::ROOT, WindowAction::SetLevel(WindowLevel::AlwaysOnTop)));
    let out = windows.frame(vec![], vec![], |_| {});
    assert!(
        out.commands.contains(&(ViewportId::ROOT, ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop))),
        "the level command went out: {:?}",
        out.commands
    );
}

/// After a move or resize is handed to the OS, the next input without a release ends the press, in any chrome.
#[trial]
fn the_press_handed_to_the_os_is_released_on_the_next_input() {
    let windows = Windows::new();
    let (down, hand_off) = (Cell::new(false), Cell::new(false));
    let frame = |events: Vec<Event>| {
        windows.frame(events, vec![], |ui| {
            if hand_off.take() {
                assert!(act(ui.ctx(), ViewportId::ROOT, WindowAction::StartMove));
            }
            down.set(ui.ctx().input(|input| input.pointer.any_down()));
        });
    };
    let at = HANDLE.center();
    frame(vec![]);
    frame(vec![moved(at)]);
    frame(vec![press(at)]);
    assert!(down.get(), "the press is down");
    hand_off.set(true);
    frame(vec![moved(at + Vec2::splat(30.0))]);
    assert!(down.get(), "egui still holds the press in the pass that handed it to the OS");
    frame(vec![moved(at + Vec2::splat(40.0))]);
    assert!(!down.get(), "the press ended on the next input");
}

/// The frame decided at install is the root window's chrome and captions.
#[trial]
fn the_installed_frame_is_the_root_windows() {
    let desktop = Desktop { kind: DesktopKind::Other, ssd: None, gtk_theme: None, icon_theme: None };
    let captions = Captions { family: CaptionFamily::Windows, layout: craft_window::Layout::default() };
    let decision = Decision { frame: Frame::Integrated(captions), overridden: None, desktop };
    let windows = Windows::with(InstallSpec::new(ShellInfo::default(), Arc::new(NoWindows)).frame(decision.clone()));
    let seen = RefCell::new(None);
    windows.frame(vec![], vec![], |ui| {
        *seen.borrow_mut() = Some((Env::of(ui).view().shell().window.chrome, WindowHost::of(ui.ctx()).frame()));
    });
    assert_eq!(seen.take(), Some((WindowChrome::Integrated, decision.frame)));
}

/// What typing in a tool window does: a shortcut chord pressed there fires its command once, and text typed there
/// reaches the text part focused in that window.
#[trial]
fn typing_in_a_tool_viewport_reaches_the_command_board_and_its_focused_text() {
    let windows = Windows::new();
    let text = RefCell::new(String::new());
    let field = Cell::new(Rect::ZERO);
    let frame = |child: Vec<Event>| {
        windows.frame(vec![], child, |ui| {
            Palette::new("tool", Some(WindowSpec::tool())).show(ui, |ui, _| {
                let shown = TextField::new("Name", Bind::bound(&mut *text.borrow_mut())).id_salt("name").show(ui);
                field.set(shown.response.rect);
            });
        });
    };
    for _ in 0..4 {
        frame(vec![]);
    }
    take_output(&windows.ctx);
    frame(vec![key_press(Key::K, Modifiers::COMMAND)]);
    frame(vec![]);
    let fired = take_output(&windows.ctx).fired;
    let ids: Vec<_> = fired.iter().map(|fired| (fired.command.id.as_str(), fired.source)).collect();
    assert_eq!(ids, [("trial.save", FireSource::Key)], "the chord fires its command once");
    let at = field.get().center();
    frame(vec![moved(at)]);
    frame(vec![press(at)]);
    frame(vec![release(at)]);
    frame(vec![Event::Text("tool".into())]);
    frame(vec![]);
    assert_eq!(*text.borrow(), "tool", "the text reached the field focused in the tool window");
}
