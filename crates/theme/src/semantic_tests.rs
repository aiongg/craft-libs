#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::ThemeId;
use ColorToken as T;

pub(super) fn hex(s: &str) -> Rgba {
    Rgba::from_hex(s).unwrap()
}

fn theme(id: &str, neutral: &str, accent: &str, status: [&str; 4]) -> ThemeDef {
    ThemeDef {
        id: ThemeId::parse(id).unwrap(),
        name: id.to_owned(),
        version: 1,
        seeds: Seeds {
            neutral: hex(neutral),
            accent: hex(accent),
            status: Status { good: hex(status[0]), warn: hex(status[1]), bad: hex(status[2]), info: hex(status[3]) },
        },
        shape: None,
        variants: Variants::Both,
        pins: ModePins::default(),
        high_contrast: ModePins::default(),
        components: BTreeMap::new(),
        contrast_intent: ContrastIntent::Standard,
        apps: BTreeMap::new(),
    }
}

pub(super) fn grey_blue() -> ThemeDef {
    theme("test.grey-blue", "#8b8d98", "#3e63dd", ["#30a46c", "#ffc53d", "#e5484d", "#0090ff"])
}

pub(super) fn violet() -> ThemeDef {
    theme("test.violet", "#7a6b9e", "#9b30ff", ["#00c853", "#ffab00", "#ff1744", "#2979ff"])
}

/// Text pinned far too faint in both modes.
fn pinned_faint() -> ThemeDef {
    let mut t = grey_blue();
    t.id = ThemeId::parse("test.pinned-faint").unwrap();
    t.pins.light = Pins::from_iter([(T::TextFaint, hex("#c8c8c8"))]);
    t.pins.dark = Pins::from_iter([(T::TextFaint, hex("#404040"))]);
    t
}

pub(super) fn appearances() -> Vec<Appearance> {
    let mut out = Vec::new();
    for mode in [Mode::Light, Mode::Dark] {
        for contrast in [Contrast::Standard, Contrast::High] {
            for size in [Size::Compact, Size::Comfortable] {
                for text_size in TextSize::ALL {
                    out.push(Appearance { mode, contrast, size, text_size, reduced_motion: false });
                }
            }
        }
    }
    out
}

pub(super) fn app(mode: Mode, contrast: Contrast) -> Appearance {
    Appearance { mode, contrast, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false }
}

pub(super) fn over(fg: Rgba, bg: Rgba) -> Rgba {
    let a = f64::from(fg.a()) / 255.0;
    let ch = |i: usize| (f64::from(fg.0[i]) * a + f64::from(bg.0[i]) * (1.0 - a)).round() as u8;
    Rgba::rgb(ch(0), ch(1), ch(2))
}

const TEXT_BGS: [T; 6] = [T::SurfaceBackground, T::SurfacePanel, T::SurfaceRaised, T::SurfaceSunken, T::StateHover, T::StatePressed];
const CONTROL_BGS: [T; 3] = [T::SurfacePanel, T::SurfaceRaised, T::SurfaceSunken];

/// The §3 pairs, written out independently of the pass's own table: (fg, bg, standard, high).
fn spec_pairs() -> Vec<(T, T, f64, f64)> {
    let mut p = Vec::new();
    for fg in [T::TextNormal, T::TextMuted, T::TextFaint] {
        p.extend(TEXT_BGS.map(|bg| (fg, bg, 4.5, 7.0)));
    }
    p.extend([T::StateSelected, T::StateSelectedHover].map(|bg| (T::TextOnSelected, bg, 4.5, 7.0)));
    p.extend(CONTROL_BGS.map(|bg| (T::AccentText, bg, 4.5, 7.0)));
    p.extend([T::AccentSolid, T::AccentHover, T::AccentPressed].map(|bg| (T::TextOnAccent, bg, 4.5, 7.0)));
    for (fg, bg, solids) in [
        (T::StatusGoodFg, T::StatusGoodBg, [T::StatusGoodSolid, T::StatusGoodSolidHover, T::StatusGoodSolidPressed]),
        (T::StatusWarnFg, T::StatusWarnBg, [T::StatusWarnSolid, T::StatusWarnSolidHover, T::StatusWarnSolidPressed]),
        (T::StatusBadFg, T::StatusBadBg, [T::StatusBadSolid, T::StatusBadSolidHover, T::StatusBadSolidPressed]),
        (T::StatusInfoFg, T::StatusInfoBg, [T::StatusInfoSolid, T::StatusInfoSolidHover, T::StatusInfoSolidPressed]),
    ] {
        p.extend([T::SurfacePanel, T::SurfaceRaised, bg].map(|b| (fg, b, 4.5, 7.0)));
        p.extend(solids.map(|solid| (T::TextOnStatus, solid, 4.5, 7.0)));
    }
    p.extend(CONTROL_BGS.map(|bg| (T::LineFocus, bg, 3.0, 4.5)));
    p.extend([T::SurfacePanel, T::SurfaceRaised, T::SurfaceSunken].map(|bg| (T::AccentSolid, bg, 3.0, 4.5)));
    p
}

fn assert_meets_spec(name: &str, r: &Resolved, contrast: Contrast) {
    let t = &r.tokens;
    let high = contrast == Contrast::High;
    for (fg, bg, std, hi) in spec_pairs() {
        let min = if high { hi } else { std };
        let ratio = contrast_ratio(t.color(fg), t.color(bg));
        assert!(ratio >= min, "{name} {contrast:?}: {} on {} is {ratio:.2}, needs {min}", fg.path(), bg.path());
    }
    let min = if high { 4.5 } else { 3.0 };
    for backdrop in [Rgba::BLACK, Rgba::WHITE] {
        let halo = over(t.color(T::OverlayHalo), backdrop);
        for fg in [T::OverlayStroke, T::OverlayMarquee] {
            let ratio = contrast_ratio(t.color(fg), halo);
            assert!(ratio >= min, "{name} {contrast:?}: {} on halo over {backdrop:?} is {ratio:.2}", fg.path());
        }
    }
}

#[test]
fn color_token_paths_round_trip_and_are_unique() {
    let paths: BTreeSet<_> = ColorToken::ALL.iter().map(|t| t.path()).collect();
    assert_eq!(paths.len(), ColorToken::ALL.len());
    for &t in ColorToken::ALL {
        assert_eq!(ColorToken::from_path(t.path()), Some(t));
    }
    assert_eq!(ColorToken::from_path("surface.nope"), None);
}

#[test]
fn samples_resolve_and_derived_tokens_meet_every_minimum() {
    for def in [grey_blue(), violet(), pinned_faint()] {
        let pinned = def.id == pinned_faint().id;
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert_eq!(r.variant_used, a.mode);
            if pinned && a.contrast == Contrast::Standard {
                continue;
            }
            assert!(r.warnings.is_empty(), "{} {a:?}: {:?}", def.name, r.warnings);
            assert_meets_spec(&def.name, &r, a.contrast);
        }
    }
}

#[test]
fn pinned_failing_text_warns_in_standard_and_is_fixed_in_high() {
    let def = pinned_faint();
    for mode in [Mode::Light, Mode::Dark] {
        let pin = def.pins.get(mode).get(T::TextFaint).unwrap();
        let r = resolve(&def, app(mode, Contrast::Standard), None);
        let t = &r.tokens;
        assert_eq!(t.color(T::TextFaint), pin);
        assert_eq!(t.color(T::CaptionInactive), pin, "caption.inactive follows the pinned text.faint");
        let mut expected: Vec<_> = TEXT_BGS
            .map(|bg| ContrastCheck { fg: T::TextFaint.into(), bg: bg.into(), ratio: contrast_ratio(pin, t.color(bg)), minimum: 4.5 })
            .into_iter()
            .collect();
        // The outline and the off thumb of check boxes, radio buttons and switches are `text.faint`.
        let off = |part| ComponentField::Check(CheckField::Color { on_off: OnOff::Off, part, state: State::Rest });
        for bg in [T::SurfacePanel, T::SurfaceRaised] {
            expected.push(ContrastCheck { fg: off(CheckPart::Border).into(), bg: bg.into(), ratio: contrast_ratio(pin, t.color(bg)), minimum: 3.0 });
        }
        let sunken = t.color(T::SurfaceSunken);
        expected.push(ContrastCheck {
            fg: off(CheckPart::Mark).into(),
            bg: off(CheckPart::Fill).into(),
            ratio: contrast_ratio(pin, sunken),
            minimum: 3.0,
        });
        // Text field placeholders are `text.faint` on the field fill, `surface.sunken`.
        let field = |part| ComponentField::Field(FieldField::Color { part, disabled: false });
        expected.push(ContrastCheck {
            fg: field(FieldPart::Placeholder).into(),
            bg: field(FieldPart::Fill).into(),
            ratio: contrast_ratio(pin, sunken),
            minimum: 4.5,
        });
        assert_eq!(r.warnings, expected, "{mode:?}");

        let high = resolve(&def, app(mode, Contrast::High), None);
        assert!(high.warnings.is_empty(), "{mode:?}: {:?}", high.warnings);
        assert_ne!(high.tokens.color(T::TextFaint), pin);
        for bg in TEXT_BGS {
            assert!(contrast_ratio(high.tokens.color(T::TextFaint), high.tokens.color(bg)) >= 7.0);
        }
    }
}

#[test]
fn pinned_edge_hover_pressed_and_divider_are_kept_without_warnings() {
    let mut def = grey_blue();
    def.id = ThemeId::parse("test.pinned-targets").unwrap();
    for mode in [Mode::Light, Mode::Dark] {
        let panel = resolve(&def, app(mode, Contrast::Standard), None).tokens.color(T::SurfacePanel);
        let pins = Pins::from_iter([(T::LineControlEdge, panel), (T::StateHover, panel), (T::StatePressed, panel), (T::LineDivider, panel)]);
        match mode {
            Mode::Light => def.pins.light = pins,
            Mode::Dark => def.pins.dark = pins,
        }
        let r = resolve(&def, app(mode, Contrast::Standard), None);
        for t in [T::LineControlEdge, T::StateHover, T::StatePressed, T::LineDivider] {
            assert_eq!(r.tokens.color(t), panel, "{mode:?} {t:?}");
        }
        assert!(r.warnings.is_empty(), "{mode:?}: {:?}", r.warnings);
        let high = resolve(&def, app(mode, Contrast::High), None);
        for t in [T::LineControlEdge, T::StateHover, T::StatePressed, T::LineDivider] {
            assert_eq!(high.tokens.color(t), panel, "{mode:?} {t:?} in high contrast");
        }
        assert!(high.warnings.is_empty(), "{mode:?}: {:?}", high.warnings);
    }
}

#[test]
fn pinned_mid_tone_panel_with_default_text_has_no_high_contrast_warnings() {
    let mut def = grey_blue();
    def.id = ThemeId::parse("test.mid-panel").unwrap();
    // Mid-tones where white (dark mode) or black (light mode) still reaches 7:1.
    def.pins.light = Pins::from_iter([(T::SurfacePanel, hex("#a6a6a6"))]);
    def.pins.dark = Pins::from_iter([(T::SurfacePanel, hex("#555555"))]);
    for mode in [Mode::Light, Mode::Dark] {
        let r = resolve(&def, app(mode, Contrast::High), None);
        assert_eq!(r.tokens.color(T::SurfacePanel), if mode == Mode::Light { hex("#a6a6a6") } else { hex("#555555") });
        assert!(r.warnings.is_empty(), "{mode:?}: {:?}", r.warnings);
    }
}

#[test]
fn dark_accent_seed_in_light_mode_still_meets_the_overlay_pairs() {
    let def = theme("test.navy", "#d4d0c8", "#0a246a", ["#4e712e", "#806000", "#a00000", "#0a246a"]);
    for contrast in [Contrast::Standard, Contrast::High] {
        let r = resolve(&def, app(Mode::Light, contrast), None);
        assert!(r.warnings.is_empty(), "{contrast:?}: {:?}", r.warnings);
    }
}

#[test]
fn pins_replace_defaults_and_aliases_follow() {
    let mut def = grey_blue();
    let bg = hex("#203040");
    def.pins.dark = Pins::from_iter([(T::SurfaceBackground, bg), (T::OverlayStroke, hex("#ffee00"))]);
    def.high_contrast.dark = Pins::from_iter([(T::SurfaceBackground, hex("#000000"))]);
    let r = resolve(&def, app(Mode::Dark, Contrast::Standard), None);
    assert_eq!(r.tokens.color(T::SurfaceBackground), bg);
    assert_eq!(r.tokens.color(T::SurfaceCanvas), bg);
    assert_eq!(r.tokens.color(T::OverlayHandleStroke), hex("#ffee00"));
    let high = resolve(&def, app(Mode::Dark, Contrast::High), None);
    assert_eq!(high.tokens.color(T::SurfaceCanvas), Rgba::BLACK, "high-contrast pins apply over mode pins");
}

#[test]
fn single_variant_theme_resolves_both_modes() {
    let mut def = violet();
    def.variants = Variants::DarkOnly;
    let dark = resolve(&def, app(Mode::Dark, Contrast::Standard), None);
    let light = resolve(&def, app(Mode::Light, Contrast::Standard), None);
    assert_eq!(light.variant_used, Mode::Dark);
    assert_eq!(light.tokens, dark.tokens);
    def.variants = Variants::LightOnly;
    assert_eq!(resolve(&def, app(Mode::Dark, Contrast::High), None).variant_used, Mode::Light);
}

#[test]
fn size_presets_text_size_shape_and_motion() {
    let def = grey_blue();
    let at = |size, text_size| resolve(&def, Appearance { size, text_size, ..app(Mode::Light, Contrast::Standard) }, None).tokens;
    let compact = at(Size::Compact, TextSize::Default);
    let comfy = at(Size::Comfortable, TextSize::Default);

    assert_eq!((compact.size.control.md, compact.size.control.sm), (24.0, 20.0));
    assert_eq!((comfy.size.control.md, comfy.size.control.sm), (30.0, 24.0));
    assert_eq!((compact.size.bar.title, compact.size.bar.tab, compact.size.bar.status, compact.size.bar.options), (32.0, 26.0, 22.0, 32.0));
    assert_eq!((comfy.size.rail.width, comfy.size.rail.button, comfy.size.panel_header), (50.0, 36.0, 34.0));
    assert_eq!((comfy.size.icon.sm, comfy.size.icon.md, comfy.size.icon.lg), (14.0, 18.0, 22.0));
    assert_eq!((compact.space.gap.inline, compact.space.gap.stack, compact.space.gap.section), (4.0, 6.0, 12.0));
    assert_eq!((comfy.space.pad.control, comfy.space.pad.panel, comfy.space.pad.dialog), (8.0, 12.0, 20.0));
    let r = compact.shape.radius;
    assert_eq!((r.control, r.popup, r.window, r.panel, r.button), (3.0, 4.0, 6.0, 0.0, Radius::Full.px()));
    assert_eq!(comfy.shape.radius.window, 12.0);
    assert_eq!(compact.shape.edge, Edge::Flat);

    let body = compact.typography.get(TypeRole::Body);
    assert_eq!((body.size, body.line_height, body.font), (12.0, 16.0, FontSlot::Ui));
    let title = comfy.typography.title;
    assert_eq!((title.size, title.line_height, title.font), (14.0, 20.0, FontSlot::UiSemibold));
    let caption = at(Size::Compact, TextSize::Small).typography.caption;
    assert_eq!((caption.size, caption.line_height), (9.5, 12.5));
    let body = at(Size::Compact, TextSize::Large).typography.body;
    assert_eq!((body.size, body.line_height), (14.0, 18.5));
    let heading = at(Size::Comfortable, TextSize::Larger).typography.heading;
    assert_eq!((heading.size, heading.line_height), (22.0, 31.0));
    assert_eq!(at(Size::Comfortable, TextSize::Larger).size, comfy.size, "text size scales type only");

    // A theme with its own shape: square corners and a bevel from pinned edge colours.
    let mut classic = grey_blue();
    classic.shape = Some(ShapeSeed {
        radius: Some(RadiusSet { control: 0.0, popup: 0.0, window: 0.0, panel: 0.0, button: 0.0 }),
        edge: Some(EdgeSeed::Bevel { width: 1.0 }),
    });
    classic.pins.light = Pins::from_iter([(T::EdgeLight, Rgba::WHITE)]);
    let t = resolve(&classic, Appearance { size: Size::Comfortable, ..app(Mode::Light, Contrast::Standard) }, None).tokens;
    assert_eq!(t.shape.radius.control, 0.0);
    assert_eq!(t.shape.edge, Edge::Bevel { light: Rgba::WHITE, dark: t.color(T::EdgeDark), width: 1.0 });

    assert_eq!((compact.motion.hover.millis, compact.motion.open.millis), (80, 150));
    let still = resolve(&def, Appearance { reduced_motion: true, ..app(Mode::Dark, Contrast::Standard) }, None).tokens;
    assert_eq!((still.motion.hover.millis, still.motion.open.millis, still.motion.r#move.millis), (0, 0, 0));
    assert!(still.elevation.popup.alpha > compact.elevation.popup.alpha, "dark shadows are stronger");
    assert_eq!(compact.elevation.dialog, Elevation::level(3));
}

#[test]
fn translucent_state_fill_is_measured_over_the_panel() {
    let mut def = grey_blue();
    let derived = resolve(&def, app(Mode::Dark, Contrast::Standard), None).tokens.color(T::TextNormal);
    def.pins.dark = Pins::from_iter([(T::StateHover, hex("#ffffff14"))]);
    let r = resolve(&def, app(Mode::Dark, Contrast::Standard), None);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(r.tokens.color(T::TextNormal), derived, "text.normal stays light");
    assert_eq!(r.tokens.color(T::StateHover), hex("#ffffff14"));
}

#[test]
fn coloured_selection_leaves_text_normal_alone() {
    let mut def = grey_blue();
    let derived = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.color(T::TextNormal);
    def.pins.light = Pins::from_iter([(T::StateSelected, hex("#0A246A"))]);
    let t = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens;
    let normal = t.color(T::TextNormal);
    assert!(relative_luminance(normal) <= relative_luminance(derived), "{normal:?} is lighter than {derived:?}");
    for bg in [T::SurfaceBackground, T::SurfacePanel, T::SurfaceRaised, T::SurfaceSunken] {
        assert!(contrast_ratio(normal, t.color(bg)) >= 4.5, "text.normal on {}", bg.path());
    }
}

#[test]
fn text_between_fills_on_both_sides_keeps_its_best_value() {
    // A navy hover under dark text: no colour passes on the light surfaces and the navy fill,
    // and moving toward the fill would lose contrast on the surfaces, so text.normal doesn't.
    let mut def = grey_blue();
    let derived = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.color(T::TextNormal);
    def.pins.light = Pins::from_iter([(T::StateHover, hex("#0A246A"))]);
    let r = resolve(&def, app(Mode::Light, Contrast::Standard), None);
    let normal = r.tokens.color(T::TextNormal);
    assert!(relative_luminance(normal) <= relative_luminance(derived), "{normal:?} is lighter than {derived:?}");
    for bg in [T::SurfaceBackground, T::SurfacePanel, T::SurfaceRaised, T::SurfaceSunken] {
        assert!(contrast_ratio(normal, r.tokens.color(bg)) >= 4.5, "text.normal on {}", bg.path());
    }
    assert!(r.warnings.iter().any(|w| w.fg == T::TextNormal.into() && w.bg == T::StateHover.into()));

    // Navy selected, light selected_hover, no surfaces in the rule: text.on_selected takes the
    // candidate with the best lowest ratio.
    def.pins.light = Pins::from_iter([(T::StateSelected, hex("#0A246A"))]);
    let t = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens;
    let worst =
        |fg: Rgba| [T::StateSelected, T::StateSelectedHover].map(|bg| contrast_ratio(fg, t.color(bg))).into_iter().fold(f64::INFINITY, f64::min);
    let got = worst(t.color(T::TextOnSelected));
    for other in [derived, Rgba::WHITE, Rgba::BLACK] {
        assert!(got >= worst(other), "{got:.2} is below {other:?}'s {:.2}", worst(other));
    }
}

#[test]
fn pinned_on_accent_text_moves_the_unpinned_solids() {
    let mut def = theme("test.light-accent", "#8b8d98", "#78ADF6", ["#30a46c", "#ffc53d", "#e5484d", "#0090ff"]);
    let solids = [T::AccentSolid, T::AccentHover, T::AccentPressed];
    let before = resolve(&def, app(Mode::Dark, Contrast::Standard), None).tokens;
    def.pins.dark = Pins::from_iter([(T::TextOnAccent, Rgba::WHITE)]);
    let r = resolve(&def, app(Mode::Dark, Contrast::Standard), None);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(r.tokens.color(T::TextOnAccent), Rgba::WHITE);
    for s in solids {
        let (old, new) = (before.color(s), r.tokens.color(s));
        assert!(relative_luminance(new) < relative_luminance(old), "{} {old:?} -> {new:?}", s.path());
        assert!(contrast_ratio(Rgba::WHITE, new) >= 4.5, "{}", s.path());
    }
}

/// A failing pin is reported on its semantic pairs and on every component pair whose default
/// reads it; the colours stay.
#[test]
fn pinned_failing_text_warns_on_its_pairs_and_the_component_pairs_that_read_it() {
    let mut def = grey_blue();
    let pin = hex("#c8c8c8");
    def.pins.light = Pins::from_iter([(T::TextNormal, pin)]);
    let (r, steps) = resolve::resolve_with_steps(&def, app(Mode::Light, Contrast::Standard), None);
    let t = &r.tokens;
    let expected: Vec<_> = TEXT_BGS
        .map(|bg| ContrastCheck { fg: T::TextNormal.into(), bg: bg.into(), ratio: contrast_ratio(pin, t.color(bg)), minimum: 4.5 })
        .into_iter()
        .collect();
    let (semantic, component): (Vec<ContrastCheck>, Vec<ContrastCheck>) = r.warnings.iter().partition(|w| matches!(w.fg, TokenRef::Color(_)));
    assert_eq!(semantic, expected);
    let failing: Vec<_> = steps.component_pairs.iter().filter(|p| !p.passes()).copied().collect();
    assert!(!component.is_empty());
    assert_eq!(component, failing);
    for w in &component {
        let fg = match w.fg {
            TokenRef::Component(f) => t.component.color(f),
            TokenRef::Color(c) => Some(t.color(c)),
        };
        assert_eq!(fg, Some(pin), "{} reads the pin", w.fg.path());
    }

    // A pinned component colour is reported.
    let text = ComponentField::from_path("button.secondary.text.rest").unwrap();
    def.pins.light = Pins::from_iter([(TokenRef::Component(text), pin)]);
    let r = resolve(&def, app(Mode::Light, Contrast::Standard), None);
    assert_eq!(r.warnings.iter().map(|w| w.fg).collect::<Vec<_>>(), [TokenRef::Component(text)]);
}
