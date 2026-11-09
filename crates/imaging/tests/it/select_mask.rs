//! Tests of the Select and Mask workspace: its tools, session, panel and keys.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use craft_behavior::edit_history::Step;
use craft_behavior::kurbo::Point;
use craft_behavior::{CancelReason, CanvasView, Cursor, PixelSize, PointerButton};
use craft_host::{AppInfo, Host, InlineJobs};
use craft_imaging::trace::MaskSource;
use craft_imaging::ui::mask_view::{MaskViewMode, MaskViewState};
use craft_imaging::ui::select_mask::{
    EditMode, EditPhase, LimitRefused, ObjectTarget, OutputTo, RefineStroke, SelectMaskCanvas, SelectMaskEdit, SelectMaskEvent, SelectMaskLimits,
    SelectMaskOpen, SelectMaskPreviewRequest, SelectMaskSession, SelectMaskSettings, SelectMaskSource, SelectMaskTool, SelectMaskToolOptions,
    SelectMaskWorkspace,
};
use craft_imaging::ui::wide_angle::PreviewDone;
use craft_keymap::Mods;
use craft_ui::canvas::{CanvasTool, PixelRect, RasterSource, ToolAction, ToolContext, ToolKey, ToolPath, ToolPointer};
use craft_ui::primitives::{Draw, Revision};
use craft_ui::runtime::access::Role;
use craft_ui::runtime::pending::Job;
use craft_ui::sections::DialogRole;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, ToolFixture, pointer, trial};
use egui::{Color32, ColorImage, Key, Modifiers, Pos2, Rect, Vec2};
use serde_json::json;

/// The document's size in pixels.
const DOCUMENT: [u32; 2] = [800, 600];

/// The edit limits the session tests use.
const LIMITS: SelectMaskLimits = SelectMaskLimits::new(3, 10, 2, 4);

fn quick(points: usize) -> SelectMaskEdit {
    SelectMaskEdit::quick(vec![[1.0, 2.0]; points], 30.0, EditMode::Add, false)
}

fn refine_stroke(points: usize) -> RefineStroke {
    RefineStroke::new(vec![[3.0, 4.0]; points], 20.0, EditMode::Subtract)
}

// ---- The tools, on a tool fixture ----

/// A Select and Mask tool with the state it keeps between passes, as one owned `CanvasTool`.
struct Rig {
    canvas: SelectMaskCanvas,
    options: SelectMaskToolOptions,
    tool: SelectMaskTool,
    real_time: bool,
}

impl Rig {
    fn new(tool: SelectMaskTool, real_time: bool) -> Self {
        Rig { canvas: SelectMaskCanvas::new(), options: SelectMaskToolOptions::default(), tool, real_time }
    }
}

macro_rules! pass {
    ($self:ident) => {
        $self.canvas.tool($self.tool, &mut $self.options, $self.real_time)
    };
}

impl CanvasTool for Rig {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        pass!(self).each_pass(cx);
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        pass!(self).press(pointer, cx)
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        pass!(self).drag(pointer, cx);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        pass!(self).release(pointer, cx);
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        pass!(self).hover(pointer, cx);
    }

    fn cancel(&mut self, reason: CancelReason, cx: &mut ToolContext<'_>) {
        pass!(self).cancel(reason, cx);
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        pass!(self).key(key, cx)
    }

    fn claims(&self) -> craft_ui::canvas::ToolClaims {
        let mut options = self.options;
        let mut canvas = self.canvas.clone();
        canvas.tool(self.tool, &mut options, self.real_time).claims()
    }

    fn paint(&self, draw: &Draw<'_>, cx: &ToolContext<'_>) {
        let mut options = self.options;
        let mut canvas = self.canvas.clone();
        canvas.tool(self.tool, &mut options, self.real_time).paint(draw, cx);
    }

    fn cursor(&self, cx: &ToolContext<'_>) -> Cursor {
        let mut canvas = self.canvas.clone();
        let mut options = self.options;
        canvas.tool(self.tool, &mut options, self.real_time).cursor(cx)
    }

    fn takes(&self, button: PointerButton, mods: Mods) -> bool {
        let mut canvas = self.canvas.clone();
        let mut options = self.options;
        canvas.tool(self.tool, &mut options, self.real_time).takes(button, mods)
    }

    fn command_form(&self) -> ToolPath {
        let mut canvas = self.canvas.clone();
        let mut options = self.options;
        canvas.tool(self.tool, &mut options, self.real_time).command_form()
    }
}

/// A fixture over a view whose document point and screen point coincide (the canvas keeps a view's centre inside the
/// document, so the origin cannot sit at the room's centre).
fn rig(tool: SelectMaskTool, real_time: bool) -> ToolFixture<Rig> {
    let label = match tool {
        SelectMaskTool::Quick => "Quick Selection",
        SelectMaskTool::RefineEdge => "Refine Edge Brush",
        SelectMaskTool::Brush => "Brush",
        SelectMaskTool::Lasso => "Lasso",
        SelectMaskTool::PolygonalLasso => "Polygonal Lasso",
        _ => "Canvas",
    };
    let view = |room: Rect| {
        let centre = room.center();
        CanvasView::new(room, Point::new(f64::from(centre.x), f64::from(centre.y)), 1.0)
    };
    ToolFixture::with_view(label, Rig::new(tool, real_time), view, |_, _| {})
}

/// The screen point of document point `(x, y)`.
fn at(x: f32, y: f32) -> Pos2 {
    Pos2::new(x, y)
}

fn events(fixture: &ToolFixture<Rig>) -> Vec<SelectMaskEvent> {
    fixture.tool_mut().canvas.take_events()
}

/// The shape of the edit events: `begin`, `update`, `commit`, `cancel` or `set`.
fn tags(events: &[SelectMaskEvent]) -> Vec<&'static str> {
    fn tag<T>(phase: &EditPhase<T>) -> &'static str {
        match phase {
            EditPhase::Begin { .. } => "begin",
            EditPhase::Update { .. } => "update",
            EditPhase::Commit { .. } => "commit",
            EditPhase::Cancel { .. } => "cancel",
            _ => "set",
        }
    }
    events
        .iter()
        .map(|event| match event {
            SelectMaskEvent::Edit(phase) => tag(phase),
            SelectMaskEvent::Refine(phase) => tag(phase),
            _ => "other",
        })
        .collect()
}

/// The edit the last event commits.
fn committed(events: &[SelectMaskEvent]) -> SelectMaskEdit {
    let Some(SelectMaskEvent::Edit(EditPhase::Commit { value, .. })) = events.last() else { panic!("a commit last: {events:?}") };
    value.clone()
}

fn click(fixture: &mut ToolFixture<Rig>, to: Pos2) {
    fixture.press_at(to).release_at(to);
}

#[trial]
fn select_mask_object_click_and_drag_make_point_or_rect() {
    let mut fixture = rig(SelectMaskTool::Object, false);
    let origin = at(0.0, 0.0);
    click(&mut fixture, origin + Vec2::new(100.0, 100.0));
    let got = events(&fixture);
    assert_eq!(tags(&got), ["begin", "commit"]);
    let SelectMaskEdit::Object { target, mode, .. } = committed(&got) else { panic!("an object edit") };
    assert_eq!((target, mode), (ObjectTarget::Point([100.0, 100.0]), EditMode::Add));

    fixture.press_at(origin + Vec2::new(50.0, 50.0)).move_to(origin + Vec2::new(52.0, 51.0)).release_at(origin + Vec2::new(52.0, 51.0));
    let SelectMaskEdit::Object { target, .. } = committed(&events(&fixture)) else { panic!("an object edit") };
    assert_eq!(target, ObjectTarget::Point([50.0, 50.0]), "a drag shorter than the click reach is a click");

    fixture.press_at(origin + Vec2::new(50.0, 50.0)).move_to(origin + Vec2::new(120.0, 90.0)).release_at(origin + Vec2::new(150.0, 120.0));
    let SelectMaskEdit::Object { target, .. } = committed(&events(&fixture)) else { panic!("an object edit") };
    assert_eq!(target, ObjectTarget::Rect([50.0, 50.0, 100.0, 70.0]));
}

#[trial]
fn select_mask_polygonal_lasso_enter_escape_backspace() {
    let mut fixture = rig(SelectMaskTool::PolygonalLasso, false);
    let origin = at(0.0, 0.0);
    let corners = [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0), (50.0, 150.0)];
    for (x, y) in corners {
        click(&mut fixture, origin + Vec2::new(x, y));
    }
    assert!(fixture.tool().canvas.polygon_open());
    fixture.fixture().key(Key::Backspace, Modifiers::NONE);
    assert!(events(&fixture).is_empty(), "a vertex is no edit");

    fixture.fixture().key(Key::Enter, Modifiers::NONE);
    let got = events(&fixture);
    assert_eq!(tags(&got), ["begin", "commit"]);
    let SelectMaskEdit::Lasso { points, mode, .. } = committed(&got) else { panic!("a lasso edit") };
    assert_eq!((points, mode), (vec![[50.0, 50.0], [150.0, 50.0], [150.0, 150.0]], EditMode::Add));
    assert!(!fixture.tool().canvas.polygon_open());

    // Escape drops a polygon without an edit; fewer than three points make none either.
    click(&mut fixture, origin + Vec2::new(50.0, 50.0));
    click(&mut fixture, origin + Vec2::new(150.0, 50.0));
    fixture.fixture().key(Key::Escape, Modifiers::NONE);
    assert!(!fixture.tool().canvas.polygon_open());
    click(&mut fixture, origin + Vec2::new(50.0, 50.0));
    fixture.fixture().key(Key::Enter, Modifiers::NONE);
    assert!(events(&fixture).is_empty());

    // A click within the close reach of the first vertex closes.
    for (x, y) in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0), (53.0, 52.0)] {
        click(&mut fixture, origin + Vec2::new(x, y));
    }
    let SelectMaskEdit::Lasso { points, .. } = committed(&events(&fixture)) else { panic!("a lasso edit") };
    assert_eq!(points.len(), 3);
    assert!(!fixture.tool().canvas.polygon_open());
}

#[trial]
fn select_mask_real_time_refinement_reports_updates_only_when_on() {
    for (real_time, expected) in [(false, vec!["begin", "commit"]), (true, vec!["begin", "update", "update", "commit"])] {
        let mut fixture = rig(SelectMaskTool::Quick, real_time);
        let origin = at(0.0, 0.0);
        fixture.press_at(origin + Vec2::new(10.0, 10.0));
        fixture.move_to(origin + Vec2::new(30.0, 10.0)).move_to(origin + Vec2::new(60.0, 10.0)).release_at(origin + Vec2::new(60.0, 10.0));
        let got = events(&fixture);
        assert_eq!(tags(&got), expected, "real time {real_time}");
        let SelectMaskEdit::Quick { points, size, .. } = committed(&got) else { panic!("a quick edit") };
        assert_eq!((points, size), (vec![[10.0, 10.0], [30.0, 10.0], [60.0, 10.0]], 30.0));
    }
}

// ---- The session and its data ----

#[trial]
fn select_mask_refine_edge_strokes_are_kept_apart() {
    let mut session = SelectMaskSession::default();
    assert!(session.add_stroke(refine_stroke(2), &LIMITS).is_ok());
    assert!(session.edits.is_empty());
    assert_eq!(session.strokes.len(), 1);
    let value = serde_json::to_value(session.command(false, false)).unwrap_or_default();
    assert_eq!(value["refineStrokes"], json!([{"points": [[3.0, 4.0], [3.0, 4.0]], "size": 20.0, "mode": "subtract"}]));
    assert!(value.get("edits").is_none());
}

#[trial]
fn select_mask_limits_refuse_at_the_brush_with_a_message() {
    let mut session = SelectMaskSession::default();
    for _ in 0..3 {
        assert!(session.add_edit(SelectMaskEdit::Clear, &LIMITS).is_ok());
    }
    assert_eq!(session.add_edit(SelectMaskEdit::Clear, &LIMITS), Err(LimitRefused::Edits(3)));
    assert_eq!(session.edits.len(), 3);
    let mut session = SelectMaskSession::default();
    assert!(session.add_edit(quick(8), &LIMITS).is_ok());
    assert_eq!(session.add_edit(quick(3), &LIMITS), Err(LimitRefused::EditPoints(10)));
    assert!(session.add_edit(quick(2), &LIMITS).is_ok(), "exactly the limit is allowed");
    let mut session = SelectMaskSession::default();
    assert!(session.add_stroke(refine_stroke(3), &LIMITS).is_ok());
    assert_eq!(session.add_stroke(refine_stroke(2), &LIMITS), Err(LimitRefused::StrokePoints(4)));
    assert!(session.add_stroke(refine_stroke(1), &LIMITS).is_ok());
    assert_eq!(session.add_stroke(refine_stroke(0), &LIMITS), Err(LimitRefused::Strokes(2)));
    assert_eq!(session.strokes.len(), 2);
}

#[trial]
fn select_mask_command_matches_the_engine_params() {
    let mut session = SelectMaskSession::default();
    session.refine.radius = 5.0;
    session.refine.smart_radius = true;
    session.refine.output = OutputTo::NewLayerWithMask;
    let plain = serde_json::to_value(session.command(false, false)).unwrap_or_default();
    assert_eq!(
        plain,
        json!({
            "radius": 5.0, "smartRadius": true, "smooth": 0.0, "feather": 0.0, "contrast": 0.0, "shiftEdge": 0.0,
            "decontaminate": false, "amount": 100.0, "output": "newLayerWithMask", "sampleAllLayers": false
        })
    );
    session.edits = vec![
        quick(1),
        SelectMaskEdit::brush(vec![[0.5, 0.5]], 9.0, 50.0, EditMode::Subtract),
        SelectMaskEdit::object(ObjectTarget::Rect([1.0, 2.0, 3.0, 4.0]), EditMode::Add, true),
        SelectMaskEdit::object(ObjectTarget::Point([7.0, 8.0]), EditMode::Add, false),
        SelectMaskEdit::subject(true),
        SelectMaskEdit::Clear,
    ];
    let full = serde_json::to_value(session.command(true, true)).unwrap_or_default();
    assert_eq!(full["target"], "mask");
    assert_eq!(full["sampleAllLayers"], true);
    assert_eq!(full["edits"][0], json!({"tool": "quick", "points": [[1.0, 2.0]], "size": 30.0, "mode": "add", "sampleAllLayers": false}));
    assert_eq!(full["edits"][1], json!({"tool": "brush", "points": [[0.5, 0.5]], "size": 9.0, "hardness": 50.0, "mode": "subtract"}));
    assert_eq!(full["edits"][2], json!({"tool": "object", "rect": [1.0, 2.0, 3.0, 4.0], "mode": "add", "sampleAllLayers": true}));
    assert_eq!(full["edits"][3], json!({"tool": "object", "point": [7.0, 8.0], "mode": "add", "sampleAllLayers": false}));
    assert_eq!(full["edits"][4], json!({"tool": "subject", "sampleAllLayers": true}));
    assert_eq!(full["edits"][5], json!({"tool": "clear"}));
    assert!(full.get("refineStrokes").is_none());
}

#[trial]
fn select_mask_settings_round_trip_and_carry_the_view() {
    let defaults = SelectMaskSettings::default();
    assert_eq!(defaults.view, MaskViewMode::OnionSkin);
    assert_eq!(defaults.overlay_color, [255, 0, 0]);
    assert!(defaults.real_time && !defaults.high_quality && !defaults.remember);
    let empty: SelectMaskSettings = serde_json::from_str("{}").unwrap_or_default();
    assert_eq!(empty, defaults);
    let mut settings = defaults.clone();
    settings.view = MaskViewMode::Overlay;
    settings.show_edge = true;
    settings.overlay_color = [12, 200, 77];
    settings.opacities[2] = 35.0;
    settings.refine.shift_edge = -40.0;
    settings.remember = true;
    let text = serde_json::to_string(&settings).unwrap_or_default();
    assert_eq!(serde_json::from_str::<SelectMaskSettings>(&text).ok(), Some(settings.clone()));
    let state = settings.view_state();
    assert_eq!(state.mode, MaskViewMode::Overlay);
    assert!(state.show_edge && !state.hide_view);
    let mut other = SelectMaskSettings::default();
    other.take_view(&state);
    assert_eq!((other.view, other.show_edge, other.overlay_color, other.opacities), (settings.view, true, [12, 200, 77], settings.opacities));
    assert_eq!(SelectMaskSettings::default().view_state(), MaskViewState::default());
}

// ---- The workspace, on a fixture ----

/// A solid picture.
struct Solid {
    size: PixelSize,
}

impl RasterSource for Solid {
    fn size(&self) -> PixelSize {
        self.size
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        None
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::from_rgb(40, 120, 200));
    }
}

/// The app's side: a picture and a count of the preview jobs asked for.
struct Source {
    image: Solid,
    jobs: Arc<AtomicUsize>,
}

impl SelectMaskSource for Source {
    fn view(&self) -> &dyn RasterSource {
        &self.image
    }

    fn image(&self) -> &dyn RasterSource {
        &self.image
    }

    fn ants(&self) -> Option<(&dyn MaskSource, u32)> {
        None
    }

    fn preview_job(&self, _: &SelectMaskPreviewRequest) -> Job<PreviewDone, String> {
        self.jobs.fetch_add(1, Ordering::SeqCst);
        Job::new(|_, _| Ok(PreviewDone::new(Duration::ZERO)))
    }
}

/// What the app keeps of the workspace and what it saw.
struct App {
    workspace: SelectMaskWorkspace,
    source: Source,
    events: Vec<SelectMaskEvent>,
    closed: Option<DialogRole>,
}

/// The workspace over a solid picture, settled.
fn open() -> (Fixture, Rc<RefCell<App>>, Arc<AtomicUsize>) {
    let jobs = Arc::new(AtomicUsize::new(0));
    let size = PixelSize::new(DOCUMENT[0], DOCUMENT[1]);
    let app = Rc::new(RefCell::new(App {
        workspace: SelectMaskWorkspace::new(SelectMaskOpen::new(DOCUMENT, "Photo", LIMITS)),
        source: Source { image: Solid { size }, jobs: Arc::clone(&jobs) },
        events: Vec::new(),
        closed: None,
    }));
    let shown = Rc::clone(&app);
    let mut fixture = Fixture::new(move |ui| {
        let mut app = shown.borrow_mut();
        let App { workspace, source, .. } = &mut *app;
        let out = workspace.show(ui, source);
        app.events.extend(out.events);
        app.closed = app.closed.or(out.closed);
    });
    fixture.host(Arc::new(Host::builder(AppInfo { id: "trial", name: "Trial", version: "0" }).jobs(Arc::new(InlineJobs)).build()));
    fixture.room(Vec2::new(1200.0, 900.0));
    fixture.settle();
    (fixture, app, jobs)
}

fn canvas_point(fixture: &Fixture, offset: Vec2) -> Pos2 {
    let canvas = fixture.by_role(Role::CANVAS).into_iter().next().expect("the workspace shows a canvas");
    canvas.center() + offset
}

fn press_label(fixture: &mut Fixture, label: &str) {
    let target = fixture.by_label(label).unwrap_or_else(|| panic!("a control named {label}"));
    fixture.click(&target);
    fixture.settle();
}

/// Presses at `from` and moves 30 points away, leaving the stroke open.
fn open_stroke(fixture: &mut Fixture, from: Pos2) {
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    fixture.frame(vec![pointer::moved(from + Vec2::new(30.0, 0.0))]);
}

#[trial]
fn select_mask_tools_make_edits_and_undo_redo_them() {
    let (mut fixture, app, _) = open();
    press_label(&mut fixture, "Select Subject");
    press_label(&mut fixture, "Invert");
    assert_eq!(app.borrow().workspace.session().edits.len(), 2);
    assert!(app.borrow_mut().workspace.step(Step::Undo));
    assert_eq!(app.borrow().workspace.session().edits.len(), 1);
    assert!(app.borrow_mut().workspace.step(Step::Undo));
    assert!(app.borrow().workspace.session().edits.is_empty());
    assert!(!app.borrow_mut().workspace.step(Step::Undo));
    assert!(app.borrow_mut().workspace.step(Step::Redo));
    assert!(app.borrow_mut().workspace.step(Step::Redo));
    assert_eq!(app.borrow().workspace.session().edits, [SelectMaskEdit::subject(false), SelectMaskEdit::Invert]);
}

#[trial]
fn select_mask_reset_is_one_undo_step_that_keeps_what_it_replaced() {
    let (mut fixture, app, _) = open();
    press_label(&mut fixture, "Select Subject");
    press_label(&mut fixture, "Invert");
    let before = app.borrow().workspace.session().clone();
    press_label(&mut fixture, "Reset");
    assert_eq!(*app.borrow().workspace.session(), SelectMaskSession::default());
    assert!(app.borrow_mut().workspace.step(Step::Undo));
    assert_eq!(*app.borrow().workspace.session(), before);
    assert!(app.borrow_mut().workspace.step(Step::Redo));
    assert_eq!(*app.borrow().workspace.session(), SelectMaskSession::default());
}

#[trial]
fn select_mask_decontaminate_moves_the_output_target() {
    let (mut fixture, app, _) = open();
    assert_eq!(app.borrow().workspace.session().refine.output, OutputTo::Selection);
    press_label(&mut fixture, "Decontaminate Colors");
    let refine = app.borrow().workspace.session().refine.clone();
    assert!(refine.decontaminate);
    assert_eq!(refine.output, OutputTo::NewLayerWithMask, "Output To follows Decontaminate Colors");
}

#[trial]
fn select_mask_tool_switch_mid_stroke_cancels_the_stroke() {
    let (mut fixture, app, _) = open();
    let from = canvas_point(&fixture, Vec2::ZERO);
    open_stroke(&mut fixture, from);
    app.borrow_mut().events.clear();
    fixture.key(Key::R, Modifiers::NONE);
    let events = app.borrow().events.clone();
    assert!(events.iter().any(|event| matches!(event, SelectMaskEvent::Edit(EditPhase::Cancel { reason: CancelReason::Removed, .. }))), "{events:?}");
    fixture.frame(vec![pointer::release(from + Vec2::new(30.0, 0.0))]);
    assert!(app.borrow().workspace.session().edits.is_empty() && app.borrow().workspace.session().strokes.is_empty());
}

#[trial]
fn select_mask_undo_mid_stroke_cancels_the_stroke() {
    let (mut fixture, app, _) = open();
    let from = canvas_point(&fixture, Vec2::ZERO);
    open_stroke(&mut fixture, from);
    app.borrow_mut().events.clear();
    app.borrow_mut().workspace.step(Step::Undo);
    fixture.frame(Vec::new());
    let events = app.borrow().events.clone();
    assert!(
        events.iter().any(|event| matches!(event, SelectMaskEvent::Edit(EditPhase::Cancel { reason: CancelReason::UndoDuringGesture, .. }))),
        "{events:?}"
    );
    fixture.frame(vec![pointer::release(from + Vec2::new(30.0, 0.0))]);
    assert!(app.borrow().workspace.session().edits.is_empty());
}

#[trial]
fn select_mask_preview_is_asked_for_only_when_it_changes() {
    let (mut fixture, _, jobs) = open();
    let first = jobs.load(Ordering::SeqCst);
    assert_eq!(first, 1, "one preview for the opening state");
    fixture.frame(vec![pointer::moved(canvas_point(&fixture, Vec2::ZERO))]);
    fixture.settle();
    assert_eq!(jobs.load(Ordering::SeqCst), first, "hovering changes nothing");
    press_label(&mut fixture, "Invert");
    assert_eq!(jobs.load(Ordering::SeqCst), first + 1, "an edit asks for a new preview");
}

// The workspace's zoom keys belong to the dialog: with focus on the footer's OK a zoom chord still zooms the view.
#[trial]
fn select_mask_zoom_chord_works_with_focus_on_the_footer_button() {
    let (mut fixture, _app, _jobs) = open();
    let ok = fixture.by_label("OK").expect("the footer has OK");
    fixture.access(AccessActionKind::Focus, &ok);
    fixture.idle(1);
    let zoom = |fixture: &Fixture| fixture.texts().iter().map(|run| run.text.clone()).find(|text| text.contains("Photo,") && text.contains('%'));
    let before = zoom(&fixture);
    fixture.key(Key::Equals, Modifiers::COMMAND);
    fixture.idle(2);
    assert!(before.is_some() && zoom(&fixture) != before, "the title's zoom changes: {before:?} then {:?}", zoom(&fixture));
}
