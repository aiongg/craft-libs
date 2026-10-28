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
        mode_seeds: ModeSeeds::default(),
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
        let ratio = contrast_ratio(t.color(T::OverlayStroke), halo);
        assert!(ratio >= min, "{name} {contrast:?}: overlay.stroke on halo over {backdrop:?} is {ratio:.2}");
    }
    let ratio = contrast_ratio(t.color(T::OverlayMarquee), t.color(T::OverlayMarqueeDash));
    assert!(ratio >= min, "{name} {contrast:?}: overlay.marquee on overlay.marquee_dash is {ratio:.2}");
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
        // The off outline of check boxes, radio buttons and switches is `text.faint`.
        let off_border = ComponentField::Check(CheckField::Off { part: CheckOffPart::Border, state: State::Rest });
        for bg in [T::SurfacePanel, T::SurfaceRaised] {
            expected.push(ContrastCheck { fg: off_border.into(), bg: bg.into(), ratio: contrast_ratio(pin, t.color(bg)), minimum: 3.0 });
        }
        // The inline empty-state message is `text.faint` on the panel.
        expected.push(ContrastCheck {
            fg: ComponentField::EmptyState(EmptyStateField::Text).into(),
            bg: T::SurfacePanel.into(),
            ratio: contrast_ratio(pin, t.color(T::SurfacePanel)),
            minimum: 4.5,
        });
        let sunken = t.color(T::SurfaceSunken);
        // Text field placeholders are `text.faint` on the field fill, `surface.sunken`.
        let field = |part| ComponentField::Field(FieldField::Color { part, disabled: false });
        expected.push(ContrastCheck {
            fg: field(FieldPart::Placeholder).into(),
            bg: field(FieldPart::Fill).into(),
            ratio: contrast_ratio(pin, sunken),
            minimum: 4.5,
        });
        // A toast's "more" line is `text.faint` on the card fill, `surface.raised`.
        expected.push(ContrastCheck {
            fg: ComponentField::Toast(ToastField::More).into(),
            bg: ComponentField::Toast(ToastField::Fill).into(),
            ratio: contrast_ratio(pin, t.color(T::SurfaceRaised)),
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
        let panel = r.tokens.color(T::SurfacePanel);
        assert_eq!(panel, if mode == Mode::Light { hex("#a6a6a6") } else { hex("#555555") });
        assert!(r.warnings.is_empty(), "{mode:?}: {:?}", r.warnings);
        // The raised surface's anchored target lies above the panel; the pull for 7:1 text
        // stops short of the panel, never past it.
        assert!(r.tokens.color(T::SurfaceRaised).to_oklch().l >= panel.to_oklch().l, "{mode:?}");
    }
}

/// The background greys a theme doesn't pin.
const ANCHORED: [T; 8] =
    [T::SurfaceBackground, T::SurfaceRaised, T::SurfaceSunken, T::StateHover, T::StatePressed, T::StateSelected, T::LineDivider, T::LineControlEdge];

#[test]
fn background_greys_keep_their_curve_distance_from_a_pinned_panel() {
    let plain = grey_blue();
    let mut def = grey_blue();
    def.id = ThemeId::parse("test.panel-only").unwrap();
    def.pins.light = Pins::from_iter([(T::SurfacePanel, hex("#c4c4c4"))]);
    def.pins.dark = Pins::from_iter([(T::SurfacePanel, hex("#3a3a3a"))]);
    let l = |c: Rgba| c.to_oklch().l;
    for mode in [Mode::Light, Mode::Dark] {
        let curve = resolve(&plain, app(mode, Contrast::Standard), None).tokens;
        let e = explain(&def, app(mode, Contrast::Standard), None);
        let t = &e.resolved.tokens;
        let panel = t.color(T::SurfacePanel);
        for token in ANCHORED {
            let want = l(curve.color(token)) - l(curve.color(T::SurfacePanel));
            let got = l(t.color(token)) - l(panel);
            // Both distances are measured on 8-bit colours.
            assert!((want - got).abs() < 0.006, "{mode:?} {token:?}: {got:.3}, curve {want:.3}");
        }
        assert_eq!(t.color(T::StateDisabledFill), panel, "{mode:?}");
        assert!(e.colors.iter().all(|o| !matches!(o.pass_move, Some(PassMove::Pulled(_)))), "{mode:?}: nothing needs a pull");
        assert!(e.resolved.warnings.is_empty(), "{mode:?}: {:?}", e.resolved.warnings);
    }
}

#[test]
fn a_pinned_grey_overrides_its_anchored_value() {
    let mut def = grey_blue();
    def.id = ThemeId::parse("test.panel-and-raised").unwrap();
    def.pins.dark = Pins::from_iter([(T::SurfacePanel, hex("#3a3a3a")), (T::SurfaceRaised, hex("#202020")), (T::StateHover, hex("#4a4a4a"))]);
    let e = explain(&def, app(Mode::Dark, Contrast::Standard), None);
    for (token, value) in [(T::SurfaceRaised, "#202020"), (T::StateHover, "#4a4a4a")] {
        let o = e.color(token).unwrap();
        assert_eq!(o.value, hex(value), "{token:?}");
        assert_ne!(o.derived, o.value, "{token:?}: the anchored value is what the pin replaces");
    }
}

#[test]
fn anchoring_leaves_a_theme_without_a_panel_pin_on_the_curve() {
    for mode in [Mode::Light, Mode::Dark] {
        for contrast in [Contrast::Standard, Contrast::High] {
            let e = explain(&grey_blue(), app(mode, contrast), None);
            for o in &e.colors {
                if let ColorSource::Anchored(n) | ColorSource::StateVariant { step: n, .. } = o.source {
                    assert_eq!(o.value, e.ramps.neutral.step(n), "{mode:?} {contrast:?} {:?}", o.token);
                    assert_eq!(o.anchoring.map(|a| (a.on, a.steps)), Some((T::SurfacePanel, e.ramps.neutral)), "{mode:?} {contrast:?} {:?}", o.token);
                }
            }
        }
    }
}

#[test]
fn a_panel_pinned_only_for_high_contrast_anchors_the_greys_there() {
    let mut def = grey_blue();
    def.id = ThemeId::parse("test.high-panel").unwrap();
    def.high_contrast.dark = Pins::from_iter([(T::SurfacePanel, hex("#2a2a2a"))]);
    let l = |c: Rgba| c.to_oklch().l;
    let standard = explain(&def, app(Mode::Dark, Contrast::Standard), None);
    let high = explain(&def, app(Mode::Dark, Contrast::High), None);
    assert_eq!(standard.color(T::SurfaceRaised).unwrap().derived, standard.ramps.neutral.step(4), "standard: on the curve");
    let panel = high.resolved.tokens.color(T::SurfacePanel);
    assert_eq!(panel, hex("#2a2a2a"));
    for token in [T::SurfaceRaised, T::SurfaceSunken, T::StateHover, T::LineDivider] {
        let o = high.color(token).unwrap();
        let ColorSource::Anchored(n) = o.source else { panic!("{token:?}") };
        let want = ramp::DARK_L[n - 1] - ramp::DARK_L[2];
        assert!((l(o.derived) - l(panel) - want).abs() < 0.006, "{token:?}: {:.3}, curve {want:.3}", l(o.derived) - l(panel));
    }
}

/// A state variant keeps its curve distance from its own base when the theme pins the base, so a
/// pinned selection stays on its side of the panel under the pointer.
#[test]
fn a_pinned_state_carries_its_variant() {
    let gray = crate::builtins().iter().find(|t| t.id.as_str() == "craft.pro-light-gray").unwrap();
    let e = explain(gray, app(Mode::Light, Contrast::Standard), None);
    let t = &e.resolved.tokens;
    let l = |token| t.color(token).to_oklch().l;
    let (selected, hover, panel) = (l(T::StateSelected), l(T::StateSelectedHover), l(T::SurfacePanel));
    let want = ramp::LIGHT_L[5] - ramp::LIGHT_L[4];
    assert!((hover - selected - want).abs() < 0.006, "selected_hover {hover:.3} from selected {selected:.3}, curve {want:.3}");
    assert!(selected > panel && hover > panel, "both lighter than the {panel:.3} panel");
    assert_eq!(e.color(T::StateSelectedHover).unwrap().source_words(), "neutral 6, anchored on state.selected");

    let mut def = grey_blue();
    def.id = ThemeId::parse("test.pinned-hover").unwrap();
    def.pins.dark = Pins::from_iter([(T::StateHover, hex("#505050"))]);
    let t = resolve(&def, app(Mode::Dark, Contrast::Standard), None).tokens;
    let want = ramp::DARK_L[4] - ramp::DARK_L[3];
    let got = t.color(T::StatePressed).to_oklch().l - hex("#505050").to_oklch().l;
    assert!((got - want).abs() < 0.006, "pressed from the pinned hover: {got:.3}, curve {want:.3}");
    let plain = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    assert_eq!(t.color(T::StateSelectedHover), plain.color(T::StateSelectedHover), "an unpinned base leaves its variant on the panel");
}

/// With a pinned panel the anchored greys are the panel's grey: its hue and chroma.
#[test]
fn anchored_greys_take_a_pinned_panels_tint() {
    let mut def = grey_blue();
    def.id = ThemeId::parse("test.warm-panel").unwrap();
    let warm = hex("#d4d0c8");
    def.pins.light = Pins::from_iter([(T::SurfacePanel, warm)]);
    let t = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens;
    let p = warm.to_oklch();
    for token in [T::SurfaceRaised, T::StateHover, T::StatePressed, T::LineBorder, T::LineControlEdge] {
        let o = t.color(token).to_oklch();
        assert!((o.c - p.c).abs() < 0.004 && (o.h - p.h).abs() < 8.0, "{token:?}: c {:.4} h {:.1}, panel c {:.4} h {:.1}", o.c, o.h, p.c, p.h);
    }
}

/// A bevel's edges are the panel lit and shaded, so a theme that pins only a grey panel still
/// gets a bevel that reads, in either mode.
#[test]
fn a_bevel_lights_and_shades_a_pinned_panel() {
    let mut def = grey_blue();
    def.id = ThemeId::parse("test.bevel-panel").unwrap();
    def.shape = Some(ShapeSeed { radius: None, edge: Some(EdgeSeed::Bevel { width: 1.0 }) });
    def.pins.light = Pins::from_iter([(T::SurfacePanel, hex("#c0c0c0"))]);
    def.pins.dark = Pins::from_iter([(T::SurfacePanel, hex("#404040"))]);
    for mode in [Mode::Light, Mode::Dark] {
        let t = resolve(&def, app(mode, Contrast::Standard), None).tokens;
        let (panel, light, dark) = (t.color(T::SurfacePanel), t.color(T::EdgeLight), t.color(T::EdgeDark));
        let l = |c: Rgba| c.to_oklch().l;
        assert!((l(light) - l(panel) - values::BEVEL_LIT).abs() < 0.006, "{mode:?}: lit {}", light.to_hex());
        assert!((l(panel) - l(dark) - values::BEVEL_SHADED).abs() < 0.006, "{mode:?}: shaded {}", dark.to_hex());
        assert!(
            contrast_ratio(light, panel) >= 1.5 && contrast_ratio(panel, dark) >= 1.5,
            "{mode:?}: {} {} {}",
            light.to_hex(),
            panel.to_hex(),
            dark.to_hex()
        );
        assert_eq!(t.shape.edge, Edge::Bevel { light, dark, width: 1.0 });
    }
}

/// A pull for one pair never breaks another on the same background: a dark focus line and light
/// text pinned on a mid-tone panel both pass on the pulled raised surface; when no lightness
/// serves both, the earlier rule (focus) keeps its pair and the text is reported.
#[test]
fn a_later_pull_never_breaks_an_earlier_pair() {
    let focus = hex("#0a246a");
    let fails = |r: &Resolved, fg: T, bg: T| r.warnings.iter().any(|w| w.fg == fg.into() && w.bg == bg.into());
    let mut def = grey_blue();
    def.id = ThemeId::parse("test.dark-focus").unwrap();
    def.pins.dark = Pins::from_iter([(T::SurfacePanel, hex("#747474")), (T::LineFocus, focus), (T::TextNormal, Rgba::WHITE)]);
    let e = explain(&def, app(Mode::Dark, Contrast::Standard), None);
    assert_eq!(e.color(T::SurfaceRaised).unwrap().pass_move, Some(PassMove::Pulled(T::TextNormal)));
    for bg in [T::SurfaceRaised, T::SurfaceSunken] {
        assert!(!fails(&e.resolved, T::LineFocus, bg) && !fails(&e.resolved, T::TextNormal, bg), "{bg:?}: {:?}", e.resolved.warnings);
    }

    def.pins.dark = Pins::from_iter([(T::SurfacePanel, hex("#6a6a6a")), (T::LineFocus, focus), (T::TextNormal, hex("#f4f4f4"))]);
    let r = resolve(&def, app(Mode::Dark, Contrast::Standard), None);
    assert!(!fails(&r, T::LineFocus, T::SurfaceRaised), "the earlier pair keeps passing: {:?}", r.warnings);
    assert!(fails(&r, T::TextNormal, T::SurfaceRaised), "{:?}", r.warnings);
}

/// Every move the explanation names is one the pass made last: a pulled grey lies between its
/// target and its anchor, never past the anchor.
#[test]
fn pulled_greys_lie_between_their_targets_and_anchors() {
    for theme in crate::builtins() {
        for a in appearances().into_iter().filter(|a| a.size == Size::Compact && a.text_size == TextSize::Default) {
            let e = explain(theme, a, None);
            for o in &e.colors {
                let (Some(PassMove::Pulled(_)), Some(anchoring)) = (o.pass_move, o.anchoring) else { continue };
                let on = e.color(anchoring.on).unwrap();
                let (value, target, home) = (o.value.to_oklch().l, o.derived.to_oklch().l, on.contrast_move.unwrap_or(on.value).to_oklch().l);
                let (lo, hi) = (target.min(home) - 0.005, target.max(home) + 0.005);
                assert!(lo <= value && value <= hi, "{} {a:?} {:?}: {value:.3} outside {target:.3}..{home:.3}", theme.id, o.token);
            }
        }
    }
}
/// Text the theme pins can't move, so an anchored fill under it comes back toward the panel
/// until the pair passes; a pinned fill stays.
#[test]
fn anchored_fills_are_pulled_toward_the_panel_for_pinned_text() {
    let mut def = grey_blue();
    def.id = ThemeId::parse("test.pinned-muted").unwrap();
    def.pins.dark = Pins::from_iter([(T::SurfacePanel, hex("#535353")), (T::TextMuted, hex("#d4d4d4")), (T::StateSelected, hex("#6b6b6b"))]);
    let e = explain(&def, app(Mode::Dark, Contrast::Standard), None);
    let t = &e.resolved.tokens;
    let panel = t.color(T::SurfacePanel);
    for bg in [T::SurfaceRaised, T::StateHover, T::StatePressed] {
        let o = e.color(bg).unwrap();
        assert_eq!(o.pass_move, Some(PassMove::Pulled(T::TextMuted)), "{bg:?}");
        assert!(contrast_ratio(hex("#d4d4d4"), o.value) >= 4.5, "{bg:?}");
        assert!(o.value.to_oklch().l >= panel.to_oklch().l && o.value.to_oklch().l < o.derived.to_oklch().l, "{bg:?}");
    }
    assert_eq!(t.color(T::StateSelected), hex("#6b6b6b"));
    assert!(e.resolved.warnings.is_empty(), "{:?}", e.resolved.warnings);
}

#[test]
fn dark_accent_seed_in_light_mode_still_meets_the_overlay_pairs() {
    let def = theme("test.navy", "#d4d0c8", "#0a246a", ["#4e712e", "#806000", "#a00000", "#0a246a"]);
    for contrast in [Contrast::Standard, Contrast::High] {
        let r = resolve(&def, app(Mode::Light, contrast), None);
        assert!(r.warnings.is_empty(), "{contrast:?}: {:?}", r.warnings);
    }
}

/// Guide colours are the user's: a pinned guide colour that vanishes on the panel and the halo is
/// kept in both contrasts and reported nowhere. The marquee's dashes are a pair, so a pinned
/// marquee that matches them is fixed in high contrast.
#[test]
fn guide_colours_are_exempt_from_the_overlay_pairs() {
    let mut def = grey_blue();
    let faint = hex("#202020");
    let guides = [T::OverlayGuide, T::OverlayGuideSelected, T::OverlaySmartGuide];
    def.pins.dark = Pins::from_iter(guides.map(|t| (t, faint)).into_iter().chain([(T::OverlayMarquee, hex("#101010"))]));
    let standard = resolve(&def, app(Mode::Dark, Contrast::Standard), None);
    for t in guides {
        assert_eq!(standard.tokens.color(t), faint, "{}", t.path());
        assert!(!standard.warnings.iter().any(|w| w.fg == TokenRef::Color(t) || w.bg == TokenRef::Color(t)), "{}: {:?}", t.path(), standard.warnings);
    }
    assert!(
        standard.warnings.iter().any(|w| w.fg == TokenRef::Color(T::OverlayMarquee) && w.bg == TokenRef::Color(T::OverlayMarqueeDash)),
        "{:?}",
        standard.warnings
    );
    let high = resolve(&def, app(Mode::Dark, Contrast::High), None);
    for t in guides {
        assert_eq!(high.tokens.color(t), faint, "{}", t.path());
    }
    let ratio = contrast_ratio(high.tokens.color(T::OverlayMarquee), high.tokens.color(T::OverlayMarqueeDash));
    assert!(ratio >= 4.5, "high contrast fixes the pinned marquee: {ratio:.2}");
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
        radius: Some(RadiusSet { control: 0.0, popup: 0.0, window: 0.0, panel: 0.0, button: 0.0, pill: 0.0 }),
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
    def.pins.light = Pins::from_iter([(T::StateSelected, hex("#0A246A")), (T::StateSelectedHover, hex("#e8e8f0"))]);
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
