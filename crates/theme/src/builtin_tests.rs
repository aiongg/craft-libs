#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use super::builtins::{SOURCES, suite_default};
use super::format;
use super::semantic_tests::appearances;
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

#[test]
fn builtins_resolve_everywhere_without_warnings_beyond_their_intent() {
    for theme in builtins() {
        for a in appearances() {
            let r = resolve(theme, a, None);
            let high = a.contrast == Contrast::High;
            if high || theme.contrast_intent == ContrastIntent::Standard {
                let reported: Vec<_> = r.warnings.iter().map(|w| (w.fg.path(), w.bg.path())).collect();
                assert_eq!(reported, known_warnings(theme, a), "{} {a:?}: {:?}", theme.id, r.warnings);
            }
        }
    }
}

/// The warnings a built-in is known to have. Adwaita dark pins libadwaita's accent (#3584e4) and a
/// sunken surface (#404044) 2.74:1 apart, below the accent's 3:1 on the tracks of sliders and
/// progress bars; libadwaita's own dark troughs (15 % of white over the window or view) are 2.6
/// to 2.8:1 from that accent, so no pin true to GNOME passes. High contrast moves them apart.
fn known_warnings(theme: &ThemeDef, a: Appearance) -> Vec<(String, String)> {
    if theme.id.as_str() != "craft.adwaita" || a.mode != Mode::Dark || a.contrast != Contrast::Standard {
        return Vec::new();
    }
    let pair = |fg: &str, bg: &str| (fg.to_owned(), bg.to_owned());
    vec![pair("accent.solid", "surface.sunken"), pair("progress.fill", "progress.track"), pair("slider.fill.rest", "slider.track")]
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

/// The text, placeholder, unit, scrub label and selected text read on their backgrounds and the
/// focused and invalid edges show on the field's fill, in every built-in and appearance, pins
/// included.
#[test]
fn builtin_field_text_and_state_edges_meet_their_minimums() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Field(_)))).collect();
            assert_eq!(pairs.len(), 7, "{} {a:?}", theme.id);
            for p in pairs {
                assert!(p.passes(), "{} {a:?}: {} on {} is {:.2}, needs {}", theme.id, p.fg.path(), p.bg.path(), p.ratio, p.minimum);
            }
        }
    }
}

/// The text of a popup list's rows reads on the popup surface and on the active fill, and the check
/// mark shows on both, in every built-in and appearance, pins included.
#[test]
fn builtin_listbox_text_and_check_mark_meet_their_minimums() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Listbox(_)))).collect();
            assert_eq!(pairs.len(), 6, "{} {a:?}", theme.id);
            for p in pairs {
                assert!(p.passes(), "{} {a:?}: {} on {} is {:.2}, needs {}", theme.id, p.fg.path(), p.bg.path(), p.ratio, p.minimum);
            }
        }
    }
}

/// The active menu row's text reads on the accent fill and the shortcuts on the popup surface, in
/// every built-in and appearance, pins included.
#[test]
fn builtin_menu_text_meets_its_minimum() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Menu(_)))).collect();
            assert_eq!(pairs.len(), 2, "{} {a:?}", theme.id);
            for p in pairs {
                assert!(p.passes(), "{} {a:?}: {} on {} is {:.2}, needs {}", theme.id, p.fg.path(), p.bg.path(), p.ratio, p.minimum);
            }
        }
    }
}

/// A list row's labels, dimmed labels and detail lines read on the panel and on the fills a row
/// takes (a hidden layer that is selected), the ghost's label on its fill, and the drop marker on
/// its halo, in every built-in and appearance, pins included.
#[test]
fn builtin_list_text_and_drop_marker_meet_their_minimums() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::List(_)))).collect();
            assert_eq!(pairs.len(), 12, "{} {a:?}", theme.id);
            for p in pairs {
                assert!(p.passes(), "{} {a:?}: {} on {} is {:.2}, needs {}", theme.id, p.fg.path(), p.bg.path(), p.ratio, p.minimum);
            }
        }
    }
}

/// `text.on_strong` is white and reads on `accent.strong` in every built-in and appearance;
/// `accent.strong` is `accent.solid` where white already reads on it, and otherwise `accent.solid`
/// with only its lightness moved (Pro's dark #378EF0 deepens, Graphite's light accent stays).
#[test]
fn builtin_accent_strong_carries_white_text() {
    for theme in builtins() {
        for a in appearances() {
            let t = resolve(theme, a, None).tokens;
            let min = if a.contrast == Contrast::High { 7.0 } else { 4.5 };
            let (solid, strong, text) = (t.color(ColorToken::AccentSolid), t.color(ColorToken::AccentStrong), t.color(ColorToken::TextOnStrong));
            let at = format!("{} {a:?}", theme.id);
            assert_eq!(text, Rgba::WHITE, "{at}");
            assert!(contrast_ratio(text, strong) >= min, "{at}: {:.2}", contrast_ratio(text, strong));
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

/// The marks and outlines of check boxes, radio buttons and switches identify the control and its
/// state, so every built-in meets their non-text minimum at rest in every appearance, pins
/// included; a disabled one that is on doesn't look off: its box or track differs from an off
/// one's, and its mark shows on it (off has no mark).
#[test]
fn builtin_check_marks_and_outlines_meet_the_non_text_minimum() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Check(_)))).collect();
            assert_eq!(pairs.len(), 5, "{} {a:?}", theme.id);
            for p in pairs {
                assert!(p.passes(), "{} {a:?}: {} on {} is {:.2}, needs {}", theme.id, p.fg.path(), p.bg.path(), p.ratio, p.minimum);
            }
            let c = resolve(theme, a, None).tokens.component.check;
            assert_ne!(c.on.fill.disabled, c.off.fill.disabled, "{} {a:?}: a disabled fill that is on looks off", theme.id);
            assert_ne!(c.on.mark.disabled, c.on.fill.disabled, "{} {a:?}: a disabled mark doesn't show", theme.id);
        }
    }
}

/// The labels of badges, the keys on keycaps, the progress fill, the swatch selection ring and the
/// mixed dash meet their minimums in every built-in and appearance, pins included, except where a
/// built-in has a [known warning](known_warnings).
#[test]
fn builtin_small_control_pairs_meet_their_minimums() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let small = |p: &&ContrastCheck| {
                matches!(
                    p.fg,
                    TokenRef::Component(ComponentField::Badge(_) | ComponentField::Kbd(_) | ComponentField::Progress(_) | ComponentField::Swatch(_))
                )
            };
            let pairs: Vec<_> = steps.component_pairs.iter().filter(small).collect();
            assert_eq!(pairs.len(), 12 + 1 + 1 + 2, "{} {a:?}", theme.id);
            let known = known_warnings(theme, a);
            for p in pairs {
                let warned = known.contains(&(p.fg.path(), p.bg.path()));
                assert!(warned || p.passes(), "{} {a:?}: {} on {} is {:.2}, needs {}", theme.id, p.fg.path(), p.bg.path(), p.ratio, p.minimum);
            }
        }
    }
}

/// Knobs are light in both modes: `surface.knob` is near white (dark mode: about 236/255) and
/// takes a tinted neutral's tint; a light knob has a `line.control_edge` outline and a dark-mode
/// one none, and both cast the knob shadow. Classic draws its knob as a bevelled panel-grey
/// surface with no shadow.
#[test]
fn knobs_are_near_white_in_both_modes_and_classic_bevels_a_panel_grey_knob() {
    let at = |theme: &ThemeDef, mode| resolve(theme, Appearance { mode, ..semantic_tests::app(Mode::Light, Contrast::Standard) }, None).tokens;
    for mode in [Mode::Light, Mode::Dark] {
        let t = at(suite_default(), mode);
        let knob = t.color(ColorToken::SurfaceKnob);
        let lightness = knob.to_oklch().l;
        assert!(lightness >= 0.93, "{mode:?}: surface.knob {} has lightness {lightness:.3}", knob.to_hex());
        let edge = t.color(ColorToken::LineKnobEdge);
        match mode {
            Mode::Light => assert_eq!(edge, t.color(ColorToken::LineControlEdge)),
            Mode::Dark => assert_eq!(edge.a(), 0, "dark knobs have no outline"),
        }
        assert_eq!(t.elevation.knob.offset, Elevation::level(values::KNOB_ELEVATION).offset, "{mode:?}");
        assert!(t.elevation.knob.alpha > 0.0, "{mode:?}");

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

/// `radius.pill` is fully round unless the theme sets it; Classic squares it.
#[test]
fn pill_radius_is_full_by_default_and_zero_in_classic() {
    let at = |id: &str| {
        let theme = builtins().iter().find(|t| t.id.as_str() == id).unwrap().clone();
        resolve(&theme, semantic_tests::app(Mode::Light, Contrast::Standard), None).tokens
    };
    assert_eq!(at("craft.solarized-dark").shape.radius.pill, Radius::Full.px());
    let t = at("craft.classic");
    assert_eq!(t.shape.radius.pill, 0.0);
    assert_eq!([t.component.badge.radius, t.component.progress.radius, t.component.slider.radius], [0.0; 3]);
}

/// The edited curve and its points are the cue that shows a curve editor's state: the channel
/// colours must reach 3:1 on the plot (`field.fill`) in every built-in, mode and contrast.
#[test]
fn data_channel_colours_reach_three_to_one_on_the_field_fill() {
    for theme in builtins() {
        for a in appearances() {
            let t = resolve(theme, a, None).tokens;
            let fill = t.component.field.fill.rest;
            let d = t.color.data;
            for (name, c) in [("red", d.red), ("green", d.green), ("blue", d.blue), ("cyan", d.cyan), ("magenta", d.magenta), ("yellow", d.yellow)] {
                let ratio = contrast_ratio(c, fill);
                assert!(ratio >= 3.0, "{} {a:?}: data.{name} {} on field.fill {} is {ratio:.2}", theme.id, c.to_hex(), fill.to_hex());
            }
        }
    }
}

/// `data.scope` is dark in every theme: #282828, and the values PhotoCraft pins for Studio dark
/// and Solarized.
#[test]
fn data_scope_is_dark_and_pinned_by_studio_and_solarized() {
    let scope = |id: &str, mode: Mode| {
        let theme = builtins().iter().find(|t| t.id.as_str() == id).unwrap().clone();
        resolve(&theme, semantic_tests::app(mode, Contrast::Standard), None).tokens.color.data.scope.to_hex()
    };
    assert_eq!(scope("craft.pro", Mode::Light), "#282828");
    assert_eq!(scope("craft.pro", Mode::Dark), "#282828");
    assert_eq!(scope("craft.studio", Mode::Dark), "#0e0e0f");
    assert_eq!(scope("craft.solarized-dark", Mode::Dark), "#002b36");
}
