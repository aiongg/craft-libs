//! One stroke for theme and data colours and widths: a theme colour at a document width, a dashed data colour, and
//! the content painter's fade.

use craft_behavior::{PartKey, Sense};
use craft_i18n::Direction;
use craft_theme::{Contrast, Mode};
use craft_ui::contract::{Common, Component, Cx, PaintUi, PartsCx, Pointer as _, Props as _, Role, Show as _};
use craft_ui::primitives::{Dashes, DataWidth, PaintShape, StrokeWidth};
use craft_ui::style::{ComponentStyle, DataColor, Ink, StyleCx};
use craft_ui::testing::style::{paint_frame, tokens, with_cx};
use craft_ui::testing::{Frames, trial};
use egui::{Color32, Rect, Response, Shape, Vec2, pos2};

const BODY: PartKey = PartKey::new("body");
const DOC_WIDTH: f32 = 5.0;
const SERIES: Color32 = Color32::from_rgb(200, 30, 90);
const SHARE: f32 = 0.5;
const LINE: [egui::Pos2; 2] = [pos2(0.0, 20.0), pos2(60.0, 20.0)];

/// The width and colour of each solid stroke among `shapes`.
fn strokes(shapes: &[Shape]) -> Vec<(f32, Color32)> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::LineSegment { stroke, .. } => Some((stroke.width, stroke.color)),
            Shape::Path(path) => match path.stroke.color {
                egui::epaint::ColorMode::Solid(color) => Some((path.stroke.width, color)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[trial]
fn a_theme_colour_strokes_at_a_document_width() {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let theme =
        with_cx(&frames.ctx, &tokens, craft_theme::Variant::SECONDARY, craft_theme::ControlSize::Md, |cx| cx.color(tokens.color.accent.solid));
    let (shapes, _) = paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| {
        draw.stroke(&PaintShape::Path(LINE.to_vec()), DataWidth::points(DOC_WIDTH), theme);
    });
    let accent = Color32::from_rgba_unmultiplied(
        tokens.color.accent.solid.0[0],
        tokens.color.accent.solid.0[1],
        tokens.color.accent.solid.0[2],
        tokens.color.accent.solid.0[3],
    );
    assert!(
        strokes(&shapes).iter().any(|(width, color)| (width - DOC_WIDTH).abs() < 1e-3 && *color == accent),
        "the theme colour strokes at the document's width: {:?}",
        strokes(&shapes)
    );
}

#[derive(ComponentStyle, Clone, Debug, Default, PartialEq)]
struct SwatchLook {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

/// A content painter that fills a rect plainly and again inside a fade.
#[derive(craft_ui::contract::Component)]
#[component(content_painter)]
struct Preview {
    #[common]
    common: Common,
}

impl Component for Preview {
    const ROLE: Role = Role::GROUP;
    type Style = SwatchLook;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PreviewStyleKey) -> SwatchLook {
        SwatchLook { metrics: craft_behavior::Metrics::new().min_size(Vec2::new(60.0, 20.0)) }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(BODY, rect);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rect = cx.part(BODY).rect;
        let painter = ui.content_painter(cx);
        painter.painter.rect_filled(rect, 0.0, SERIES);
        painter.faded(SHARE, |faded| {
            faded.painter.rect_filled(rect, 0.0, SERIES);
        });
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

