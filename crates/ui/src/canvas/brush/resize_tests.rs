//! Tests of the brush resize gesture and the tip falloff.

use std::sync::Arc;

use egui::{Color32, Context, LayerId, Modifiers, Painter, PointerButton, Pos2, Rect as ScreenRect, Shape, Vec2, pos2};
use kurbo::Point;

use super::{BrushDimensions, BrushLevel, BrushResize, BrushResizeLabels, BrushResizeOptions, BrushStyle, TipFalloff};
use crate::canvas::CanvasView;
use crate::edit::{CancelReason, EditEvent, GestureHost, UndoMarks};
use crate::localized::{Labels, LocaleNumbers, Units, Words};
use crate::theme::{Base, current, install};

fn style() -> BrushStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    BrushStyle::of(&current(&ctx))
}

fn view() -> CanvasView {
    CanvasView::new(ScreenRect::from_min_size(pos2(0.0, 0.0), Vec2::splat(400.0)), Point::new(200.0, 200.0), 1.0)
}

fn start() -> BrushDimensions {
    BrushDimensions { diameter: 40.0, level: 0.5 }
}

fn options() -> BrushResizeOptions {
    BrushResizeOptions::new(&style(), 1000.0)
}

fn running(view: &CanvasView) -> BrushResize {
    let mut resize = BrushResize::default();
    assert!(resize.begin(view, pos2(200.0, 200.0), start(), &options()).is_some());
    resize
}

fn value_of(event: Option<EditEvent<BrushDimensions>>) -> BrushDimensions {
    *event.expect("an event").value().expect("a value")
}

// Alt with the secondary and Ctrl + Alt with the primary are chords; Alt with the primary alone is not.
#[test]
fn chords_are_alt_secondary_and_ctrl_alt_primary() {
    let ctrl_alt = Modifiers { ctrl: true, alt: true, ..Modifiers::NONE };
    assert!(BrushResize::chord(PointerButton::Secondary, Modifiers::ALT));
    assert!(BrushResize::chord(PointerButton::Primary, ctrl_alt));
    assert!(!BrushResize::chord(PointerButton::Primary, Modifiers::ALT));
    assert!(!BrushResize::chord(PointerButton::Primary, Modifiers::NONE));
    assert!(!BrushResize::chord(PointerButton::Secondary, Modifiers::NONE));
    assert!(!BrushResize::chord(PointerButton::Primary, Modifiers::CTRL));
}

// PhotoCraft's `resized` maths: `start + 2 × dx` rounded and clamped to 1..=max; the level clamped and in whole percent.
#[test]
fn right_grows_and_down_hardens() {
    let view = view();
    let mut resize = running(&view);
    let now = value_of(resize.track(&view, pos2(210.0, 220.0), Modifiers::NONE));
    assert_eq!(now.diameter, 60.0);
    assert!((now.level - 0.6).abs() < 1e-9, "{now:?}");
    let now = value_of(resize.track(&view, pos2(0.0, 1000.0), Modifiers::NONE));
    assert_eq!(now.diameter, 1.0);
    assert_eq!(now.level, 1.0);
    let now = value_of(resize.track(&view, pos2(5000.0, -1000.0), Modifiers::NONE));
    assert_eq!(now.diameter, 1000.0);
    assert_eq!(now.level, 0.0);
    let now = value_of(resize.track(&view, pos2(200.4, 200.4), Modifiers::NONE));
    assert_eq!(now.level, 0.5);
    assert!(resize.track(&view, pos2(200.4, 200.4), Modifiers::NONE).is_none(), "no change, no event");
}

// Right stays bigger and down harder in a rotated or flipped view (PhotoCraft's `a_rotated_view_keeps_horizontal_for_size_and_vertical_for_hardness`).
#[test]
fn directions_are_screen_relative_in_a_turned_or_flipped_view() {
    for (rotation, flip_x, zoom) in [(90.0, false, 1.0), (33.0, true, 2.0), (180.0, true, 0.5)] {
        let view = CanvasView { rotation, flip_x, zoom, ..view() };
        let at = view.to_screen(Point::new(200.0, 200.0));
        let mut resize = BrushResize::default();
        resize.begin(&view, at, start(), &options());
        let now = value_of(resize.track(&view, at + Vec2::new(10.0, 0.0), Modifiers::NONE));
        assert_eq!(now.diameter, (40.0 + 20.0 / zoom).round(), "right, {rotation} {flip_x}");
        assert_eq!(now.level, 0.5);
        let now = value_of(resize.track(&view, at + Vec2::new(0.0, 20.0), Modifiers::NONE));
        assert_eq!(now.diameter, 40.0);
        assert!((now.level - 0.6).abs() < 1e-9, "down, {rotation} {flip_x}: {now:?}");
    }
}

// With `BrushLevel::Opacity` the vertical travel changes the level and the readout says Opacity.
#[test]
fn vary_hardness_off_moves_opacity() {
    let view = view();
    let mut resize = BrushResize::default();
    resize.begin(&view, pos2(200.0, 200.0), start(), &options().level(BrushLevel::Opacity));
    resize.track(&view, pos2(200.0, 240.0), Modifiers::NONE);
    assert!((resize.dimensions().unwrap().level - 0.7).abs() < 1e-9);
    let readout = resize.readout(&english_units(), &BrushResizeLabels::english()).unwrap();
    let labels: Vec<&str> = readout.rows().iter().map(|(label, _)| label.as_str()).collect();
    assert_eq!(labels, ["Diameter:", "Opacity:"]);
}

// Begin, Update per change and Commit share one id; an unmoved click commits with `changed` false.
#[test]
fn one_gesture_is_begin_update_commit() {
    let view = view();
    let mut resize = BrushResize::default();
    let begin = resize.begin(&view, pos2(200.0, 200.0), start(), &options()).unwrap();
    let id = begin.id().unwrap();
    assert!(resize.active());
    let update = resize.track(&view, pos2(220.0, 200.0), Modifiers::NONE).unwrap();
    assert!(matches!(update, EditEvent::Update { .. }));
    assert_eq!(update.id(), Some(id));
    let commit = resize.finish(&view, pos2(220.0, 200.0), Modifiers::NONE).unwrap();
    assert!(matches!(commit, EditEvent::Commit { changed: true, .. }));
    assert_eq!((commit.id(), commit.value().copied()), (Some(id), Some(BrushDimensions { diameter: 80.0, level: 0.5 })));
    assert!(!resize.active());
    assert!(resize.finish(&view, pos2(0.0, 0.0), Modifiers::NONE).is_none());

    let mut click = running(&view);
    let commit = click.finish(&view, pos2(200.0, 200.0), Modifiers::NONE).unwrap();
    assert!(matches!(commit, EditEvent::Commit { changed: false, .. }));
}

#[derive(Default)]
struct Brush {
    now: BrushDimensions,
    undo: Vec<BrushDimensions>,
}

impl UndoMarks for Brush {
    type Mark = usize;
    fn undo_mark(&self) -> usize {
        self.undo.len()
    }
    fn revert_to(&mut self, mark: usize) {
        if let Some(&v) = self.undo.get(mark) {
            self.now = v;
        }
        self.undo.truncate(mark);
    }
    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

// `cancel` returns `Cancel`; a `GestureHost` fed the events is back at the start values.
#[test]
fn escape_restores_the_start_values() {
    let (view, ctx) = (view(), Context::default());
    let mut resize = BrushResize::default();
    let mut brush = Brush { now: start(), undo: Vec::new() };
    let mut host = GestureHost::new();
    let apply = |brush: &mut Brush, v| {
        brush.undo.push(brush.now);
        brush.now = v;
    };
    let begin = resize.begin(&view, pos2(200.0, 200.0), start(), &options()).unwrap();
    host.handle(&ctx, &mut brush, begin, apply);
    let update = resize.track(&view, pos2(260.0, 230.0), Modifiers::NONE).unwrap();
    host.handle(&ctx, &mut brush, update, apply);
    assert_ne!(brush.now, start());
    let cancel = resize.cancel().unwrap();
    assert!(matches!(cancel, EditEvent::Cancel { reason: CancelReason::Escape, .. }));
    host.handle(&ctx, &mut brush, cancel, apply);
    assert_eq!(brush.now, start());
    assert!(!resize.active());
    assert!(resize.cancel().is_none());
}

// `begin` while active drops the old run; the new run starts from the new `start` with a new id.
#[test]
fn a_new_press_ends_a_resize_whose_release_was_lost() {
    let view = view();
    let mut resize = running(&view);
    let first = resize.track(&view, pos2(230.0, 200.0), Modifiers::NONE).unwrap().id();
    let again = BrushDimensions { diameter: 10.0, level: 0.2 };
    let begin = resize.begin(&view, pos2(100.0, 100.0), again, &options()).unwrap();
    assert_ne!(begin.id(), first);
    assert_eq!(resize.dimensions(), Some(again));
    assert!(resize.begin(&view, pos2(f32::NAN, 0.0), again, &options()).is_none());
    assert!(!resize.active());
}

fn mesh_of(falloff: &TipFalloff, tint: Color32) -> egui::Mesh {
    let ctx = Context::default();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        let painter = Painter::new(ui.ctx().clone(), LayerId::background(), ScreenRect::from_min_size(Pos2::ZERO, Vec2::splat(400.0)));
        falloff.paint(&painter, pos2(200.0, 200.0), 1.0, tint);
    });
    output.textures_delta.clear();
    output
        .shapes
        .into_iter()
        .find_map(|clipped| match clipped.shape {
            Shape::Mesh(mesh) => Some(Arc::unwrap_or_clone(mesh)),
            _ => None,
        })
        .expect("a mesh")
}

// The preview mesh's first ring has the full tint and its last is transparent (PhotoCraft's `the_tip_preview_follows_the_brush_falloff`).
#[test]
fn falloff_mesh_is_solid_to_the_core_and_clear_at_the_reach() {
    let tint = Color32::from_rgba_unmultiplied(255, 0, 0, 110);
    let mesh = mesh_of(&TipFalloff::from_hardness(50.0, 0.4), tint);
    let n = (mesh.vertices.len() - 1) / (TipFalloff::RINGS + 1);
    assert_eq!(mesh.vertices[0].color, tint);
    assert!(mesh.vertices[1..=n].iter().all(|v| v.color == tint), "core ring");
    let last = &mesh.vertices[1 + TipFalloff::RINGS * n..];
    assert!(last.iter().all(|v| v.color == Color32::TRANSPARENT), "reach ring");
    assert!((mesh.vertices[1 + n].pos - pos2(200.0, 200.0)).length() > 1.0);
    let reach = (last[0].pos - pos2(200.0, 200.0)).length();
    assert!((reach - 50.0).abs() < 0.01, "{reach}");
    let alpha: Vec<u8> = (0..=TipFalloff::RINGS).map(|k| mesh.vertices[1 + k * n].color.a()).collect();
    assert!(alpha.windows(2).all(|w| w[0] >= w[1]), "{alpha:?}");

    let outside = TipFalloff::hard(10.0);
    let ctx = Context::default();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        let painter = Painter::new(ui.ctx().clone(), LayerId::background(), ScreenRect::from_min_size(Pos2::ZERO, Vec2::splat(100.0)));
        outside.paint(&painter, pos2(1000.0, 1000.0), 1.0, tint);
        TipFalloff::sampled(f64::NAN, 5.0, |_| 1.0).paint(&painter, pos2(50.0, 50.0), 1.0, tint);
    });
    output.textures_delta.clear();
    assert!(output.shapes.iter().all(|c| !matches!(c.shape, Shape::Mesh(_))));
}

// The readout rows are Diameter and the level's label, with whole pixels and a whole percentage.
#[test]
fn readout_lists_diameter_and_the_level() {
    let view = view();
    let mut resize = running(&view);
    resize.track(&view, pos2(217.0, 200.0), Modifiers::NONE);
    let units = english_units();
    let readout = resize.readout(&units, &BrushResizeLabels::english()).unwrap();
    let rows: Vec<(&str, &str)> = readout.rows().iter().map(|(label, value)| (label.as_str(), value.as_str())).collect();
    assert_eq!(rows, [("Diameter:", "74 px"), ("Hardness:", "50%")]);
    assert!(BrushResize::default().readout(&units, &BrushResizeLabels::english()).is_none());
}

// A translated pixel word and the region's digits reach the readout (the pseudo-locale's marker stands for a catalog row).
#[test]
fn readout_values_use_the_unit_words_and_digits() {
    let view = view();
    let mut resize = running(&view);
    resize.track(&view, pos2(217.0, 200.0), Modifiers::NONE);
    let words = Words::new(|word| (word.english == "{n} px").then(|| "⟦{n} px⟧".to_owned()));
    let units = Units::new(Arc::new(LocaleNumbers::from_tag("ar-EG")), Arc::new(words));
    let readout = resize.readout(&units, &BrushResizeLabels::english()).unwrap();
    let rows = readout.rows();
    assert!(rows[0].1.starts_with('⟦') && rows[0].1.contains("٧٤"), "{rows:?}");
    // The percent word is untranslated here: English template, Arabic-Indic digits.
    assert!(rows[1].1.contains("٥٠") && rows[1].1.ends_with('%'), "{rows:?}");
}

// The primary button always starts a brush gesture; the secondary only with Alt.
#[test]
fn takes_the_primary_button_and_the_secondary_with_alt() {
    assert!(BrushResize::takes(PointerButton::Primary, Modifiers::NONE));
    assert!(BrushResize::takes(PointerButton::Primary, Modifiers::ALT));
    assert!(BrushResize::takes(PointerButton::Secondary, Modifiers::ALT));
    assert!(!BrushResize::takes(PointerButton::Secondary, Modifiers::NONE));
    assert!(!BrushResize::takes(PointerButton::Secondary, Modifiers::CTRL));
    assert!(!BrushResize::takes(PointerButton::Middle, Modifiers::ALT));
}

fn english_units() -> Units {
    Units::new(Arc::new(LocaleNumbers::from_tag("en-US")), Arc::new(Words::english()))
}
