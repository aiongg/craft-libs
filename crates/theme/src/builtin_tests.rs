#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use super::builtins::SOURCES;
use super::format;
use super::semantic_tests::{appearances, resolve};
use super::*;

#[test]
fn builtins_parse_and_round_trip_to_their_files() {
    let themes = builtins();
    assert_eq!(themes.len(), SOURCES.len());
    for ((name, text), theme) in SOURCES.iter().zip(themes) {
        let file = format::parse(text.as_bytes()).unwrap();
        assert_eq!(&file.theme, theme, "{name}");
        assert_eq!(&format::write(&file).unwrap(), text, "{name}.json is not in the writer's form");
        assert_eq!(theme.id.as_str(), format!("craft.{name}"));
    }
    let ids: BTreeSet<_> = themes.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids.len(), themes.len(), "ids are unique");
}

/// High contrast meets every minimum in every built-in. Standard contrast keeps PhotoCraft's pinned
/// values and reports the pairs they miss as warnings; those show in the token dumps
/// (`tests/dumps/tokens`), where a changed pair shows in review.
#[test]
fn builtins_resolve_in_high_contrast_without_warnings() {
    for theme in builtins() {
        for a in appearances().into_iter().filter(|a| a.contrast == Contrast::High) {
            let r = resolve(theme, a, None);
            assert!(r.warnings.is_empty(), "{} {a:?}: {:?}", theme.id, r.warnings);
        }
    }
}

/// Links and other accent-coloured text must read as accent, not as body text: a neutral
/// `accent.text` (PhotoCraft's `accent_text`, the content colour on `accent_soft`, is white in
/// several dark themes) draws hyperlinks like plain text. High contrast is left out: 7:1 on a
/// mid-grey surface leaves room only for near-white text.
#[test]
fn builtin_accent_text_is_an_accent_hue() {
    let mut low = Vec::new();
    for theme in builtins() {
        for a in appearances().into_iter().filter(|a| a.contrast == Contrast::Standard) {
            let t = resolve(theme, a, None).tokens;
            let chroma = t.color(ColorToken::AccentText).to_oklch().c;
            if chroma < ACCENT_TEXT_MIN_CHROMA {
                low.push(format!("{} {a:?}: {chroma:.3}", theme.id));
            }
        }
    }
    assert!(low.is_empty(), "accent.text is near grey:\n{}", low.join("\n"));
}

/// OKLCH chroma below which a colour reads as grey.
const ACCENT_TEXT_MIN_CHROMA: f64 = 0.04;

/// `text.on_strong` reads on `accent.strong` in every built-in and appearance and is white unless
/// the theme pins it; `accent.strong` is `accent.solid` where white already reads on it, and
/// otherwise `accent.solid` with only its lightness moved (Pro's dark #378EF0 deepens, Graphite's
/// light accent stays), unless the theme pins it.
#[test]
fn builtin_accent_strong_carries_white_text() {
    for theme in builtins() {
        for a in appearances() {
            let t = resolve(theme, a, None).tokens;
            let min = if a.contrast == Contrast::High { 7.0 } else { 4.5 };
            let (solid, strong, text) = (t.color(ColorToken::AccentSolid), t.color(ColorToken::AccentStrong), t.color(ColorToken::TextOnStrong));
            let at = format!("{} {a:?}", theme.id);
            let pins = if theme.variants.mode_for(a.mode) == Mode::Dark { &theme.pins.dark } else { &theme.pins.light };
            if pins.get(ColorToken::TextOnStrong).is_none() {
                assert_eq!(text, Rgba::WHITE, "{at}");
            }
            assert!(contrast_ratio(text, strong) >= min, "{at}: {:.2}", contrast_ratio(text, strong));
            if pins.get(ColorToken::AccentStrong).is_some() {
                continue;
            }
            if contrast_ratio(text, solid) >= min {
                assert_eq!(strong, solid, "{at}");
            } else {
                let (s, g) = (solid.to_oklch(), strong.to_oklch());
                assert!(g.l < s.l, "{at}: {solid:?} -> {strong:?}");
                assert!((g.h - s.h).abs() < 2.0, "{at}: hue {:.1} -> {:.1}", s.h, g.h);
            }
        }
    }
    let strong = |id: &str, mode| {
        let theme = builtins().iter().find(|t| t.id.as_str() == id).unwrap();
        let t = resolve(theme, semantic_tests::app(mode, Contrast::Standard), None).tokens;
        (t.color(ColorToken::AccentSolid), t.color(ColorToken::AccentStrong))
    };
    let (solid, deeper) = strong("craft.pro", Mode::Dark);
    assert_eq!(solid, Rgba::rgb(0x37, 0x8e, 0xf0));
    assert!(relative_luminance(deeper) < relative_luminance(solid), "{deeper:?}");
    let (solid, same) = strong("craft.graphite", Mode::Light);
    assert_eq!(same, solid);
}

/// A theme whose neutral seed and neutral pins are greys derives every neutral token as a grey:
/// `r == g == b`, with no one-step tint from the contrast pass.
#[test]
fn builtin_greys_from_grey_seeds_stay_neutral() {
    use values::ColorSource as S;
    let is_grey = |c: Rgba| c.0[0] == c.0[1] && c.0[1] == c.0[2];
    let mut checked = 0;
    for theme in builtins() {
        for mode in [Mode::Light, Mode::Dark] {
            let neutral: Vec<ColorToken> = ColorToken::ALL
                .iter()
                .copied()
                .filter(|&t| matches!(values::source(t, mode), S::Neutral(_) | S::Anchored(_) | S::StateVariant { .. } | S::Lighting(_)))
                .collect();
            let seed = theme.mode_seeds.get(mode).neutral.unwrap_or(theme.seeds.neutral);
            let pins = [theme.pins.get(mode), theme.high_contrast.get(mode)];
            if !is_grey(seed) || neutral.iter().any(|&t| pins.iter().any(|p| p.get(t).is_some_and(|c| !is_grey(c)))) {
                continue;
            }
            checked += 1;
            for contrast in [Contrast::Standard, Contrast::High] {
                let a = Appearance { mode, contrast, size: Size::default(), text_size: TextSize::default(), reduced_motion: false };
                let tokens = resolve(theme, a, None).tokens;
                for &t in &neutral {
                    assert!(is_grey(tokens.color(t)), "{} {mode:?} {contrast:?} {}: {:?}", theme.id, t.path(), tokens.color(t));
                }
            }
        }
    }
    assert!(checked >= 4, "ArtCraft and Pro are grey in both modes");
}

/// Slider marks and the badge ring are drawn in `line.mark`, which must be seen on the control
/// surfaces and the hover and pressed fills in every theme, mode and contrast.
#[test]
fn line_mark_holds_the_non_text_minimum_on_the_control_surfaces() {
    let on = [ColorToken::SurfacePanel, ColorToken::SurfaceRaised, ColorToken::SurfaceSunken, ColorToken::StateHover, ColorToken::StatePressed];
    for theme in builtins() {
        for a in appearances() {
            let t = resolve(theme, a, None).tokens;
            let min = if a.contrast == Contrast::High { super::values::NON_TEXT_MIN.1 } else { super::values::NON_TEXT_MIN.0 };
            for bg in on {
                let ratio = contrast_ratio(t.color(ColorToken::LineMark), t.color(bg));
                assert!(ratio >= min, "{} {:?} {:?}: line.mark on {bg:?} is {ratio:.2}:1", theme.id.as_str(), a.mode, a.contrast);
            }
        }
    }
}

/// Adwaita, Breeze, Soft, Solarized and Studio select list rows with the accent tint, and the
/// selected text reads on it in every appearance.
#[test]
fn accent_tinted_themes_select_with_accent_soft() {
    for id in ["craft.adwaita", "craft.breeze", "craft.soft", "craft.solarized-dark", "craft.studio"] {
        let theme = builtins().iter().find(|t| t.id.as_str() == id).unwrap();
        for a in appearances() {
            let t = resolve(theme, a, None).tokens;
            let min = if a.contrast == Contrast::High { 7.0 } else { 4.5 };
            let soft = t.color(ColorToken::AccentSoft);
            if a.contrast != Contrast::High {
                assert_eq!(t.color(ColorToken::StateSelected), soft, "{id} {a:?}");
            }
            let fill = crate::derive::over(t.color(ColorToken::StateSelected), t.color(ColorToken::SurfacePanel));
            assert!(contrast_ratio(t.color(ColorToken::TextOnSelected), fill) >= min, "{id} {a:?}");
        }
    }
}
