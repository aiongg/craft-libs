//! What a component records lands in call order, a face's paint included.

use craft_behavior::PartKey;
use craft_ui::contract::{Common, Component, Cx, MeasureCx, PaintUi, PartsCx, Props as _, Role, Show as _};
use craft_ui::style::StyleCx;
use craft_ui::testing::{Frames, trial};
use craft_ui_test_kit::contract::PlainStyle;
use egui::{Color32, Pos2, Rect, Shape, Vec2, pos2};

const SLOT: PartKey = PartKey::new("slot");

fn fill(ui: &PaintUi<'_>, x: f32) {
    ui.draw().canvas().rect_filled(Rect::from_min_size(pos2(x, 0.0), Vec2::splat(4.0)), 0.0, Color32::WHITE);
}

/// A face that records one fill at x = 20.
#[derive(craft_ui::contract::Component)]
struct Stamp {
    #[common]
    common: Common,
}

impl Component for Stamp {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = egui::Response;

    fn style(_: &StyleCx<'_>, _: StampStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, _: &Cx<'_, Self>) {
        fill(ui, 20.0);
    }

    fn finish(self, cx: Cx<'_, Self>) -> egui::Response {
        cx.response.clone()
    }
}

/// Records a fill at x = 10, paints a `Stamp` face, then records a fill at x = 30.
#[derive(craft_ui::contract::Component)]
struct Sandwich {
    #[common]
    common: Common,
}

impl Component for Sandwich {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = egui::Response;

    fn style(_: &StyleCx<'_>, _: SandwichStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(60.0, 30.0)
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(SLOT, rect);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, _: &Cx<'_, Self>) {
        fill(ui, 10.0);
        ui.face(SLOT, Stamp { common: Common::new() });
        fill(ui, 30.0);
    }

    fn finish(self, cx: Cx<'_, Self>) -> egui::Response {
        cx.response.clone()
    }
}

fn corners(shape: &Shape, out: &mut Vec<Pos2>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| corners(shape, out)),
        Shape::Rect(rect) if rect.rect.size() == Vec2::splat(4.0) => out.push(rect.rect.min),
        _ => {}
    }
}

/// A fill recorded before a face, the face's own fill and a fill recorded after it paint in that order. Guards the
/// host's recorded items flushing after a face's (a thumbnail landing over the badges painted on it).
#[trial]
fn recorded_items_and_a_face_paint_in_call_order() {
    let mut frames = Frames::new();
    let shapes = frames.frame_shapes(|ui| {
        Sandwich { common: Common::new() }.label("Sandwich").show(ui);
    });
    let mut at = Vec::new();
    shapes.iter().for_each(|shape| corners(shape, &mut at));
    let xs: Vec<f32> = at.iter().map(|p| p.x).collect();
    assert_eq!(xs.len(), 3, "the three fills paint: {at:?}");
    assert!(xs[0] < xs[1] && xs[1] < xs[2], "fill, face fill, fill in call order: {xs:?}");
}
