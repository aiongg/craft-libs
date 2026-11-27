//! Count trials of the runtime services on craft-ui-rt: the open fade and motion off, env readers
//! (direction, theme scope, layer frame), the cursor, and the texture store's pictures. Relations
//! only, no times. Jobs started from a view are trialled in `windows.rs`; session memory in
//! `memory.rs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::skeleton::{IconButton, Stack};
use craft_behavior::{CursorIcon, IconId};
use craft_i18n::Direction;
use craft_ui_contract::contract::{Common, Component, Cx, Id, MeasureCx, Props as _, Response, Role, Show, Ui, spec};
use craft_ui_contract::primitives::textures::{CachedTexture, Priority, Revision, TextureLook, TextureState, TextureStore, TextureStoreOptions};
use craft_ui_contract::runtime::env::{EnvCells, EnvScope, LayerFrame, ThemeBase, ThemeScope};
use craft_ui_contract::runtime::pending::JobBoard;
use craft_ui_contract::runtime::windows::{WindowEnv, window_root_with};
use craft_ui_contract::style::StyleCx;
use craft_ui_contract::testing::{FRAME, Harness, Target};
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{CornerRadii, Cursor, Item, Pos2, Rect, RgbaImage, Sense, SizeU, StepReport, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use craft_ui_test_kit::rt::Rig;

// ----- the open fade and motion off

/// A fixed-size leaf that aims its opacity at `open`.
#[derive(Component)]
#[component(unregistered)]
struct Fader {
    #[common]
    common: Common,
    open: bool,
}

impl Component for Fader {
    const ROLE: Role = Role::GRAPHICS_OBJECT;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FaderStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(40.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.fade_to(if self.open { 1.0 } else { 0.0 }, spec::OPEN_FADE);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn fade_view(ui: &mut Ui<'_, '_>, open: &mut bool) {
    Stack::new().show(ui, |ui| {
        let _ = IconButton::new(IconId::new("plus"), "Other").show(ui);
        let _ = Show::show(Fader { common: Common::new(), open: *open }, ui);
    });
}

#[test]
fn a_running_fade_repaints_only_its_node_per_frame_runs_no_hook_and_ends_without_a_frame_request() {
    let mut h = Harness::builder().with_motion(true).build_ui_state(fade_view, true);
    h.run_to_rest();
    assert!(!h.driver.runtime.wants_time(), "a settled window reads no time");
    *h.state_mut() = false;
    let started = h.step_dt(0.0);
    assert_eq!(started.frame.runs, 1, "the run that names the new target: only the fader: {started:?}");
    assert!(h.driver.runtime.wants_time(), "the running fade reads the frame time");
    let mut frames = 0;
    while h.driver.runtime.wants_time() && frames < 100 {
        let f = h.step_dt(FRAME);
        frames += 1;
        assert_eq!((f.frame.runs, f.frame.scopes, f.frame.built), (0, 0, 0), "no hook, scope or build runs during the fade: {f:?}");
        assert!(f.frame.painted <= 1, "only the fader repaints: {f:?}");
    }
    assert!(frames >= 2, "the fade ran over several frames");
    let after = h.step_dt(FRAME);
    assert_eq!(after.frame.painted, 0, "a finished fade paints nothing");
    assert!(!h.driver.runtime.needs_frame() && !h.driver.runtime.wants_time(), "a finished animation asks for no frame");
}

#[test]
fn a_motion_mode_of_off_snaps_the_fade() {
    let mut h = Harness::builder().with_motion(true).build_ui_state(fade_view, true);
    h.run_to_rest();
    h.motion(craft_behavior::MotionMode::Off);
    h.step_dt(0.0);
    *h.state_mut() = false;
    h.step_dt(0.0);
    assert!(!h.driver.runtime.wants_time(), "with motion off the opacity is at its target in the frame of the run");
}

// ----- env readers

const FLIPPER: &str = "services-flipper";

/// Reads the flow direction in `measure`.
#[derive(Component)]
#[component(unregistered)]
struct Flipper {
    #[common]
    common: Common,
}

impl Component for Flipper {
    const ROLE: Role = Role::GRAPHICS_OBJECT;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FlipperStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        if cx.env.direction() == Direction::Rtl { Vec2::new(60.0, 20.0) } else { Vec2::new(40.0, 20.0) }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_direction_change_runs_only_the_nodes_that_read_it() {
    let mut h = Harness::new_ui(|ui| {
        Stack::new().show(ui, |ui| {
            let _ = IconButton::new(IconId::new("plus"), "Other").show(ui);
            let _ = Flipper { common: Common::new() }.id(Id::new(FLIPPER)).show(ui);
        });
    });
    h.run();
    let width = |h: &Harness<()>| h.rect_of(Id::new(FLIPPER)).expect("the flipper is shown").width();
    assert_eq!(width(&h), 40.0);
    h.direction(Direction::Rtl);
    let changed = h.step_dt(0.0);
    assert_eq!(changed.frame.runs, 1, "only the flipper reads the direction: {changed:?}");
    assert_eq!(width(&h), 60.0, "it measured again in the new direction");
    let settled = h.step_dt(0.0);
    assert_eq!((settled.frame.painted, settled.frame.runs, settled.frame.scopes, settled.frame.built), (0, 0, 0, 0), "{settled:?}");
}

thread_local! {
    static SEEN: RefCell<Vec<(&'static str, [u8; 4])>> = const { RefCell::new(Vec::new()) };
}

/// Notes the panel colour of the tokens its measuring run saw.
#[derive(Component)]
#[component(unregistered)]
struct Probe {
    #[common]
    common: Common,
    name: &'static str,
}

impl Component for Probe {
    const ROLE: Role = Role::GRAPHICS_OBJECT;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ProbeStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        SEEN.with(|s| s.borrow_mut().push((self.name, cx.tokens.theme().color.surface.panel.0)));
        Vec2::new(20.0, 20.0)
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
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

fn probe(name: &'static str) -> Probe {
    Probe { common: Common::new(), name }
}

#[test]
fn a_theme_scope_resolves_its_own_tokens_and_a_base_change_runs_only_inside_it() {
    let mut w = World::new((), |ui, ()| {
        let _ = probe("outside").show(ui);
        ThemeScope::new().mode(craft_theme::Mode::Light).show(ui, |ui| {
            let _ = probe("inside").show(ui);
        });
    });
    w.cells.theme.set(w.rig.driver.runtime.store_mut(), Some(base(craft_theme::Mode::Dark, craft_theme::Contrast::Standard)));
    SEEN.with(|s| s.borrow_mut().clear());
    w.step();
    let seen = SEEN.with(|s| s.borrow().clone());
    assert_eq!(seen.len(), 1, "only the probe under the scope measured: {seen:?}");
    let outside = base(craft_theme::Mode::Dark, craft_theme::Contrast::Standard).tokens().color.surface.panel.0;
    assert_ne!(seen[0].1, outside, "it saw the light tokens, the window's are dark");
    SEEN.with(|s| s.borrow_mut().clear());
    w.cells.theme.set(w.rig.driver.runtime.store_mut(), Some(base(craft_theme::Mode::Dark, craft_theme::Contrast::High)));
    let changed = w.step();
    let seen = SEEN.with(|s| s.borrow().clone());
    assert!(seen.iter().all(|(n, _)| *n == "inside"), "only the node under the scope ran: {seen:?}");
    assert!(changed.frame.scopes >= 1, "the scope reruns: {changed:?}");
    assert_eq!(w.step().frame.runs, 0, "and then it is settled");
}

thread_local! {
    static FRAMES: RefCell<Vec<Option<LayerFrame>>> = const { RefCell::new(Vec::new()) };
}

/// Notes the layer frame its measuring run saw.
#[derive(Component)]
#[component(unregistered)]
struct FrameReader {
    #[common]
    common: Common,
}

impl Component for FrameReader {
    const ROLE: Role = Role::GRAPHICS_OBJECT;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FrameReaderStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        FRAMES.with(|f| f.borrow_mut().push(cx.env.frame()));
        Vec2::new(10.0, 10.0)
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn layer_frame() -> LayerFrame {
    LayerFrame { rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 100.0)), radii: CornerRadii::same(8.0) }
}

#[test]
fn the_layer_frame_reaches_the_nodes_under_a_scope_and_rounds_the_surfaces_that_meet_it() {
    let frame = layer_frame();
    let mut h = Harness::new_ui(|ui| {
        EnvScope::new().frame(Some(layer_frame())).show(ui, |ui| {
            let _ = FrameReader { common: Common::new() }.show(ui);
        });
    });
    h.run();
    let seen = FRAMES.with(|f| f.borrow().clone());
    assert_eq!(seen.last(), Some(&Some(frame)), "the node under the scope saw the frame: {seen:?}");
    let surface = |rect: Rect| Item::Rect { rect, radii: CornerRadii::ZERO, fill: craft_theme::Rgba([1, 2, 3, 255]), stroke: None };
    let mut items = vec![surface(frame.rect), surface(Rect::from_min_size(Pos2::new(50.0, 50.0), Vec2::new(20.0, 20.0)))];
    frame.fit(Pos2::ZERO, &mut items);
    let radii = |i: &Item| if let Item::Rect { radii, .. } = i { *radii } else { unreachable!() };
    assert_eq!(radii(&items[0]), CornerRadii::same(8.0), "a surface that fills the frame takes its corners");
    assert_eq!(radii(&items[1]), CornerRadii::ZERO, "one inside it keeps its own");
}

// ----- the cursor

const HANDLE: &str = "services-handle";

/// A leaf that names a cursor over its own shape.
#[derive(Component)]
#[component(unregistered)]
struct Handle {
    #[common]
    common: Common,
}

impl Component for Handle {
    const ROLE: Role = Role::GRAPHICS_OBJECT;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HandleStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(30.0, 30.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn cursor(&self) -> Option<craft_behavior::Cursor> {
        Some(CursorIcon::ResizeHorizontal.into())
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn hovering_a_node_with_a_cursor_sets_it_with_no_run() {
    let mut h = Harness::new_ui(|ui| {
        Stack::new().show(ui, |ui| {
            let _ = IconButton::new(IconId::new("plus"), "Other").show(ui);
            let _ = Handle { common: Common::new() }.id(Id::new(HANDLE)).show(ui);
        });
    });
    h.run();
    let button = h.get_by_label("Other");
    let handle = Target::area(h.rect_of(Id::new(HANDLE)).expect("the handle is shown"));
    h.hover(&button);
    assert_eq!(h.cursor_icon(), Cursor::Default);
    h.hover(&handle);
    assert_eq!(h.cursor_icon(), Cursor::Icon(CursorIcon::ResizeHorizontal));
    let over = h.last_report();
    assert_eq!((over.frame.runs, over.frame.scopes, over.frame.built), (0, 0, 0), "no hook, scope or build runs: {over:?}");
    h.hover(&button);
    assert_eq!(h.cursor_icon(), Cursor::Default);
    let away = h.last_report();
    assert_eq!((away.frame.runs, away.frame.scopes), (0, 0), "{away:?}");
}

// ----- the texture store's pictures

/// A window whose host provides env cells and a job board whose jobs run when the trial says so,
/// delivered by the driver on the wake as an app host does.
struct World<A> {
    rig: Rig<RecordingRenderer>,
    app: Box<A>,
    view: fn(&mut Ui<'_, '_>, &mut A),
    cells: EnvCells,
    board: JobBoard,
    queue: Manual,
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

    /// A world whose app is made from the job board (a store keeps the board it makes pictures on).
    fn build(make: impl FnOnce(&JobBoard) -> A, view: fn(&mut Ui<'_, '_>, &mut A)) -> World<A> {
        let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
        let main = rig.main;
        let tokens = rig.driver.runtime.env(main).tokens;
        let cells = EnvCells::in_store(rig.driver.runtime.store_mut(), tokens);
        let queue = Manual::default();
        let board = JobBoard::new(Arc::new(queue.clone()), rig.driver.waker());
        let deliver = board.clone();
        rig.driver.set_wake_handler(move |store, images| {
            deliver.deliver(store, images);
        });
        let app = make(&board);
        let mut world = World { rig, app: Box::new(app), view, cells, board, queue };
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
        self.rig.frame_with(dt, &mut |_, cx| window_root_with(cx, host.clone(), |ui| view(ui, app)))
    }

    /// The app changed its state: the root runs.
    fn touch(&mut self) {
        let main = self.rig.main;
        self.rig.driver.runtime.mark_root(main);
    }

    /// Steps until a step runs nothing; the reports of the steps that ran.
    fn settle(&mut self) -> Vec<StepReport> {
        let mut ran = Vec::new();
        for _ in 0..20 {
            let report = self.step();
            if !report.ran && !self.rig.driver.runtime.needs_frame() {
                return ran;
            }
            ran.push(report);
        }
        panic!("the rig did not settle");
    }
}

const REV: Revision = Revision::new(1);

fn solid(side: u32) -> RgbaImage {
    let mut image = RgbaImage::new(SizeU { width: side, height: side });
    image.pixels.fill(200);
    image
}

/// A leaf that reads the picture `id` of a store from its hook and asks for it when nobody has.
#[derive(Component)]
#[component(unregistered)]
struct Thumb {
    #[common]
    common: Common,
    id: u32,
    #[key(skip)]
    store: TextureStore<u32>,
    /// Every state a run read, with the key it read.
    #[key(skip)]
    seen: Rc<RefCell<Vec<(u32, TextureState)>>>,
    #[key(skip)]
    runs: Rc<Cell<u32>>,
}

impl Component for Thumb {
    const ROLE: Role = Role::IMAGE;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ThumbStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::new(32.0, 32.0))
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.runs.set(self.runs.get() + 1);
        let state = cx.texture_state(&self.store, &self.id, REV);
        if matches!(state, TextureState::Missing) {
            self.store.cache().request(self.id, REV, Priority::VISIBLE, |_| Ok(Some(solid(8))));
        }
        self.seen.borrow_mut().push((self.id, state));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct Pictures {
    store: TextureStore<u32>,
    ids: Vec<u32>,
    seen: Rc<RefCell<Vec<(u32, TextureState)>>>,
    runs: Rc<Cell<u32>>,
}

impl Pictures {
    fn new(board: &JobBoard, ids: &[u32]) -> Pictures {
        Pictures {
            store: TextureStore::on(board, "services-thumbs", TextureStoreOptions::default()),
            ids: ids.to_vec(),
            seen: Rc::default(),
            runs: Rc::default(),
        }
    }

    fn latest(&self, id: u32) -> Option<TextureState> {
        self.seen.borrow().iter().rev().find(|(i, _)| *i == id).map(|(_, s)| s.clone())
    }
}

fn pictures_view(ui: &mut Ui<'_, '_>, app: &mut Pictures) {
    Stack::new().show(ui, |ui| {
        let _ = IconButton::new(IconId::new("plus"), "Other").show(ui);
        for &id in &app.ids {
            let thumb = Thumb { common: Common::new(), id, store: app.store.clone(), seen: app.seen.clone(), runs: app.runs.clone() };
            let _ = thumb.id(Id::new("services-thumb").with(id)).show(ui);
        }
    });
}

fn image_key_of(app: &Pictures, id: u32) -> craft_ui_rt::ImageKey {
    app.store.cache().image_key(&id, REV)
}

#[test]
fn a_late_picture_reaches_the_node_that_reads_it_through_the_image_store_and_then_nothing_runs() {
    let mut w = World::build(|board| Pictures::new(board, &[3]), pictures_view);
    assert!(matches!(w.app.latest(3), Some(TextureState::Missing)), "the first run found nothing and asked: {:?}", w.app.latest(3));
    assert!(w.app.store.cache().requested(&3, REV), "the request is kept");
    assert_eq!(w.queue.run_all(), 1, "one job makes the picture");
    let runs = w.app.runs.get();
    let arrived = w.step();
    assert_eq!(w.app.runs.get(), runs + 1, "the node that reads the picture ran again, and no other: {arrived:?}");
    assert_eq!(arrived.frame.runs, 1, "{arrived:?}");
    assert_eq!(arrived.render.images_uploaded, 1, "the renderer uploads it: {arrived:?}");
    let key = image_key_of(&w.app, 3);
    match w.app.latest(3) {
        Some(TextureState::Ready(CachedTexture { key: shown, .. })) => assert_eq!(shown, key),
        other => panic!("the picture is ready: {other:?}"),
    }
    assert!(!w.rig.driver.runtime.needs_frame(), "the window is settled again");
    let idle = w.step();
    assert_eq!((idle.frame.runs, idle.frame.scopes, idle.frame.built, idle.render.images_uploaded), (0, 0, 0, 0), "idle after delivery: {idle:?}");
}

/// Whether the cache holds a picture of `id` that is ready.
fn is_ready(app: &Pictures, id: u32) -> bool {
    matches!(app.store.cache().look(&id, REV), TextureLook::Ready(_))
}

struct Toggled {
    shown: bool,
    pictures: Pictures,
}

#[test]
fn a_loading_picture_is_cancelled_when_its_last_reader_leaves_and_asked_again_when_one_returns() {
    let mut w = World::build(
        |board| Toggled { shown: true, pictures: Pictures::new(board, &[]) },
        |ui, app| {
            app.pictures.ids = if app.shown { vec![5] } else { Vec::new() };
            pictures_view(ui, &mut app.pictures);
        },
    );
    assert!(w.app.pictures.store.cache().requested(&5, REV), "the shown node asked for the picture");
    w.app.shown = false;
    w.touch();
    w.settle();
    assert!(!w.app.pictures.store.cache().requested(&5, REV), "the last reader left: the request was cancelled");
    w.queue.run_all();
    let late = w.settle();
    assert!(late.iter().all(|r| r.render.images_uploaded == 0), "a cancelled job's picture is never delivered: {late:?}");
    assert!(!is_ready(&w.app.pictures, 5), "and the key shows nothing");
    w.app.shown = true;
    w.touch();
    w.settle();
    assert!(w.app.pictures.store.cache().requested(&5, REV), "a reader that returns asks again");
    assert_eq!(w.queue.run_all(), 1, "with one new job");
    w.settle();
    assert!(is_ready(&w.app.pictures, 5));
}

#[test]
fn a_picture_reads_ready_only_once_it_is_in_the_image_store() {
    let mut w = World::build(|board| Pictures::new(board, &[1, 2]), pictures_view);
    {
        let (_, images) = w.rig.driver.runtime.store_and_images_mut();
        images.set_upload_budget(1);
    }
    assert_eq!(w.queue.run_all(), 2, "one job per picture");
    let frames = w.settle();
    let uploads: Vec<u32> = frames.iter().map(|r| r.render.images_uploaded).collect();
    assert_eq!(uploads.iter().sum::<u32>(), 2, "both pictures reach the renderer: {uploads:?}");
    assert!(uploads.iter().all(|&n| n <= 1), "a frame uploads no more than its budget takes: {uploads:?}");
    let (_, images) = w.rig.driver.runtime.store_and_images_mut();
    assert!(images.contains(image_key_of(&w.app, 1)) && images.contains(image_key_of(&w.app, 2)), "both are in the image store");
    assert_eq!(images.waiting(), 0);
    for id in [1, 2] {
        assert!(matches!(w.app.latest(id), Some(TextureState::Ready(_))), "{id}: {:?}", w.app.latest(id));
    }
    assert!(!w.rig.driver.runtime.needs_frame());
}

// ----- drawing from a build

mod drawn {
    use craft_ui_contract::contract::{MemoView, Ui};
    use craft_ui_contract::primitives::Color32;
    use craft_ui_contract::testing::Harness;
    use craft_ui_rt::{Item, Part, Pos2, Rect, Vec2};

    const UNDER: Rect = Rect { min: Pos2::new(0.0, 0.0), max: Pos2::new(10.0, 10.0) };
    const OVER: Rect = Rect { min: Pos2::new(20.0, 0.0), max: Pos2::new(30.0, 10.0) };
    const OTHER: Rect = Rect { min: Pos2::new(40.0, 0.0), max: Pos2::new(50.0, 10.0) };

    /// While `on`, draws one rect under the scope's children and one over them, at the scope's start; else one rect
    /// elsewhere, under them.
    fn drawer(on: &mut bool, ui: &mut Ui) -> u32 {
        let at = ui.available_rect_before_wrap().min.to_vec2();
        let draw = ui.draw();
        let canvas = draw.canvas();
        if *on {
            canvas.rect_filled(UNDER.translate(at), 0, Color32::RED);
            canvas.over(|| canvas.rect_filled(OVER.translate(at), 0, Color32::BLUE));
        } else {
            canvas.rect_filled(OTHER.translate(at), 0, Color32::GREEN);
        }
        0
    }

    /// The part of the newest recorded list holding a rect at `rect` (node coordinates), with the list's node.
    fn newest(h: &Harness<bool>, rect: Rect) -> Option<(Part, craft_ui_rt::NodeId)> {
        let draws = h.driver.renderer.frames.iter().flat_map(|frame| frame.draws.iter());
        draws
            .filter(|draw| draw.list.items.iter().any(|item| matches!(item, Item::Rect { rect: r, .. } if *r == rect)))
            .last()
            .map(|draw| (draw.part, draw.node))
    }

    /// What a build draws through `Ui::draw` paints with its node, in the node's coordinates: the under list beneath
    /// the children, the over list above them. The next build's drawing replaces it.
    #[test]
    fn ui_draw_paints_under_and_over_the_node_children() {
        let mut h = Harness::new_ui_state(
            |ui, on: &mut bool| {
                ui.add_space(30.0);
                let _ = MemoView::new("drawer", *on, drawer).show(ui);
            },
            true,
        );
        h.run();
        let (under, node) = newest(&h, UNDER).expect("the under rect paints, at the scope's own origin");
        let (over, over_node) = newest(&h, OVER).expect("the over rect paints");
        assert_eq!((under, over), (Part::Content, Part::Overlay));
        assert_eq!(node, over_node, "both lists belong to the drawing scope");

        *h.state_mut() = false;
        h.run();
        assert_eq!(newest(&h, OTHER), Some((Part::Content, node)), "the next build's drawing paints with the scope");
        let content = h.driver.renderer.frames.iter().flat_map(|frame| frame.draws.iter());
        let content = content.filter(|draw| draw.node == node && draw.part == Part::Content).last().expect("the scope's list");
        assert!(!content.list.items.iter().any(|item| matches!(item, Item::Rect { rect, .. } if *rect == UNDER)), "and replaces the old one");
    }
}
