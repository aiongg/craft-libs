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

/// The alpha of each filled rect and the opacity factor of each text run `paint` leaves, in paint order.
fn painted(paint: impl Fn(&Draw<'_>)) -> (Vec<u8>, Vec<f32>) {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let (shapes, _) = paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| paint(draw));
    let fills = shapes.iter().filter_map(|s| if let Shape::Rect(r) = s { Some(r.fill.a()) } else { None }).collect();
    let texts = shapes.iter().filter_map(|s| if let Shape::Text(t) = s { Some(t.opacity_factor) } else { None }).collect();
    (fills, texts)
}

#[trial]
fn a_faded_draw_paints_fills_and_text_less_opaque_and_its_parent_does_not() {
    let (fills, texts) = painted(|draw| {
        draw.faded(SHARE, content);
        content(draw);
    });
    assert_eq!((fills.len(), texts.len()), (2, 2), "each Draw painted a fill and a text run");
    assert!(fills[0] < fills[1], "the faded fill is less opaque than the parent's: {fills:?}");
    assert!(texts[0] < texts[1], "the faded text run is less opaque than the parent's: {texts:?}");
}

#[trial]
fn fades_multiply_and_a_zero_share_paints_nothing() {
    let (once, _) = painted(|draw| draw.faded(SHARE, content));
    let (twice, _) = painted(|draw| draw.faded(SHARE, |faded| faded.faded(SHARE, content)));
    assert!(twice[0] < once[0], "a fade inside a fade multiplies the shares: {twice:?} against {once:?}");
    let (none, _) = painted(|draw| draw.faded(0.0, content));
    assert!(none.iter().all(|alpha| *alpha == 0), "nothing is visible at share 0: {none:?}");
}
