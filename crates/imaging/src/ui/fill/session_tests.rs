//! Tests of Content-Aware Fill's session, edits and tools (task WS-CA1).

use serde_json::json;

use craft_ui::canvas::kurbo::{Point, Size};
use craft_ui::canvas::{CanvasId, CanvasView, SelectionOperation, ToolHost, ToolInput};
use craft_ui::dialog::{LocalHistory, Step};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};
use egui::{Event, Modifiers, PointerButton, Sense, pos2, vec2};
use egui_kittest::Harness;

use super::session::{FillCommand, FillLimits, FillReset, FillSession, FillSettings, SamplingMode, SamplingStroke};
use super::tool::{FillCanvas, FillTool, FillToolOptions};
use super::{ContentAwareFillWorkspace, FillAreaEdit, FillEvent, FillOpen};
use crate::ui::select_mask::EditMode;

const NONE: Modifiers = Modifiers::NONE;
const GENEROUS: FillLimits = FillLimits { edits: 1000, points: 200_000 };

struct Rig {
    canvas: FillCanvas,
    tool: FillTool,
    options: FillToolOptions,
    session: FillSession,
    limits: FillLimits,
    id: Option<CanvasId>,
}

/// Document and screen points coincide: the view's centre is the canvas's.
fn rig(tool: FillTool, brush_add: bool, session: FillSession, limits: FillLimits) -> Harness<'static, Rig> {
    let view = CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
    let options = FillToolOptions { brush_add, brush_size: 20.0, lasso: SelectionOperation::New, grow_by: 3.0 };
    let state = Rig { canvas: FillCanvas::new(), tool, options, session, limits, id: None };
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        move |ui, rig: &mut Rig| {
            let canvas_id = CanvasId::new(ui, "canvas");
            let canvas = canvas_id.interact(ui, view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
            let host = ToolHost::new(canvas_id).document(Size::new(400.0, 300.0));
            let Rig { canvas: state, tool, options, session, limits, id } = rig;
            let mut pass = state.tool(*tool, options, session, limits);
            *id = Some(host.show(ui, &view, &canvas, &mut pass).id);
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
    input(h, ToolInput::Down { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

fn drag_to(h: &mut Harness<'_, Rig>, at: (f64, f64)) {
    input(h, ToolInput::Move { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers: NONE });
}

fn up(h: &mut Harness<'_, Rig>, at: (f64, f64)) {
    input(h, ToolInput::Up { doc: Point::new(at.0, at.1), modifiers: NONE });
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

fn events(h: &mut Harness<'_, Rig>) -> Vec<FillEvent> {
    h.state_mut().canvas.take_events()
}

/// The stroke a finished gesture committed, and the kinds of the events around it.
fn committed(events: &[FillEvent]) -> (SamplingStroke, Vec<&'static str>) {
    let kinds = events
        .iter()
        .map(|e| match e {
            FillEvent::Stroke(EditEvent::Begin { .. }) => "begin",
            FillEvent::Stroke(EditEvent::Update { .. }) => "update",
            FillEvent::Stroke(EditEvent::Commit { .. }) => "commit",
            FillEvent::Stroke(EditEvent::Cancel { .. }) => "cancel",
            _ => "other",
        })
        .collect();
    let Some(FillEvent::Stroke(EditEvent::Commit { value, .. })) =
        events.iter().rev().find(|e| matches!(e, FillEvent::Stroke(EditEvent::Commit { .. })))
    else {
        panic!("a stroke committed: {events:?}")
    };
    (value.clone(), kinds)
}

fn stroke(points: &[[f64; 2]]) -> SamplingStroke {
    SamplingStroke { mode: EditMode::Add, size: 20.0, points: points.to_vec() }
}

const SQUARE: [(f64, f64); 4] = [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0), (50.0, 150.0)];

fn lasso_with(bar: SelectionOperation, modifiers: Modifiers) -> Vec<FillEvent> {
    let mut h = rig(FillTool::Lasso, false, FillSession::default(), GENEROUS);
    h.state_mut().options.lasso = bar;
    down(&mut h, SQUARE[0], modifiers);
    for at in &SQUARE[1..] {
        drag_to(&mut h, *at);
    }
    up(&mut h, SQUARE[3]);
    events(&mut h)
}

fn square_points() -> Vec<[f64; 2]> {
    SQUARE.iter().map(|&(x, y)| [x, y]).collect()
}

// In Auto and Rectangular the brush subtracts, in Custom it adds; Alt at the press reverses; a
// stroke keeps its points from before the drag threshold.
#[test]
fn fill_brush_adds_in_custom_and_subtracts_in_auto_and_alt_reverses() {
    // Alt reverses the bar's mode, on the rule and on a real Alt + primary press through the host.
    for (mode, alt, expected) in [
        (SamplingMode::Auto, true, EditMode::Add),
        (SamplingMode::Rectangular, true, EditMode::Add),
        (SamplingMode::Custom, true, EditMode::Subtract),
    ] {
        assert_eq!(FillToolOptions::stroke_mode(mode.brush_adds(), alt), expected, "{mode:?}, alt");
        let mut h = rig(FillTool::SamplingBrush, mode.brush_adds(), FillSession::default(), GENEROUS);
        press_button(&mut h, PointerButton::Primary, (10.0, 10.0), true, Modifiers::ALT);
        press_button(&mut h, PointerButton::Primary, (10.0, 10.0), false, Modifiers::ALT);
        assert_eq!(committed(&events(&mut h)).0.mode, expected, "{mode:?}, Alt + primary press");
    }
    for (mode, expected) in
        [(SamplingMode::Custom, EditMode::Add), (SamplingMode::Auto, EditMode::Subtract), (SamplingMode::Rectangular, EditMode::Subtract)]
    {
        let mut h = rig(FillTool::SamplingBrush, mode.brush_adds(), FillSession::default(), GENEROUS);
        down(&mut h, (10.0, 10.0), NONE);
        drag_to(&mut h, (20.0, 10.0));
        drag_to(&mut h, (20.0, 10.0));
        up(&mut h, (30.0, 10.0));
        let (stroke, kinds) = committed(&events(&mut h));
        assert_eq!(stroke.mode, expected, "{mode:?}");
        assert_eq!(stroke.points, [[10.0, 10.0], [20.0, 10.0], [30.0, 10.0]], "a repeated point is skipped, the release point is kept");
        assert!((stroke.size - 20.0).abs() < f64::EPSILON);
        assert_eq!(kinds, ["begin", "update", "update", "commit"]);
        assert!(!h.state().canvas.busy());
    }
    // Control + Alt with the primary button resizes: right grows the diameter by twice the
    // travel, nothing is painted, and the new size is the tool's option.
    let mut h = rig(FillTool::SamplingBrush, true, FillSession::default(), GENEROUS);
    let chord = Modifiers::CTRL | Modifiers::ALT;
    down(&mut h, (100.0, 100.0), chord);
    assert!(h.state().canvas.busy());
    drag_to(&mut h, (110.0, 100.0));
    input(&mut h, ToolInput::Up { doc: Point::new(110.0, 100.0), modifiers: chord });
    assert_eq!(events(&mut h), []);
    assert!((h.state().options.brush_size - 40.0).abs() < f64::EPSILON);
    assert!(!h.state().canvas.busy());
    // Alt + right-drag resizes the same way and paints nothing.
    let mut h = rig(FillTool::SamplingBrush, true, FillSession::default(), GENEROUS);
    press_button(&mut h, PointerButton::Secondary, (100.0, 100.0), true, Modifiers::ALT);
    assert!(h.state().canvas.busy());
    move_to(&mut h, (110.0, 100.0), Modifiers::ALT);
    press_button(&mut h, PointerButton::Secondary, (110.0, 100.0), false, Modifiers::ALT);
    assert_eq!(events(&mut h), []);
    assert!((h.state().options.brush_size - 40.0).abs() < f64::EPSILON);
    assert!(!h.state().canvas.busy());
    // The secondary button without Alt is not the brush's.
    press_button(&mut h, PointerButton::Secondary, (100.0, 100.0), true, NONE);
    assert!(!h.state().canvas.busy());
    press_button(&mut h, PointerButton::Secondary, (100.0, 100.0), false, NONE);
    // A click is a one-point stroke.
    let mut h = rig(FillTool::SamplingBrush, true, FillSession::default(), GENEROUS);
    down(&mut h, (10.0, 10.0), NONE);
    up(&mut h, (10.0, 10.0));
    assert_eq!(committed(&events(&mut h)).0.points, [[10.0, 10.0]]);
}

// The lasso's operation is the bar's or Shift, Alt, Shift+Alt at the press; Expand and Contract
// clamp to 1..=100; each reports a fill-area edit and serialises as PhotoCraft's `fillEdits`.
#[test]
fn fill_lasso_modes_and_expand_contract_report_fill_area_edits() {
    let points = square_points();
    for (bar, modifiers, expected) in [
        (SelectionOperation::New, NONE, FillAreaEdit::New { points: points.clone() }),
        (SelectionOperation::Add, NONE, FillAreaEdit::Add { points: points.clone() }),
        (SelectionOperation::New, Modifiers::SHIFT, FillAreaEdit::Add { points: points.clone() }),
        (SelectionOperation::New, Modifiers::ALT, FillAreaEdit::Subtract { points: points.clone() }),
        (SelectionOperation::New, Modifiers::SHIFT | Modifiers::ALT, FillAreaEdit::Intersect { points: points.clone() }),
        (SelectionOperation::Intersect, NONE, FillAreaEdit::Intersect { points: points.clone() }),
    ] {
        assert_eq!(lasso_with(bar, modifiers), [FillEvent::FillArea(expected)], "{bar:?} {modifiers:?}");
    }
    // Under three points nothing is reported.
    let mut h = rig(FillTool::Lasso, false, FillSession::default(), GENEROUS);
    down(&mut h, (50.0, 50.0), NONE);
    up(&mut h, (60.0, 50.0));
    assert_eq!(events(&mut h), []);
    assert_eq!(FillAreaEdit::lasso(SelectionOperation::Add, vec![[0.0, 0.0], [1.0, 0.0]]), None);

    assert_eq!(FillAreaEdit::grow(true, 500.0), FillAreaEdit::Expand { by: 100.0 });
    assert_eq!(FillAreaEdit::grow(false, 0.0), FillAreaEdit::Contract { by: 1.0 });
    assert_eq!(FillAreaEdit::grow(true, 4.0), FillAreaEdit::Expand { by: 4.0 });
    let edits =
        [FillAreaEdit::Subtract { points: vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]] }, FillAreaEdit::grow(true, 4.0), FillAreaEdit::grow(false, 2.0)];
    assert_eq!(
        serde_json::to_value(edits).ok(),
        Some(json!([{"op": "subtract", "points": [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]}, {"op": "expand", "by": 4.0}, {"op": "contract", "by": 2.0}]))
    );
}

fn session_with_work() -> FillSession {
    let mut session = FillSession::default();
    session.set_sampling(SamplingMode::Custom);
    session.add_stroke(stroke(&[[1.0, 1.0], [2.0, 2.0]]));
    assert_eq!(session.add_fill_edit(FillAreaEdit::grow(true, 3.0), &GENEROUS), Ok(()));
    session
}

// Reset Sampling clears the strokes and returns to Auto; Reset Fill only the fill settings; the
// footer's Reset everything; each session change is one undo step that gives back what it replaced.
#[test]
fn fill_reset_scopes_sampling_fill_and_all() {
    let changed = FillSettings { sample_all_layers: true, opacity: 10.0, scale: true, mirror: true, ..FillSettings::default() };

    // Sampling: the strokes and the option; the fill edits stay.
    let (mut session, mut settings) = (session_with_work(), changed.clone());
    settings.sampling = SamplingMode::Custom;
    let mut history = LocalHistory::new(&session);
    assert!(session.reset(FillReset::Sampling, &mut settings));
    history.settle(&session, false);
    assert_eq!((session.sampling, session.strokes.len(), session.fill_edits.len()), (SamplingMode::Auto, 0, 1));
    assert!(!settings.sample_all_layers && settings.sampling == SamplingMode::Auto);
    assert!(settings.scale, "the fill settings stay");
    assert!(history.step(&mut session, Step::Undo));
    assert_eq!(session, session_with_work());

    // Fill: only the settings, and the session reports no change.
    let (mut session, mut settings) = (session_with_work(), changed.clone());
    assert!(!session.reset(FillReset::Fill, &mut settings));
    assert_eq!(session, session_with_work());
    assert!(!settings.scale && !settings.mirror && settings.sample_all_layers && (settings.opacity - 10.0).abs() < f64::EPSILON);

    // Fill area: only the lasso edits.
    let (mut session, mut settings) = (session_with_work(), changed.clone());
    assert!(session.reset(FillReset::FillArea, &mut settings));
    assert_eq!((session.strokes.len(), session.fill_edits.len()), (1, 0));
    assert!(!session.reset(FillReset::FillArea, &mut settings), "nothing left to reset");

    // All: the session and every setting; one undo step.
    let (mut session, mut settings) = (session_with_work(), changed);
    let mut history = LocalHistory::new(&session);
    assert!(session.reset(FillReset::All, &mut settings));
    history.settle(&session, false);
    assert_eq!(session, FillSession::default());
    assert_eq!(settings, FillSettings::default());
    assert!(history.step(&mut session, Step::Undo));
    assert_eq!(session, session_with_work());
    assert!(!history.can_undo());

    // A new option with the same value and no strokes changes nothing.
    let mut session = FillSession::default();
    assert!(!session.set_sampling(SamplingMode::Auto));
    assert!(session.set_sampling(SamplingMode::Rectangular));
}

// At the stroke or points limit a stroke does not begin and the message is reported; a stroke at
// the points limit stops taking points.
#[test]
fn fill_limits_refuse_at_the_brush() {
    let limits = FillLimits { edits: 2, points: 5 };
    let mut session = FillSession::default();
    session.add_stroke(stroke(&[[0.0, 0.0]]));
    assert_eq!(session.may_begin_stroke(&limits), Ok(()));
    session.add_stroke(stroke(&[[0.0, 0.0], [1.0, 1.0]]));
    let Err(message) = session.may_begin_stroke(&limits) else { panic!("two strokes are the limit") };
    assert_eq!(message, "at most 2 sampling strokes: undo some, or reset the sampling area");

    // Points are counted over strokes and lasso edits; a total equal to the limit passes an edit.
    let limits = FillLimits { edits: 10, points: 5 };
    let mut session = FillSession::default();
    session.add_stroke(stroke(&[[0.0, 0.0], [1.0, 1.0]]));
    let triangle = || FillAreaEdit::Add { points: vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]] };
    assert_eq!(session.add_fill_edit(triangle(), &limits), Ok(()));
    assert_eq!(session.points_used(None), 5);
    assert_eq!(session.points_used(Some(&stroke(&[[9.0, 9.0]]))), 6);
    assert_eq!(session.may_begin_stroke(&limits), Err("at most 5 points in all sampling strokes and lasso edits: undo some, or reset".to_owned()));
    assert_eq!(session.add_fill_edit(triangle(), &limits), Err(limits.points_message()));
    assert_eq!(session.fill_edits.len(), 1, "a refused edit records nothing");
    let tight = FillLimits { edits: 1, points: 100 };
    assert_eq!(session.add_fill_edit(triangle(), &tight), Err("at most 1 lasso edits: undo some, or reset the fill area".to_owned()));

    // The brush: no stroke begins at the stroke limit.
    let mut full = FillSession::default();
    full.add_stroke(stroke(&[[0.0, 0.0]]));
    let mut h = rig(FillTool::SamplingBrush, true, full, FillLimits { edits: 1, points: 100 });
    down(&mut h, (10.0, 10.0), NONE);
    up(&mut h, (10.0, 10.0));
    assert_eq!(events(&mut h), [FillEvent::Refused("at most 1 sampling strokes: undo some, or reset the sampling area".to_owned())]);
    assert!(!h.state().canvas.busy());

    // A stroke at the points limit stops taking points; the message comes once.
    let limits = FillLimits { edits: 10, points: 3 };
    let mut h = rig(FillTool::SamplingBrush, true, FillSession::default(), limits);
    down(&mut h, (10.0, 10.0), NONE);
    for x in [20.0, 30.0, 40.0, 50.0] {
        drag_to(&mut h, (x, 10.0));
    }
    up(&mut h, (60.0, 10.0));
    let all = events(&mut h);
    let refusals: Vec<_> = all.iter().filter(|e| matches!(e, FillEvent::Refused(_))).collect();
    assert_eq!(refusals, [&FillEvent::Refused(limits.points_message())]);
    assert_eq!(committed(&all).0.points, [[10.0, 10.0], [20.0, 10.0], [30.0, 10.0]]);
}

// The settings round-trip through serde with defaults; `FillCommand` serialises as `plan_params`.
#[test]
fn fill_settings_and_command_serialise_as_photocraft() {
    let defaults = FillSettings::default();
    assert_eq!(serde_json::from_value::<FillSettings>(json!({})).ok(), Some(defaults.clone()));
    let Ok(round) = serde_json::to_value(&defaults) else { panic!("settings serialise") };
    assert_eq!(serde_json::from_value::<FillSettings>(round).ok(), Some(defaults.clone()));
    let Ok(partial) = serde_json::from_value::<FillSettings>(
        json!({"opacity": 30.0, "colorAdaptation": "veryHigh", "rotation": "full", "output": "duplicate", "color": [1, 2, 3]}),
    ) else {
        panic!("a partial object reads")
    };
    assert_eq!((partial.opacity, partial.color, partial.show_overlay), (30.0, [1, 2, 3], true));
    assert_eq!(partial.output, super::FillOutputTo::DuplicateLayer);

    // No strokes: no `samplingStrokes` key; the layer is the app's.
    let mut session = FillSession::default();
    assert_eq!(
        serde_json::to_value(FillCommand::new(&session, &defaults)).ok(),
        Some(json!({
            "sampling": "auto", "sampleAllLayers": false, "colorAdaptation": "default", "rotationAdaptation": "none",
            "scale": false, "mirror": false, "output": "new", "fillEdits": [],
        }))
    );
    session.set_sampling(SamplingMode::Custom);
    session.add_stroke(SamplingStroke { mode: EditMode::Subtract, size: 12.0, points: vec![[1.0, 2.0]] });
    assert_eq!(session.add_fill_edit(FillAreaEdit::grow(false, 5.0), &GENEROUS), Ok(()));
    let settings = FillSettings { sample_all_layers: true, scale: true, rotation: super::RotationAdaptation::Medium, ..defaults };
    assert_eq!(
        serde_json::to_value(FillCommand::new(&session, &settings)).ok(),
        Some(json!({
            "sampling": "custom", "sampleAllLayers": true, "colorAdaptation": "default", "rotationAdaptation": "medium",
            "scale": true, "mirror": false, "output": "new", "fillEdits": [{"op": "contract", "by": 5.0}],
            "samplingStrokes": [{"mode": "subtract", "size": 12.0, "points": [[1.0, 2.0]]}],
        }))
    );
}

// `rebase` clears the history and keeps the strokes and edits; `refuse_fill_edit` takes the last
// edit back with no redo.
#[test]
#[ignore = "needs WS-CA2: `rebase` and `refuse_fill_edit` are the workspace's"]
fn fill_rebase_clears_history_and_a_refused_edit_is_taken_back() {
    let mut w = ContentAwareFillWorkspace::new(FillOpen::new([100, 100], "Layer 1", GENEROUS));
    w.session.add_stroke(stroke(&[[1.0, 1.0]]));
    w.history.settle(&w.session, false);
    w.rebase();
    assert!(!w.history.can_undo() && !w.history.can_redo());
    assert_eq!(w.session.strokes.len(), 1);

    assert_eq!(w.session.add_fill_edit(FillAreaEdit::grow(true, 3.0), &GENEROUS), Ok(()));
    w.history.settle(&w.session, false);
    w.refuse_fill_edit("the fill area is empty".to_owned());
    assert!(w.session.fill_edits.is_empty());
    assert_eq!(w.session.strokes.len(), 1);
    assert!(!w.history.can_redo(), "a refused edit leaves no redo");
    assert_eq!(w.error.as_deref(), Some("the fill area is empty"));
}
