//! Data paint through `Draw`: a stroke at a width from the document and a texture, for an active and an inactive
//! part.

use craft_behavior::{PartKey, Sense};
use craft_theme::{Contrast, Mode, Rgba};
use craft_ui::contract::{Common, Component, Cx, PaintUi, PartsCx, Pointer as _, Props as _, Role, Show as _};
use craft_ui::primitives::{DataWidth, PaintShape, StrokeWidth, TextureCache};
use craft_ui::style::{ComponentStyle, DataColor, StyleCx};
use craft_ui::testing::style::tokens;
use craft_ui::testing::{Frames, trial};
use egui::{Color32, Rect, Response, Shape, TextureId, Vec2, pos2};

const BODY: PartKey = PartKey::new("body");
const INK: Color32 = Color32::from_rgb(12, 34, 56);
const DATA_WIDTH: f32 = 6.0;
const UV: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
const EPS: f32 = 1e-3;

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
struct PlateLook {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

/// A leaf whose one part strokes a document-width line and paints a texture.
#[derive(craft_ui::contract::Component)]
struct Plate {
    #[common]
    common: Common,
    inactive: bool,
    texture: TextureId,
}

impl Component for Plate {
    const ROLE: Role = Role::GROUP;
    type Style = PlateLook;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PlateStyleKey) -> PlateLook {
        PlateLook { metrics: craft_behavior::Metrics::new().min_size(Vec2::new(60.0, 20.0)) }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(BODY, rect).inactive(self.inactive);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rect = cx.part(BODY).rect;
        let draw = ui.draw_part(BODY);
        draw.stroke(&PaintShape::Path(vec![rect.left_center(), rect.right_center()]), DataWidth::points(DATA_WIDTH), DataColor::new(INK));
        draw.stroke(&PaintShape::Path(vec![rect.left_top(), rect.right_top()]), StrokeWidth::Thick, DataColor::new(INK));
        draw.texture(rect, self.texture, UV);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn color(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.0[0], c.0[1], c.0[2], c.0[3])
}

fn stroke_of(shape: &Shape) -> Option<(f32, Color32)> {
    match shape {
        Shape::LineSegment { stroke, .. } => Some((stroke.width, stroke.color)),
        Shape::Path(path) => match path.stroke.color {
            egui::epaint::ColorMode::Solid(color) => Some((path.stroke.width, color)),
            _ => None,
        },
        _ => None,
    }
}

/// The strokes, the textured meshes and the filled rects the plate painted in its last frame, and its texture.
fn run(inactive: bool) -> (Vec<(f32, Color32)>, Vec<Shape>, TextureId) {
    let mut frames = Frames::new();
    let texture = TextureCache::texture(&frames.ctx, "plate", || egui::ColorImage::filled([2, 2], Default::default()));
    let mut show = |ui: &mut egui::Ui| {
        Plate { common: Common::new(), inactive, texture }.label("Plate").show(ui);
    };
    let shapes = frames.frame_shapes(&mut show);
    let strokes = shapes.iter().filter_map(stroke_of).collect();
    (strokes, shapes, texture)
}

#[trial]
fn a_data_paint_strokes_and_textures_for_an_active_and_an_inactive_part() {
    {
        let tokens = tokens(Mode::Light, Contrast::Standard);
        let (strokes, ..) = run(false);
        assert!(
            strokes.iter().any(|(width, c)| (width - DATA_WIDTH).abs() < EPS && *c == INK),
            "the document width is the stroke width: {strokes:?}"
        );
        assert!(strokes.iter().any(|(width, _)| (width - tokens.stroke.thick).abs() < EPS), "a named width keeps the theme size");
    }
    {
        let disabled_fill = color(tokens(Mode::Light, Contrast::Standard).color.state.disabled_fill);
        let (_, shapes, texture) = run(false);
        assert!(
            shapes.iter().any(|s| matches!(s, Shape::Mesh(mesh) if mesh.texture_id == texture && !mesh.is_empty())),
            "the texture paints as a textured mesh"
        );
        let (strokes, shapes, texture) = run(true);
        assert!(!shapes.iter().any(|s| matches!(s, Shape::Mesh(mesh) if mesh.texture_id == texture)), "an inactive part paints no picture");
        assert!(shapes.iter().any(|s| matches!(s, Shape::Rect(rect) if rect.fill == disabled_fill)), "it fills the disabled fill instead");
        assert!(
            strokes.iter().any(|(width, c)| (width - DATA_WIDTH).abs() < EPS && *c == disabled_fill),
            "the data-width stroke takes the disabled fill: {strokes:?}"
        );
    }
}
