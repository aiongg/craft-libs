//! The preview view maths and what a remembering dialog saves.

use craft_behavior::CanvasTransform;
use craft_ui::patterns::form_dialogs::{DialogFields, FieldValue, Param, ParamDialog, ParamKind, PreviewPane};
use craft_ui::testing::trial;
use craft_ui::value_drag::SliderScale;
use egui::Vec2;

const PICTURE: Vec2 = Vec2::new(400.0, 300.0);
const WELL: Vec2 = Vec2::new(200.0, 200.0);
const START: Vec2 = Vec2::new(10.0, -20.0);
const STEPPED: Vec2 = STEPPED;
const FAR: Vec2 = Vec2::new(5000.0, -5000.0);

/// Zoom moves one level at a time, keeps the picture's centre where it is on the screen and stops at the ends.
#[trial]
fn zoom_steps_through_the_levels_and_stops_at_the_ends() {
    let view = CanvasTransform::new(1.0, START);
    let bigger = PreviewPane::zoom_in(view);
    assert_eq!(bigger.scale, 1.5);
    assert_eq!(bigger.offset, STEPPED);
    assert_eq!(PreviewPane::zoom_out(view).scale, 0.75);

    let mut view = view;
    for _ in 0..20 {
        view = PreviewPane::zoom_in(view);
    }
    assert!(!PreviewPane::can_zoom_in(view) && PreviewPane::can_zoom_out(view));
    for _ in 0..20 {
        view = PreviewPane::zoom_out(view);
    }
    assert!(PreviewPane::can_zoom_in(view) && !PreviewPane::can_zoom_out(view));
}

/// A picture smaller than the well sits in its middle; a larger one never shows a gap at an edge.
#[trial]
fn the_view_keeps_the_picture_in_the_well() {
    let small = PreviewPane::settled(CanvasTransform::new(0.25, FAR), PICTURE, WELL);
    assert_eq!(small.offset, Vec2::ZERO);

    let panned = PreviewPane::settled(CanvasTransform::new(1.0, FAR), PICTURE, WELL);
    assert_eq!(panned.offset, Vec2::new((PICTURE.x - WELL.x) / 2.0, -(PICTURE.y - WELL.y) / 2.0));
}

/// Only the portable rows are remembered: numbers, choices, check boxes and unit choices, not colours or text.
#[trial]
fn a_remembering_dialog_saves_the_portable_values() {
    let params = vec![
        Param::new("radius", "Radius", ParamKind::range(0.1, 1000.0, 1.0, SliderScale::Log)),
        Param::new("smooth", "Smooth", ParamKind::bool(false)),
        Param::new("note", "Note", ParamKind::text("")),
        Param::new("scratch", "Scratch", ParamKind::bool(false)).not_remembered(),
    ];
    let mut fields = DialogFields::new();
    fields.set("radius", 4.0);
    fields.set("smooth", true);
    fields.set("note", "hello");
    fields.set("scratch", true);
    fields.set("stale", 1.0);

    let saved = ParamDialog::remembered(&params, &fields);
    assert_eq!(saved.number("radius"), Some(4.0));
    assert_eq!(saved.bool("smooth"), Some(true));
    assert!(!saved.contains("note") && !saved.contains("scratch") && !saved.contains("stale"));
    assert_eq!(fields.get("note"), Some(&FieldValue::Text("hello".into())));
}
