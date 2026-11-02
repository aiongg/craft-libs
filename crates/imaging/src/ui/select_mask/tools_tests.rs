//! Tests of Select and Mask's tools (task WS-SM2), on a `ToolHost` with injected input.

use craft_ui::canvas::kurbo::{Point, Size};
use craft_ui::canvas::{CanvasId, CanvasView, ToolHost, ToolInput};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};
use egui::{Event, Key, Modifiers, PointerButton, Sense, pos2, vec2};
use egui_kittest::Harness;

use super::{BrushSizes, EditMode, ObjectTarget, SelectMaskCanvas, SelectMaskEdit, SelectMaskEvent, SelectMaskTool, SelectMaskToolOptions};

const NONE: Modifiers = Modifiers::NONE;

struct Rig {
    canvas: SelectMaskCanvas,
    options: SelectMaskToolOptions,
    tool: SelectMaskTool,
    real_time: bool,
    id: Option<CanvasId>,
    focus: bool,
}

/// Document and screen points coincide: the view's centre is the canvas's.
fn rig(tool: SelectMaskTool, real_time: bool) -> Harness<'static, Rig> {
    let view = CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
    let state = Rig { canvas: SelectMaskCanvas::new(), options: SelectMaskToolOptions::default(), tool, real_time, id: None, focus: false };
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        move |ui, rig: &mut Rig| {
            let canvas_id = CanvasId::new(ui, "canvas");
            let canvas = canvas_id.interact(ui, view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
            if std::mem::take(&mut rig.focus) {
                canvas.request_focus();
            }
            let Rig { canvas: state, options, tool, real_time, id, .. } = rig;
            let mut pass = state.tool(*tool, options, *real_time);
            *id = Some(ToolHost::new(canvas_id).document(Size::new(400.0, 300.0)).show(ui, &view, &canvas, &mut pass).id);
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

fn down(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
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

fn drag_to(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
    input(h, ToolInput::Move { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

fn up(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
    input(h, ToolInput::Up { doc: Point::new(at.0, at.1), modifiers });
}

/// A real pointer press or release (screen and document points coincide).
fn press_button(h: &mut Harness<'_, Rig>, button: PointerButton, at: (f64, f64), pressed: bool, modifiers: Modifiers) {
    let pos = pos2(at.0 as f32, at.1 as f32);
    h.event_modifiers(Event::PointerMoved(pos), modifiers);
    h.event_modifiers(Event::PointerButton { pos, button, pressed, modifiers }, modifiers);
    h.step();
}

fn move_to(h: &mut Harness<'_, Rig>, at: (f64, f64), modifiers: Modifiers) {
    h.event_modifiers(Event::PointerMoved(pos2(at.0 as f32, at.1 as f32)), modifiers);
    h.step();
}

fn click(h: &mut Harness<'_, Rig>, at: (f64, f64)) {
    down(h, at, NONE);
    up(h, at, NONE);
}

fn press_key(h: &mut Harness<'_, Rig>, key: Key) {
    h.event(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: NONE });
    h.step();
}

fn events(h: &mut Harness<'_, Rig>) -> Vec<SelectMaskEvent> {
    h.state_mut().canvas.take_events()
}

/// The shape of an edit event: `begin`, `update`, `commit` or `cancel`.
fn tags(events: &[SelectMaskEvent]) -> Vec<&'static str> {
    events
        .iter()
        .map(|e| match e {
            SelectMaskEvent::Edit(EditEvent::Begin { .. }) | SelectMaskEvent::Refine(EditEvent::Begin { .. }) => "begin",
            SelectMaskEvent::Edit(EditEvent::Update { .. }) | SelectMaskEvent::Refine(EditEvent::Update { .. }) => "update",
            SelectMaskEvent::Edit(EditEvent::Commit { .. }) | SelectMaskEvent::Refine(EditEvent::Commit { .. }) => "commit",
            SelectMaskEvent::Edit(EditEvent::Cancel { .. }) | SelectMaskEvent::Refine(EditEvent::Cancel { .. }) => "cancel",
            _ => "other",
        })
        .collect()
}

/// The edit the last event commits.
fn committed(events: &[SelectMaskEvent]) -> SelectMaskEdit {
    let Some(SelectMaskEvent::Edit(EditEvent::Commit { value, .. })) = events.last() else { panic!("a commit last: {events:?}") };
    value.clone()
}

// A click (or a drag under 4 screen points) gives `ObjectTarget::Point`; a longer drag a rect.
#[test]
fn select_mask_object_click_and_drag_make_point_or_rect() {
    let mut h = rig(SelectMaskTool::Object, false);
    click(&mut h, (100.0, 100.0));
    let got = events(&mut h);
    assert_eq!(tags(&got), ["begin", "commit"]);
    let SelectMaskEdit::Object { target, mode, .. } = committed(&got) else { panic!("an object edit") };
    assert_eq!((target, mode), (ObjectTarget::Point([100.0, 100.0]), EditMode::Add));

    down(&mut h, (50.0, 50.0), NONE);
    drag_to(&mut h, (52.0, 51.0), NONE);
    up(&mut h, (52.0, 51.0), NONE);
    let SelectMaskEdit::Object { target, .. } = committed(&events(&mut h)) else { panic!("an object edit") };
    assert_eq!(target, ObjectTarget::Point([50.0, 50.0]), "2 points is a click");

    down(&mut h, (50.0, 50.0), NONE);
    drag_to(&mut h, (120.0, 90.0), NONE);
    up(&mut h, (150.0, 120.0), NONE);
    let SelectMaskEdit::Object { target, .. } = committed(&events(&mut h)) else { panic!("an object edit") };
    assert_eq!(target, ObjectTarget::Rect([50.0, 50.0, 100.0, 70.0]));
}

// Polygonal Lasso: clicks add vertices; Backspace removes the last; Enter closes (3 or more) into
// a Lasso edit; Esc drops the polygon; a click on the first vertex within 6 points closes.
#[test]
fn select_mask_polygonal_lasso_enter_escape_backspace() {
    let mut h = rig(SelectMaskTool::PolygonalLasso, false);
    h.state_mut().focus = true;
    h.step();
    for at in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0), (50.0, 150.0)] {
        click(&mut h, at);
    }
    assert!(h.state().canvas.polygon_open());
    press_key(&mut h, Key::Backspace);
    assert!(events(&mut h).is_empty(), "a vertex is no edit");

    assert!(h.state_mut().canvas.close_polygon());
    let got = events(&mut h);
    assert_eq!(tags(&got), ["begin", "commit"]);
    let SelectMaskEdit::Lasso { points, mode } = committed(&got) else { panic!("a lasso edit") };
    assert_eq!((points, mode), (vec![[50.0, 50.0], [150.0, 50.0], [150.0, 150.0]], EditMode::Add));
    assert!(!h.state().canvas.polygon_open());
    assert!(!h.state_mut().canvas.close_polygon(), "nothing left to close");

    // Esc drops a polygon without an edit; fewer than 3 points make none either.
    click(&mut h, (50.0, 50.0));
    click(&mut h, (150.0, 50.0));
    assert!(h.state_mut().canvas.drop_polygon());
    assert!(!h.state().canvas.polygon_open());
    click(&mut h, (50.0, 50.0));
    assert!(h.state_mut().canvas.close_polygon());
    assert!(events(&mut h).is_empty());

    // A click within 6 points of the first vertex closes; so does a click far from it.
    for at in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0), (53.0, 52.0)] {
        click(&mut h, at);
    }
    let got = events(&mut h);
    let SelectMaskEdit::Lasso { points, .. } = committed(&got) else { panic!("a lasso edit") };
    assert_eq!(points.len(), 3);
    assert!(!h.state().canvas.polygon_open());
}

// With Real-Time Refinement on, a stroke reports Updates; off, only Begin and Commit.
#[test]
fn select_mask_real_time_refinement_reports_updates_only_when_on() {
    for (real_time, expected) in [(false, vec!["begin", "commit"]), (true, vec!["begin", "update", "commit"])] {
        let mut h = rig(SelectMaskTool::Quick, real_time);
        down(&mut h, (10.0, 10.0), NONE);
        drag_to(&mut h, (30.0, 10.0), NONE);
        drag_to(&mut h, (60.0, 10.0), NONE);
        up(&mut h, (60.0, 10.0), NONE);
        let got = events(&mut h);
        assert_eq!(tags(&got), expected, "real time {real_time}");
        let SelectMaskEdit::Quick { points, size, .. } = committed(&got) else { panic!("a quick edit") };
        assert_eq!((points, size), (vec![[10.0, 10.0], [30.0, 10.0], [60.0, 10.0]], 30.0));
    }
}

// [ and ] change the current brush tool's own size by at least a pixel within 1..=5000; Alt at
// the press reverses the bar's mode.
#[test]
fn select_mask_brush_sizes_are_per_tool_and_alt_reverses_the_mode() {
    let mut sizes = BrushSizes::default();
    sizes.set(SelectMaskTool::Quick, 100.0);
    sizes.step(SelectMaskTool::Quick, false);
    sizes.step(SelectMaskTool::Brush, true);
    assert_eq!((sizes.quick, sizes.refine, sizes.brush), (90.0, 30.0, 33.0));
    // At least one pixel of change, inside the range.
    sizes.set(SelectMaskTool::RefineEdge, 5.0);
    sizes.step(SelectMaskTool::RefineEdge, false);
    sizes.step(SelectMaskTool::RefineEdge, true);
    assert_eq!(sizes.refine, 5.0);
    sizes.set(SelectMaskTool::RefineEdge, 1.0);
    sizes.step(SelectMaskTool::RefineEdge, false);
    assert_eq!(sizes.refine, 1.0);
    sizes.set(SelectMaskTool::RefineEdge, 4999.0);
    sizes.step(SelectMaskTool::RefineEdge, true);
    assert_eq!(sizes.refine, 5000.0);
    sizes.set(SelectMaskTool::Hand, 7.0);
    sizes.set(SelectMaskTool::Brush, f64::NAN);
    assert_eq!(sizes.brush, 33.0, "another tool or a non-finite size changes nothing");

    // Alt at the press reverses the bar's mode.
    let mut h = rig(SelectMaskTool::Lasso, false);
    for (alt, mode) in [(NONE, EditMode::Add), (Modifiers::ALT, EditMode::Subtract)] {
        down(&mut h, (10.0, 10.0), alt);
        drag_to(&mut h, (60.0, 10.0), alt);
        drag_to(&mut h, (60.0, 60.0), alt);
        up(&mut h, (60.0, 60.0), alt);
        let SelectMaskEdit::Lasso { mode: got, .. } = committed(&events(&mut h)) else { panic!("a lasso edit") };
        assert_eq!(got, mode);
    }

    // On the brush tools Alt + right-drag and Control + Alt + drag resize the tool's own size and
    // paint nothing; Alt + primary paints, reversed.
    let mut h = rig(SelectMaskTool::Quick, false);
    press_button(&mut h, PointerButton::Secondary, (100.0, 100.0), true, Modifiers::ALT);
    assert!(h.state().canvas.busy());
    move_to(&mut h, (110.0, 100.0), Modifiers::ALT);
    press_button(&mut h, PointerButton::Secondary, (110.0, 100.0), false, Modifiers::ALT);
    assert!(events(&mut h).is_empty());
    assert!(!h.state().canvas.busy());
    assert_eq!(h.state().options.sizes.quick, 50.0, "Alt + right-drag grew the brush");
    h.state_mut().options.sizes.quick = 30.0;
    press_button(&mut h, PointerButton::Primary, (100.0, 100.0), true, Modifiers::ALT);
    press_button(&mut h, PointerButton::Primary, (100.0, 100.0), false, Modifiers::ALT);
    let got = events(&mut h);
    assert!(!got.is_empty(), "Alt + primary paints");
    assert!(matches!(committed(&got), SelectMaskEdit::Quick { mode: EditMode::Subtract, .. }), "{got:?}");
    let chord = Modifiers::ALT | Modifiers::CTRL;
    down(&mut h, (100.0, 100.0), chord);
    assert!(h.state().canvas.busy());
    drag_to(&mut h, (110.0, 100.0), chord);
    up(&mut h, (110.0, 100.0), chord);
    assert!(events(&mut h).is_empty());
    assert!(!h.state().canvas.busy());
    let sizes = h.state().options.sizes;
    assert_eq!((sizes.quick, sizes.refine, sizes.brush), (50.0, 30.0, 30.0));
}

// The release builds its value from the tool the stroke began with and reports it under that
// tool's kind, however the strip changed since the press.
#[test]
fn select_mask_release_uses_the_gestures_own_tool() {
    let mut h = rig(SelectMaskTool::Quick, true);
    down(&mut h, (100.0, 100.0), NONE);
    h.state_mut().tool = SelectMaskTool::Brush;
    drag_to(&mut h, (160.0, 100.0), NONE);
    h.state_mut().tool = SelectMaskTool::RefineEdge;
    up(&mut h, (160.0, 100.0), NONE);
    let got = events(&mut h);
    assert_eq!(tags(&got), ["begin", "update", "commit"], "{got:?}");
    assert!(got.iter().all(|e| matches!(e, SelectMaskEvent::Edit(_))), "{got:?}");
    assert!(matches!(committed(&got), SelectMaskEdit::Quick { .. }), "{got:?}");

    let mut h = rig(SelectMaskTool::RefineEdge, false);
    down(&mut h, (100.0, 100.0), NONE);
    h.state_mut().tool = SelectMaskTool::Quick;
    up(&mut h, (160.0, 100.0), NONE);
    let got = events(&mut h);
    assert_eq!(tags(&got), ["begin", "commit"], "{got:?}");
    assert!(got.iter().all(|e| matches!(e, SelectMaskEvent::Refine(_))), "{got:?}");

    // A stroke begun on a brush ends even when the strip moved to the Polygonal Lasso.
    let mut h = rig(SelectMaskTool::Brush, false);
    down(&mut h, (100.0, 100.0), NONE);
    h.state_mut().tool = SelectMaskTool::PolygonalLasso;
    up(&mut h, (160.0, 100.0), NONE);
    assert!(matches!(committed(&events(&mut h)), SelectMaskEdit::Brush { .. }));
    assert!(!h.state().canvas.busy());
}

// The released stroke stays painted until the workspace says the preview shows it.
#[test]
fn select_mask_stroke_trail_stays_until_the_preview_shows_it() {
    let mut h = rig(SelectMaskTool::Quick, false);
    down(&mut h, (100.0, 100.0), NONE);
    drag_to(&mut h, (160.0, 100.0), NONE);
    assert!(!h.state().canvas.trail_shown(), "painted as the gesture while it is open");
    up(&mut h, (160.0, 100.0), NONE);
    assert!(h.state().canvas.trail_shown());
    h.state_mut().canvas.drop_trail();
    assert!(!h.state().canvas.trail_shown());
    // A new press drops the old trail.
    down(&mut h, (100.0, 100.0), NONE);
    up(&mut h, (100.0, 100.0), NONE);
    assert!(h.state().canvas.trail_shown());
    down(&mut h, (20.0, 20.0), NONE);
    assert!(!h.state().canvas.trail_shown());
}

// Only the brush tools take Alt + right; the others take the primary button alone.
#[test]
fn select_mask_secondary_button_is_taken_by_brushes_with_alt_only() {
    use craft_ui::canvas::CanvasTool as _;
    let mut canvas = SelectMaskCanvas::new();
    let mut options = SelectMaskToolOptions::default();
    for (tool, brush) in [(SelectMaskTool::Quick, true), (SelectMaskTool::Lasso, false), (SelectMaskTool::Object, false)] {
        let pass = canvas.tool(tool, &mut options, false);
        assert!(pass.takes(PointerButton::Primary, NONE));
        assert!(!pass.takes(PointerButton::Secondary, NONE));
        assert_eq!(pass.takes(PointerButton::Secondary, Modifiers::ALT), brush);
    }
}
