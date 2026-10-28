#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use super::builtins::SOURCES;
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

/// The text, placeholder and selected text read on their backgrounds and the focused and invalid
/// edges show on the field's fill, in every built-in and appearance, pins included.
#[test]
fn builtin_field_text_and_state_edges_meet_their_minimums() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Field(_)))).collect();
            assert_eq!(pairs.len(), 5, "{} {a:?}", theme.id);
            for p in pairs {
                assert!(p.passes(), "{} {a:?}: {} on {} is {:.2}, needs {}", theme.id, p.fg.path(), p.bg.path(), p.ratio, p.minimum);
            }
        }
    }
}

/// The marks and outlines of check boxes, radio buttons and switches identify the control and its
/// state, so every built-in meets their non-text minimum at rest in every appearance, pins
/// included; a disabled one that is on doesn't look off.
#[test]
fn builtin_check_marks_and_outlines_meet_the_non_text_minimum() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Check(_)))).collect();
            assert_eq!(pairs.len(), 6, "{} {a:?}", theme.id);
            for p in pairs {
                assert!(p.passes(), "{} {a:?}: {} on {} is {:.2}, needs {}", theme.id, p.fg.path(), p.bg.path(), p.ratio, p.minimum);
            }
            let c = resolve(theme, a, None).tokens.component.check;
            for part in [CheckPart::Fill, CheckPart::Mark] {
                assert_ne!(
                    c.on.part(part).disabled,
                    c.off.part(part).disabled,
                    "{} {a:?}: a disabled {} that is on looks off",
                    theme.id,
                    part.name()
                );
            }
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
