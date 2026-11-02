//! Tests of the Lasso on the tool host (task SE-F9). The draft's own rules are tested in
//! `craft_ui::canvas::draft`; these check what the tool adds: events, operation, keys, the drag
//! inside the selection. The rig also drives the Polygonal Lasso (`polygon_tests`).

use craft_ui::canvas::kurbo::{Point, Rect, Size};
use craft_ui::canvas::{CanvasId, CanvasView, SelectionOperation, ToolHost, ToolInput, ToolKey};
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::testing::canvas;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, Sense, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{LassoTool, PolygonLassoTool, SelectEvent, SelectPhase, SelectToolPass, SelectionProbe, SelectionShape};

pub(super) const NONE: Modifiers = Modifiers::NONE;

/// A document with an optional rectangle selection.
pub(super) struct Fake {
    pub selection: Option<Rect>,
    pub document: u64,
}

impl Fake {
    pub fn empty() -> Self {
        Self { selection: None, document: 1 }
    }

    pub fn selected(rect: Rect) -> Self {
        Self { selection: Some(rect), document: 1 }
    }
}

impl SelectionProbe for Fake {
    fn document(&self) -> u64 {
        self.document
    }

    fn size(&self) -> [u32; 2] {
        [400, 300]
    }

    fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    fn inside_selection(&self, at: Point) -> bool {
        self.selection.is_some_and(|r| r.contains(at))
    }
}

/// Both lassos; `polygon` picks the one the pass runs.
pub(super) struct Rig {
    pub lasso: LassoTool,
    pub polygon: PolygonLassoTool,
    pub use_polygon: bool,
    pub probe: Fake,
    id: Option<CanvasId>,
    pub focus: bool,
    pub ignored: bool,
    pub took_key: Option<ToolKey>,
}

impl Rig {
    pub fn events(&mut self) -> Vec<SelectEvent> {
        if self.use_polygon { self.polygon.take_events() } else { self.lasso.take_events() }
    }

    pub fn phase(&self) -> SelectPhase {
        if self.use_polygon { self.polygon.phase() } else { self.lasso.phase() }
    }
}

/// Document point and screen point coincide: the view's centre is the canvas's.
pub(super) fn rig(use_polygon: bool, probe: Fake) -> Harness<'static, Rig> {
    let view = CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
    let state =
        Rig { lasso: LassoTool::new(), polygon: PolygonLassoTool::new(), use_polygon, probe, id: None, focus: false, ignored: false, took_key: None };
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        move |ui, rig: &mut Rig| {
            let canvas_id = CanvasId::new(ui, "canvas");
            let canvas = canvas_id.interact(ui, view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
            if std::mem::take(&mut rig.focus) {
                canvas.request_focus();
            }
            let host = ToolHost::new(canvas_id).document(Size::new(400.0, 300.0));
            let r = if rig.use_polygon {
                host.show(ui, &view, &canvas, &mut SelectToolPass::new(&mut rig.polygon, &rig.probe))
            } else {
                host.show(ui, &view, &canvas, &mut SelectToolPass::new(&mut rig.lasso, &rig.probe))
            };
            rig.id = Some(r.id);
            rig.ignored |= r.ignored_press.is_some();
            rig.took_key = rig.took_key.or(r.took_key);
        },
        state,
    );
    h.step();
    h
}

fn input(h: &mut Harness<'_, Rig>, input: ToolInput) {
    let Some(id) = h.state().id else { panic!("a pass ran") };
    ToolHost::inject(&h.ctx, id, input);
    h.step();
}

pub(super) fn down(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
    input(
        h,
        ToolInput::Down {
            button: egui::PointerButton::Primary,
            doc: Point::new(at.0, at.1),
            pressure: 1.0,
            tilt: [0.0; 2],
            rotation: 0.0,
            modifiers,
        },
    );
}

pub(super) fn drag_to(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
    input(h, ToolInput::Move { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

pub(super) fn up(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
    input(h, ToolInput::Up { doc: Point::new(at.0, at.1), modifiers });
}

/// A press and release at `at`.
pub(super) fn click(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
    down(h, at, modifiers);
    up(h, at, modifiers);
}

/// The modifiers the canvas sees from the next pass on (the pointer samples carry their own).
pub(super) fn hold(h: &mut Harness<'_, Rig>, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.step();
}

pub(super) fn press_key(h: &mut Harness<'_, Rig>, key: Key) {
    h.event(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: NONE });
    h.step();
}

pub(super) fn polygon(points: &[(f64, f64)]) -> SelectionShape {
    SelectionShape::Polygon(points.iter().map(|&(x, y)| Point::new(x, y)).collect())
}

/// "begin", "update 1,0", "commit 2,0 changed", "cancel"; other events as `Debug`.
pub(super) fn tags(events: &[SelectEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| match e {
            SelectEvent::DragSelection(EditEvent::Begin { .. }) => "begin".to_owned(),
            SelectEvent::DragSelection(EditEvent::Update { value, .. }) => format!("update {},{}", value.delta[0], value.delta[1]),
            SelectEvent::DragSelection(EditEvent::Commit { value, changed, .. }) => {
                format!("commit {},{}{}", value.delta[0], value.delta[1], if *changed { " changed" } else { "" })
            }
            SelectEvent::DragSelection(EditEvent::Cancel { .. }) => "cancel".to_owned(),
            other => format!("{other:?}"),
        })
        .collect()
}

// A freehand drag commits one `Polygon` with the operation latched at the press (⇧ Add), and
// nothing more.
#[test]
fn lasso_commits_closed_outline_with_latched_operation() {
    let mut h = rig(false, Fake::empty());
    let shift = Modifiers::SHIFT;
    down(&mut h, (50.0, 50.0), shift);
    drag_to(&mut h, (150.0, 50.0), NONE);
    drag_to(&mut h, (150.0, 150.0), NONE);
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 3 });
    up(&mut h, (150.0, 150.0), NONE);
    assert_eq!(
        h.state_mut().events(),
        [SelectEvent::Commit { shape: polygon(&[(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)]), operation: SelectionOperation::Add }]
    );
    assert_eq!(h.state().phase(), SelectPhase::Idle);
}

// Between ⌥ clicks the tool takes Backspace (retract) and Enter (close) and keeps the keys from
// the app; with no outline open it takes neither.
#[test]
fn lasso_takes_retract_and_enter_only_while_open() {
    let mut h = rig(false, Fake::empty());
    h.state_mut().focus = true;
    h.step();
    press_key(&mut h, Key::Backspace);
    assert_eq!(h.state_mut().took_key.take(), None, "no outline: the key is the app's");
    // ⌥ with nothing selected starts straight segments.
    hold(&mut h, Modifiers::ALT);
    for at in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)] {
        click(&mut h, at, Modifiers::ALT);
    }
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 3 });
    press_key(&mut h, Key::Backspace);
    assert_eq!(h.state_mut().took_key.take().map(|k| k.key), Some(Key::Backspace));
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 2 });
    click(&mut h, (100.0, 150.0), Modifiers::ALT);
    press_key(&mut h, Key::Enter);
    assert_eq!(h.state_mut().took_key.take().map(|k| k.key), Some(Key::Enter));
    assert_eq!(
        h.state_mut().events(),
        [SelectEvent::Commit { shape: polygon(&[(50.0, 50.0), (150.0, 50.0), (100.0, 150.0)]), operation: SelectionOperation::New }]
    );
}

// Releasing ⌥ with the button up closes the outline in the next pass (`each_pass`), and a
// double-click closes it.
#[test]
fn lasso_alt_release_and_double_click_close() {
    let mut h = rig(false, Fake::empty());
    hold(&mut h, Modifiers::ALT);
    for at in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)] {
        click(&mut h, at, Modifiers::ALT);
    }
    assert!(h.state_mut().events().is_empty());
    hold(&mut h, NONE);
    let expected = polygon(&[(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)]);
    assert_eq!(h.state_mut().events(), [SelectEvent::Commit { shape: expected.clone(), operation: SelectionOperation::New }]);

    // The second press of a double-click lands within egui's double-click distance but outside
    // `select.same_point`, so only the click count closes. The harness steps a quarter second a
    // frame, so the delay is widened to cover the press, release and press.
    h.ctx.options_mut(|o| o.input_options.max_double_click_delay = 5.0);
    hold(&mut h, Modifiers::ALT);
    for at in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)] {
        click(&mut h, at, Modifiers::ALT);
    }
    down(&mut h, (150.0, 155.0), Modifiers::ALT);
    up(&mut h, (150.0, 155.0), Modifiers::ALT);
    let events = h.state_mut().events();
    assert_eq!(events.len(), 1, "{events:?}");
    assert!(matches!(&events[0], SelectEvent::Commit { shape: SelectionShape::Polygon(p), .. } if p.len() >= 3));
    assert_eq!(h.state().phase(), SelectPhase::Idle);
}

// A plain press inside the selection drags the outline instead of drawing.
#[test]
fn lasso_press_inside_drags_selection() {
    let mut h = rig(false, Fake::selected(Rect::new(100.0, 100.0, 200.0, 200.0)));
    down(&mut h, (150.0, 150.0), NONE);
    assert_eq!(h.state().phase(), SelectPhase::Dragging(super::DragKind::Outline));
    drag_to(&mut h, (160.0, 150.0), NONE);
    up(&mut h, (160.0, 150.0), NONE);
    assert_eq!(tags(&h.state_mut().events()), ["begin", "update 10,0", "commit 10,0 changed"]);
    assert_eq!(h.state().phase(), SelectPhase::Idle);
}

// An open outline is dropped without an event by `reset` and by a change of the probe's document.
#[test]
fn lasso_dropped_on_reset_or_document_change() {
    let mut h = rig(false, Fake::empty());
    hold(&mut h, Modifiers::ALT);
    click(&mut h, (50.0, 50.0), Modifiers::ALT);
    click(&mut h, (150.0, 50.0), Modifiers::ALT);
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 2 });
    h.state_mut().lasso.reset();
    assert_eq!(h.state().phase(), SelectPhase::Idle);
    click(&mut h, (50.0, 50.0), Modifiers::ALT);
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 1 });
    h.state_mut().probe.document = 2;
    h.step();
    assert_eq!(h.state().phase(), SelectPhase::Idle);
    assert!(h.state_mut().events().is_empty());
}

// While a freehand outline is open the tool's `node` is "Selection outline" with its point count
// and bounds; after the commit it is `None`.
#[test]
fn lasso_outline_being_drawn_is_one_accesskit_object() {
    let mut h = rig(false, Fake::empty());
    h.run();
    assert!(h.query_by_label("Selection outline").is_none());
    down(&mut h, (50.0, 50.0), NONE);
    drag_to(&mut h, (150.0, 50.0), NONE);
    drag_to(&mut h, (150.0, 150.0), NONE);
    h.run();
    let node = h.get_by_label("Selection outline");
    assert_eq!(node.accesskit_node().role(), Role::GraphicsObject);
    assert_eq!(node.accesskit_node().value().as_deref(), Some("3 points"));
    up(&mut h, (150.0, 150.0), NONE);
    h.run();
    assert!(h.query_by_label("Selection outline").is_none());
}

/// Cancels the gesture the rig's tool holds with `reason`, as the app does on a tool switch.
pub(super) fn cancel_with(h: &mut Harness<'_, Rig>, reason: CancelReason) {
    let id = h.state().id.expect("a pass ran");
    let ctx = h.ctx.clone();
    let rig = h.state_mut();
    let probe = Fake::empty();
    let _ = if rig.use_polygon {
        ToolHost::cancel(&ctx, id, &mut SelectToolPass::new(&mut rig.polygon, &probe), reason)
    } else {
        ToolHost::cancel(&ctx, id, &mut SelectToolPass::new(&mut rig.lasso, &probe), reason)
    };
    h.step();
}

// A press and release that draw fewer than 3 points (a click, or a two-point drag) deselect, as in
// PhotoCraft; Esc during the press drops the outline and reports nothing.
#[test]
fn lasso_click_deselects() {
    let mut h = rig(false, Fake::empty());
    h.state_mut().focus = true;
    h.step();
    canvas::press(&mut h, pos2(50.0, 50.0), NONE);
    canvas::release(&mut h, pos2(50.0, 50.0), NONE);
    assert_eq!(h.state_mut().events(), [SelectEvent::Deselect]);
    assert_eq!(h.state().phase(), SelectPhase::Idle);

    canvas::press(&mut h, pos2(50.0, 50.0), NONE);
    canvas::move_to(&mut h, pos2(90.0, 60.0));
    canvas::release(&mut h, pos2(90.0, 60.0), NONE);
    assert_eq!(h.state_mut().events(), [SelectEvent::Deselect], "two points are no outline");

    canvas::press(&mut h, pos2(50.0, 50.0), NONE);
    canvas::move_to(&mut h, pos2(90.0, 60.0));
    canvas::key(&mut h, Key::Escape, NONE);
    canvas::release(&mut h, pos2(90.0, 60.0), NONE);
    assert!(h.state_mut().events().is_empty(), "Esc is not a click");
    assert_eq!(h.state().phase(), SelectPhase::Idle);
}

// Esc during a press drops the Lasso's outline; so does every other cancel reason.
#[test]
fn lasso_escape_during_press_drops_outline() {
    let mut h = rig(false, Fake::empty());
    h.state_mut().focus = true;
    h.step();
    canvas::press(&mut h, pos2(50.0, 50.0), NONE);
    canvas::move_to(&mut h, pos2(150.0, 50.0));
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 2 });
    canvas::key(&mut h, Key::Escape, NONE);
    assert_eq!(h.state().phase(), SelectPhase::Idle);
    canvas::release(&mut h, pos2(150.0, 50.0), NONE);
    assert!(h.state_mut().events().is_empty());

    canvas::press(&mut h, pos2(50.0, 50.0), NONE);
    canvas::move_to(&mut h, pos2(150.0, 50.0));
    cancel_with(&mut h, CancelReason::Removed);
    assert_eq!(h.state().phase(), SelectPhase::Idle);
}
