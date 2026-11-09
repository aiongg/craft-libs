//! `Draw::faded`: what the faded `Draw` paints is less opaque, what its parent paints is not.

use craft_i18n::Direction;
use craft_theme::{Contrast, Mode};
use craft_ui::primitives::{Draw, PaintShape, TextRun};
use craft_ui::style::{DataColor, Ink};
use craft_ui::testing::style::{paint_frame, tokens};
use craft_ui::testing::{Frames, trial};
use egui::{Color32, CornerRadius, Rect, Shape, pos2};

const SHARE: f32 = 0.5;
const RECT: Rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(90.0, 40.0));

/// A data fill and a text run.
fn content(draw: &Draw<'_>) {
    draw.data_fill(&PaintShape::Rect(RECT, CornerRadius::ZERO), DataColor::new(Color32::from_rgb(200, 30, 90)));
    draw.text(RECT, &TextRun::new("Label"));
}

/// The alpha of each filled rect and the alpha of each text run's colour `paint` leaves, in paint order.
fn painted(paint: impl Fn(&Draw<'_>)) -> (Vec<u8>, Vec<u8>) {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let (shapes, _) = paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| paint(draw));
    let fills = shapes.iter().filter_map(|s| if let Shape::Rect(r) = s { Some(r.fill.a()) } else { None }).collect();
    let texts = shapes.iter().filter_map(|s| if let Shape::Text(t) = s { Some(t.fallback_color.a()) } else { None }).collect();
    (fills, texts)
}

