#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use super::semantic_tests::{app, appearances, grey_blue, hex, over, violet};
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
            assert!(r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
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
            assert!(r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
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
    assert_eq!(ComponentField::ALL.len(), 1 + 4 * 3 * 4 + 2 * 4 + 1 + 2 * 2 * 4);
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
    ] {
        assert_eq!(ComponentField::from_path(bad), None, "{bad}");
    }
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
        assert_eq!(r.warnings, vec![ContrastCheck { fg: text.into(), bg: fill.into(), ratio, minimum: 4.5 }], "{mode:?}");
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
