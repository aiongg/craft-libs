//! Tests of Select and Mask's session and edits (task WS-SM1).

use craft_ui::dialog::{LocalHistory, Step};
use serde_json::json;

use super::edits::{EditMode, ObjectTarget, RefineStroke, SelectMaskEdit};
use super::session::{LimitRefused, OutputTo, SelectMaskLimits, SelectMaskSession, SelectMaskSettings};
use crate::ui::mask_view::MaskViewMode;

const LIMITS: SelectMaskLimits = SelectMaskLimits { edits: 3, edit_points: 10, strokes: 2, stroke_points: 4 };

fn quick(points: usize) -> SelectMaskEdit {
    SelectMaskEdit::Quick { points: vec![[1.0, 2.0]; points], size: 30.0, mode: EditMode::Add, sample_all_layers: false }
}

fn refine_stroke(points: usize) -> RefineStroke {
    RefineStroke { points: vec![[3.0, 4.0]; points], size: 20.0, mode: EditMode::Subtract }
}

fn record(session: &mut SelectMaskSession, history: &mut LocalHistory<SelectMaskSession>, edit: SelectMaskEdit) {
    assert!(session.add_edit(edit, &LIMITS).is_ok());
    history.settle(session, false);
}

// Edits and strokes are added in order and undo and redo (through `LocalHistory` over the session)
// take them back and forth.
#[test]
fn select_mask_tools_make_edits_and_undo_redo_them() {
    let mut session = SelectMaskSession::default();
    let mut history = LocalHistory::new(&session);
    record(&mut session, &mut history, quick(2));
    record(&mut session, &mut history, SelectMaskEdit::Invert);
    assert_eq!(session.edits, [quick(2), SelectMaskEdit::Invert]);
    assert!(history.step(&mut session, Step::Undo));
    assert_eq!(session.edits, [quick(2)]);
    assert!(history.step(&mut session, Step::Undo));
    assert!(session.edits.is_empty());
    assert!(!history.step(&mut session, Step::Undo));
    assert!(history.step(&mut session, Step::Redo));
    assert!(history.step(&mut session, Step::Redo));
    assert_eq!(session.edits, [quick(2), SelectMaskEdit::Invert]);
}

// Refine Edge Brush strokes go to `strokes`, never `edits`, and serialise as `refineStrokes`.
#[test]
fn select_mask_refine_edge_strokes_are_kept_apart() {
    let mut session = SelectMaskSession::default();
    assert!(session.add_stroke(refine_stroke(2), &LIMITS).is_ok());
    assert!(session.edits.is_empty());
    assert_eq!(session.strokes.len(), 1);
    let value = serde_json::to_value(session.command(false, false)).unwrap_or_default();
    assert_eq!(value["refineStrokes"], json!([{"points": [[3.0, 4.0], [3.0, 4.0]], "size": 20.0, "mode": "subtract"}]));
    assert!(value.get("edits").is_none());
}

// At the edit count or point limit `add_edit` returns the message and changes nothing; the same
// for strokes.
#[test]
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
    assert_eq!(session.edits.len(), 2);

    let mut session = SelectMaskSession::default();
    assert!(session.add_stroke(refine_stroke(3), &LIMITS).is_ok());
    assert_eq!(session.add_stroke(refine_stroke(2), &LIMITS), Err(LimitRefused::StrokePoints(4)));
    assert!(session.add_stroke(refine_stroke(1), &LIMITS).is_ok());
    assert_eq!(session.add_stroke(refine_stroke(0), &LIMITS), Err(LimitRefused::Strokes(2)));
    assert_eq!(session.strokes.len(), 2);
}

// The refusal is worded with the count: "1 edit" and "3 edits" take their own forms.
#[test]
fn select_mask_limit_messages_are_counted_words() {
    let ctx = egui::Context::default();
    craft_ui::theme::install(&ctx, &craft_ui::theme::Base::suite_default());
    assert_eq!(LimitRefused::Edits(1).message(&ctx), "at most 1 edit: undo some, or reset");
    assert_eq!(LimitRefused::Edits(3).message(&ctx), "at most 3 edits: undo some, or reset");
    assert_eq!(LimitRefused::EditPoints(100).message(&ctx), "at most 100 points in all edits: undo some, or reset");
    assert_eq!(LimitRefused::StrokePoints(4).message(&ctx), "at most 4 points in all Refine Edge Brush strokes: undo some, or reset");
}

// Reset clears edits, strokes and the refinement values in one step; undo gives all of them back.
#[test]
fn select_mask_reset_is_one_undo_step_that_keeps_what_it_replaced() {
    let mut session = SelectMaskSession::default();
    let mut history = LocalHistory::new(&session);
    record(&mut session, &mut history, quick(2));
    assert!(session.add_stroke(refine_stroke(1), &LIMITS).is_ok());
    session.refine.radius = 12.0;
    session.refine.feather = 3.0;
    history.settle(&session, false);
    let before = session.clone();
    session.reset();
    history.settle(&session, false);
    assert_eq!(session, SelectMaskSession::default());
    assert!(history.step(&mut session, Step::Undo));
    assert_eq!(session, before);
    assert!(history.step(&mut session, Step::Redo));
    assert_eq!(session, SelectMaskSession::default());
}

// Decontaminate Colors moves Output To from Selection or Layer Mask to New Layer with Mask and
// allows only the new targets.
#[test]
fn select_mask_decontaminate_limits_output_to_new_targets() {
    let mut refine = SelectMaskSession::default().refine;
    refine.follow_decontaminate();
    assert_eq!(refine.output, OutputTo::Selection, "nothing moves while Decontaminate Colors is off");
    for start in [OutputTo::Selection, OutputTo::LayerMask] {
        refine.output = start;
        refine.decontaminate = true;
        refine.follow_decontaminate();
        assert_eq!(refine.output, OutputTo::NewLayerWithMask);
    }
    refine.output = OutputTo::NewDocument;
    refine.follow_decontaminate();
    assert_eq!(refine.output, OutputTo::NewDocument, "a new target stays");
    let allowed: Vec<OutputTo> = OutputTo::ALL.into_iter().filter(|o| o.allowed(true)).collect();
    assert_eq!(allowed, [OutputTo::NewLayer, OutputTo::NewLayerWithMask, OutputTo::NewDocument, OutputTo::NewDocumentWithMask]);
    assert!(OutputTo::ALL.into_iter().all(|o| o.allowed(false)));
}

// `command` serialises as PhotoCraft's `commit_params` (refinement keys at the top, tool-tagged
// edits, `target: "mask"` for a layer mask, empty lists left out).
#[test]
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
        SelectMaskEdit::Brush { points: vec![[0.5, 0.5]], size: 9.0, hardness: 50.0, mode: EditMode::Subtract },
        SelectMaskEdit::Object { target: ObjectTarget::Rect([1.0, 2.0, 3.0, 4.0]), mode: EditMode::Add, sample_all_layers: true },
        SelectMaskEdit::Object { target: ObjectTarget::Point([7.0, 8.0]), mode: EditMode::Add, sample_all_layers: false },
        SelectMaskEdit::Subject { sample_all_layers: true },
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

// The settings round-trip through serde with defaults; `view_state` and `take_view` keep the
// view's fields.
#[test]
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
    assert_eq!(SelectMaskSettings::default().view_state(), crate::ui::mask_view::MaskViewState::default());
}
