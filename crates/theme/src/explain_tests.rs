#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use ColorToken as T;

fn appearance(mode: Mode, contrast: Contrast, size: Size) -> Appearance {
    Appearance { mode, contrast, size, text_size: TextSize::Default, reduced_motion: false }
}

fn graphite() -> ThemeDef {
    builtins().iter().find(|t| t.id.as_str() == "craft.graphite").unwrap().clone()
}

/// Graphite without its own pins, so every pin in a test is the test's.
fn unpinned() -> ThemeDef {
    let mut def = graphite();
    def.pins = ModePins::default();
    def.high_contrast = ModePins::default();
    def.apps.clear();
    def
}

fn field(path: &str) -> ComponentField {
    ComponentField::from_path(path).unwrap()
}

#[test]
fn explained_resolution_is_resolve() {
    for theme in builtins() {
        for mode in [Mode::Light, Mode::Dark] {
            for contrast in [Contrast::Standard, Contrast::High] {
                for size in [Size::Compact, Size::Comfortable] {
                    let a = appearance(mode, contrast, size);
                    let e = explain(theme, a, None);
                    assert_eq!(e.resolved, resolve(theme, a, None), "{} {a:?}", theme.id);
                    assert_eq!(e.colors.iter().map(|o| o.token).collect::<Vec<_>>(), ColorToken::ALL);
                    for o in &e.colors {
                        assert_eq!(o.value, e.resolved.tokens.color(o.token), "{} {a:?} {}", theme.id, o.token.path());
                    }
                }
            }
        }
    }
}

#[test]
fn pins_report_their_layer() {
    let photo = AppId::parse("photocraft").unwrap();
    let grey = Rgba::rgb(0x40, 0x40, 0x40);
    let mut def = unpinned();
    def.pins.dark = Pins::from_iter([(T::LineDivider, grey), (T::SurfaceRaised, grey)]);
    def.pins.dark.0.insert(TokenRef::Component(field("button.ghost.border.rest")), grey);
    def.high_contrast.dark = Pins::from_iter([(T::LineBorder, grey)]);
    def.apps.insert(
        photo.clone(),
        AppOverrides {
            pins: ModePins { light: Pins::default(), dark: Pins::from_iter([(T::SurfaceRaised, Rgba::rgb(0x30, 0x30, 0x30))]) },
            high_contrast: ModePins { light: Pins::default(), dark: Pins::from_iter([(T::LineControlEdge, grey)]) },
            components: Default::default(),
        },
    );
    let pin = |e: &Explained, t: ColorToken| e.color(t).unwrap().pin;

    let standard = explain(&def, appearance(Mode::Dark, Contrast::Standard, Size::Compact), None);
    assert_eq!(pin(&standard, T::LineDivider), Some(PinLayer::Theme));
    assert_eq!(pin(&standard, T::SurfaceRaised), Some(PinLayer::Theme));
    assert_eq!(pin(&standard, T::LineBorder), None, "high-contrast pins apply only in high contrast");
    assert_eq!(pin(&standard, T::TextNormal), None);
    assert_eq!(standard.field(field("button.ghost.border.rest")).unwrap().pin, Some(PinLayer::Theme));
    assert_eq!(standard.field(field("button.ghost.border.hover")).unwrap().pin, None);

    let for_app = explain(&def, appearance(Mode::Dark, Contrast::Standard, Size::Compact), Some(&photo));
    assert_eq!(pin(&for_app, T::SurfaceRaised), Some(PinLayer::App));
    assert_eq!(pin(&for_app, T::LineDivider), Some(PinLayer::Theme));

    let high = explain(&def, appearance(Mode::Dark, Contrast::High, Size::Compact), Some(&photo));
    assert_eq!(pin(&high, T::LineBorder), Some(PinLayer::ThemeHighContrast));
    assert_eq!(pin(&high, T::LineControlEdge), Some(PinLayer::AppHighContrast));
    assert_eq!(pin(&high, T::SurfaceRaised), Some(PinLayer::App));
}

#[test]
fn contrast_moves_report_the_value_before_the_pass() {
    let e = explain(&graphite(), appearance(Mode::Dark, Contrast::High, Size::Compact), None);
    let faint = e.color(T::TextFaint).unwrap();
    assert_eq!((faint.source, faint.pin), (ColorSource::Neutral(10), None));
    assert_ne!(faint.derived, faint.value);
    assert_eq!(faint.contrast_move, Some(faint.derived));
    let pairs: Vec<_> = e.pairs_of(TokenRef::Color(T::TextFaint)).collect();
    assert!(!pairs.is_empty() && pairs.iter().all(|p| p.fixed_by_pass && p.passes()));
    // `caption.inactive` follows `text.faint`, so it moved with it.
    let inactive = e.color(T::CaptionInactive).unwrap();
    assert_eq!((inactive.value, inactive.contrast_move), (faint.value, faint.contrast_move));
    assert_eq!(e.color(T::SurfacePanel).unwrap().contrast_move, None);
}

#[test]
fn component_fields_read_their_semantic_token() {
    let e = explain(&graphite(), appearance(Mode::Light, Contrast::Standard, Size::Compact), None);
    let reads = |path: &str| e.field(field(path)).unwrap().reads;
    assert_eq!(reads("button.primary.fill.rest"), Some(T::AccentSolid));
    assert_eq!(reads("button.primary.text.rest"), Some(T::TextOnAccent));
    assert_eq!(reads("button.secondary.border.rest"), Some(T::LineBorder));
    assert_eq!(reads("button.ghost.fill.rest"), None, "transparent is a constant");
    assert!(e.field(field("button.radius")).is_none(), "only colour fields are listed");
}

#[test]
fn every_warning_is_a_failing_pair_with_its_ratio() {
    let a = appearance(Mode::Dark, Contrast::Standard, Size::Compact);
    let mut def = unpinned();
    let panel = resolve(&def, a, None).tokens.color(T::SurfacePanel);
    def.pins.dark = Pins::from_iter([(TokenRef::Color(T::TextNormal), panel)]);
    def.pins.dark.0.insert(TokenRef::Component(field("button.primary.fill.rest")), Rgba::rgb(0x80, 0x80, 0x80));
    def.pins.dark.0.insert(TokenRef::Component(field("button.primary.text.rest")), Rgba::rgb(0x88, 0x88, 0x88));

    let mut cases: Vec<(ThemeDef, Appearance)> = vec![(def, a)];
    for theme in builtins() {
        for mode in [Mode::Light, Mode::Dark] {
            for contrast in [Contrast::Standard, Contrast::High] {
                cases.push((theme.clone(), appearance(mode, contrast, Size::Compact)));
            }
        }
    }
    let (mut semantic, mut component) = (0, 0);
    for (theme, a) in &cases {
        let e = explain(theme, *a, None);
        for w in &e.resolved.warnings {
            let found = e.pairs.iter().find(|p| p.fg == w.fg && p.bg == w.bg).unwrap_or_else(|| panic!("{} {a:?}: {w:?} not in pairs", theme.id));
            assert_eq!((found.ratio, found.minimum), (w.ratio, w.minimum));
            assert!(!found.passes());
            match w.fg {
                TokenRef::Color(_) => semantic += 1,
                TokenRef::Component(_) => component += 1,
            }
        }
    }
    assert!(semantic > 0 && component > 0, "the pinned theme has warnings of both layers");
}
