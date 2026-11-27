//! Count trials of the runtime services on craft-ui-rt (B9): motion read in paint, env cells and
//! scopes, node-owned memory, jobs, the cursor and the texture cache. Relations only, no times.

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use craft_behavior::{CursorIcon, IconId, MotionMode};
use craft_i18n::Direction;
use craft_ui_contract::contract::rt::skeleton::IconButton;
use craft_ui_contract::contract::rt::{
    Component, ComponentKey, Cx, EnvCells, EnvScope, JobBoard, LayerFrame, MeasureCx, MemoView, Paint, TextureCache, TextureLook, ThemeBase, Ui,
    WindowEnv, spec, window_root_with,
};
use craft_ui_contract::contract::rt::{Response, show_leaf};
use craft_ui_contract::contract::{MemoKey, MemoProps};
use craft_ui_contract::runtime::env::ThemeScope;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{CornerRadii, Cursor, ImageKey, Item, Pos2, Rect, RgbaImage, Sense, SizeU, State, StepReport, Vec2};
use craft_ui_test_kit::rt::Rig;
use craft_ui_test_kit::rt::rig::{FRAME, test_tokens};
use egui::Color32;

/// A window with host cells, a job board that runs jobs when told, and a view function.
struct World<A> {
    rig: Rig<RecordingRenderer>,
    app: Box<A>,
    view: fn(&mut Ui<'_, '_>, &mut A),
    cells: EnvCells,
    board: JobBoard,
    queue: Manual,
    /// A cell the root reads: bumping it runs the root, as an app's own state change would.
    rev: State<u64>,
}

/// Jobs that run when the trial says so, in place of threads.
#[derive(Clone, Default)]
struct Manual(Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>);

impl craft_host::Jobs for Manual {
    fn spawn(&self, _: &str, job: Box<dyn FnOnce() + Send>) {
        self.0.lock().unwrap().push(job);
    }
}

impl Manual {
    fn run_all(&self) -> usize {
        let jobs = std::mem::take(&mut *self.0.lock().unwrap());
        let n = jobs.len();
        for job in jobs {
            job();
        }
        n
    }
}

impl<A> World<A> {
    fn new(app: A, view: fn(&mut Ui<'_, '_>, &mut A)) -> World<A> {
        World::build(|_| app, view)
    }

    /// A world whose app is made from the job board (a cache keeps the board it makes pictures on).
    fn build(make: impl FnOnce(&JobBoard) -> A, view: fn(&mut Ui<'_, '_>, &mut A)) -> World<A> {
        let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
        let main = rig.main;
        let tokens = rig.driver.runtime.env(main).tokens;
        let cells = EnvCells::in_store(rig.driver.runtime.store_mut(), tokens);
        let queue = Manual::default();
        let board = JobBoard::new(Arc::new(queue.clone()), rig.driver.waker());
        let app = make(&board);
        let rev = rig.driver.runtime.store_mut().create(0u64);
        let mut world = World { rig, app: Box::new(app), view, cells, board, queue, rev };
        for _ in 0..6 {
            if !world.step().ran {
                return world;
            }
        }
        panic!("the rig did not settle");
    }

    fn step(&mut self) -> StepReport {
        self.frame(0.0)
    }

    fn frame(&mut self, dt: f64) -> StepReport {
        let (app, view) = (&mut *self.app, self.view);
        let host = WindowEnv { cells: Some(self.cells), jobs: Some(self.board.clone()), commands: None };
        let rev = self.rev;
        self.rig.frame_with(dt, &mut |_, cx| {
            window_root_with(cx, host.clone(), |ui| {
                rev.get(ui.build_cx());
                view(ui, app)
            })
        })
    }

    /// The app changed its state: the root runs.
    fn touch(&mut self) {
        let rev = self.rev;
        rev.update(self.rig.driver.runtime.store_mut(), |v| *v += 1);
    }

    fn input(&mut self, input: Input) -> StepReport {
        let main = self.rig.main;
        self.rig.inject(main, input);
        self.step()
    }

    /// What the host does when the platform wakes the loop: hands finished jobs to their cells and images.
    fn wake(&mut self) -> usize {
        let (store, images) = self.rig.driver.runtime.store_and_images_mut();
        self.board.deliver(store, images)
    }
}

/// A fixed-size leaf that fills with `Color32::RED` and aims its opacity at `open`.
struct Fader {
    open: bool,
}

impl ComponentKey for Fader {
    fn key(&self, key: &mut MemoKey) {
        self.open.key(key);
    }
}

impl Component for Fader {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn measure(&mut self, _: &MeasureCx<'_>) -> Vec2 {
        Vec2::new(40.0, 20.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.fade_to(if self.open { 1.0 } else { 0.0 }, spec::OPEN_FADE);
    }

    fn paint(&mut self, paint: &Paint<'_>, _: &Cx<'_, Self>) {
        paint.draw().canvas().rect_filled(paint.bounds(), 0.0, Color32::RED);
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn counts(r: &StepReport) -> (u32, u32, u32, u32) {
    (r.frame.painted, r.frame.runs, r.frame.scopes, r.frame.built)
}

struct Faded {
    open: bool,
}

#[test]
fn a_running_fade_repaints_only_its_node_per_frame_runs_no_hook_and_ends_without_a_frame_request() {
    let mut w = World::new(Faded { open: true }, |ui, app| {
        let _ = IconButton::new(IconId::new("plus"), "Other").show(ui);
        let _ = (Fader { open: app.open }).show_in(ui);
    });
    assert!(!w.rig.driver.runtime.wants_time(), "a settled window reads no time");
    w.app.open = false;
    w.touch();
    let started = w.step();
    assert_eq!(started.frame.runs, 1, "the run that names the new target: only the fader");
    let duration = (f64::from(test_tokens().motion.open.millis) / 1000.0).max(FRAME);
    assert!(duration > 2.0 * FRAME, "the trial needs a fade longer than two frames");
    assert!(w.rig.driver.runtime.wants_time(), "the running fade reads the frame time");
    let mut frames = 0;
    while w.rig.driver.runtime.wants_time() && frames < 100 {
        let f = w.frame(FRAME);
        frames += 1;
        assert_eq!((f.frame.runs, f.frame.scopes, f.frame.built), (0, 0, 0), "no hook, scope or build runs during the fade: {f:?}");
        assert!(f.frame.painted <= 1, "only the fader repaints: {f:?}");
    }
    assert!(frames >= 2, "the fade ran over several frames");
    let after = w.frame(FRAME);
    assert_eq!(counts(&after).0, 0, "a finished fade paints nothing");
    assert!(!w.rig.driver.runtime.needs_frame() && !w.rig.driver.runtime.wants_time(), "a finished animation asks for no frame");
}

#[test]
fn a_motion_mode_of_off_snaps_the_fade() {
    let mut w = World::new(Faded { open: true }, |ui, app| {
        let _ = (Fader { open: app.open }).show_in(ui);
    });
    w.cells.motion.set(w.rig.driver.runtime.store_mut(), MotionMode::Off);
    w.step();
    w.app.open = false;
    w.touch();
    w.step();
    assert!(!w.rig.driver.runtime.wants_time(), "with motion off the opacity is at its target in the frame of the run");
}

/// Reads the flow direction in `measure`.
struct Flipper;

impl ComponentKey for Flipper {
    fn key(&self, _: &mut MemoKey) {}
}

impl Component for Flipper {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        if cx.direction() == Direction::Rtl { Vec2::new(60.0, 20.0) } else { Vec2::new(40.0, 20.0) }
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_direction_change_runs_only_the_nodes_that_read_it() {
    let mut w = World::new((Rect::NOTHING, Rect::NOTHING), |ui, app| {
        app.0 = IconButton::new(IconId::new("plus"), "Other").show(ui).rect;
        app.1 = Flipper.show_in(ui);
    });
    assert_eq!(w.app.1.width(), 40.0);
    w.cells.direction.set(w.rig.driver.runtime.store_mut(), Direction::Rtl);
    let changed = w.step();
    assert_eq!(changed.frame.runs, 1, "only the flipper reads the direction: {changed:?}");
    assert_eq!(w.app.1.width(), 60.0, "it measured again in the new direction");
    let settled = w.step();
    assert_eq!(counts(&settled), (0, 0, 0, 0));
}

thread_local! {
    static SEEN: RefCell<Vec<(&'static str, [u8; 4])>> = const { RefCell::new(Vec::new()) };
}

/// Notes the panel colour of the tokens its run saw.
struct Probe(&'static str);

impl ComponentKey for Probe {
    fn key(&self, key: &mut MemoKey) {
        self.0.key(key);
    }
}

impl Component for Probe {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        SEEN.with(|s| s.borrow_mut().push((self.0, cx.tokens.color.surface.panel.0)));
        Vec2::new(20.0, 20.0)
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn base(mode: craft_theme::Mode, contrast: craft_theme::Contrast) -> Arc<ThemeBase> {
    let appearance =
        craft_theme::Appearance { mode, contrast, size: craft_theme::Size::Compact, text_size: craft_theme::TextSize::Small, reduced_motion: false };
    Arc::new(ThemeBase {
        def: Arc::new(craft_theme::builtins()[0].clone()),
        appearance,
        app: None,
        sizes: Arc::new(craft_theme::AppSizes::default()),
    })
}

#[test]
fn a_theme_scope_resolves_its_own_tokens_and_a_base_change_runs_only_inside_it() {
    let mut w = World::new((), |ui, ()| {
        Probe("outside").show_in(ui);
        ThemeScope::new().mode(craft_theme::Mode::Light).show(ui, |ui| {
            Probe("inside").show_in(ui);
        });
    });
    w.cells.theme.set(w.rig.driver.runtime.store_mut(), Some(base(craft_theme::Mode::Dark, craft_theme::Contrast::Standard)));
    SEEN.with(|s| s.borrow_mut().clear());
    w.step();
    let seen = SEEN.with(|s| s.borrow().clone());
    assert_eq!(seen.len(), 1, "only the probe under the scope measured: {seen:?}");
    assert_ne!(seen[0].1, test_tokens().color.surface.panel.0, "it saw the light tokens, the window's are dark");
    SEEN.with(|s| s.borrow_mut().clear());
    w.cells.theme.set(w.rig.driver.runtime.store_mut(), Some(base(craft_theme::Mode::Dark, craft_theme::Contrast::High)));
    let changed = w.step();
    let seen = SEEN.with(|s| s.borrow().clone());
    assert!(seen.iter().all(|(n, _)| *n == "inside"), "only the node under the scope ran: {seen:?}");
    assert!(changed.frame.scopes >= 1, "the scope reruns: {changed:?}");
    assert_eq!(w.step().frame.runs, 0, "and then it is settled");
}

/// A leaf that names a cursor over its own shape.
struct Handle;

impl ComponentKey for Handle {
    fn key(&self, _: &mut MemoKey) {}
}

impl Component for Handle {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn measure(&mut self, _: &MeasureCx<'_>) -> Vec2 {
        Vec2::new(30.0, 30.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn cursor(&self) -> Option<Cursor> {
        Some(Cursor::Icon(CursorIcon::ResizeHorizontal))
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn hovering_a_node_with_a_cursor_sets_it_with_no_run() {
    let mut w = World::new((Rect::NOTHING, Rect::NOTHING), |ui, app| {
        app.0 = IconButton::new(IconId::new("plus"), "Other").show(ui).rect;
        app.1 = Handle.show_in(ui);
    });
    let main = w.rig.main;
    let (button, handle) = (w.app.0.center(), w.app.1.center());
    w.input(Input::mouse_move(button));
    assert_eq!(w.rig.driver.platform.cursor(main), Cursor::Default);
    let over = w.input(Input::mouse_move(handle));
    assert_eq!(w.rig.driver.platform.cursor(main), Cursor::Icon(CursorIcon::ResizeHorizontal));
    assert_eq!((over.frame.runs, over.frame.scopes, over.frame.built), (0, 0, 0), "no hook, scope or build runs: {over:?}");
    let away = w.input(Input::mouse_move(button));
    assert_eq!(w.rig.driver.platform.cursor(main), Cursor::Default);
    assert_eq!((away.frame.runs, away.frame.scopes), (0, 0));
}

struct Jobbed {
    result: Option<u32>,
    view_runs: u32,
    other_runs: u32,
}

#[test]
fn a_finished_job_runs_only_its_waiting_view_and_the_path() {
    let mut w = World::new(Jobbed { result: None, view_runs: 0, other_runs: 0 }, |ui, app| {
        let _ = IconButton::new(IconId::new("plus"), "Other").show(ui);
        MemoView::new(1u32).show(ui, |ui, _| {
            app.view_runs += 1;
            let pending = ui.job("answer", || Ok::<u32, ()>(7));
            app.result = match pending {
                craft_ui_contract::contract::rt::Pending::Ready(v) => Some(v),
                _ => None,
            };
            let _ = IconButton::new(IconId::new("brush"), "In view").show(ui);
        });
        MemoView::new(2u32).show(ui, |ui, _| {
            app.other_runs += 1;
            let _ = IconButton::new(IconId::new("eraser"), "Beside").show(ui);
        });
    });
    assert_eq!((w.app.result, w.app.view_runs, w.app.other_runs), (None, 1, 1), "the job started and nothing arrived");
    assert_eq!(w.queue.run_all(), 1, "one job was queued");
    assert_eq!(w.wake(), 1);
    let done = w.step();
    assert_eq!(w.app.result, Some(7));
    assert_eq!((w.app.view_runs, w.app.other_runs), (2, 1), "only the waiting view's closure ran again");
    assert_eq!(done.frame.scopes, 2, "the root and the waiting view: {done:?}");
    assert_eq!(done.frame.runs, 0, "no component ran: the view kept its button");
    assert_eq!(w.step().frame.scopes, 0);
}

/// A leaf that draws the picture `id` of a cache.
struct Thumb<'a> {
    cache: &'a TextureCache<u32>,
    id: u32,
}

impl ComponentKey for Thumb<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.id.key(key);
    }
}

impl Component for Thumb<'_> {
    const ROLE: accesskit::Role = accesskit::Role::Image;
    type Output = Response;

    fn measure(&mut self, _: &MeasureCx<'_>) -> Vec2 {
        Vec2::new(32.0, 32.0)
    }

    fn paint(&mut self, paint: &Paint<'_>, _: &Cx<'_, Self>) {
        let key: ImageKey = self.cache.image_key(&self.id, 1);
        let look = self.cache.look(&self.id, 1);
        if look == TextureLook::Loading {
            self.cache.request(self.id, 1, || Some(solid(8, 8)));
        }
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        paint.draw().canvas().image(key, paint.bounds(), uv, Color32::WHITE);
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn solid(w: u32, h: u32) -> RgbaImage {
    let mut image = RgbaImage::new(SizeU { width: w, height: h });
    image.pixels.fill(200);
    image
}

#[test]
fn a_late_picture_reaches_its_node_through_the_image_store_with_no_run() {
    struct App {
        cache: TextureCache<u32>,
    }
    let mut w = World::build(
        |board| App { cache: TextureCache::new("thumbs", 1 << 20, board.clone()) },
        |ui, app| {
            let _ = IconButton::new(IconId::new("plus"), "Other").show(ui);
            Thumb { cache: &app.cache, id: 3 }.show_in(ui);
        },
    );
    let cache = w.app.cache.clone();
    assert_eq!(cache.look(&3, 1), TextureLook::Loading);
    assert_eq!(w.queue.run_all(), 1, "the first paint asked for the picture once");
    assert_eq!(w.wake(), 1);
    assert_eq!(cache.look(&3, 1), TextureLook::Ready(cache.image_key(&3, 1)));
    let arrived = w.step();
    assert_eq!((arrived.frame.runs, arrived.frame.scopes, arrived.frame.built), (0, 0, 0), "nothing runs when the picture arrives: {arrived:?}");
    assert_eq!(arrived.render.images_uploaded, 1, "the renderer uploads it: {arrived:?}");
    assert!(!w.rig.driver.runtime.needs_frame(), "and the window is settled again");
}

/// Shows a leaf and returns its rect.
trait ShowIn: Component<Output = Response> + Sized {
    fn show_in(self, ui: &mut Ui<'_, '_>) -> Rect {
        show_leaf(self, ui).rect
    }
}

impl<C: Component<Output = Response>> ShowIn for C {}

/// Counts the live memory slots of every `Held`.
static LIVE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct Guard;

impl Guard {
    fn new() -> Guard {
        LIVE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Guard
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        LIVE.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Clone)]
struct Held {
    /// Counts the slot as live until the slot drops.
    _guard: Arc<Guard>,
}

impl Default for Held {
    fn default() -> Held {
        Held { _guard: Arc::new(Guard::new()) }
    }
}

/// Remembers a slot in its node's memory.
struct Keeper;

impl ComponentKey for Keeper {
    fn key(&self, _: &mut MemoKey) {}
}

impl Component for Keeper {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn measure(&mut self, _: &MeasureCx<'_>) -> Vec2 {
        Vec2::new(10.0, 10.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.memory::<Held>();
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_nodes_memory_lives_with_the_node() {
    use std::sync::atomic::Ordering::SeqCst;
    let base = LIVE.load(SeqCst);
    let mut w = World::new(true, |ui, shown| {
        if *shown {
            Keeper.show_in(ui);
        }
    });
    assert_eq!(LIVE.load(SeqCst), base + 1, "the run made the slot");
    w.touch();
    w.step();
    assert_eq!(LIVE.load(SeqCst), base + 1, "a kept node keeps its slot");
    *w.app = false;
    w.touch();
    w.step();
    assert_eq!(LIVE.load(SeqCst), base, "the node left the tree and its slot went with it");
}

thread_local! {
    static FRAMES: RefCell<Vec<Option<LayerFrame>>> = const { RefCell::new(Vec::new()) };
}

/// Notes the layer frame its run saw.
struct FrameReader;

impl ComponentKey for FrameReader {
    fn key(&self, _: &mut MemoKey) {}
}

impl Component for FrameReader {
    const ROLE: accesskit::Role = accesskit::Role::GenericContainer;
    type Output = Response;

    fn measure(&mut self, _: &MeasureCx<'_>) -> Vec2 {
        Vec2::new(10.0, 10.0)
    }

    fn paint(&mut self, paint: &Paint<'_>, _: &Cx<'_, Self>) {
        FRAMES.with(|f| f.borrow_mut().push(paint.frame()));
    }

    fn finish(self, cx: &Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn the_layer_frame_reaches_the_nodes_under_a_scope_and_rounds_the_surfaces_that_meet_it() {
    let frame = LayerFrame { rect: Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(200.0, 100.0)), radii: CornerRadii::same(8.0) };
    let mut w = World::new(frame, |ui, frame| {
        EnvScope::new().frame(Some(*frame)).show(ui, |ui| {
            FrameReader.show_in(ui);
        });
    });
    w.step();
    let seen = FRAMES.with(|f| f.borrow().clone());
    assert!(seen.contains(&Some(frame)) || seen.last() == Some(&Some(frame)), "the node under the scope saw the frame: {seen:?}");
    let mut items = vec![
        Item::Rect {
            rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 100.0)),
            radii: CornerRadii::ZERO,
            fill: craft_theme::Rgba([1, 2, 3, 255]),
            stroke: None,
        },
        Item::Rect {
            rect: Rect::from_min_size(Pos2::new(50.0, 50.0), Vec2::new(20.0, 20.0)),
            radii: CornerRadii::ZERO,
            fill: craft_theme::Rgba([1, 2, 3, 255]),
            stroke: None,
        },
    ];
    frame.fit(Pos2::ZERO, &mut items);
    let radii = |i: &Item| if let Item::Rect { radii, .. } = i { *radii } else { unreachable!() };
    assert_eq!(radii(&items[0]), CornerRadii::same(8.0), "a surface that fills the frame takes its corners");
    assert_eq!(radii(&items[1]), CornerRadii::ZERO, "one inside it keeps its own");
}
