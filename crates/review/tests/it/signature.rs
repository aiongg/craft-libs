//! The signature pad's stored and drawn forms.

use craft_behavior::Metrics;
use craft_review::signature::{SavedSignature, SignatureDraft, SignatureImage, SignatureTab};
use craft_review::ui::signature::{DrawPad, paint_signature};
use craft_ui::contract::{Bind, Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role, Show as _};
use craft_ui::style::{ComponentStyle, DataColor, StyleCx};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Color32, Key, Modifiers, Response, Shape, TextureId, Vec2, vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// Apply returns what the current tab holds: typed text, the strokes or the picked image.
#[trial]
fn apply_returns_the_current_tab() {
    let mut draft = SignatureDraft::new(false, "Ana Reyes");
    draft.tab = SignatureTab::Type;
    assert!(matches!(draft.saved(), Some(SavedSignature::Typed(text)) if text == "Ana Reyes"));
    draft.tab = SignatureTab::Draw;
    draft.strokes = vec![vec![[0.1, 0.2], [0.3, 0.4]]];
    assert!(matches!(draft.saved(), Some(SavedSignature::Drawn(strokes)) if strokes.len() == 1));
}

/// Apply waits for content: an empty draft is not ready.
#[trial]
fn apply_waits_for_content() {
    let mut draft = SignatureDraft::new(false, "");
    assert!(!draft.ready());
    draft.tab = SignatureTab::Draw;
    assert!(!draft.ready());
    draft.strokes = vec![vec![[0.1, 0.2]]];
    assert!(!draft.ready(), "a bare click is not a stroke");
    draft.strokes = vec![vec![[0.1, 0.2], [0.3, 0.4]]];
    assert!(draft.ready());
}

/// Command+Z in the draw pad removes the last stroke and leaves the ones before it.
#[trial]
fn undo_removes_the_last_stroke() {
    let strokes = Rc::new(RefCell::new(Vec::new()));
    let shown = strokes.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut strokes = shown.borrow_mut();
        DrawPad::new("Draw", Bind::bound(&mut strokes)).show(ui);
    });
    fixture.settle();
    let pad = fixture.by_label("Draw").expect("the pad");
    fixture.drag(&pad, vec2(40.0, 10.0));
    fixture.drag(&pad, vec2(-30.0, 20.0));
    assert_eq!(strokes.borrow().len(), 2, "two strokes drawn");
    fixture.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(strokes.borrow().len(), 1, "Command+Z removes the last stroke");
}

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
struct PlacedStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A box that paints a saved signature the way an app places it.
#[derive(Component)]
struct Placed<'a> {
    #[common]
    common: Common,
    signature: &'a SavedSignature,
}

impl Component for Placed<'_> {
    const ROLE: Role = Role::IMAGE;
    const PARTS: bool = false;
    const OVERLAYS: bool = false;
    type Style = PlacedStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PlacedStyleKey) -> PlacedStyle {
        PlacedStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        vec2(200.0, 80.0)
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        paint_signature(&cx.response.ctx, &ui.draw(), cx.response.rect, self.signature, DataColor::new(Color32::from_rgb(10, 20, 30)));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// How close two aspect ratios are.
const SAME: f32 = 1e-3;

/// A placed image signature paints its picture as a texture inside the box it is placed in, keeping the picture's
/// shape.
#[trial]
fn a_placed_image_paints_a_texture_in_its_rect() {
    let (width, height) = (4, 2);
    let image = SignatureImage::new(width, height, vec![u8::MAX; width as usize * height as usize * 4]);
    let shape = width as f32 / height as f32;
    let signature = SavedSignature::Image(image);
    let rect = std::cell::Cell::new(egui::Rect::NOTHING);
    let mut frames = Frames::new();
    let shapes = frames.frame_shapes(|ui| rect.set(Placed { common: Common::new(), signature: &signature }.label("Signature").show(ui).rect));
    let rect = rect.get();
    let pictures: Vec<egui::Rect> = shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Mesh(mesh) if mesh.texture_id != TextureId::default() => Some(mesh.calc_bounds()),
            _ => None,
        })
        .collect();
    assert_eq!(pictures.len(), 1, "one picture: {shapes:?}");
    let picture = pictures[0];
    assert!(rect.contains_rect(picture), "{picture:?} inside {rect:?}");
    assert!((picture.aspect_ratio() - shape).abs() < SAME, "the picture keeps its shape: {picture:?}");
}
