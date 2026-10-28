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

/// The decided contrast scope: the filled part is an accent state indicator, 3:1 on the track
/// (4.5:1 with high contrast).
#[test]
fn slider_defaults_come_from_semantic_tokens_and_the_fill_meets_the_indicator_minimum() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
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
    assert_eq!(s.thumb_colors, knob(&t));
    let (clear, soft) = (Rgba::TRANSPARENT, t.color(T::AccentSoft));
    assert_eq!((s.ring, s.ring_width), (States::new(clear, soft, soft, clear), 2.0));
}

#[test]
fn curve_defaults_come_from_semantic_tokens_and_the_point_outline_meets_the_indicator_minimum() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
            let min = if a.contrast == Contrast::High { 4.5 } else { 3.0 };
            let (stroke, fill) = (ComponentField::Curve(CurveField::PointStroke), ComponentField::Curve(CurveField::Fill));
            let ratio = pair_ratio(&r.tokens, stroke, Some(fill));
            assert!(ratio >= min, "{} {a:?}: the point outline on the plot is {ratio:.2}, needs {min}", def.name);
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let c = &t.component.curve;
    assert_eq!((c.fill, c.border), (t.component.field.fill.rest, t.component.field.border.rest));
    assert_eq!((c.grid, c.baseline), (t.color(T::LineDivider), t.color(T::LineBorder)));
    assert_eq!((c.point_fill, c.point_stroke), (t.color(T::SurfaceSunken), t.color(T::TextNormal)));
    assert_eq!((c.width, c.other_width, c.other_alpha), (1.5, 1.0, 0.8));
    // The Compact sizes scale with the control height.
    let k = t.size.control.md / 24.0;
    let scaled = |v: f32| (v * k).round();
    assert_eq!((c.min_side, c.max_side, c.pad), (scaled(120.0), scaled(300.0), scaled(5.0)));
    assert_eq!((c.bar, c.bar_gap, c.hit, c.drag_off), (scaled(8.0), scaled(4.0), scaled(9.0), scaled(12.0)));
}

/// The channel colours a curve is drawn in are checked on `curve.fill`, not only on the surface
/// it defaults to: a pinned fill that hides them warns, and high contrast moves the fill.
#[test]
fn a_pinned_curve_fill_is_checked_against_the_channel_colours() {
    let fill = ComponentField::Curve(CurveField::Fill);
    let mut def = grey_blue();
    let red = resolve(&def, app(Mode::Dark, Contrast::Standard), None).tokens.color(T::DataRed);
    def.pins.dark.0.insert(TokenRef::Component(fill), red);
    let r = resolve(&def, app(Mode::Dark, Contrast::Standard), None);
    let warned = |r: &crate::Resolved| r.warnings.iter().any(|w| w.fg == TokenRef::Color(T::DataRed) && w.bg == TokenRef::Component(fill));
    assert!(warned(&r), "{:?}", r.warnings);
    let r = resolve(&def, app(Mode::Dark, Contrast::High), None);
    let ratio = contrast_ratio(r.tokens.color(T::DataRed), r.tokens.component.color(fill).unwrap());
    assert!(ratio >= 4.5 && !warned(&r), "{ratio:.2}: {:?}", r.warnings);
}

#[test]
fn angle_dial_defaults_come_from_semantic_tokens_and_the_needle_meets_the_face() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
            let min = if a.contrast == Contrast::High { 4.5 } else { 3.0 };
            let (needle, face) = (ComponentField::AngleDial(AngleDialField::Needle), ComponentField::AngleDial(AngleDialField::Face));
            let ratio = pair_ratio(&r.tokens, needle, Some(face));
            assert!(ratio >= min, "{} {a:?}: the needle on the face is {ratio:.2}, needs {min}", def.name);
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let d = &t.component.angle_dial;
    assert_eq!((d.face, d.edge), (t.color(T::SurfaceSunken), t.color(T::LineControlEdge)));
    assert_eq!((d.needle, d.hub_fill), (t.color(T::AccentSolid), t.color(T::TextMuted)));
    // The Compact sizes scale with the control height.
    let k = t.size.control.md / 24.0;
    let scaled = |v: f32| (v * k).round();
    assert_eq!((d.size, d.needle_width, d.needle_inset), (scaled(42.0), scaled(2.0), scaled(4.0)));
    assert_eq!((d.hub, d.dead_zone), (scaled(2.0), scaled(2.0)));
}

/// The knob look of slider and switch thumbs: `surface.knob` with a `line.knob_edge` outline while
/// enabled; disabled, a face halfway to `surface.panel` with a `line.divider` outline.
fn knob(t: &Tokens) -> ThumbColors {
    let (face, edge) = (t.color(T::SurfaceKnob), t.color(T::LineKnobEdge));
    ThumbColors {
        fill: States::new(face, face, face, crate::derive::mix(face, t.color(T::SurfacePanel), 0.5)),
        border: States::new(edge, edge, edge, t.color(T::LineDivider)),
    }
}

/// The field pairs, written out independently of the family's list: (fg, bg, minimum), at rest.
fn field_pairs() -> Vec<(ComponentField, ComponentField, (f64, f64))> {
    let field = ComponentField::Field;
    let fill = field(FieldField::Color { part: FieldPart::Fill, disabled: false });
    vec![
        (field(FieldField::Color { part: FieldPart::Text, disabled: false }), fill, (4.5, 7.0)),
        (field(FieldField::Color { part: FieldPart::Placeholder, disabled: false }), fill, (4.5, 7.0)),
        (field(FieldField::Color { part: FieldPart::Unit, disabled: false }), fill, (4.5, 7.0)),
        (field(FieldField::Color { part: FieldPart::Scrub, disabled: false }), fill, (4.5, 7.0)),
        (field(FieldField::Border(BorderState::Focused)), fill, (3.0, 4.5)),
        (field(FieldField::Invalid), fill, (3.0, 4.5)),
        (field(FieldField::SelectedText), field(FieldField::Selection), (4.5, 7.0)),
    ]
}

#[test]
fn field_defaults_come_from_semantic_tokens_and_their_pairs_meet_their_minimums() {
    let listed: Vec<_> = ComponentTokens::pairs()
        .into_iter()
        .filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Field(_))))
        .map(|p| (p.fg, p.bg, p.minimum))
        .collect();
    let expected: Vec<_> = field_pairs().into_iter().map(|(fg, bg, min)| (TokenRef::Component(fg), TokenRef::Component(bg), min)).collect();
    assert_eq!(listed, expected);
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
            for (fg, bg, (standard, high)) in field_pairs() {
                let min = if a.contrast == Contrast::High { high } else { standard };
                let ratio = pair_ratio(&r.tokens, fg, Some(bg));
                assert!(ratio >= min, "{} {a:?}: {} is {ratio:.2}, needs {min}", def.name, fg.path());
            }
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let f = &t.component.field;
    let control = BySize { sm: t.size.control.sm, md: t.size.control.md };
    assert_eq!((f.height, f.width), (control, BySize { sm: 8.0 * control.sm, md: 8.0 * control.md }));
    assert_eq!(f.number_width, BySize { sm: 4.0 * control.sm, md: 4.0 * control.md });
    assert_eq!(f.icon, BySize { sm: t.size.icon.sm, md: t.size.icon.md });
    assert_eq!((f.pad_x, f.gap, f.radius, f.role), (t.space.pad.control, t.space.gap.inline, t.shape.radius.control, TypeRole::Body));
    assert_eq!(f.fill, Dimmed { rest: t.color(T::SurfaceSunken), disabled: t.color(T::StateDisabledFill) });
    let border = BorderColors {
        rest: t.color(T::LineControlEdge),
        hover: t.color(T::TextFaint),
        focused: t.color(T::LineFocus),
        disabled: t.color(T::LineDivider),
    };
    assert_eq!(f.border, border);
    assert_eq!(f.text, Dimmed { rest: t.color(T::TextNormal), disabled: t.color(T::TextDisabled) });
    assert_eq!(f.placeholder, Dimmed { rest: t.color(T::TextFaint), disabled: t.color(T::TextDisabled) });
    assert_eq!((f.unit, f.scrub), (Dimmed { rest: t.color(T::TextMuted), disabled: t.color(T::TextDisabled) }, f.unit));
    assert_eq!((f.selection, f.selected_text), (t.color(T::StateSelected), t.color(T::TextOnSelected)));
    assert_eq!((f.caret, f.invalid), (t.color(T::TextNormal), t.color(T::StatusBadFg)));
}

fn listbox_pairs() -> Vec<(ListboxField, TokenRef, (f64, f64))> {
    let popup = TokenRef::Color(T::SurfaceRaised);
    let active = TokenRef::Component(ComponentField::Listbox(ListboxField::ActiveFill));
    vec![
        (ListboxField::Text { disabled: false }, popup, (4.5, 7.0)),
        (ListboxField::ChosenText, popup, (4.5, 7.0)),
        (ListboxField::Group, popup, (4.5, 7.0)),
        (ListboxField::ActiveText, active, (4.5, 7.0)),
        (ListboxField::Mark, popup, (3.0, 4.5)),
        (ListboxField::Mark, active, (3.0, 4.5)),
    ]
}

#[test]
fn listbox_defaults_come_from_semantic_tokens_and_their_pairs_meet_their_minimums() {
    let listed: Vec<_> = ComponentTokens::pairs()
        .into_iter()
        .filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Listbox(_))))
        .map(|p| (p.fg, p.bg, p.minimum))
        .collect();
    let expected: Vec<_> = listbox_pairs().into_iter().map(|(fg, bg, min)| (TokenRef::Component(ComponentField::Listbox(fg)), bg, min)).collect();
    assert_eq!(listed, expected);
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
            let t = &r.tokens;
            for (fg, bg, (standard, high)) in listbox_pairs() {
                let min = if a.contrast == Contrast::High { high } else { standard };
                let bg = match bg {
                    TokenRef::Color(token) => t.color(token),
                    TokenRef::Component(field) => over(t.component.color(field).unwrap(), t.color(T::SurfaceRaised)),
                };
                let ratio = contrast_ratio(over(t.component.color(ComponentField::Listbox(fg)).unwrap(), bg), bg);
                assert!(ratio >= min, "{} {a:?}: {} is {ratio:.2}, needs {min}", def.name, ComponentField::Listbox(fg).path());
            }
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let l = &t.component.listbox;
    assert_eq!(l.height, BySize { sm: t.size.control.sm, md: t.size.control.md });
    assert_eq!(l.icon, BySize { sm: t.size.icon.sm, md: t.size.icon.md });
    assert_eq!((l.pad_x, l.gap, l.separator_pad, l.radius), (t.space.pad.control, t.space.gap.inline, t.space.gap.inline, t.shape.radius.control));
    assert_eq!((l.label, l.group_label), (TypeRole::Body, TypeRole::Caption));
    assert_eq!(l.text, Dimmed { rest: t.color(T::TextNormal), disabled: t.color(T::TextDisabled) });
    assert_eq!((l.active_fill, l.active_text), (t.color(T::StateHover), t.color(T::TextNormal)));
    assert_eq!((l.chosen_text, l.mark), (t.color(T::TextNormal), t.color(T::TextNormal)));
    assert_eq!((l.group, l.separator), (t.color(T::TextMuted), t.color(T::LineDivider)));
}

/// The active menu row is `accent.strong` with `text.on_strong` (owner, 2026-10-28), the shortcuts are
/// muted text on the popup surface, and the sizes scale PhotoCraft's Compact ones with the control
/// height.
#[test]
fn menu_defaults_come_from_semantic_tokens_and_their_pairs_meet_their_minimums() {
    let listed: Vec<_> =
        ComponentTokens::pairs().into_iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Menu(_)))).map(|p| (p.fg, p.bg)).collect();
    let menu = |f| TokenRef::Component(ComponentField::Menu(f));
    let expected = vec![(menu(MenuField::ActiveText), menu(MenuField::ActiveFill)), (menu(MenuField::Shortcut), TokenRef::Color(T::SurfaceRaised))];
    assert_eq!(listed, expected);
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let t = resolve(&def, a, None).tokens;
            let m = &t.component.menu;
            assert_eq!((m.active_fill, m.active_text, m.shortcut), (t.color(T::AccentStrong), t.color(T::TextOnStrong), t.color(T::TextMuted)));
            let k = t.size.control.md / 24.0;
            assert_eq!((m.min_width, m.shortcut_gap, m.scroll_strip), ((220.0 * k).round(), (16.0 * k).round(), (16.0 * k).round()), "{a:?}");
        }
    }
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
    let on = |part| ComponentField::Check(CheckField::On { part, state: State::Rest });
    let off_border = ComponentField::Check(CheckField::Off { part: CheckOffPart::Border, state: State::Rest });
    let mut p = Vec::new();
    for bg in [T::SurfacePanel, T::SurfaceRaised] {
        p.push((on(CheckOnPart::Fill), TokenRef::Color(bg)));
        p.push((off_border, TokenRef::Color(bg)));
    }
    p.push((on(CheckOnPart::Mark), TokenRef::Component(on(CheckOnPart::Fill))));
    p
}

#[test]
fn check_pairs_are_the_rest_cues_on_the_panel_and_raised_surfaces() {
    let listed: Vec<_> = ComponentTokens::pairs().into_iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Check(_)))).collect();
    assert!(listed.iter().all(|p| p.minimum == values::NON_TEXT_MIN));
    let listed: BTreeSet<_> = listed.into_iter().map(|p| (p.fg, p.bg)).collect();
    let expected: BTreeSet<_> = check_pairs().into_iter().map(|(fg, bg)| (TokenRef::Component(fg), bg)).collect();
    assert_eq!(listed, expected);
}

#[test]
fn check_defaults_come_from_semantic_tokens_and_their_marks_and_outlines_meet_the_non_text_minimum() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
            let min = if a.contrast == Contrast::High { 4.5 } else { 3.0 };
            let t = &r.tokens;
            let color = |r: TokenRef| match r {
                TokenRef::Color(c) => t.color(c),
                TokenRef::Component(f) => t.component.color(f).unwrap(),
            };
            for (fg, bg) in check_pairs() {
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
    assert_eq!(c.off.border, outline);
    let sunken = t.color(T::SurfaceSunken);
    assert_eq!(c.off.fill, States::new(sunken, sunken, t.color(T::StatePressed), t.color(T::StateDisabledFill)));
    assert_eq!(c.on.fill, States::new(t.color(T::AccentSolid), t.color(T::AccentHover), t.color(T::AccentPressed), t.color(T::TextDisabled)));
    assert_eq!(c.on.border, c.on.fill);
    let on_accent = t.color(T::TextOnAccent);
    assert_eq!(c.on.mark, States::new(on_accent, on_accent, on_accent, t.color(T::StateDisabledFill)));
    assert_eq!(c.thumb, knob(&t));
}

/// A disabled switch, check box or radio button that is on must not look off: its track or box
/// differs from the off one, and its mark shows on it (off has no mark; a switch thumb is the
/// same off and on).
#[test]
fn a_disabled_check_that_is_on_differs_from_one_that_is_off() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let c = resolve(&def, a, None).tokens.component.check;
            assert_ne!(c.on.fill.disabled, c.off.fill.disabled, "{} {a:?}: the fill looks off", def.name);
            assert_ne!(c.on.mark.disabled, c.on.fill.disabled, "{} {a:?}: the mark doesn't show", def.name);
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
    def.shape = Some(ShapeSeed { radius: Some(RadiusSet { control: 0.0, popup: 0.0, window: 0.0, panel: 0.0, button: 0.0, pill: 0.0 }), edge: None });
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

/// An override of any listed colour field reads back as set: no field is listed that the tokens
/// can't hold.
#[test]
fn every_colour_field_holds_its_override() {
    let mut tokens = resolve(&grey_blue(), app(Mode::Light, Contrast::Standard), None).tokens.component;
    let value = Rgba([1, 2, 3, 4]);
    for &f in ComponentField::ALL.iter().filter(|f| f.kind() == FieldKind::Color) {
        tokens.set(f, OverrideValue::Color(value)).unwrap();
        assert_eq!(tokens.get(f), OverrideValue::Color(value), "{}", f.path());
    }
}

#[test]
fn component_paths_round_trip_and_stay_clear_of_semantic_groups() {
    let paths: BTreeSet<_> = ComponentField::ALL.iter().map(|f| f.path()).collect();
    assert_eq!(paths.len(), ComponentField::ALL.len());
    assert_eq!(
        ComponentField::ALL.len(),
        1 + 4 * 3 * 4
            + 1
            + (2 + 3) * 4
            + 2 * 4
            + 2 * 4
            + 1
            + 2 * 2 * 4
            + 2
            + 4
            + 2 * 4
            + 4
            + 3
            + (1 + 6 * 2 * 3)
            + 3
            + 3
            + 7
            + (1 + 5 * 2 + 4 + 4)
            + (1 + 2 + 6)
            // Toast: radius, fill, line, more, and edge and title per tone; notice: a colour per tone; empty state: text.
            + (4 + 2 * 5)
            + 5
            + 1
            + 6
            + (1 + 17)
            + 3
            // Colour picker: the marker and its halo.
            + 2
            // Angle dial: face, edge, needle and hub.
            + 4
    );
    for &f in ComponentField::ALL {
        assert_eq!(ComponentField::from_path(&f.path()), Some(f));
        assert_eq!(TokenRef::from_path(&f.path()), Some(TokenRef::Component(f)));
    }
    // Every check colour field the types can express is listed, so none lacks a path.
    for state in State::ALL {
        let mut check: Vec<_> = CheckOffPart::ALL.map(|part| CheckField::Off { part, state }).into();
        check.extend(CheckOnPart::ALL.map(|part| CheckField::On { part, state }));
        check.extend(ThumbPart::ALL.map(|part| CheckField::Thumb { part, state }));
        for f in check {
            assert!(ComponentField::ALL.contains(&ComponentField::Check(f)), "{f:?}");
        }
    }
    let groups = ["surface", "text", "line", "accent", "status", "state", "overlay", "data", "caption", "edge"];
    let semantic: BTreeSet<_> = ColorToken::ALL.iter().map(|t| t.path().split('.').next().unwrap()).collect();
    assert!(semantic.iter().all(|g| groups.contains(g)), "{semantic:?}");
    for p in &paths {
        let head = p.split('.').next().unwrap();
        assert!(!groups.contains(&head), "{p}");
    }
    assert_eq!(TokenRef::from_path("surface.panel"), Some(TokenRef::Color(T::SurfacePanel)));
}

#[test]
fn component_paths_name_their_fields_and_refuse_near_misses() {
    let named = [
        ("button.primary.fill.hover", button(ButtonVariant::Primary, ButtonPart::Fill, State::Hover)),
        ("button.radius", ComponentField::Button(ButtonField::Radius)),
        ("icon_button.selected.text.hover", ComponentField::IconButton(IconButtonField::Selected { part: ItemPart::Text, state: State::Hover })),
        ("segmented.track", ComponentField::Segmented(SegmentedField::Track)),
        ("segmented.segment.fill.pressed", ComponentField::Segmented(SegmentedField::Segment { part: ItemPart::Fill, state: State::Pressed })),
        ("check.radius", ComponentField::Check(CheckField::Radius)),
        ("check.on.mark.hover", ComponentField::Check(CheckField::On { part: CheckOnPart::Mark, state: State::Hover })),
        ("check.off.border.disabled", ComponentField::Check(CheckField::Off { part: CheckOffPart::Border, state: State::Disabled })),
        ("check.thumb.fill.pressed", ComponentField::Check(CheckField::Thumb { part: ThumbPart::Fill, state: State::Pressed })),
        ("field.radius", ComponentField::Field(FieldField::Radius)),
        ("field.placeholder.disabled", ComponentField::Field(FieldField::Color { part: FieldPart::Placeholder, disabled: true })),
        ("field.border.focused", ComponentField::Field(FieldField::Border(BorderState::Focused))),
        ("field.selection.text", ComponentField::Field(FieldField::SelectedText)),
        ("field.unit.disabled", ComponentField::Field(FieldField::Color { part: FieldPart::Unit, disabled: true })),
        ("field.scrub.rest", ComponentField::Field(FieldField::Color { part: FieldPart::Scrub, disabled: false })),
        ("slider.radius", ComponentField::Slider(SliderField::Radius)),
        ("curve.point_stroke", ComponentField::Curve(CurveField::PointStroke)),
        ("slider.fill.hover", ComponentField::Slider(SliderField::Fill(State::Hover))),
        ("slider.thumb.border.disabled", ComponentField::Slider(SliderField::Thumb { part: ThumbPart::Border, state: State::Disabled })),
        ("slider.ring.hover", ComponentField::Slider(SliderField::Ring(State::Hover))),
        ("toggle_button.on_disabled.border", ComponentField::ToggleButton(ToggleButtonField::OnDisabled(ButtonPart::Border))),
        (
            "badge.warn.solid.border",
            ComponentField::Badge(BadgeField::Color { tone: BadgeTone::Warn, style: BadgeStyle::Solid, part: BadgePart::Border }),
        ),
        ("badge.radius", ComponentField::Badge(BadgeField::Radius)),
        ("kbd.text", ComponentField::Kbd(KbdField::Text)),
        ("progress.fill", ComponentField::Progress(ProgressField::Fill)),
        ("swatch.checker.b", ComponentField::Swatch(SwatchField::Color(SwatchColor::CheckerB))),
        ("swatch.radius", ComponentField::Swatch(SwatchField::Radius)),
        ("listbox.radius", ComponentField::Listbox(ListboxField::Radius)),
        ("listbox.text.disabled", ComponentField::Listbox(ListboxField::Text { disabled: true })),
        ("listbox.active.fill", ComponentField::Listbox(ListboxField::ActiveFill)),
        ("listbox.chosen.text", ComponentField::Listbox(ListboxField::ChosenText)),
        ("listbox.mark", ComponentField::Listbox(ListboxField::Mark)),
    ];
    for (path, field) in named {
        assert_eq!(ComponentField::from_path(path), Some(field), "{path}");
    }
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
        "slider.ring",
        "slider.ring_width",
        "slider.thumb.ring.hover",
        "toggle_button.on_disabled",
        "toggle_button.on_disabled.fill.rest",
        "toggle_button.on.fill.disabled",
        "check.size.md",
        "check.mixed.mark.rest",
        "check.on.mark",
        "check.off.thumb.rest",
        "check.off.mark.rest",
        "check.thumb.mark.rest",
        "check.thumb.fill",
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
        "field.fill",
        "field.fill.hover",
        "field.border.pressed",
        "field.selection",
        "field.caret.rest",
        "field.height.md",
        "field.invalid.rest",
        "field.unit",
        "field.scrub.hover",
        "listbox.height.md",
        "listbox.text",
        "listbox.active",
        "listbox.active.fill.rest",
        "listbox.chosen.fill",
        "listbox.mark.rest",
        "listbox.group.text",
        "listbox.separator_pad",
    ] {
        assert_eq!(ComponentField::from_path(bad), None, "{bad}");
    }
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

/// Every pair of these families, measured on the final tokens of every appearance: badge labels on
/// their fills and keys on keycaps are text (4.5:1, 7:1 high); the progress fill on its track, the
/// selection ring on the panel and the mixed dash on its well are indicators (3:1, 4.5:1 high).
#[test]
fn small_control_defaults_come_from_semantic_tokens_and_their_pairs_meet_the_minimums() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))), "{} {a:?}: {:?}", def.name, r.warnings);
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
fn swatch_keycap_and_check_corners_are_the_control_radius_or_their_override_up_to_their_caps() {
    let at = |size| resolve(&grey_blue(), Appearance { size, ..app(Mode::Light, Contrast::Standard) }, None).tokens.component;
    let comfy = at(Size::Comfortable);
    assert_eq!((comfy.swatch.radius, comfy.kbd.radius), (4.0, 3.0));
    let mut def = grey_blue();
    def.shape = Some(ShapeSeed { radius: Some(RadiusSet { control: 2.0, popup: 0.0, window: 0.0, panel: 0.0, button: 0.0, pill: 0.0 }), edge: None });
    let t = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component;
    assert_eq!((t.swatch.radius, t.kbd.radius), (2.0, 2.0));

    let mut def = grey_blue();
    def.components.insert(ComponentField::Swatch(SwatchField::Radius), OverrideValue::Length(10.0));
    def.components.insert(ComponentField::Kbd(KbdField::Radius), OverrideValue::Length(10.0));
    def.components.insert(ComponentField::Check(CheckField::Radius), OverrideValue::Length(10.0));
    let t = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component;
    assert_eq!((t.swatch.radius, t.kbd.radius, t.check.radius), (4.0, 3.0, 3.0), "overrides are capped too");
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

fn list_pairs() -> Vec<(ListField, ListField, (f64, f64))> {
    let (text, high) = ((4.5, 7.0), (3.0, 4.5));
    vec![
        (ListField::Text, ListField::Hover, text),
        (ListField::SelectedText, ListField::Selected, text),
        (ListField::SelectedText, ListField::SelectedHover, text),
        (ListField::SelectedDimmed, ListField::Selected, text),
        (ListField::SelectedDimmed, ListField::SelectedHover, text),
        (ListField::SelectedDetail, ListField::Selected, text),
        (ListField::SelectedDetail, ListField::SelectedHover, text),
        (ListField::Text, ListField::GhostFill, text),
        (ListField::Drop, ListField::DropHalo, high),
    ]
}

/// The drop marker, the only cue of where a drop lands, and the labels meet their minimums in
/// every appearance; the fills they sit on are design targets.
#[test]
fn list_pairs_meet_their_minimums_in_every_appearance() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(r.warnings.iter().all(|w| !matches!(w.fg, TokenRef::Component(ComponentField::List(_)))), "{} {a:?}: {:?}", def.name, r.warnings);
            let t = &r.tokens;
            for (fg, bg, (standard, high)) in list_pairs() {
                let min = if a.contrast == Contrast::High { high } else { standard };
                let bg = over(t.component.color(ComponentField::List(bg)).unwrap(), t.color(T::SurfacePanel));
                let ratio = contrast_ratio(over(t.component.color(ComponentField::List(fg)).unwrap(), bg), bg);
                assert!(ratio >= min, "{} {a:?}: {} is {ratio:.2}, needs {min}", def.name, ComponentField::List(fg).path());
            }
        }
    }
}

#[test]
fn list_sizes_follow_the_size_preset_and_colours_the_semantic_tokens() {
    let at = |size| resolve(&grey_blue(), Appearance { size, ..app(Mode::Dark, Contrast::Standard) }, None).tokens;
    let compact = at(Size::Compact);
    let l = &compact.component.list;
    assert_eq!((l.row_height, l.thumb.sm, l.thumb.md, l.thumb.lg), (24.0, 18.0, 24.0, 48.0));
    assert_eq!((l.thumb_row(l.thumb.md), l.thumb_row(l.thumb.lg), l.thumb_row(0.0)), (32.0, 56.0, 32.0));
    assert_eq!((l.indent, l.disclosure, l.toggle_column, l.min_label, l.trailing_pad), (14.0, 14.0, 30.0, 64.0, 10.0));
    assert_eq!((l.radius, l.dividers, l.drop_width, l.drop_halo_width), (0.0, true, 2.0, 1.0));
    assert_eq!((l.pad_x, l.gap), (compact.space.pad.control, compact.space.gap.inline));

    let comfy = at(Size::Comfortable);
    let l = &comfy.component.list;
    assert_eq!((l.row_height, l.thumb.sm, l.thumb.md, l.thumb.lg), (30.0, 18.0, 34.0, 48.0));
    assert_eq!((l.thumb_row(l.thumb.md), l.thumb_row(l.thumb.sm)), (42.0, 40.0));
    assert_eq!((l.indent, l.disclosure, l.toggle_column, l.min_label, l.trailing_pad), (18.0, 18.0, 36.0, 80.0, 12.0));
    assert_eq!((l.radius, l.dividers), (comfy.shape.radius.control, false));

    let t = &comfy;
    let l = &t.component.list;
    assert_eq!(
        (l.text, l.dimmed, l.selected_text, l.detail),
        (t.color(T::TextNormal), t.color(T::TextFaint), t.color(T::TextOnSelected), t.color(T::TextFaint))
    );
    assert_eq!((l.icon, l.icon_hover), (t.color(T::TextMuted), t.color(T::TextNormal)));
    assert_eq!((l.hover, l.selected, l.selected_hover), (t.color(T::StateHover), t.color(T::StateSelected), t.color(T::StateSelectedHover)));
    assert_eq!((l.divider, l.thumb_border), (t.color(T::LineDivider), t.color(T::LineBorder)));
    assert_eq!((l.ghost_fill, l.ghost_border), (t.color(T::SurfaceRaised), t.color(T::AccentSolid)));
    assert_eq!((l.drop, l.drop_halo), (t.color(T::AccentSolid), t.color(T::SurfacePanel)));
}

/// The drop marker meets its minimum on its halo in every built-in, mode and contrast.
#[test]
fn builtin_drop_markers_meet_their_minimum_on_their_halo() {
    for theme in builtins() {
        for a in appearances().into_iter().filter(|a| a.size == Size::Compact && a.text_size == TextSize::default()) {
            let l = resolve(theme, a, None).tokens.component.list;
            let min = if a.contrast == Contrast::High { values::NON_TEXT_MIN.1 } else { values::NON_TEXT_MIN.0 };
            let ratio = contrast_ratio(l.drop, l.drop_halo);
            assert!(ratio >= min, "{} {a:?}: list.drop on list.drop_halo is {ratio:.2}, needs {min}", theme.id);
        }
    }
}

#[test]
fn builtin_colour_picker_markers_meet_their_minimum_on_their_halo() {
    for theme in builtins() {
        for a in appearances().into_iter().filter(|a| a.size == Size::Compact && a.text_size == TextSize::default()) {
            let tokens = resolve(theme, a, None).tokens;
            let p = tokens.component.color_picker;
            let min = if a.contrast == Contrast::High { values::NON_TEXT_MIN.1 } else { values::NON_TEXT_MIN.0 };
            let halo = over(p.marker_halo, tokens.color.surface.panel);
            let ratio = contrast_ratio(p.marker, halo);
            assert!(ratio >= min, "{} {a:?}: color_picker.marker on its halo is {ratio:.2}, needs {min}", theme.id);
            assert!(p.marker.0[..3].iter().all(|&c| c == p.marker.0[0]), "the marker is achromatic");
        }
    }
}

#[test]
fn a_list_radius_override_replaces_the_size_default() {
    let mut def = grey_blue();
    def.components.insert(ComponentField::List(ListField::Radius), OverrideValue::Length(2.0));
    assert_eq!(resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component.list.radius, 2.0);
    assert_eq!(ComponentField::from_path("list.selected.hover"), Some(ComponentField::List(ListField::SelectedHover)));
    assert_eq!(ComponentField::from_path("list.row_height"), None);
}
