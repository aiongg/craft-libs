//! The canvas surround: a theme's pattern draws the dot texture over the fill; a theme without one, or the preference
//! turned off, draws the fill alone and makes no texture (#4542).

use craft_theme::{Contrast, Mode, Rgba, Size};
use craft_ui::primitives::{Canvas, CheckerArea, FrameServices, paint_surround};
use craft_ui::style::{SurroundLook, UiTokens};
use craft_ui::testing::trial;
use craft_ui_rt::{ImageStore, Item};
use egui::{CornerRadius, Pos2, Rect, pos2, vec2};
use std::sync::Arc;

/// `base` with its canvas pattern set to `pattern`, or to the canvas fill for none.
fn with_pattern(base: &UiTokens, pattern: Option<Rgba>) -> UiTokens {
    let mut tokens = (**base.tokens()).clone();
    tokens.color.surface.canvas_pattern = pattern.unwrap_or(tokens.color.surface.canvas);
    UiTokens::new(Arc::new(tokens), Contrast::Standard, Size::Compact)
}

#[trial]
fn only_a_theme_pattern_draws_the_canvas_surround_texture() {
    let base = craft_ui::testing::style::tokens(Mode::Dark, Contrast::Standard);
    let plain = with_pattern(&base, None);
    let dot = Rgba([200, 40, 40, 255]);
    assert_ne!(dot, base.color.surface.canvas, "the pattern differs from the fill");
    let dotted = with_pattern(&base, Some(dot));

    let area = Rect::from_min_size(pos2(40.0, 30.0), vec2(200.0, 120.0));
    let painted = |look: SurroundLook| -> (Vec<Item>, usize) {
        let mut store = ImageStore::default();
        let services = FrameServices::new(None, &mut store, 1.0);
        let canvas = Canvas::new(&services, Pos2::ZERO, Rect::EVERYTHING);
        paint_surround(&canvas, CheckerArea::Rect(area, CornerRadius::ZERO), &look);
        let items = canvas.finish().0.items;
        drop(services);
        (items, store.changes().len())
    };

    let cases = [
        ("a theme whose pattern is its fill", SurroundLook::of(&plain), false),
        ("the same theme with the preference on", SurroundLook::of(&plain).shown(true), false),
        ("a theme with a pattern", SurroundLook::of(&dotted), true),
        ("a theme with a pattern and the preference on", SurroundLook::of(&dotted).shown(true), true),
        ("a theme with a pattern and the preference off", SurroundLook::of(&dotted).shown(false), false),
    ];
    for (case, look, textured) in cases {
        assert_eq!(look.pattern.is_some(), textured, "{case}: the look's pattern");
        let (items, images) = painted(look);
        let fills = items.iter().filter(|item| matches!(item, Item::Rect { rect, .. } if *rect == area)).count();
        let dots = items.iter().filter(|item| matches!(item, Item::Image { rect, .. } if *rect == area)).count();
        assert_eq!((fills, dots), (1, usize::from(textured)), "{case}: one fill, and the dot texture only with a pattern");
        assert_eq!(images, usize::from(textured), "{case}: the dot texture is made only with a pattern");
    }
}
