#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use super::semantic_tests::{app, appearances, grey_blue, hex, over, unexpected_warnings, violet};
use super::*;
use ColorToken as T;

fn button(variant: ButtonVariant, part: ButtonPart, state: State) -> ComponentField {
    ComponentField::Button(ButtonField::Color { variant, part, state })
}

/// The DS-03 §5 button pairs, written out independently of the families' own lists:
/// (fg, bg, standard, high). `None` as bg means `surface.panel`.
fn button_pairs() -> Vec<(ComponentField, Option<ComponentField>, f64, f64)> {
    let mut p = Vec::new();
    for v in ButtonVariant::ALL {
        for s in [State::Rest, State::Hover, State::Pressed] {
            p.push((button(v, ButtonPart::Text, s), Some(button(v, ButtonPart::Fill, s)), 4.5, 7.0));
        }
    }
    p
}

fn pair_ratio(t: &Tokens, fg: ComponentField, bg: Option<ComponentField>) -> f64 {
    let panel = t.color(T::SurfacePanel);
    let bg = over(bg.map_or(panel, |b| t.component.color(b).unwrap()), panel);
    contrast_ratio(over(t.component.color(fg).unwrap(), bg), bg)
}

#[test]
fn button_defaults_meet_every_minimum() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            let t = &r.tokens;
            assert!(unexpected_warnings(&r).iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
            for (fg, bg, std, hi) in button_pairs() {
                let min = if a.contrast == Contrast::High { hi } else { std };
                let ratio = pair_ratio(t, fg, bg);
                assert!(ratio >= min, "{} {a:?}: {} is {ratio:.2}, needs {min}", def.name, fg.path());
            }
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let b = &t.component.button;
    assert_eq!(b.primary.fill.hover, t.color(T::AccentHover));
    assert_eq!(b.danger.fill.pressed, t.color(T::StatusBadSolidPressed));
    assert_eq!(b.colors(ButtonVariant::Secondary).border.get(State::Hover), t.color(T::LineControlEdge));
    assert_eq!(b.secondary.border.rest, t.color(T::LineBorder));
    assert_eq!(b.ghost.fill.rest, Rgba::TRANSPARENT);
    assert_eq!(b.danger.text, States::new(t.color(T::TextOnStatus), t.color(T::TextOnStatus), t.color(T::TextOnStatus), t.color(T::TextDisabled)));
}

/// The icon-button and segmented pairs, written out independently of the families' lists.
fn item_pairs() -> Vec<(ComponentField, ComponentField)> {
    let icon = |part, state| ComponentField::IconButton(IconButtonField::Selected { part, state });
    let segment = |part, state| ComponentField::Segmented(SegmentedField::Segment { part, state });
    let selected = |part, state| ComponentField::Segmented(SegmentedField::Selected { part, state });
    let mut p = vec![(segment(ItemPart::Text, State::Rest), ComponentField::Segmented(SegmentedField::Track))];
    for s in [State::Hover, State::Pressed] {
        p.push((segment(ItemPart::Text, s), segment(ItemPart::Fill, s)));
    }
    for s in [State::Rest, State::Hover, State::Pressed] {
        p.push((icon(ItemPart::Text, s), icon(ItemPart::Fill, s)));
        p.push((selected(ItemPart::Text, s), selected(ItemPart::Fill, s)));
    }
    p
}

#[test]
fn icon_button_and_segmented_defaults_come_from_semantic_tokens_and_meet_their_minimums() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(unexpected_warnings(&r).iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
            let min = if a.contrast == Contrast::High { 7.0 } else { 4.5 };
            for (fg, bg) in item_pairs() {
                let ratio = pair_ratio(&r.tokens, fg, Some(bg));
                assert!(ratio >= min, "{} {a:?}: {} on {} is {ratio:.2}, needs {min}", def.name, fg.path(), bg.path());
            }
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let (icon, seg) = (&t.component.icon_button, &t.component.segmented);
    let control = BySize { sm: t.size.control.sm, md: t.size.control.md };
    let icons = BySize { sm: t.size.icon.sm, md: t.size.icon.md };
    assert_eq!((icon.size, icon.icon, seg.height, seg.icon), (control, icons, control, icons));
    assert_eq!(seg.pad_x, BySize { sm: t.space.pad.control, md: t.space.pad.control + t.space.gap.inline });
    assert_eq!((seg.inset, seg.label), (2.0, TypeRole::Label));
    let selected = ItemColors {
        fill: States::new(t.color(T::StateSelected), t.color(T::StateSelected), t.color(T::StateSelected), t.color(T::StateDisabledFill)),
        text: States::new(t.color(T::TextOnSelected), t.color(T::TextOnSelected), t.color(T::TextOnSelected), t.color(T::TextDisabled)),
    };
    assert_eq!((icon.selected, seg.selected), (selected, selected));
    assert_eq!(seg.track, t.color(T::SurfaceSunken));
    assert_eq!(seg.segment.fill, States::new(Rgba::TRANSPARENT, t.color(T::StateHover), t.color(T::StatePressed), Rgba::TRANSPARENT));
    assert_eq!(seg.segment.text, States::new(t.color(T::TextNormal), t.color(T::TextNormal), t.color(T::TextNormal), t.color(T::TextDisabled)));
}

/// The decided contrast scope: the filled part is an accent state indicator, 3:1 on the track
/// (4.5:1 with high contrast).
#[test]
fn slider_defaults_come_from_semantic_tokens_and_the_fill_meets_the_indicator_minimum() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(unexpected_warnings(&r).iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
            let min = if a.contrast == Contrast::High { 4.5 } else { 3.0 };
            let track = ComponentField::Slider(SliderField::Track);
            for s in [State::Rest, State::Hover, State::Pressed] {
                let fill = ComponentField::Slider(SliderField::Fill(s));
                let ratio = pair_ratio(&r.tokens, fill, Some(track));
                assert!(ratio >= min, "{} {a:?}: {} on the track is {ratio:.2}, needs {min}", def.name, fill.path());
            }
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let s = &t.component.slider;
    let control = BySize { sm: t.size.control.sm, md: t.size.control.md };
    assert_eq!((s.height, s.thumb), (control, BySize { sm: t.size.icon.sm, md: t.size.icon.md }));
    assert_eq!(s.width, BySize { sm: 6.0 * control.sm, md: 6.0 * control.md });
    assert_eq!((s.track_height, s.radius), (4.0, Radius::Full.px()));
    assert_eq!(s.track, t.color(T::SurfaceSunken));
    assert_eq!(s.fill, States::new(t.color(T::AccentSolid), t.color(T::AccentHover), t.color(T::AccentPressed), t.color(T::TextDisabled)));
    let edge = t.color(T::LineControlEdge);
    assert_eq!(
        s.thumb_colors,
        ThumbColors {
            fill: States::new(t.color(T::SurfaceRaised), t.color(T::StateHover), t.color(T::StatePressed), t.color(T::StateDisabledFill)),
            border: States::new(edge, edge, edge, t.color(T::LineDivider)),
        }
    );
}

#[test]
fn a_toggle_button_on_and_disabled_keeps_the_on_fill() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let t = resolve(&def, a, None).tokens;
            let toggle = t.component.toggle_button;
            assert_eq!(toggle.on_disabled(ButtonPart::Fill), t.color(T::StateSelected));
            assert_eq!(toggle.on_disabled(ButtonPart::Border), t.color(T::LineDivider));
            assert_eq!(toggle.on_disabled(ButtonPart::Text), t.color(T::TextDisabled));
            assert_ne!(toggle.on_disabled_fill, t.component.button.secondary.fill.disabled, "{} {a:?}: on looks off", def.name);
        }
    }
}

/// The check pairs, written out independently of the family's list: (fg, bg), at rest only.
fn check_pairs() -> Vec<(ComponentField, TokenRef)> {
    let check = |on_off, part| ComponentField::Check(CheckField::Color { on_off, part, state: State::Rest });
    let mut p = Vec::new();
    for bg in [T::SurfacePanel, T::SurfaceRaised] {
        p.push((check(OnOff::On, CheckPart::Fill), TokenRef::Color(bg)));
        p.push((check(OnOff::Off, CheckPart::Border), TokenRef::Color(bg)));
    }
    for on_off in OnOff::ALL {
        p.push((check(on_off, CheckPart::Mark), TokenRef::Component(check(on_off, CheckPart::Fill))));
    }
    p
}

#[test]
fn check_pairs_are_the_rest_cues_on_the_panel_and_raised_surfaces() {
    let listed: Vec<_> = ComponentTokens::pairs().into_iter().filter(|p| matches!(p.fg, ComponentField::Check(_))).collect();
    assert!(listed.iter().all(|p| p.minimum == values::NON_TEXT_MIN));
    let listed: BTreeSet<_> = listed.into_iter().map(|p| (p.fg, p.bg)).collect();
    let expected: BTreeSet<_> = check_pairs().into_iter().collect();
    assert_eq!(listed, expected);
}

#[test]
fn check_defaults_come_from_semantic_tokens_and_their_marks_and_outlines_meet_the_non_text_minimum() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(unexpected_warnings(&r).iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
            let min = if a.contrast == Contrast::High { 4.5 } else { 3.0 };
            let t = &r.tokens;
            let color = |r: TokenRef| match r {
                TokenRef::Color(c) => t.color(c),
                TokenRef::Component(f) => t.component.color(f).unwrap(),
            };
            // `accent.solid` has a floor on `surface.panel` only, so the default on fill isn't held
            // to the minimum on `surface.raised`; the built-ins meet it there.
            let on_fill = ComponentField::Check(CheckField::Color { on_off: OnOff::On, part: CheckPart::Fill, state: State::Rest });
            for (fg, bg) in check_pairs().into_iter().filter(|&p| p != (on_fill, TokenRef::Color(T::SurfaceRaised))) {
                let bg = over(color(bg), t.color(T::SurfacePanel));
                let ratio = contrast_ratio(over(t.component.color(fg).unwrap(), bg), bg);
                assert!(ratio >= min, "{} {a:?}: {} is {ratio:.2}, needs {min}", def.name, fg.path());
            }
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let c = &t.component.check;
    assert_eq!(c.height, BySize { sm: t.size.control.sm, md: t.size.control.md });
    assert_eq!(c.size, BySize { sm: t.size.icon.sm, md: t.size.icon.md });
    assert_eq!((c.gap, c.label, c.radius, c.thumb_inset), (t.space.gap.inline, TypeRole::Body, 3.0, 2.0));
    assert_eq!((c.mark, c.dot.md, c.track.md), (BySize { sm: 1.5, md: 2.0 }, t.size.icon.md * 0.4, t.size.icon.md * 2.0));
    let outline = States::new(t.color(T::TextFaint), t.color(T::TextMuted), t.color(T::TextMuted), t.color(T::TextDisabled));
    assert_eq!((c.off.border, c.off.mark), (outline, outline));
    let sunken = t.color(T::SurfaceSunken);
    assert_eq!(c.off.fill, States::new(sunken, sunken, t.color(T::StatePressed), t.color(T::StateDisabledFill)));
    assert_eq!(c.on.fill, States::new(t.color(T::AccentSolid), t.color(T::AccentHover), t.color(T::AccentPressed), t.color(T::TextDisabled)));
    assert_eq!(c.on.border, c.on.fill);
    let on_accent = t.color(T::TextOnAccent);
    assert_eq!(c.on.mark, States::new(on_accent, on_accent, on_accent, t.color(T::StateDisabledFill)));
}

/// A disabled switch, check box or radio button that is on must not look off: its track or box
/// and its thumb or mark differ from the off ones.
#[test]
fn a_disabled_check_that_is_on_differs_from_one_that_is_off() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let c = resolve(&def, a, None).tokens.component.check;
            for part in [CheckPart::Fill, CheckPart::Mark] {
                assert_ne!(c.on.part(part).disabled, c.off.part(part).disabled, "{} {a:?}: {} looks off", def.name, part.name());
            }
        }
    }
}

#[test]
fn a_check_box_radius_stays_below_the_cap_and_follows_square_controls() {
    for (size, radius) in [(Size::Compact, 3.0), (Size::Comfortable, 3.0)] {
        let t = resolve(&grey_blue(), Appearance { size, ..app(Mode::Light, Contrast::Standard) }, None).tokens;
        assert!(t.shape.radius.control >= radius);
        assert_eq!(t.component.check.radius, radius, "{size:?}");
    }
    let mut def = grey_blue();
    def.shape = Some(ShapeSeed { radius: Some(RadiusSet { control: 0.0, popup: 0.0, window: 0.0, panel: 0.0, button: 0.0 }), edge: None });
    assert_eq!(resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component.check.radius, 0.0);
}

#[test]
fn button_sizes_follow_the_size_preset() {
    let at = |size| resolve(&grey_blue(), Appearance { size, ..app(Mode::Light, Contrast::Standard) }, None).tokens.component.button;
    let compact = at(Size::Compact);
    assert_eq!((compact.height.sm, compact.height.md), (20.0, 24.0));
    assert_eq!((compact.pad_x.sm, compact.pad_x.md), (10.0, 12.0));
    assert_eq!((compact.icon.get(ControlSize::Sm), compact.icon.get(ControlSize::Md)), (12.0, 16.0));
    assert_eq!(compact.gap, 4.0);
    let comfy = at(Size::Comfortable);
    assert_eq!((comfy.height.sm, comfy.height.md), (24.0, 30.0));
    assert_eq!((comfy.pad_x.sm, comfy.pad_x.md), (14.0, 16.0));
    assert_eq!((comfy.icon.sm, comfy.icon.md), (14.0, 18.0));
    assert_eq!(comfy.gap, 6.0);
    assert_eq!((comfy.label, comfy.radius, comfy.border_width), (TypeRole::Label, Radius::Full.px(), 1.0));
}

#[test]
fn component_paths_round_trip_and_stay_clear_of_semantic_groups() {
    let paths: BTreeSet<_> = ComponentField::ALL.iter().map(|f| f.path()).collect();
    assert_eq!(paths.len(), ComponentField::ALL.len());
    assert_eq!(ComponentField::ALL.len(), 1 + 4 * 3 * 4 + 1 + 2 * 3 * 4 + 2 * 4 + 1 + 2 * 2 * 4 + 2 + 4 + 2 * 4 + 3 + (1 + 6 * 2 * 3) + 3 + 3 + 7);
    for &f in ComponentField::ALL {
        assert_eq!(ComponentField::from_path(&f.path()), Some(f));
        assert_eq!(TokenRef::from_path(&f.path()), Some(TokenRef::Component(f)));
    }
    let groups = ["surface", "text", "line", "accent", "status", "state", "overlay", "data", "caption", "edge"];
    let semantic: BTreeSet<_> = ColorToken::ALL.iter().map(|t| t.path().split('.').next().unwrap()).collect();
    assert!(semantic.iter().all(|g| groups.contains(g)), "{semantic:?}");
    for p in &paths {
        let head = p.split('.').next().unwrap();
        assert!(!groups.contains(&head), "{p}");
    }
    assert_eq!(ComponentField::from_path("button.primary.fill.hover"), Some(button(ButtonVariant::Primary, ButtonPart::Fill, State::Hover)));
    assert_eq!(ComponentField::from_path("button.radius"), Some(ComponentField::Button(ButtonField::Radius)));
    assert_eq!(
        ComponentField::from_path("icon_button.selected.text.hover"),
        Some(ComponentField::IconButton(IconButtonField::Selected { part: ItemPart::Text, state: State::Hover }))
    );
    assert_eq!(ComponentField::from_path("segmented.track"), Some(ComponentField::Segmented(SegmentedField::Track)));
    assert_eq!(
        ComponentField::from_path("segmented.segment.fill.pressed"),
        Some(ComponentField::Segmented(SegmentedField::Segment { part: ItemPart::Fill, state: State::Pressed }))
    );
    assert_eq!(ComponentField::from_path("check.radius"), Some(ComponentField::Check(CheckField::Radius)));
    assert_eq!(
        ComponentField::from_path("check.on.mark.hover"),
        Some(ComponentField::Check(CheckField::Color { on_off: OnOff::On, part: CheckPart::Mark, state: State::Hover }))
    );
    assert_eq!(ComponentField::from_path("slider.radius"), Some(ComponentField::Slider(SliderField::Radius)));
    assert_eq!(ComponentField::from_path("slider.fill.hover"), Some(ComponentField::Slider(SliderField::Fill(State::Hover))));
    assert_eq!(
        ComponentField::from_path("slider.thumb.border.disabled"),
        Some(ComponentField::Slider(SliderField::Thumb { part: ThumbPart::Border, state: State::Disabled }))
    );
    assert_eq!(
        ComponentField::from_path("toggle_button.on_disabled.border"),
        Some(ComponentField::ToggleButton(ToggleButtonField::OnDisabled(ButtonPart::Border)))
    );
    for bad in [
        "button.height",
        "button.pad_x.md",
        "button.primary.fill",
        "button.primary.fill.hover.x",
        "button",
        "button.edge",
        "surface.panel",
        "nope.radius",
        "icon_button.size.sm",
        "icon_button.selected.border.rest",
        "segmented.inset",
        "segmented.track.rest",
        "segmented.selected.fill",
        "slider.thumb",
        "slider.thumb.text.rest",
        "slider.fill",
        "slider.track.rest",
        "slider.height.md",
        "toggle_button.on_disabled",
        "toggle_button.on_disabled.fill.rest",
        "toggle_button.on.fill.disabled",
        "check.size.md",
        "check.mixed.mark.rest",
        "check.on.mark",
        "check.off.thumb.rest",
        "badge.good",
        "badge.good.soft",
        "badge.good.soft.glow",
        "badge.good.hard.fill",
        "badge.error.soft.fill",
        "badge.good.soft.fill.rest",
        "badge.height",
        "kbd.pad_x",
        "kbd.fill.rest",
        "progress.track_height",
        "progress.fill.rest",
        "swatch.edge.outer",
        "swatch.size.md",
        "swatch.mixed",
        "tooltip.max_width",
        "tooltip.fill",
    ] {
        assert_eq!(ComponentField::from_path(bad), None, "{bad}");
    }
    assert_eq!(
        ComponentField::from_path("badge.warn.solid.border"),
        Some(ComponentField::Badge(BadgeField::Color { tone: BadgeTone::Warn, style: BadgeStyle::Solid, part: BadgePart::Border }))
    );
    assert_eq!(ComponentField::from_path("badge.radius"), Some(ComponentField::Badge(BadgeField::Radius)));
    assert_eq!(ComponentField::from_path("kbd.text"), Some(ComponentField::Kbd(KbdField::Text)));
    assert_eq!(ComponentField::from_path("progress.fill"), Some(ComponentField::Progress(ProgressField::Fill)));
    assert_eq!(ComponentField::from_path("swatch.checker.b"), Some(ComponentField::Swatch(SwatchField::Color(SwatchColor::CheckerB))));
    assert_eq!(ComponentField::from_path("swatch.radius"), Some(ComponentField::Swatch(SwatchField::Radius)));
    assert_eq!(TokenRef::from_path("surface.panel"), Some(TokenRef::Color(T::SurfacePanel)));
}

#[test]
fn every_field_reads_back_and_kinds_are_checked() {
    let r = resolve(&grey_blue(), app(Mode::Light, Contrast::Standard), None);
    let mut c = r.tokens.component.clone();
    for &f in ComponentField::ALL {
        let v = c.get(f);
        assert_eq!(v.kind(), f.kind(), "{}", f.path());
        assert_eq!(c.set(f, v), Ok(()));
    }
    assert_eq!(c, r.tokens.component);
    let radius = ComponentField::Button(ButtonField::Radius);
    assert_eq!(
        c.set(radius, OverrideValue::Color(Rgba::WHITE)),
        Err(OverrideError::WrongKind { field: radius, expected: FieldKind::Length, found: FieldKind::Color })
    );
}

#[test]
fn pinned_failing_button_text_warns_in_standard_and_is_fixed_in_high() {
    let text = button(ButtonVariant::Primary, ButtonPart::Text, State::Rest);
    let fill = button(ButtonVariant::Primary, ButtonPart::Fill, State::Rest);
    let mut def = grey_blue();
    let pin = hex("#4060e0");
    def.pins.light = Pins::from_iter([(text, pin)]);
    def.pins.dark = Pins::from_iter([(text, pin)]);
    for mode in [Mode::Light, Mode::Dark] {
        let r = resolve(&def, app(mode, Contrast::Standard), None);
        let t = &r.tokens;
        assert_eq!(t.component.color(text), Some(pin));
        assert_eq!(t.component.button.primary.text.hover, t.color(T::TextOnAccent));
        let ratio = pair_ratio(t, text, Some(fill));
        assert_eq!(unexpected_warnings(&r), vec![ContrastCheck { fg: text.into(), bg: fill.into(), ratio, minimum: 4.5 }], "{mode:?}");
        assert_eq!(r.warnings[0].fg.path(), "button.primary.text.rest");

        let high = resolve(&def, app(mode, Contrast::High), None);
        assert!(high.warnings.is_empty(), "{mode:?}: {:?}", high.warnings);
        assert_ne!(high.tokens.component.color(text), Some(pin));
        assert!(pair_ratio(&high.tokens, text, Some(fill)) >= 7.0);
    }

    // Grey on grey: no text colour reaches 7:1 on the pinned fill, so the fill moves too.
    let ghost_text = button(ButtonVariant::Ghost, ButtonPart::Text, State::Hover);
    let ghost_fill = button(ButtonVariant::Ghost, ButtonPart::Fill, State::Hover);
    let mut def = grey_blue();
    def.high_contrast.light = Pins::from_iter([(ghost_text, hex("#787878")), (ghost_fill, hex("#777777"))]);
    let standard = resolve(&def, app(Mode::Light, Contrast::Standard), None);
    assert!(standard.warnings.is_empty(), "high-contrast pins apply only with high contrast");
    let high = resolve(&def, app(Mode::Light, Contrast::High), None);
    assert!(high.warnings.is_empty(), "{:?}", high.warnings);
    let c = &high.tokens.component;
    assert_ne!(c.color(ghost_fill), Some(hex("#777777")));
    assert!(pair_ratio(&high.tokens, ghost_text, Some(ghost_fill)) >= 7.0);
}

#[test]
fn shape_overrides_apply_and_misplaced_overrides_are_errors() {
    let radius = ComponentField::Button(ButtonField::Radius);
    let fill = button(ButtonVariant::Primary, ButtonPart::Fill, State::Rest);
    let mut def = grey_blue();
    def.components.insert(radius, OverrideValue::Length(4.0));
    assert_eq!(def.override_errors(), vec![]);
    for mode in [Mode::Light, Mode::Dark] {
        let t = resolve(&def, app(mode, Contrast::High), None).tokens;
        assert_eq!(t.component.button.radius, 4.0);
        assert_eq!(t.shape.radius.button, Radius::Full.px(), "the semantic shape is unchanged");
    }

    let mut bad = grey_blue();
    bad.components.insert(fill, OverrideValue::Color(Rgba::WHITE));
    bad.components.insert(radius, OverrideValue::Elevation(Elevation::level(1)));
    bad.pins.dark = Pins::from_iter([(TokenRef::from(radius), Rgba::WHITE)]);
    assert_eq!(
        bad.override_errors(),
        vec![
            OverrideError::PinNotColor(radius),
            OverrideError::WrongKind { field: radius, expected: FieldKind::Length, found: FieldKind::Elevation },
            OverrideError::ColorNotPerMode(fill),
        ]
    );
    assert_eq!(OverrideError::ColorNotPerMode(fill).to_string(), "button.primary.fill.rest: colours are set per mode");
    let t = resolve(&bad, app(Mode::Dark, Contrast::Standard), None).tokens;
    assert_eq!(t.component, resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens.component, "invalid overrides are skipped");
}

#[test]
fn handle_defaults_are_the_same_at_every_size_and_have_no_overridable_path() {
    for def in [grey_blue(), violet()] {
        for size in [Size::Compact, Size::Comfortable] {
            for mode in [Mode::Light, Mode::Dark] {
                let t = resolve(&def, Appearance { size, ..app(mode, Contrast::High) }, None).tokens;
                assert_eq!(t.component.handle, HandleTokens { size: 8.0, stroke: 1.0, halo: 1.0, hit: 14.0 }, "{} {size:?} {mode:?}", def.name);
            }
        }
    }
    assert!(ComponentField::ALL.iter().all(|f| !matches!(f, ComponentField::Handle(_))));
    for path in ["handle.size", "handle.stroke", "handle.halo", "handle.hit", "handle"] {
        assert_eq!(ComponentField::from_path(path), None, "{path}");
        assert_eq!(TokenRef::from_path(path), None, "{path}");
    }
}

/// Every pair of these families, measured on the final tokens of every appearance: badge labels on
/// their fills and keys on keycaps are text (4.5:1, 7:1 high); the progress fill on its track, the
/// selection ring on the panel and the mixed dash on its well are indicators (3:1, 4.5:1 high).
#[test]
fn small_control_defaults_come_from_semantic_tokens_and_their_pairs_meet_the_minimums() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(unexpected_warnings(&r).iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
            let t = &r.tokens;
            let (text_min, indicator_min) = if a.contrast == Contrast::High { (7.0, 4.5) } else { (4.5, 3.0) };
            for tone in BadgeTone::ALL {
                for style in BadgeStyle::ALL {
                    let field = |part| ComponentField::Badge(BadgeField::Color { tone, style, part });
                    let ratio = pair_ratio(t, field(BadgePart::Text), Some(field(BadgePart::Fill)));
                    assert!(ratio >= text_min, "{} {a:?}: badge {tone:?} {style:?} text is {ratio:.2}, needs {text_min}", def.name);
                }
            }
            let kbd = |f| ComponentField::Kbd(f);
            let ratio = pair_ratio(t, kbd(KbdField::Text), Some(kbd(KbdField::Fill)));
            assert!(ratio >= text_min, "{} {a:?}: kbd text is {ratio:.2}", def.name);
            let progress = |f| ComponentField::Progress(f);
            let ratio = pair_ratio(t, progress(ProgressField::Fill), Some(progress(ProgressField::Track)));
            assert!(ratio >= indicator_min, "{} {a:?}: progress fill is {ratio:.2}", def.name);
            let swatch = |c| ComponentField::Swatch(SwatchField::Color(c));
            let ratio = pair_ratio(t, swatch(SwatchColor::Selected), None);
            assert!(ratio >= indicator_min, "{} {a:?}: swatch selection ring is {ratio:.2}", def.name);
            let ratio = pair_ratio(t, swatch(SwatchColor::MixedMark), Some(swatch(SwatchColor::MixedFill)));
            assert!(ratio >= indicator_min, "{} {a:?}: swatch mixed dash is {ratio:.2}", def.name);
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let c = &t.component;
    let status = |s: &StatusColors| (s.bg, s.fg, s.border);
    for (tone, colors) in [
        (BadgeTone::Good, &t.color.status.good),
        (BadgeTone::Warn, &t.color.status.warn),
        (BadgeTone::Bad, &t.color.status.bad),
        (BadgeTone::Info, &t.color.status.info),
    ] {
        let soft = c.badge.colors(tone, BadgeStyle::Soft);
        assert_eq!((soft.fill, soft.text, soft.border), status(colors), "{tone:?} soft");
        let solid = c.badge.colors(tone, BadgeStyle::Solid);
        assert_eq!((solid.fill, solid.text, solid.border), (colors.solid, t.color(T::TextOnStatus), colors.solid), "{tone:?} solid");
    }
    let accent = c.badge.colors(BadgeTone::Accent, BadgeStyle::Solid);
    assert_eq!((accent.fill, accent.text), (t.color(T::AccentSolid), t.color(T::TextOnAccent)));
    let soft = c.badge.colors(BadgeTone::Accent, BadgeStyle::Soft);
    assert_eq!((soft.fill, soft.text), (t.color(T::StateSelected), t.color(T::TextOnSelected)));
    let neutral = c.badge.colors(BadgeTone::Neutral, BadgeStyle::Soft);
    assert_eq!((neutral.fill, neutral.text, neutral.border), (t.color(T::SurfaceSunken), t.color(T::TextMuted), t.color(T::LineBorder)));
    let caption = t.typography.caption.line_height;
    assert_eq!((c.badge.height, c.badge.pad_y, c.badge.radius, c.badge.label), (caption + 2.0, 1.0, Radius::Full.px(), TypeRole::Caption));
    assert_eq!((c.kbd.height, c.kbd.pad_y, c.kbd.label), (caption + 2.0, 1.0, TypeRole::Caption));
    assert_eq!((c.kbd.fill, c.kbd.text), (t.color(T::SurfaceRaised), t.color(T::TextMuted)));
    assert_eq!((c.progress.track, c.progress.fill, c.progress.track_height), (t.color(T::SurfaceSunken), t.color(T::AccentSolid), 4.0));
    let s = &c.swatch;
    assert_eq!(s.size, BySize { sm: t.size.control.sm, md: t.size.control.md });
    assert_eq!((s.edge, s.selected), (t.color(T::LineControlEdge), t.color(T::AccentSolid)));
    assert_eq!((s.selected_gap, s.selected_width), (1.0, 2.0));
    assert_eq!((s.mixed_fill, s.mixed_mark), (t.color(T::SurfaceSunken), t.color(T::TextMuted)));
    assert_eq!((s.checker_a, s.checker_b), (Rgba::rgb(64, 64, 64), Rgba::rgb(102, 102, 102)), "dark greys in a dark theme");
    let light = resolve(&grey_blue(), app(Mode::Light, Contrast::Standard), None).tokens.component.swatch;
    assert_eq!((light.checker_a, light.checker_b), (Rgba::WHITE, Rgba::rgb(204, 204, 204)), "light greys in a light theme");
}

/// Badges and keycaps hold one caption line, so they grow with the Text size setting.
#[test]
fn badge_and_keycap_heights_follow_the_text_size() {
    for size in [Size::Compact, Size::Comfortable] {
        let at = |text_size| resolve(&grey_blue(), Appearance { size, text_size, ..app(Mode::Light, Contrast::Standard) }, None).tokens;
        let (default, large) = (at(TextSize::Default), at(TextSize::Large));
        for t in [&default, &large] {
            let line = t.typography.caption.line_height;
            assert_eq!(t.component.badge.height, line + 2.0 * t.component.badge.pad_y, "{size:?}");
            assert_eq!(t.component.kbd.height, line + 2.0 * t.component.kbd.pad_y, "{size:?}");
        }
        assert!(large.component.badge.height > default.component.badge.height, "{size:?}");
        assert!(large.component.kbd.height > default.component.kbd.height, "{size:?}");
    }
}

/// The ring inside a swatch's edge stands out from the well's colour, measured where the colour
/// equals the panel (the outer edge then may not) in every built-in's light and dark panel.
#[test]
fn a_swatch_inner_edge_stands_out_from_a_colour_equal_to_the_panel() {
    for theme in builtins() {
        for mode in [Mode::Light, Mode::Dark] {
            let t = resolve(theme, app(mode, Contrast::Standard), None).tokens;
            let panel = t.color(T::SurfacePanel);
            let ring = t.component.swatch.inner_edge(panel);
            let ratio = contrast_ratio(over(ring, panel), panel);
            assert!(ratio >= SWATCH_INNER_EDGE_MIN, "{} {mode:?}: the inner edge on {} is {ratio:.2}:1", theme.id, panel.to_hex());
        }
    }
    let swatch = resolve(&grey_blue(), app(Mode::Light, Contrast::Standard), None).tokens.component.swatch;
    let alpha = |c: Rgba| c.with_alpha(0xff);
    assert_eq!(alpha(swatch.inner_edge(Rgba::WHITE)), Rgba::BLACK, "dark on light");
    assert_eq!(alpha(swatch.inner_edge(Rgba::BLACK)), Rgba::WHITE, "light on dark");
    assert_eq!(alpha(swatch.inner_edge(Rgba::TRANSPARENT)), Rgba::BLACK, "a clear colour shows the light checkerboard");
}

/// The ratio the inner edge reaches on every built-in panel: the non-text minimum, as the ring is
/// what shows the well there.
const SWATCH_INNER_EDGE_MIN: f64 = 3.0;

#[test]
fn swatch_and_keycap_corners_are_the_control_radius_or_their_override_up_to_their_caps() {
    let at = |size| resolve(&grey_blue(), Appearance { size, ..app(Mode::Light, Contrast::Standard) }, None).tokens.component;
    let comfy = at(Size::Comfortable);
    assert_eq!((comfy.swatch.radius, comfy.kbd.radius), (4.0, 3.0));
    let mut def = grey_blue();
    def.shape = Some(ShapeSeed { radius: Some(RadiusSet { control: 2.0, popup: 0.0, window: 0.0, panel: 0.0, button: 0.0 }), edge: None });
    let t = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component;
    assert_eq!((t.swatch.radius, t.kbd.radius), (2.0, 2.0));

    let mut def = grey_blue();
    def.components.insert(ComponentField::Swatch(SwatchField::Radius), OverrideValue::Length(10.0));
    def.components.insert(ComponentField::Kbd(KbdField::Radius), OverrideValue::Length(10.0));
    let t = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component;
    assert_eq!((t.swatch.radius, t.kbd.radius), (4.0, 3.0), "overrides are capped too");
    def.components.insert(ComponentField::Swatch(SwatchField::Radius), OverrideValue::Length(1.0));
    let t = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component;
    assert_eq!(t.swatch.radius, 1.0);
}

#[test]
fn tooltips_have_a_width_and_no_overridable_path() {
    let t = resolve(&grey_blue(), app(Mode::Light, Contrast::High), None).tokens;
    assert_eq!(t.component.tooltip.max_width, 320.0);
    assert!(ComponentField::ALL.iter().all(|f| !matches!(f, ComponentField::Tooltip(_))));
}
