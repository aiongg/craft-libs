#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use super::builtins::{SOURCES, suite_default};
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

/// ArtCraft dark is PhotoCraft's ProMedium in Standard contrast, including the greys and the
/// accent whose pairs miss the minimums; those pairs are reported.
#[test]
fn artcraft_dark_standard_reports_the_pairs_its_pinned_values_miss() {
    let theme = builtins().iter().find(|t| t.id.as_str() == "craft.artcraft").unwrap();
    let r = resolve(theme, semantic_tests::app(Mode::Dark, Contrast::Standard), None);
    assert!(r.warnings.iter().any(|w| w.fg == ColorToken::TextFaint.into() && w.bg == ColorToken::StateHover.into()));
    assert!(r.warnings.iter().any(|w| w.fg == ColorToken::AccentSolid.into() && w.bg == ColorToken::SurfacePanel.into()));
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

/// Soft keeps its own rounder corners at every Size (7/10/14 at Comfortable, 7 on buttons); the other built-ins
/// that set no offset take the Size preset's radii.
#[test]
fn soft_offsets_the_size_preset_radii_and_others_keep_them() {
    let radii = |theme: &ThemeDef, size| {
        let a = Appearance { size, ..appearances()[0] };
        let r = resolve(theme, a, None).tokens.shape.radius;
        (r.control, r.popup, r.window, r.panel, r.button, r.pill)
    };
    let full = scale::Radius::Full.px();
    let soft = builtins().iter().find(|t| t.id.as_str() == "craft.soft").unwrap();
    let pro = builtins().iter().find(|t| t.id.as_str() == "craft.pro").unwrap();
    for size in [Size::Comfortable, Size::Compact] {
        let (s, p) = (radii(soft, size), radii(pro, size));
        assert!(s.0 > p.0 && s.1 > p.1 && s.2 > p.2, "{size:?}: soft {s:?} rounds more than the preset {p:?}");
        assert_eq!(s.5, full);
    }
}

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

/// Knobs are light in both modes: `surface.knob` is near white (dark mode: about 236/255) and
/// takes a tinted neutral's tint; a light knob has a `text.muted` outline and a dark-mode
/// one none (the Pro themes pin PhotoCraft's #282828 ring), and both cast the knob shadow. Classic draws its knob as a bevelled panel-grey
/// surface with no shadow.
#[test]
fn knobs_are_near_white_in_both_modes_and_classic_bevels_a_panel_grey_knob() {
    let at = |theme: &ThemeDef, mode| resolve(theme, Appearance { mode, ..semantic_tests::app(Mode::Light, Contrast::Standard) }, None).tokens;
    for mode in [Mode::Light, Mode::Dark] {
        let t = at(suite_default(), mode);
        let knob = t.color(ColorToken::SurfaceKnob);
        let lightness = knob.to_oklch().l;
        let track = t.color(ColorToken::SurfaceSunken).to_oklch().l;
        assert!(lightness > track, "{mode:?}: surface.knob {} (lightness {lightness:.3}) stands out from the track", knob.to_hex());
        let plain = at(&semantic_tests::grey_blue(), mode).color(ColorToken::LineKnobEdge);
        match mode {
            Mode::Light => {
                assert_eq!(plain, at(&semantic_tests::grey_blue(), mode).color(ColorToken::TextMuted), "light knobs take the muted text colour")
            }
            Mode::Dark => assert_eq!(plain.a(), 0, "dark knobs have no outline"),
        }
        assert_eq!(t.elevation.knob, values::KNOB_SHADOW, "{mode:?}");

        let tinted = at(&semantic_tests::grey_blue(), mode).color(ColorToken::SurfaceKnob).to_oklch();
        assert!(tinted.c > 0.001 && (240.0..300.0).contains(&tinted.h), "{mode:?}: {tinted:?} takes the blue-grey neutral's tint");
    }

    let classic = builtins().iter().find(|t| t.id.as_str() == "craft.classic").unwrap();
    let t = at(classic, Mode::Light);
    assert_eq!(t.color(ColorToken::SurfaceKnob), t.color(ColorToken::SurfacePanel));
    assert_eq!(t.elevation.knob, Elevation::level(0));
    assert_eq!(TokenRef::from_path("surface.knob"), Some(TokenRef::Color(ColorToken::SurfaceKnob)));
    assert_eq!(TokenRef::from_path("line.knob_edge"), Some(TokenRef::Color(ColorToken::LineKnobEdge)));
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

/// Bare canvas lines are the accent itself, as PhotoCraft draws them; `overlay.stroke`, drawn over
/// the halo, is lightened to read on it.
#[test]
fn builtin_overlay_line_is_the_plain_accent() {
    let theme = builtins().iter().find(|t| t.id.as_str() == "craft.pro").unwrap();
    let t = resolve(theme, semantic_tests::app(Mode::Dark, Contrast::Standard), None).tokens;
    assert_eq!(t.color(ColorToken::OverlayLine), t.color(ColorToken::AccentSolid));
    assert_ne!(t.color(ColorToken::OverlayStroke), t.color(ColorToken::OverlayLine));
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

/// Classic casts no shadow (PhotoCraft `theme.rs:415-455`, shadow alpha 0): `surface.shadow` is
/// clear, so no popup, tooltip, toast, dialog, floating window or knob has one. Every other
/// theme's shadows are its `surface.shadow` alpha.
#[test]
fn every_shadow_has_the_alpha_of_surface_shadow() {
    for theme in builtins() {
        for size in [Size::Compact, Size::Comfortable] {
            for mode in [Mode::Light, Mode::Dark] {
                let a = Appearance { size, ..semantic_tests::app(mode, Contrast::Standard) };
                let t = resolve(theme, a, None).tokens;
                let id = theme.id.as_str();
                let alpha = t.color(ColorToken::SurfaceShadow).a();
                let e = &t.elevation;
                for (what, got) in [("popup", e.popup.alpha), ("floating", e.floating.alpha)] {
                    assert_eq!((got * 255.0).round() as u8, alpha, "{id} {what} {a:?}");
                }
                assert_eq!((e.popup.offset, e.popup.blur), (values::POPUP_SHADOW.offset, values::POPUP_SHADOW.blur), "{id}");
                assert_eq!((e.floating.offset, e.floating.blur), (values::WINDOW_SHADOW.offset, values::WINDOW_SHADOW.blur), "{id}");
                if id == "craft.classic" {
                    assert_eq!(alpha, 0, "{a:?}");
                    assert_eq!(e.knob.alpha, 0.0, "{a:?}");
                }
            }
        }
    }
}

/// `surface.chrome` is PhotoCraft's `chrome`: the panel's colour where PhotoCraft's chrome equals
/// its card, pinned to its own value in the Studio, Adwaita dark, Breeze, and Soft themes.
#[test]
fn chrome_is_the_panel_except_where_photocraft_pins_its_own() {
    for theme in builtins() {
        for mode in [Mode::Light, Mode::Dark] {
            if (mode == Mode::Dark && theme.variants == Variants::LightOnly) || (mode == Mode::Light && theme.variants == Variants::DarkOnly) {
                continue;
            }
            let t = resolve(theme, semantic_tests::app(mode, Contrast::Standard), None).tokens;
            let chrome = t.color(ColorToken::SurfaceChrome);
            let pins = if mode == Mode::Dark { &theme.pins.dark } else { &theme.pins.light };
            match pins.get(ColorToken::SurfaceChrome) {
                Some(pinned) => assert_eq!(chrome, pinned, "{} {mode:?}", theme.id),
                None => assert_eq!(chrome, t.color(ColorToken::SurfacePanel), "{} {mode:?}", theme.id),
            }
            assert_eq!(TokenRef::from_path("surface.chrome"), Some(TokenRef::Color(ColorToken::SurfaceChrome)));
        }
    }
}

/// Classic's slider thumb is a 10 x 16 box at the standard size; the small size keeps its step
/// under it. The other built-ins' thumbs are round, the icon size less 4 (2 small).
#[test]
fn slider_thumbs_are_round_except_in_classic() {
    for theme in builtins() {
        for size in [Size::Compact, Size::Comfortable] {
            let a = Appearance { size, ..appearances()[0] };
            let t = resolve(theme, a, None).tokens;
            let s = t.component.slider;
            if theme.id.as_str() == "craft.classic" {
                assert!(s.thumb_width.md < s.thumb_height.md, "{size:?}: classic's thumb is a box, taller than wide");
            } else {
                assert_eq!(s.thumb_height, s.thumb_width, "{} {size:?}", theme.id);
                assert!(
                    s.thumb_height.md < t.size.icon.md && s.thumb_height.sm < t.size.icon.sm,
                    "{} {size:?}: the thumb sits inside the icon size",
                    theme.id
                );
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
