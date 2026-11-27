//! Count trials of windows on craft-ui-rt (B10): a second window builds only itself, input and
//! closes go to one window, a close guard answers close requests and Quit, an app-modal dialog
//! refuses the other windows' close, a job wake runs only its view, chrome zones reach the hit sink
//! and placements persist. Relations only, no times.

use std::sync::{Arc, Mutex};

use crate::skeleton::IconButton;
use craft_behavior::placement::Defaults;
use craft_behavior::{IconId, PartKey};
use craft_ui::contract::overlay::{Overlay, show_dialog};
use craft_ui::contract::{Common, Component, Cx, MeasureCx, MemoView, NoValue, Outbox, Outcome, PartsCx, Pending, Response, Role, Show as _, Ui};
use craft_ui::runtime::env::EnvCells;
use craft_ui::runtime::pending::JobBoard;
use craft_ui::runtime::windows::{CaptionRegions, HitSink, WindowEnv, WindowHit, WindowRegistry, caption_regions, rt_spec, window_root_with};
use craft_ui::style::StyleCx;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{CloseReason, MonitorInfo, Platform, Pos2, Rect, Sense, StepReport, Vec2, WindowCommand, WindowId, WindowSpec};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;
use craft_window::Button;
use craft_window::restore::Store;

const FADE_END: f64 = 0.25;

/// A zero-input leaf that answers its window's close requests.
#[derive(craft_ui::contract::Component)]
#[component(unregistered)]
struct CloseGuard {
    #[common]
    common: Common,
}

struct GuardOut {
    response: Response,
    ask: Option<CloseReason>,
}

impl Outcome for GuardOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl Component for CloseGuard {
    const ROLE: Role = Role::Node(accesskit::Role::GenericContainer);
    const STOPS: bool = false;
    type Style = PlainStyle;
    type Output = GuardOut;

    fn style(_: &StyleCx<'_>, _: CloseGuardStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::ZERO)
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(4.0, 4.0)
    }

    fn close_guard(&self) -> bool {
        true
    }

    fn finish(self, cx: Cx<'_, Self>) -> GuardOut {
        GuardOut { response: cx.response.clone(), ask: cx.close_request() }
    }
}

/// A title bar: a drag area for the window with a button over it.
#[derive(craft_ui::contract::Component)]
#[component(unregistered)]
struct TitleBar {
    #[common]
    common: Common,
}

impl TitleBar {
    const DRAG: PartKey = PartKey::new("drag");
}

impl Component for TitleBar {
    const ROLE: Role = Role::Node(accesskit::Role::TitleBar);
    const STOPS: bool = false;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: TitleBarStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::ZERO)
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(300.0, 30.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(Self::DRAG, rect).exclusive(WindowHit::Move);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct App {
    main_runs: u32,
    second_runs: u32,
    open: Rect,
    second_button: Rect,
    second_clicks: u32,
    main_guard: bool,
    second_guard: bool,
    allow_close: bool,
    asked: Vec<(&'static str, CloseReason)>,
    modal_open: bool,
    title_bar: bool,
}

impl Default for App {
    fn default() -> App {
        App {
            main_runs: 0,
            second_runs: 0,
            open: Rect::NOTHING,
            second_button: Rect::NOTHING,
            second_clicks: 0,
            main_guard: false,
            second_guard: false,
            allow_close: false,
            asked: Vec::new(),
            modal_open: false,
            title_bar: false,
        }
    }
}

fn guard(ui: &mut Ui, app: &mut App, name: &'static str) {
    if let Some(reason) = (CloseGuard { common: Common::new() }).show(ui).ask {
        if app.allow_close {
            ui.close_window();
        } else {
            app.asked.push((name, reason));
        }
    }
}

fn main_ui(ui: &mut Ui, app: &mut App) {
    app.main_runs += 1;
    if app.title_bar {
        let _ = TitleBar { common: Common::new() }.show(ui);
    }
    let open = IconButton::new(IconId::new("plus"), "Open").show(ui);
    app.open = open.rect;
    if open.clicked() {
        ui.open_window(WindowSpec { size: Some(Vec2::new(200.0, 200.0)), ..WindowSpec::new("Second") });
    }
    if app.main_guard {
        guard(ui, app, "main");
    }
    if app.modal_open {
        let _ = show_dialog(ui, 1, &Overlay::modal(), |ui| IconButton::new(IconId::new("check"), "OK").show(ui));
    }
}

fn second_ui(ui: &mut Ui, app: &mut App) {
    app.second_runs += 1;
    let button = IconButton::new(IconId::new("plus"), "Second").show(ui);
    app.second_button = button.rect;
    app.second_clicks += u32::from(button.clicked());
    if app.second_guard {
        guard(ui, app, "second");
    }
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    let main = rig.main;
    rig.step_with(&mut |window, cx| {
        if window == main {
            window_root_with(cx, WindowEnv::default(), |ui| main_ui(ui, app));
        } else {
            window_root_with(cx, WindowEnv::default(), |ui| second_ui(ui, app));
        }
    })
}

fn settle(rig: &mut Rig<RecordingRenderer>, app: &mut App) {
    for _ in 0..8 {
        rig.driver.platform.advance(FADE_END);
        if !step(rig, app).ran {
            return;
        }
    }
    panic!("the rig did not settle");
}

fn rig(app: &mut App) -> Rig<RecordingRenderer> {
    let mut rig = Rig::inline(Vec2::new(400.0, 400.0), 1.0, RecordingRenderer::default());
    step(&mut rig, app);
    settle(&mut rig, app);
    rig
}

fn send(rig: &mut Rig<RecordingRenderer>, app: &mut App, window: WindowId, input: Input) -> StepReport {
    rig.inject(window, input);
    step(rig, app)
}

fn click(rig: &mut Rig<RecordingRenderer>, app: &mut App, window: WindowId, at: Pos2) -> StepReport {
    send(rig, app, window, Input::mouse_move(at));
    send(rig, app, window, Input::mouse_down(at));
    send(rig, app, window, Input::mouse_up(at))
}

fn windows(rig: &mut Rig<RecordingRenderer>) -> Vec<WindowId> {
    rig.driver.runtime.windows().collect()
}

/// Opens the second window through the Open button and settles; returns its id.
fn open_second(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> WindowId {
    let at = app.open.center();
    let main = rig.main;
    click(rig, app, main, at);
    settle(rig, app);
    let all = windows(rig);
    assert_eq!(all.len(), 2, "the Open click opened one window");
    *all.iter().find(|w| **w != main).unwrap()
}

#[test]
fn opening_a_second_window_builds_only_it() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    let main_runs = app.main_runs;
    let at = app.open.center();
    let main = rig.main;
    click(&mut rig, &mut app, main, at);
    let after_click = app.main_runs;
    assert_eq!(after_click, main_runs + 1, "the click ran the window that asked, once");
    assert_eq!(app.second_runs, 0);
    // The driver opened the window at the end of the click's step; the next step builds it.
    let opened = step(&mut rig, &mut app);
    assert_eq!(windows(&mut rig).len(), 2);
    assert_eq!(app.main_runs, after_click, "opening the window built nothing in the first");
    assert_eq!(app.second_runs, 1, "the new window's root ran once");
    assert_eq!(opened.frame.scopes, 1, "only the new window's root scope: {opened:?}");
}

#[test]
fn input_in_one_window_runs_nothing_in_the_other() {
    let mut app = App::default();
    let mut rig = rig(&mut app);
    let second = open_second(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    let main_runs = app.main_runs;
    let at = app.second_button.center();
    click(&mut rig, &mut app, second, at);
    assert_eq!(app.second_clicks, 1, "the click reached the second window");
    assert_eq!(app.main_runs, main_runs, "nothing ran in the first window");
    let main = rig.main;
    let at = app.open.center();
    let second_runs = app.second_runs;
    send(&mut rig, &mut app, main, Input::mouse_move(at));
    assert_eq!(app.second_runs, second_runs, "a move in the first window runs nothing in the second");
}

#[test]
fn a_refused_close_keeps_the_window_and_runs_only_the_asking_view() {
    let mut app = App { second_guard: true, ..App::default() };
    let mut rig = rig(&mut app);
    let second = open_second(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    let main_runs = app.main_runs;
    let second_runs = app.second_runs;
    let asked = send(&mut rig, &mut app, second, Input::CloseRequested);
    assert_eq!(app.asked, [("second", CloseReason::Window)], "the guard was asked once");
    assert_eq!(windows(&mut rig).len(), 2, "the app kept the window");
    assert_eq!(app.main_runs, main_runs, "the first window ran nothing");
    assert_eq!(app.second_runs, second_runs + 1, "only the asking window's view ran");
    assert_eq!(asked.frame.runs, 1, "and only the guard's hook: {asked:?}");
    app.allow_close = true;
    send(&mut rig, &mut app, second, Input::CloseRequested);
    settle(&mut rig, &mut app);
    assert_eq!(windows(&mut rig), [rig.main], "once the app agrees the same path closes the window");
}

#[test]
fn quit_asks_each_window_once_and_closes_unguarded_windows_at_once() {
    let mut app = App { main_guard: true, second_guard: true, ..App::default() };
    let mut rig = rig(&mut app);
    open_second(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    rig.driver.quit();
    step(&mut rig, &mut app);
    step(&mut rig, &mut app);
    let mut asked = app.asked.clone();
    asked.sort_by_key(|(name, _)| *name);
    assert_eq!(asked, [("main", CloseReason::Quit), ("second", CloseReason::Quit)], "each window was asked once");
    assert_eq!(windows(&mut rig).len(), 2, "both refused, so both stay");
    app.allow_close = true;
    rig.driver.quit();
    step(&mut rig, &mut app);
    step(&mut rig, &mut app);
    assert!(windows(&mut rig).is_empty(), "with the app's agreement Quit closed every window");

    let mut app = App::default();
    let mut rig = rig_with_second(&mut app);
    rig.driver.quit();
    step(&mut rig, &mut app);
    step(&mut rig, &mut app);
    assert!(windows(&mut rig).is_empty(), "windows without a guard close at once");
}

fn rig_with_second(app: &mut App) -> Rig<RecordingRenderer> {
    let mut r = rig(app);
    open_second(&mut r, app);
    settle(&mut r, app);
    r
}

#[test]
fn an_app_modal_dialog_refuses_the_close_of_the_other_window() {
    let mut app = App { second_guard: true, ..App::default() };
    let mut rig = rig(&mut app);
    let second = open_second(&mut rig, &mut app);
    app.modal_open = true;
    rig.driver.runtime.mark_root(rig.main);
    settle(&mut rig, &mut app);
    let main_runs = app.main_runs;
    let second_runs = app.second_runs;
    send(&mut rig, &mut app, second, Input::CloseRequested);
    assert!(app.asked.is_empty(), "the blocked window's guard was not asked");
    assert_eq!(windows(&mut rig).len(), 2, "the blocked window stays open");
    assert_eq!(rig.platform().focused(), Some(rig.main), "the dialog's window takes focus");
    assert_eq!((app.main_runs, app.second_runs), (main_runs, second_runs), "nothing ran");
    rig.driver.quit();
    step(&mut rig, &mut app);
    assert_eq!(app.asked, [], "Quit does not ask the blocked window either");
}

#[derive(Default)]
struct Jobbed {
    result: Option<u32>,
    view_runs: u32,
    other_runs: u32,
}

/// What the two views of the job trial report.
#[derive(Clone, Copy, Debug, PartialEq)]
enum JobEvent {
    Ran(Option<u32>),
    OtherRan,
}

fn job_view(out: &mut Outbox<'_, JobEvent>, ui: &mut Ui) {
    let result = match ui.job("answer", || Ok::<u32, ()>(7)) {
        Pending::Ready(v) => Some(v),
        _ => None,
    };
    out.send(JobEvent::Ran(result));
}

fn other_view(out: &mut Outbox<'_, JobEvent>, _: &mut Ui) {
    out.send(JobEvent::OtherRan);
}

#[derive(Clone, Default)]
struct Manual(Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>);

impl craft_host::Jobs for Manual {
    fn spawn(&self, _: &str, job: Box<dyn FnOnce() + Send>) {
        self.0.lock().unwrap().push(job);
    }
}

#[test]
fn a_job_wake_delivers_through_the_driver_and_runs_only_the_view_that_started_it() {
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let queue = Manual::default();
    let board = JobBoard::new(Arc::new(queue.clone()), rig.driver.waker());
    let deliver = board.clone();
    rig.driver.set_wake_handler(move |store, images| {
        deliver.deliver(store, images);
    });
    let mut app = Jobbed::default();
    let host = WindowEnv { cells: None::<EnvCells>, jobs: Some(board), commands: None };
    let frame = |rig: &mut Rig<RecordingRenderer>, app: &mut Jobbed| {
        rig.frame_with(0.0, &mut |_, cx| {
            window_root_with(cx, host.clone(), |ui| {
                let mut sent = Vec::new();
                let _ = MemoView::new(1u32, Outbox::new(&mut sent), job_view).show(ui);
                let _ = MemoView::new(2u32, Outbox::new(&mut sent), other_view).show(ui);
                for event in sent {
                    match event {
                        JobEvent::Ran(result) => (app.view_runs, app.result) = (app.view_runs + 1, result),
                        JobEvent::OtherRan => app.other_runs += 1,
                    }
                }
            })
        })
    };
    for _ in 0..4 {
        if !frame(&mut rig, &mut app).ran {
            break;
        }
    }
    assert_eq!((app.result, app.view_runs, app.other_runs), (None, 1, 1));
    let queued = std::mem::take(&mut *queue.0.lock().unwrap());
    assert_eq!(queued.len(), 1, "one job was queued");
    for job in queued {
        job();
    }
    // The job's waker asked the platform for a wake; the driver delivers it in the next step.
    let done = frame(&mut rig, &mut app);
    assert_eq!(app.result, Some(7), "the driver delivered the result on the wake");
    assert_eq!((app.view_runs, app.other_runs), (2, 1), "only the waiting view's closure ran again");
    assert_eq!(done.frame.scopes, 2, "the root and the waiting view: {done:?}");
}

#[test]
fn exclusive_parts_become_caption_regions_and_reach_the_hit_sink() {
    let mut app = App { title_bar: true, ..App::default() };
    let mut rig = rig(&mut app);
    let main = rig.main;
    let regions: CaptionRegions = caption_regions(&rig.driver.runtime, main).expect("the title bar declared a drag area");
    assert_eq!(regions.moves.len(), 1);
    assert_eq!(regions.covers.len(), 1);
    let band = regions.moves[0];
    assert!(band.width() > 100.0, "the drag area is the bar: {band:?}");
    assert!(!regions.covers[0].is_empty() || app.open.min.y >= band.max.y, "a button over the bar would cover it");

    let mut registry = WindowRegistry::new(Store::in_memory());
    let sink = HitSink::default();
    registry.attach_hit_sink(main, sink.clone());
    let ran = rig.step_with(&mut |_, cx| {
        window_root_with(cx, WindowEnv::default(), |ui| main_ui(ui, &mut app));
    });
    registry.after_step(&mut rig.driver.runtime, rig.driver.platform.surfaces(), true, 0.0);
    let _ = ran;
    assert_eq!(sink.answer(band.center()), Some(WindowHit::Move), "the window procedure moves the window from the bar");
    assert_eq!(sink.answer(Pos2::new(5000.0, 5000.0)), None);
    let _ = Button::Close;
}

fn desktop() -> Vec<MonitorInfo> {
    vec![MonitorInfo {
        name: Some("DP-1".into()),
        position: Pos2::ZERO,
        size: Vec2::new(1920.0, 1080.0),
        scale: 1.0,
        refresh_hz: None,
        primary: true,
    }]
}

fn keyed(title: &str) -> WindowSpec {
    WindowSpec { key: Some("doc".into()), ..WindowSpec::new(title) }
}

#[test]
fn a_keyed_window_keeps_its_placement_and_reopens_there() {
    let mut registry = WindowRegistry::new(Store::in_memory());
    let defaults = Defaults::new([300.0, 200.0], [100.0, 100.0]);
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    rig.driver.platform.set_monitors(desktop());
    let spec = registry.place(keyed("Doc"), &defaults, rig.driver.platform.surfaces());
    assert_eq!(spec.size, Some(Vec2::new(300.0, 200.0)), "nothing saved: the defaults");
    let window = rig.driver.open_inline_window(&spec);
    for input in [Input::Resize(Vec2::new(640.0, 480.0)), Input::Moved(Pos2::new(120.0, 80.0))] {
        rig.inject(window, input);
    }
    let mut app = App::default();
    step(&mut rig, &mut app);
    registry.after_step(&mut rig.driver.runtime, rig.driver.platform.surfaces(), false, 0.0);
    let saved = registry.saved("doc").expect("the window's placement was recorded");
    assert_eq!(saved.normal.size, [640.0, 480.0]);
    assert_eq!(saved.position_px, Some([120, 80]));
    assert_eq!(saved.monitor.as_ref().map(|m| m.name.as_str()), Some("DP-1"), "the registry read the monitors from the platform");
    let again = registry.place(keyed("Doc"), &defaults, rig.driver.platform.surfaces());
    assert_eq!(again.size, Some(Vec2::new(640.0, 480.0)), "the next window with the key opens at the saved size");
    assert_eq!(again.position, Some(Pos2::new(120.0, 80.0)));
}

#[test]
fn a_state_event_is_what_the_registry_saves() {
    let mut registry = WindowRegistry::new(Store::in_memory());
    let defaults = Defaults::new([300.0, 200.0], [100.0, 100.0]);
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    rig.driver.platform.set_monitors(desktop());
    let spec = registry.place(keyed("Doc"), &defaults, rig.driver.platform.surfaces());
    let window = rig.driver.open_inline_window(&spec);
    rig.inject(window, Input::Resize(Vec2::new(640.0, 480.0)));
    let mut app = App::default();
    step(&mut rig, &mut app);
    registry.after_step(&mut rig.driver.runtime, rig.driver.platform.surfaces(), false, 0.0);
    assert!(!registry.saved("doc").unwrap().maximized);

    // The platform maximizes the window: it sends the state and the larger size.
    rig.driver.platform.window(window, WindowCommand::Maximized(true));
    rig.inject(window, Input::Resize(Vec2::new(1900.0, 1000.0)));
    step(&mut rig, &mut app);
    registry.after_step(&mut rig.driver.runtime, rig.driver.platform.surfaces(), false, 1.0);
    let saved = registry.saved("doc").unwrap();
    assert!(saved.maximized, "the State event reached the saved placement");
    assert_eq!(saved.normal.size, [640.0, 480.0], "the last normal size is kept");
}

#[test]
fn a_decorated_window_reopens_at_its_outer_position_without_drift() {
    let mut registry = WindowRegistry::new(Store::in_memory());
    let defaults = Defaults::new([300.0, 200.0], [100.0, 100.0]);
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    rig.driver.platform.set_monitors(desktop());
    let mut app = App::default();
    let inset = Vec2::new(0.0, 30.0);
    let mut spec = registry.place(keyed("Doc"), &defaults, rig.driver.platform.surfaces());
    spec.position = Some(Pos2::new(120.0, 80.0));
    for round in 0..3 {
        let window = rig.driver.open_inline_window(&spec);
        // The system puts the surface below the title bar of the outer position it was given.
        let outer = spec.position.unwrap();
        for input in [Input::Resize(Vec2::new(640.0, 480.0)), Input::FrameInset(inset), Input::Moved(outer + inset)] {
            rig.inject(window, input);
        }
        step(&mut rig, &mut app);
        registry.after_step(&mut rig.driver.runtime, rig.driver.platform.surfaces(), false, round as f64);
        spec = registry.place(keyed("Doc"), &defaults, rig.driver.platform.surfaces());
        assert_eq!(spec.position, Some(Pos2::new(120.0, 80.0)), "round {round}: no drift by the title bar height");
        rig.driver.platform.window(window, WindowCommand::Close);
        step(&mut rig, &mut app);
        registry.after_step(&mut rig.driver.runtime, rig.driver.platform.surfaces(), false, round as f64 + 0.5);
    }
}

#[test]
fn a_spec_carries_level_and_taskbar_to_the_platform() {
    use craft_ui::runtime::windows::{WindowLevel as Level, WindowSpec as Spec};
    let frame = |_| None;
    let tool = rt_spec(&Spec::tool(), "Tools", None, &frame);
    assert_eq!((tool.level, tool.taskbar), (craft_ui_rt::WindowLevel::AboveOthers, false));
    let dialog = rt_spec(&Spec::dialog().level(Level::Normal), "Dlg", None, &frame);
    assert_eq!((dialog.level, dialog.taskbar), (craft_ui_rt::WindowLevel::Normal, true));
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let window = rig.driver.open_inline_window(&tool);
    let state = rig.driver.platform.window_state(window).unwrap();
    assert_eq!((state.level, state.taskbar), (craft_ui_rt::WindowLevel::AboveOthers, false), "the platform got both");
}
