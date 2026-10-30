#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use super::semantic_tests::{app, appearances, grey_blue, hex, over, resolve, violet};
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
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
            for (fg, bg, std, hi) in button_pairs() {
                let min = if a.contrast == Contrast::High { hi } else { std };
                let ratio = pair_ratio(t, fg, bg);
                // Standard contrast reports the pairs a default misses; High fixes them.
                if a.contrast == Contrast::High {
                    assert!(ratio >= min, "{} {a:?}: {} is {ratio:.2}, needs {min}", def.name, fg.path());
                }
            }
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let b = &t.component.button;
    let accent = t.color(T::AccentSolid);
    assert_eq!(b.primary.fill, States::new(accent, accent.faded(b.hover_opacity), accent.faded(b.pressed_opacity), t.color(T::StateDisabledFill)));
    assert_eq!(b.primary.text, States::new(t.color(T::TextOnAccent), t.color(T::TextOnAccent), t.color(T::TextOnAccent), t.color(T::TextDisabled)));
    assert_eq!(b.danger.fill.pressed, t.color(T::StatusBadSolidPressed));
    assert_eq!(b.colors(ButtonVariant::Secondary).border.get(State::Hover), t.color(T::LineControlEdge));
    assert_eq!(b.secondary.border.rest, t.color(T::LineControlEdge));
    assert_eq!(b.ghost.fill.rest, Rgba::TRANSPARENT);
    assert_eq!(b.danger.text, States::new(t.color(T::TextOnStatus), t.color(T::TextOnStatus), t.color(T::TextOnStatus), t.color(T::TextDisabled)));
}

/// The icon-button and segmented pairs, written out independently of the families' lists.
fn item_pairs() -> Vec<(ComponentField, ComponentField)> {
    let icon = |part| ComponentField::IconButton(IconButtonField::Selected(part));
    let segment = |part, state| ComponentField::Segmented(SegmentedField::Segment { part, state });
    let selected = |part, state| ComponentField::Segmented(SegmentedField::Selected { part, state });
    let mut p = vec![(segment(ItemPart::Text, State::Hover), segment(ItemPart::Fill, State::Hover))];
    p.push((icon(ButtonPart::Text), icon(ButtonPart::Fill)));
    for s in [State::Rest, State::Hover, State::Pressed] {
        p.push((selected(ItemPart::Text, s), selected(ItemPart::Fill, s)));
    }
    p
}

#[test]
fn icon_button_and_segmented_defaults_come_from_semantic_tokens_and_meet_their_minimums() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
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
    let glyphs = BySize { sm: icons.sm - values::ICON_BUTTON_GLYPH_LESS.0, md: icons.md - values::ICON_BUTTON_GLYPH_LESS.1 };
    assert_eq!((icon.size, icon.icon, seg.height, seg.icon), (control, glyphs, control, icons));
    assert_eq!(seg.pad_x, BySize { sm: t.space.pad.control, md: t.space.pad.control + t.space.gap.inline });
    // PhotoCraft's `pill_tab`: item spacing 8 in Compact, 24 high, text + 20 wide.
    assert_eq!((seg.gap, seg.label), (t.space.gap.inline + values::SEGMENTED_GAP_OVER_INLINE, TypeRole::Label));
    let hover = t.color(T::StateHover);
    assert_eq!(seg.selected.fill, States::new(hover, hover, hover, t.color(T::StateDisabledFill)));
    assert_eq!(seg.selected.text, States::new(t.color(T::TextNormal), t.color(T::TextNormal), t.color(T::TextNormal), t.color(T::TextDisabled)));
    let edge = t.color(T::LineControlEdge);
    assert_eq!(seg.selected_border, States::new(edge, edge, edge, t.color(T::LineDivider)));
    // An icon button that is on is `accent.soft` inside an `accent.border` outline.
    assert_eq!((icon.selected.on_fill, icon.selected.on_border), (t.color(T::AccentSoft), t.color(T::AccentBorder)));
    assert_eq!((icon.selected.on_disabled_fill, icon.selected.on_disabled_text), (t.color(T::AccentSoft), t.color(T::TextDisabled)));
    let half = hover.faded(values::SEGMENT_HOVER_OPACITY);
    assert_eq!(seg.segment.fill, States::new(Rgba::TRANSPARENT, half, half, Rgba::TRANSPARENT));
    assert_eq!(seg.segment.text, States::new(t.color(T::TextMuted), t.color(T::TextMuted), t.color(T::TextMuted), t.color(T::TextDisabled)));
}

/// The decided contrast scope: the filled part is an accent state indicator, 3:1 on the track
/// with high contrast, which also sets the track 3:1 off the panel.
#[test]
fn slider_defaults_come_from_semantic_tokens_and_the_fill_meets_the_indicator_minimum() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
            // Standard lists the pair; only high contrast needs the fill to show on the track.
            let min = if a.contrast == Contrast::High { 3.0 } else { 0.0 };
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
    let thumb = BySize { sm: t.size.icon.sm - 2.0, md: t.size.icon.md - 4.0 };
    assert_eq!((s.height, s.thumb, s.thumb_width), (BySize { sm: control.sm - 4.0, md: control.md - 6.0 }, thumb, thumb));
    assert_eq!(s.width, BySize { sm: 6.0 * control.sm, md: 6.0 * control.md });
    assert_eq!((s.track_height, s.gradient_height, s.track_radius, s.radius), (3.0, 5.0, 2.0, Radius::Full.px()));
    assert_eq!(s.track, t.color(T::LineControlEdge));
    assert_eq!(s.fill, States::new(t.color(T::AccentSolid), t.color(T::AccentSolid), t.color(T::AccentSolid), t.color(T::TextDisabled)));
    assert_eq!(s.thumb_colors, knob(&t));
    let (clear, soft) = (Rgba::TRANSPARENT, t.color(T::AccentSoft));
    assert_eq!((s.ring, s.ring_width), (States::new(clear, soft, soft, clear), 2.0));
    assert_eq!(s.ring_offset, 3.5);
}

#[test]
fn curve_defaults_come_from_semantic_tokens_and_the_point_outline_meets_the_indicator_minimum() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
            let min = if a.contrast == Contrast::High { 4.5 } else { 3.0 };
            let (stroke, fill) = (ComponentField::Curve(CurveField::PointStroke), ComponentField::Curve(CurveField::Fill));
            let ratio = pair_ratio(&r.tokens, stroke, Some(fill));
            assert!(ratio >= min, "{} {a:?}: the point outline on the plot is {ratio:.2}, needs {min}", def.name);
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let c = &t.component.curve;
    assert_eq!((c.fill, c.border), (t.component.field.fill.rest, t.component.field.border.rest));
    assert_eq!((c.grid, c.baseline), (t.color(T::LineDivider), t.color(T::LineControlEdge)));
    assert_eq!((c.point_fill, c.point_stroke), (t.color(T::SurfaceSunken), t.color(T::TextNormal)));
    assert_eq!((c.width, c.other_width, c.other_alpha), (1.5, 1.0, 0.8));
    // The Compact sizes scale with the control height.
    let k = t.size.control.md / 24.0;
    let scaled = |v: f32| (v * k).round();
    assert_eq!((c.min_side, c.max_side, c.pad), (scaled(120.0), scaled(300.0), scaled(5.0)));
    assert_eq!((c.bar, c.bar_gap, c.hit, c.drag_off), (scaled(8.0), scaled(4.0), scaled(9.0), scaled(12.0)));
}

#[test]
fn levels_defaults_come_from_semantic_tokens_and_the_thumb_edge_meets_the_indicator_minimum_in_every_builtin() {
    let edge = ComponentField::Levels(LevelsField::ThumbEdge);
    let listed: Vec<_> = ComponentTokens::pairs().into_iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Levels(_)))).collect();
    assert_eq!(
        listed.iter().map(|p| (p.fg, p.bg, p.minimum)).collect::<Vec<_>>(),
        [
            (TokenRef::Component(edge), TokenRef::Color(T::SurfacePanel), (3.0, 4.5)),
            (TokenRef::Component(edge), TokenRef::Color(T::SurfaceRaised), (3.0, 4.5)),
        ]
    );
    for theme in builtins() {
        for a in appearances().into_iter().filter(|a| a.text_size == TextSize::default()) {
            let r = resolve(theme, a, None);
            if a.contrast != Contrast::High {
                continue;
            }
            let min = 4.5;
            let ratio = pair_ratio(&r.tokens, edge, None);
            assert!(ratio >= min, "{} {a:?}: levels.thumb_edge on surface.panel is {ratio:.2}, needs {min}", theme.id);
            let raised = r.tokens.color(T::SurfaceRaised);
            let ratio = contrast_ratio(over(r.tokens.component.color(edge).unwrap(), raised), raised);
            assert!(ratio >= min, "{} {a:?}: levels.thumb_edge on surface.raised is {ratio:.2}, needs {min}", theme.id);
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let l = &t.component.levels;
    assert_eq!((l.thumb_edge, l.disabled_edge), (t.color(T::TextMuted), t.color(T::TextDisabled)));
    let (clear, soft) = (Rgba::TRANSPARENT, t.color(T::AccentSoft));
    assert_eq!(l.ring, States::new(clear, soft, soft, clear));
    // The Compact sizes scale with the control height.
    let k = t.size.control.md / 24.0;
    let scaled = |v: f32| (v * k).round();
    assert_eq!((l.min_width, l.max_width, l.row, l.histogram, l.bar), (scaled(120.0), scaled(300.0), scaled(14.0), scaled(110.0), scaled(10.0)));
    assert_eq!((l.thumb_width, l.thumb_height, l.hit), (scaled(11.0), scaled(9.0), scaled(7.0)));
    assert_eq!(ComponentField::from_path("levels.ring.hover"), Some(ComponentField::Levels(LevelsField::Ring(State::Hover))));
    assert_eq!(ComponentField::from_path("levels.thumb_edge"), Some(edge));
    assert_eq!(ComponentField::from_path("levels.row"), None, "sizes are not overridable");
}

#[test]
fn gradient_editor_defaults_come_from_semantic_tokens_and_its_cues_meet_the_indicator_minimum() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
            let min = if a.contrast == Contrast::High { 4.5 } else { 3.0 };
            for f in [GradientEditorField::Selected, GradientEditorField::MarkerEdge] {
                let ratio = pair_ratio(&r.tokens, ComponentField::GradientEditor(f), None);
                assert!(ratio >= min, "{} {a:?}: {f:?} on the panel is {ratio:.2}, needs {min}", def.name);
            }
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let g = &t.component.gradient_editor;
    assert_eq!((g.edge, g.marker_edge, g.selected), (t.color(T::LineControlEdge), t.color(T::TextMuted), t.color(T::AccentSolid)));
    let k = t.size.control.md / 24.0;
    let scaled = |v: f32| (v * k).round();
    assert_eq!((g.ramp_height, g.inset, g.marker, g.pointer, g.diamond), (scaled(24.0), scaled(8.0), scaled(10.0), scaled(5.0), scaled(3.0)));
    assert_eq!((g.hit, g.diamond_hit, g.tear_off, g.drop_line), (scaled(6.0), scaled(4.0), scaled(28.0), scaled(2.0)));
}

/// The channel colours a curve is drawn in are checked on `curve.fill`, not only on the surface
/// it defaults to: a pinned fill that hides them is moved by high contrast.
#[test]
fn a_pinned_curve_fill_is_checked_against_the_channel_colours() {
    let fill = ComponentField::Curve(CurveField::Fill);
    let mut def = grey_blue();
    let red = resolve(&def, app(Mode::Dark, Contrast::Standard), None).tokens.color(T::DataRed);
    def.pins.dark.colors.insert(TokenRef::Component(fill), red);
    let r = resolve(&def, app(Mode::Dark, Contrast::Standard), None);
    let warned = |r: &crate::Resolved| r.warnings.iter().any(|w| w.fg == TokenRef::Color(T::DataRed) && w.bg == TokenRef::Component(fill));
    assert!(!warned(&r), "Standard keeps PhotoCraft's channel colours: {:?}", r.warnings);
    let r = resolve(&def, app(Mode::Dark, Contrast::High), None);
    let ratio = contrast_ratio(r.tokens.color(T::DataRed), r.tokens.component.color(fill).unwrap());
    assert!(ratio >= 4.5 && !warned(&r), "{ratio:.2}: {:?}", r.warnings);
}

#[test]
fn angle_dial_defaults_come_from_semantic_tokens_and_the_needle_meets_the_face() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
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
    assert_eq!((d.size, d.margin, d.needle_width, d.needle_inset), (scaled(42.0), scaled(2.0), scaled(2.0), scaled(4.0)));
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
    let mut expected: Vec<_> = field_pairs().into_iter().map(|(fg, bg, min)| (TokenRef::Component(fg), TokenRef::Component(bg), min)).collect();
    // The rest and hover borders against the panel and a popup, required in high contrast only.
    let edges = [BorderState::Rest, BorderState::Hover].into_iter().flat_map(|state| {
        [T::SurfacePanel, T::SurfaceRaised]
            .map(|surface| (TokenRef::Component(ComponentField::Field(FieldField::Border(state))), TokenRef::Color(surface), (0.0, 3.0)))
    });
    expected.extend(edges);
    assert_eq!(listed, expected);
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
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
    assert_eq!(f.number_width, BySize { sm: 3.0 * control.sm, md: 3.0 * control.md });
    assert_eq!(f.icon, BySize { sm: t.size.icon.sm, md: t.size.icon.md });
    assert_eq!((f.pad_x, f.gap, f.radius, f.role), (t.space.pad.control, t.space.gap.inline, t.shape.radius.control, TypeRole::Body));
    assert_eq!(f.fill, Dimmed { rest: t.color(T::SurfaceSunken), disabled: t.color(T::StateDisabledFill) });
    let border = BorderColors {
        rest: t.color(T::LineControlEdge),
        hover: t.color(T::LineControlEdge),
        focused: t.color(T::LineFocus),
        disabled: t.color(T::LineDivider),
    };
    assert_eq!(f.border, border);
    assert_eq!(f.text, Dimmed { rest: t.color(T::TextNormal), disabled: t.color(T::TextDisabled) });
    assert_eq!(f.placeholder, Dimmed { rest: t.color(T::TextFaint), disabled: t.color(T::TextDisabled) });
    assert_eq!(f.unit, Dimmed { rest: t.color(T::TextFaint), disabled: t.color(T::TextDisabled) });
    assert_eq!(f.scrub, Dimmed { rest: t.color(T::TextMuted), disabled: t.color(T::TextDisabled) });
    assert_eq!((f.selection, f.selected_text), (t.color(T::AccentSoft), t.color(T::TextNormal)));
    assert_eq!((f.caret, f.invalid), (t.color(T::TextNormal), t.color(T::StatusBadFg)));
}

fn listbox_pairs() -> Vec<(ListboxField, TokenRef, (f64, f64))> {
    let popup = TokenRef::Component(ComponentField::Popup(PopupField::Fill));
    let active = TokenRef::Component(ComponentField::Listbox(ListboxField::ActiveFill));
    let chosen = TokenRef::Component(ComponentField::Listbox(ListboxField::ChosenFill));
    vec![
        (ListboxField::ActiveFill, popup, (1.12, 1.12)),
        (ListboxField::ActiveBorder, popup, crate::values::CONTROL_EDGE_MIN),
        (ListboxField::Text { disabled: false }, popup, (4.5, 7.0)),
        (ListboxField::Group, popup, (4.5, 7.0)),
        (ListboxField::ActiveText, active, (4.5, 7.0)),
        (ListboxField::ChosenFill, popup, (1.12, 3.0)),
        (ListboxField::ChosenText, chosen, (4.5, 7.0)),
        (ListboxField::Mark, popup, (3.0, 4.5)),
    ]
}

#[test]
fn listbox_defaults_come_from_semantic_tokens_and_their_pairs_meet_their_minimums() {
    let listed: Vec<_> = ComponentTokens::pairs()
        .into_iter()
        .filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Listbox(_))))
        .map(|p| (p.fg, p.bg, p.minimum))
        .collect();
    let mut expected: Vec<_> = listbox_pairs().into_iter().map(|(fg, bg, min)| (TokenRef::Component(ComponentField::Listbox(fg)), bg, min)).collect();
    // The menu's open row: the row text on `menu.open.fill`.
    expected.push((
        TokenRef::Component(ComponentField::Listbox(ListboxField::Text { disabled: false })),
        TokenRef::Component(ComponentField::Menu(MenuField::OpenFill)),
        (4.5, 7.0),
    ));
    assert_eq!(listed, expected);
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
            let t = &r.tokens;
            for (fg, bg, (standard, high)) in listbox_pairs() {
                let min = if a.contrast == Contrast::High { high } else { standard };
                let bg = match bg {
                    TokenRef::Color(token) => t.color(token),
                    TokenRef::Component(field) => over(t.component.color(field).unwrap(), t.color(T::SurfacePanel)),
                };
                let ratio = contrast_ratio(over(t.component.color(ComponentField::Listbox(fg)).unwrap(), bg), bg);
                // Standard reports the accent fill's text and mark pairs; High enforces them.
                assert!(
                    a.contrast != Contrast::High || ratio >= min,
                    "{} {a:?}: {} is {ratio:.2}, needs {min}",
                    def.name,
                    ComponentField::Listbox(fg).path()
                );
            }
        }
    }

    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let l = &t.component.listbox;
    assert_eq!(l.height, BySize { sm: t.size.control.sm, md: t.size.control.md });
    assert_eq!(l.icon, BySize { sm: t.size.icon.sm, md: t.size.icon.md });
    assert_eq!(
        (l.pad_x, l.gap, l.separator_pad, l.radius),
        (t.space.pad.control + 4.0, t.space.gap.inline, t.space.gap.inline, t.shape.radius.control)
    );
    assert_eq!(l.max_height, 420.0 + 12.0 * (t.size.control.md - 24.0));
    assert_eq!((l.label, l.group_label), (TypeRole::Body, TypeRole::Caption));
    assert_eq!(l.text, Dimmed { rest: t.color(T::TextNormal), disabled: t.color(T::TextDisabled) });
    assert_eq!((l.active_text, l.active_border), (t.color(T::TextNormal), t.color(T::LineControlEdge)));
    assert_eq!((l.chosen_fill, l.chosen_text, l.chosen_border), (t.color(T::AccentSolid), t.color(T::TextOnAccent), t.color(T::AccentText)));
    let popup = t.color(T::SurfacePanel);
    assert!(contrast_ratio(over(l.active_fill, popup), popup) >= values::LIST_HOVER_MIN.0);
    assert_eq!(l.mark, t.color(T::TextNormal));
    assert_eq!((l.group, l.separator), (t.color(T::TextMuted), t.color(T::LineDivider)));
}

/// The active menu row is `accent.solid` with `text.on_accent` (PhotoCraft's menus), an open
/// submenu's row is `state.hover`, the shortcuts are muted text on the popup card, and the sizes scale PhotoCraft's Compact ones with the control
/// height.
#[test]
fn menu_defaults_come_from_semantic_tokens_and_their_pairs_meet_their_minimums() {
    let listed: Vec<_> =
        ComponentTokens::pairs().into_iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Menu(_)))).map(|p| (p.fg, p.bg)).collect();
    let menu = |f| TokenRef::Component(ComponentField::Menu(f));
    let mut expected = vec![
        (menu(MenuField::ActiveFill), TokenRef::Component(ComponentField::Popup(PopupField::Fill))),
        (menu(MenuField::ActiveText), menu(MenuField::ActiveFill)),
        (menu(MenuField::Shortcut), TokenRef::Component(ComponentField::Popup(PopupField::Fill))),
    ];
    for hue in MenuHue::ALL {
        let tint = |part| menu(MenuField::Tint(hue, part));
        expected.push((tint(MenuTintPart::Text), tint(MenuTintPart::Fill)));
        expected.push((tint(MenuTintPart::Shortcut), tint(MenuTintPart::Fill)));
    }
    assert_eq!(listed, expected);
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let t = resolve(&def, a, None).tokens;
            let m = &t.component.menu;
            assert_eq!(
                (m.active_fill, m.active_text, m.open_fill, m.shortcut),
                (t.color(T::AccentSolid), t.color(T::TextOnAccent), t.color(T::StateHover), t.color(T::TextMuted))
            );
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
            assert_eq!(toggle.on_disabled(ButtonPart::Fill), t.color(T::AccentSoft));
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
    for bg in [T::SurfacePanel, T::SurfaceRaised] {
        p.push((ComponentField::Check(CheckField::Switch { part: CheckOffPart::Fill, state: State::Rest }), TokenRef::Color(bg)));
    }
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
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
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
    assert_eq!(c.box_size, BySize { sm: t.size.icon.sm, md: t.size.icon.md - 2.0 });
    assert_eq!((c.gap, c.box_gap, c.label, c.radius, c.thumb_inset), (t.space.gap.inline, t.space.gap.stack, TypeRole::Body, 2.0, 2.0));
    assert_eq!((c.mark.md, c.dot.md, c.track.md), (c.box_size.md * 0.125, c.box_size.md * 0.4, t.size.icon.md * 2.0));
    let outline = States::new(t.color(T::TextFaint), t.color(T::TextMuted), t.color(T::TextMuted), t.color(T::TextDisabled));
    assert_eq!(c.off.border, outline);
    let sunken = t.color(T::SurfaceSunken);
    let darker = Rgba(sunken.0.map(|v| (f32::from(v) * 0.82).round() as u8)).with_alpha(sunken.a());
    assert_eq!(c.off.fill, States::new(sunken, sunken, darker, t.color(T::StateDisabledFill)));
    let accent = t.color(T::AccentSolid);
    assert_eq!(c.on.fill, States::new(accent, accent, accent.with_alpha((f32::from(accent.a()) * 0.75).round() as u8), t.color(T::TextDisabled)));
    let none = Rgba::TRANSPARENT;
    assert_eq!(c.on.border, States::new(none, none, none, t.color(T::TextDisabled)));
    assert_eq!(c.switch.fill, States::new(accent, accent, accent, t.color(T::TextDisabled)));
    assert_eq!(c.switch.border, c.switch.fill);
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
fn a_check_box_radius_is_two_in_every_theme_and_an_override_stays_below_the_cap() {
    for size in [Size::Compact, Size::Comfortable] {
        let t = resolve(&grey_blue(), Appearance { size, ..app(Mode::Light, Contrast::Standard) }, None).tokens;
        assert_eq!(t.component.check.radius, 2.0, "{size:?}");
    }
    let mut def = grey_blue();
    def.shape = Some(ShapeSeed {
        radius: Some(RadiusSet { control: 0.0, popup: 0.0, window: 0.0, panel: 0.0, button: 0.0, pill: 0.0 }),
        radius_offset: None,
        edge: None,
    });
    assert_eq!(resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component.check.radius, 2.0);
    def.components.insert(ComponentField::from_path("check.radius").unwrap(), OverrideValue::Length(9.0));
    assert_eq!(resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component.check.radius, 3.0);
}

#[test]
fn button_sizes_follow_the_size_preset() {
    let at = |size| resolve(&grey_blue(), Appearance { size, ..app(Mode::Light, Contrast::Standard) }, None).tokens.component.button;
    let compact = at(Size::Compact);
    assert_eq!((compact.height.sm, compact.height.md), (22.0, 28.0));
    assert_eq!((compact.pad_x.sm, compact.pad_x.md), (14.0, 14.0));
    assert_eq!((compact.icon.get(ControlSize::Sm), compact.icon.get(ControlSize::Md)), (12.0, 16.0));
    assert_eq!(compact.gap, 4.0);
    let comfy = at(Size::Comfortable);
    assert_eq!((comfy.height.sm, comfy.height.md), (22.0, 30.0));
    assert_eq!((comfy.pad_x.sm, comfy.pad_x.md), (18.0, 18.0));
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
        4 + 4 * 3 * 4
            + 1
            + (2 + 3) * 4
            + 2 * 4
            // Icon button toggled on: a colour per part, and per part toggled on and disabled.
            + 2 * 3
            // List: the frame of the selected row's thumbnail, the thumbnail's radius, and the
            // layer form's hover, selected fill, outline and the three selected texts.
            + 1
            + 1
            + 7
            // Segmented: a colour per part and state of plain and selected pills, and the selected pill's outline per state.
            + 2 * 2 * 4
            + 4
            // Slider: radius, track and the two thumb lengths; fill, thumb and ring per state.
            + 4
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
            // Progress: the track's edge.
            + 1
            // Toast: radius, fill, line, more, and edge and title per tone; notice: a colour per tone; empty state: text.
            + (4 + 2 * 5)
            // Toggle button on: a colour per part (the on-and-disabled colours are counted above).
            + 3
            + 5
            + 1
            + 6
            + (1 + 17)
            + 3
            // Colour picker: the marker and its halo.
            + 2
            // Gradient editor: edge, marker edge and selected.
            + 3
            // Angle dial: face, edge, needle and hub.
            + 4
            // Levels: thumb edge, disabled edge and a ring per state.
            + (2 + 4)
            // Tab: the radius and twelve colours.
            + 13
            // Dock: five colours, the current icon's text and two radii; float: radius, shadow and eight colours.
            + 8
            + 10
            // Title bar: seven colours.
            + 7
            // Menu tints: a fill, text and shortcut per hue.
            + 3 * 7
            // Switch track while on: fill and border per state.
            + 2 * 4
            // Tooltip: radius, fill, text and border.
            + 4
            // Popup: fill, border and shadow; the menu's open fill replaced the list box's chosen text.
            + 3
            // List box: the active and chosen borders and the chosen fill and text (the menu's radius is one more).
            + 5
            // Histogram: the backdrop; curve: the selected point's outline.
            + 1
            + 1
    );
    for &f in ComponentField::ALL {
        assert_eq!(ComponentField::from_path(&f.path()), Some(f));
        assert_eq!(TokenRef::from_path(&f.path()), Some(TokenRef::Component(f)));
    }
    // Every check colour field the types can express is listed, so none lacks a path.
    for state in State::ALL {
        let mut check: Vec<_> = CheckOffPart::ALL.map(|part| CheckField::Off { part, state }).into();
        check.extend(CheckOnPart::ALL.map(|part| CheckField::On { part, state }));
        check.extend(CheckSwitchPart::ALL.map(|part| CheckField::Switch { part, state }));
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
        ("icon_button.selected.text", ComponentField::IconButton(IconButtonField::Selected(ButtonPart::Text))),
        ("icon_button.selected_disabled.border", ComponentField::IconButton(IconButtonField::SelectedDisabled(ButtonPart::Border))),
        ("segmented.selected.border.hover", ComponentField::Segmented(SegmentedField::SelectedBorder(State::Hover))),
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
        "icon_button.selected",
        "segmented.inset",
        "segmented.track",
        "segmented.selected.border",
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
        "tooltip.nope",
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
        "listbox.chosen",
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
        assert_eq!(t.component.button.primary.text.hover, pin, "the other states follow the pinned rest text");
        let ratio = pair_ratio(t, text, Some(fill));
        let pinned: Vec<_> = r.warnings.iter().filter(|w| w.fg == text.into()).copied().collect();
        assert_eq!(pinned, vec![ContrastCheck { fg: text.into(), bg: fill.into(), ratio, minimum: 4.5 }], "{mode:?}");
        assert_eq!(pinned[0].fg.path(), "button.primary.text.rest");

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
    assert!(standard.warnings.iter().all(|w| w.fg != ghost_text.into()), "high-contrast pins apply only with high contrast");
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
                assert_eq!(t.component.handle, HandleTokens { size: 7.0, stroke: 1.0, halo: 1.0, hit: 14.0 }, "{} {size:?} {mode:?}", def.name);
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
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
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
    assert_eq!(
        (soft.fill, soft.text, soft.border),
        (t.color(T::StateSelected), t.color(T::TextOnSelected), t.color(T::AccentSolid)),
        "the selected look"
    );
    let neutral = c.badge.colors(BadgeTone::Neutral, BadgeStyle::Soft);
    assert_eq!((neutral.fill, neutral.text, neutral.border), (t.color(T::SurfaceSunken), t.color(T::TextMuted), t.color(T::LineBorder)));
    let caption = t.typography.caption.line_height;
    assert_eq!((c.badge.height, c.badge.pad_y, c.badge.radius, c.badge.label), (caption + 2.0, 1.0, Radius::Full.px(), TypeRole::Caption));
    assert_eq!((c.kbd.height, c.kbd.pad_y, c.kbd.label), (caption + 2.0, 1.0, TypeRole::Caption));
    assert_eq!((c.kbd.fill, c.kbd.text), (t.color(T::SurfaceRaised), t.color(T::TextMuted)));
    assert_eq!((c.progress.track, c.progress.fill, c.progress.track_height), (t.color(T::SurfaceSunken), t.color(T::AccentSolid), 6.0));
    assert_eq!(c.progress.edge, t.color(T::LineControlEdge));
    let s = &c.swatch;
    assert_eq!(s.size, BySize { sm: t.size.control.sm, md: t.size.control.md });
    assert_eq!((s.edge, s.selected), (t.color(T::LineControlEdge), t.color(T::AccentSolid)));
    assert_eq!(s.selected_width, 2.0);
    assert_eq!((s.mixed_fill, s.mixed_mark), (t.color(T::SurfaceSunken), t.color(T::TextMuted)));
    // PhotoCraft's checker is the same light pair in every theme and mode.
    assert_eq!((s.checker_a, s.checker_b), (Rgba::rgb(250, 250, 250), Rgba::rgb(214, 214, 214)));
    let light = resolve(&grey_blue(), app(Mode::Light, Contrast::Standard), None).tokens.component.swatch;
    assert_eq!((light.checker_a, light.checker_b), (s.checker_a, s.checker_b));
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
    def.shape = Some(ShapeSeed {
        radius: Some(RadiusSet { control: 2.0, popup: 0.0, window: 0.0, panel: 0.0, button: 0.0, pill: 0.0 }),
        radius_offset: None,
        edge: None,
    });
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
fn tooltips_have_a_width_and_default_to_the_popup_look() {
    let t = resolve(&grey_blue(), app(Mode::Light, Contrast::High), None).tokens;
    let tip = t.component.tooltip;
    assert_eq!(tip.max_width, 280.0);
    assert_eq!(
        (tip.fill, tip.text, tip.border),
        (t.color(ColorToken::SurfacePanel), t.color(ColorToken::TextNormal), t.color(ColorToken::LineBorder))
    );
    assert_eq!(tip.radius, t.shape.radius.popup);
    assert!(ComponentField::ALL.iter().any(|f| matches!(f, ComponentField::Tooltip(_))));
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
    assert_eq!((l.text, l.dimmed, l.detail), (t.color(T::TextNormal), t.color(T::TextFaint), t.color(T::TextFaint)));
    assert_eq!((l.icon, l.icon_hover), (t.color(T::TextMuted), t.color(T::TextNormal)));
    // The selected fills and their text move to show on the panel
    // (`selected_fills_show_on_what_they_sit_on`).
    assert_eq!(l.hover.a(), 128, "state.hover at half opacity");
    assert_eq!((l.divider, l.thumb_border, l.thumb_border_selected), (t.color(T::LineDivider), t.color(T::LineControlEdge), t.color(T::TextNormal)));
    assert_eq!((l.ghost_fill, l.ghost_border), (t.color(T::SurfaceRaised), t.color(T::AccentSolid)));
    assert_eq!((l.drop, l.drop_halo), (t.color(T::AccentSolid), t.color(T::SurfacePanel)));
}

/// The drop marker meets its minimum on its halo in every built-in and mode in high contrast.
#[test]
fn builtin_drop_markers_meet_their_minimum_on_their_halo() {
    for theme in builtins() {
        for a in appearances().into_iter().filter(|a| a.size == Size::Compact && a.text_size == TextSize::default()) {
            if a.contrast != Contrast::High {
                continue;
            }
            let l = resolve(theme, a, None).tokens.component.list;
            let min = values::NON_TEXT_MIN.1;
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

fn tab_pairs() -> Vec<(TabField, TabField, (f64, f64))> {
    let (text, mark) = ((4.5, 7.0), (3.0, 4.5));
    vec![
        (TabField::Text, TabField::Active, text),
        (TabField::InactiveText, TabField::Strip, text),
        (TabField::InactiveText, TabField::Active, text),
        (TabField::Detail, TabField::Strip, text),
        (TabField::Detail, TabField::Active, text),
        (TabField::Close, TabField::Strip, mark),
        (TabField::Close, TabField::Active, mark),
        (TabField::Drop, TabField::DropHalo, mark),
        // The indicator on the selected tab; its pair on the strip may be met by the tab's fill.
        (TabField::Indicator, TabField::Active, (0.0, 3.0)),
    ]
}

/// Labels, details, the × and the drop marker meet their minimums on the fills they sit on in
/// every appearance.
#[test]
fn tab_pairs_meet_their_minimums_in_every_appearance() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(r.warnings.iter().all(|w| !matches!(w.fg, TokenRef::Component(ComponentField::Tab(_)))), "{} {a:?}: {:?}", def.name, r.warnings);
            let t = &r.tokens;
            for (fg, bg, (standard, high)) in tab_pairs() {
                let min = if a.contrast == Contrast::High { high } else { standard };
                let bg = over(t.component.color(ComponentField::Tab(bg)).unwrap(), t.color(T::SurfacePanel));
                let ratio = contrast_ratio(over(t.component.color(ComponentField::Tab(fg)).unwrap(), bg), bg);
                assert!(ratio >= min, "{} {a:?}: {} is {ratio:.2}, needs {min}", def.name, ComponentField::Tab(fg).path());
            }
        }
    }
}

#[test]
fn tab_sizes_follow_the_size_preset_and_colours_the_semantic_tokens() {
    let at = |size| resolve(&grey_blue(), Appearance { size, ..app(Mode::Dark, Contrast::Standard) }, None).tokens;
    let compact = at(Size::Compact);
    let t = &compact.component.tab;
    assert_eq!((t.height, t.panel_height, t.height_large, t.pad_x, t.gap, t.min_width), (26.0, 28.0, 34.0, 12.0, 4.0, 40.0));
    assert_eq!((t.close_box, t.close_icon, t.overflow, t.radius, t.dividers), (14.0, 10.0, 20.0, 0.0, true));
    assert_eq!((t.drop_width, t.drop_halo_width, t.progress_height, t.tear_off), (2.0, 1.0, 2.0, 24.0));
    assert_eq!((t.indicator_width, t.indicator), (2.0, Rgba::TRANSPARENT));
    assert_eq!((t.strip_height(false), t.strip_height(true)), (26.0, 34.0));
    assert_eq!((t.label_role(false), t.label_role(true), t.detail_role), (TypeRole::Label, TypeRole::Body, TypeRole::Caption));

    let comfy = at(Size::Comfortable);
    let t = &comfy.component.tab;
    assert_eq!((t.height, t.panel_height, t.height_large, t.pad_x, t.gap, t.min_width), (30.0, 40.0, 38.0, 16.0, 6.0, 44.0));
    assert_eq!((t.close_box, t.close_icon, t.overflow, t.radius, t.dividers), (16.0, 12.0, 24.0, comfy.shape.radius.control, false));

    let t = &compact;
    let tab = &t.component.tab;
    assert_eq!((tab.strip, tab.active), (t.color(T::SurfacePanel), t.color(T::SurfacePanel)));
    assert_eq!((tab.text, tab.inactive_text, tab.divider), (t.color(T::TextNormal), t.color(T::TextMuted), t.color(T::LineDivider)));
    assert_eq!((tab.close_text, tab.close_hover, tab.close_fill), (t.color(T::TextFaint), t.color(T::TextNormal), t.color(T::StateHover)));
    assert_eq!((tab.drop, tab.drop_halo), (t.color(T::AccentSolid), t.color(T::SurfacePanel)));
}

#[test]
fn a_tab_override_replaces_the_default() {
    let mut def = grey_blue();
    def.components.insert(ComponentField::Tab(TabField::Radius), OverrideValue::Length(2.0));
    assert_eq!(resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component.tab.radius, 2.0);
    assert_eq!(ComponentField::from_path("tab.text.inactive"), Some(ComponentField::Tab(TabField::InactiveText)));
    assert_eq!(ComponentField::from_path("tab.close.hover"), Some(ComponentField::Tab(TabField::CloseHover)));
    assert_eq!(ComponentField::from_path("tab.height"), None);
}

fn dock_float_pairs() -> Vec<(ComponentField, ComponentField, (f64, f64))> {
    let (text, mark) = ((4.5, 7.0), (3.0, 4.5));
    let (dock, float) = (ComponentField::Dock, ComponentField::Float);
    vec![
        (dock(DockField::IconSelectedText), dock(DockField::IconSelected), text),
        (dock(DockField::Drop), dock(DockField::Background), mark),
        (float(FloatField::TitleText), float(FloatField::Title), text),
        (float(FloatField::TitleMuted), float(FloatField::Title), mark),
        (float(FloatField::Focus), float(FloatField::Title), mark),
        (float(FloatField::Focus), dock(DockField::Background), mark),
        (float(FloatField::Focus), ComponentField::Tab(TabField::Strip), mark),
    ]
}

/// The icon cell's label, the drop outline, the title bar's text and glyphs and the focus stroke
/// meet their minimums on what they sit on in every appearance.
#[test]
fn dock_and_float_pairs_meet_their_minimums_in_every_appearance() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            let ours = |t: &TokenRef| matches!(t, TokenRef::Component(ComponentField::Dock(_) | ComponentField::Float(_)));
            assert!(r.warnings.iter().all(|w| !ours(&w.fg)), "{} {a:?}: {:?}", def.name, r.warnings);
            let t = &r.tokens;
            for (fg, bg, (standard, high)) in dock_float_pairs() {
                let min = if a.contrast == Contrast::High { high } else { standard };
                let bg = over(t.component.color(bg).unwrap(), t.color(T::SurfacePanel));
                let ratio = contrast_ratio(over(t.component.color(fg).unwrap(), bg), bg);
                assert!(ratio >= min, "{} {a:?}: {} is {ratio:.2}, needs {min}", def.name, fg.path());
            }
        }
    }
}

fn title_bar_pairs() -> Vec<(TitleBarField, TitleBarField, (f64, f64))> {
    let (text, mark) = ((4.5, 7.0), (3.0, 4.5));
    vec![
        (TitleBarField::Title, TitleBarField::Bar, text),
        (TitleBarField::Glyph, TitleBarField::Bar, mark),
        (TitleBarField::Glyph, TitleBarField::Hover, mark),
        (TitleBarField::Glyph, TitleBarField::Pressed, mark),
        (TitleBarField::GlyphInactive, TitleBarField::Bar, mark),
    ]
}

/// The title and the caption glyphs meet their minimums on the bar and its hover and pressed
/// fills in every appearance.
#[test]
fn title_bar_pairs_meet_their_minimums_in_every_appearance() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            let is_title_bar = |w: &ContrastCheck| matches!(w.fg, TokenRef::Component(ComponentField::TitleBar(_)));
            assert!(!r.warnings.iter().any(is_title_bar), "{} {a:?}: {:?}", def.name, r.warnings);
            let t = &r.tokens;
            for (fg, bg, (standard, high)) in title_bar_pairs() {
                let min = if a.contrast == Contrast::High { high } else { standard };
                let bg = over(t.component.color(ComponentField::TitleBar(bg)).unwrap(), t.color(T::SurfacePanel));
                let ratio = contrast_ratio(over(t.component.color(ComponentField::TitleBar(fg)).unwrap(), bg), bg);
                assert!(ratio >= min, "{} {a:?}: {} is {ratio:.2}, needs {min}", def.name, ComponentField::TitleBar(fg).path());
            }
        }
    }
}

#[test]
fn dock_and_float_defaults_follow_the_size_preset_and_the_semantic_tokens() {
    let at = |size| resolve(&grey_blue(), Appearance { size, ..app(Mode::Light, Contrast::Standard) }, None).tokens;
    let compact = at(Size::Compact);
    let d = &compact.component.dock;
    assert_eq!((d.gap, d.body_gap, d.border_width, d.title_height, d.column_bar_height), (4.0, 0.0, 1.0, 14.0, 14.0));
    assert_eq!((d.icon_size, d.icon_pad, d.iconic_width), (20.0, 10.0, 36.0));
    assert_eq!((d.drop_width, d.focus_width, d.marker_radius, d.icon_radius, d.hint_radius), (2.0, 1.0, 1.0, 3.0, 4.0));
    assert_eq!((d.drop_fill, d.ghost_fill, d.ghost_stroke, d.grip_alpha, d.tab_fill), (0.15, 0.3, 0.6, 0.5, 0.08));
    let t = &compact;
    assert_eq!((d.background, d.border), (t.color(T::SurfacePanel), t.color(T::LineBorder)));
    assert_eq!((d.resize, d.drop), (t.color(T::AccentSolid), t.color(T::AccentSolid)));
    // The current icon cell's fill moves to show on the panel
    // (`selected_fills_show_on_what_they_sit_on`).
    let f = &compact.component.float;
    assert_eq!((f.border_width, f.radius, f.shadow), (1.0, 0.0, compact.elevation.floating));
    assert_eq!((f.fill, f.border, f.blocked), (t.color(T::SurfacePanel), t.color(T::LineBorder), t.color(T::SurfaceScrim)));
    assert_eq!((f.title, f.title_text, f.title_muted), (compact.component.tab.strip, t.color(T::TextNormal), t.color(T::TextMuted)));
    assert_eq!((f.grip, f.focus), (t.color(T::AccentSolid), t.color(T::LineFocus)));

    let comfy = at(Size::Comfortable);
    let d = &comfy.component.dock;
    assert_eq!((d.gap, d.body_gap, d.title_height, d.column_bar_height, d.icon_size, d.iconic_width), (6.0, 6.0, 18.0, 18.0, 22.0, 44.0));
    assert_eq!((d.icon_radius, d.hint_radius), (comfy.shape.radius.control, comfy.shape.radius.popup));
    assert_eq!((compact.component.float.radius, comfy.component.float.radius), (0.0, 0.0));
}

/// The default shadow is the dark-adjusted floating level, so a dark theme's window shadow is as
/// strong as its other floating surfaces.
#[test]
fn the_float_shadow_follows_the_mode_adjusted_floating_elevation() {
    for mode in [Mode::Light, Mode::Dark] {
        let t = resolve(&grey_blue(), app(mode, Contrast::Standard), None).tokens;
        assert_eq!(t.component.float.shadow, t.elevation.floating, "{mode:?}");
    }
}

/// A `float.shadow` override names a level: its geometry in the alpha of `surface.shadow`, like the
/// default.
#[test]
fn a_float_shadow_override_takes_the_alpha_of_surface_shadow() {
    let mut def = grey_blue();
    def.components.insert(ComponentField::Float(FloatField::Shadow), OverrideValue::Elevation(Elevation::level(2)));
    let at = |mode| resolve(&def, app(mode, Contrast::Standard), None).tokens;
    for mode in [Mode::Light, Mode::Dark] {
        let t = at(mode);
        let level = Elevation::level(2);
        assert_eq!(t.component.float.shadow, Elevation { alpha: f32::from(t.color(T::SurfaceShadow).a()) / 255.0, ..level });
    }
}

#[test]
fn title_bar_sizes_follow_the_size_preset_and_colours_the_semantic_tokens() {
    let at = |size| resolve(&grey_blue(), Appearance { size, ..app(Mode::Dark, Contrast::Standard) }, None).tokens;
    let compact = at(Size::Compact);
    let b = &compact.component.title_bar;
    assert_eq!((b.height, b.height_adwaita, b.pad, b.title_gap, b.title_min_gap), (32.0, 47.0, 8.0, 16.0, 80.0));
    assert_eq!((b.edge, b.corner, b.title_role), (8.0, 12.0, TypeRole::Title));
    let comfy = at(Size::Comfortable);
    let b = &comfy.component.title_bar;
    assert_eq!((b.height, b.height_adwaita, b.pad), (38.0, 47.0, 12.0));

    let t = &compact;
    let b = &t.component.title_bar;
    assert_eq!(b.bar, t.color(T::SurfacePanel));
    assert_eq!(
        (b.title, b.glyph, b.glyph_inactive, b.separator),
        (t.color(T::TextMuted), t.color(T::TextNormal), t.color(T::TextMuted), t.color(T::LineDivider))
    );
    assert_eq!((b.hover, b.pressed), (over(t.color(T::StateHover), b.bar), over(t.color(T::StatePressed), b.bar)));
}

/// The desktops' Close reds are fixed: no override path names them.
#[test]
fn title_bar_close_colours_are_fixed() {
    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::High), None).tokens;
    let b = &t.component.title_bar;
    assert_eq!((b.windows_close, b.windows_close_glyph), (Rgba::rgb(0xC4, 0x2B, 0x1C), Rgba::WHITE));
    assert_eq!((b.breeze_close_hover, b.breeze_close_hover_glyph), (Rgba::rgb(0xFF, 0x98, 0xA2), Rgba::rgb(0x23, 0x26, 0x29)));
    assert_eq!((b.breeze_close_pressed, b.breeze_close_pressed_glyph), (Rgba::rgb(0x6D, 0x22, 0x29), Rgba::WHITE));
    for path in ["title_bar.windows_close", "title_bar.breeze_close_hover", "title_bar.breeze_close_pressed.glyph"] {
        assert_eq!(ComponentField::from_path(path), None, "{path}");
    }
}

#[test]
fn dock_and_float_overrides_replace_the_defaults() {
    let mut def = grey_blue();
    let shadow = ComponentField::Float(FloatField::Shadow);
    def.components.insert(shadow, OverrideValue::Elevation(Elevation::level(1)));
    def.components.insert(ComponentField::Float(FloatField::Radius), OverrideValue::Length(6.0));
    def.components.insert(ComponentField::Dock(DockField::HintRadius), OverrideValue::Length(2.0));
    let c = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component;
    let level1 = Elevation { alpha: 50.0 / 255.0, ..Elevation::level(1) };
    assert_eq!((c.float.shadow, c.float.radius, c.dock.hint_radius), (level1, 6.0, 2.0));
    for (path, field) in [
        ("dock.icon.selected", ComponentField::Dock(DockField::IconSelected)),
        ("dock.icon.selected.text", ComponentField::Dock(DockField::IconSelectedText)),
        ("dock.icon.radius", ComponentField::Dock(DockField::IconRadius)),
        ("dock.hint.radius", ComponentField::Dock(DockField::HintRadius)),
        ("float.shadow", shadow),
        ("float.title.text", ComponentField::Float(FloatField::TitleText)),
        ("float.title.muted", ComponentField::Float(FloatField::TitleMuted)),
    ] {
        assert_eq!(ComponentField::from_path(path), Some(field), "{path}");
    }
    assert_eq!(shadow.kind(), FieldKind::Elevation);
    for sized in ["dock.gap", "dock.drop_width", "dock.icon_size", "float.border_width"] {
        assert_eq!(ComponentField::from_path(sized), None, "{sized}");
    }
}

#[test]
fn the_windows_close_glyph_reads_on_its_red() {
    let b = resolve(&grey_blue(), app(Mode::Light, Contrast::Standard), None).tokens.component.title_bar;
    assert!(contrast_ratio(b.windows_close_glyph, b.windows_close) >= 4.5);
}

#[test]
fn the_breeze_close_glyph_reads_on_its_reds() {
    // The reds and glyphs are fixed, so this holds for light and dark bars alike.
    let b = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens.component.title_bar;
    assert!(contrast_ratio(b.breeze_close_hover_glyph, b.breeze_close_hover) >= 4.5);
    assert!(contrast_ratio(b.breeze_close_pressed_glyph, b.breeze_close_pressed) >= 4.5);
}

#[test]
fn a_title_bar_override_replaces_the_default() {
    let mut def = grey_blue();
    let pink = Rgba::rgb(255, 0, 128);
    def.pins.light.colors.insert(TokenRef::Component(ComponentField::TitleBar(TitleBarField::Separator)), pink);
    let tokens = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens;
    assert_eq!(tokens.component.title_bar.separator, pink);
    assert_eq!(ComponentField::from_path("title_bar.glyph.inactive"), Some(ComponentField::TitleBar(TitleBarField::GlyphInactive)));
    assert_eq!(ComponentField::from_path("title_bar.height"), None);
}

/// The WCAG 1.4.11 minimum for a state indicator: standard, then high contrast.
fn indicator_min(a: Appearance) -> f64 {
    if a.contrast == Contrast::High { 4.5 } else { 3.0 }
}

/// Selected and on states are identifiable in high contrast: the fill of a selected segment, icon
/// button, list row and dock icon, and a toggle button's on look (its fill or its edge), reach the
/// non-text minimum on what they sit on, and their text still reads on the fills. Standard
/// contrast keeps `state.selected` and `text.on_selected`.
#[test]
fn selected_fills_show_on_what_they_sit_on() {
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            assert!(
                a.contrast != Contrast::High || r.warnings.iter().all(|w| matches!(w.fg, TokenRef::Color(_))),
                "{} {a:?}: {:?}",
                def.name,
                r.warnings
            );
            let (t, min) = (&r.tokens, indicator_min(a));
            if a.contrast == Contrast::Standard {
                let k = &t.component;
                let selected = t.color(T::StateSelected);
                let who = format!("{} {a:?}", def.name);
                assert_eq!(
                    (k.segmented.selected.fill.rest, k.segmented.selected.text.rest),
                    (t.color(T::StateHover), t.color(T::TextNormal)),
                    "{who}"
                );
                assert_eq!((k.list.selected, k.dock.icon_selected), (selected, selected), "{who}");
                let soft = t.color(T::AccentSoft);
                assert_eq!((k.icon_button.selected.on_fill, k.toggle_button.on_fill), (soft, soft), "{who}");
                assert_eq!(k.list.selected_hover, t.color(T::StateSelectedHover), "{who}");
                continue;
            }
            let text_min = if a.contrast == Contrast::High { 7.0 } else { 4.5 };
            let k = &t.component;
            let (panel, raised) = (t.color(T::SurfacePanel), t.color(T::SurfaceRaised));
            let on = |what: &str, fill: Rgba, bgs: &[Rgba]| {
                for &bg in bgs {
                    let ratio = contrast_ratio(over(fill, bg), bg);
                    assert!(ratio >= min, "{} {a:?}: {what} is {ratio:.2} on {}, needs {min}", def.name, bg.to_hex());
                }
            };
            on("list selected", k.list.selected, &[panel]);
            on("list selected_hover", k.list.selected_hover, &[panel]);
            on("dock icon selected", k.dock.icon_selected, &[k.dock.background]);
            let (toggle, icon) = (&k.toggle_button, &k.icon_button.selected);
            for bg in [panel, raised] {
                let (fill, edge) =
                    (contrast_ratio(over(k.segmented.selected.fill.rest, bg), bg), contrast_ratio(k.segmented.selected_border.rest, bg));
                assert!(fill.max(edge) >= min, "{} {a:?}: segmented selected is {fill:.2} / {edge:.2} on {}, needs {min}", def.name, bg.to_hex());
            }
            for (what, on) in [("toggle", toggle), ("icon button", icon)] {
                for bg in [panel, raised] {
                    let (fill, edge) = (contrast_ratio(over(on.on_fill, bg), bg), contrast_ratio(on.on_border, bg));
                    assert!(fill.max(edge) >= min, "{} {a:?}: {what} on is {fill:.2} / {edge:.2} on {}, needs {min}", def.name, bg.to_hex());
                }
            }
            for (text, fill) in [
                (k.list.selected_text, k.list.selected),
                (k.list.selected_text, k.list.selected_hover),
                (k.dock.icon_selected_text, k.dock.icon_selected),
                (k.segmented.selected.text.rest, k.segmented.selected.fill.rest),
                (toggle.on_text, toggle.on_fill),
                (icon.on_text, icon.on_fill),
            ] {
                let ratio = contrast_ratio(text, fill);
                assert!(ratio >= text_min, "{} {a:?}: text {} on {} is {ratio:.2}, needs {text_min}", def.name, text.to_hex(), fill.to_hex());
            }
        }
    }
}

/// In high contrast a selected fill that already shows stays as the theme gave it; one that
/// doesn't moves in lightness only, and its text moves with it. Standard contrast keeps either.
#[test]
fn a_selected_fill_that_shows_is_kept_and_one_that_does_not_moves_in_lightness_only() {
    let pinned = |fill: &str, contrast| {
        let mut def = grey_blue();
        def.pins.light = Pins::from_iter([(T::StateSelected, hex(fill))]);
        resolve(&def, app(Mode::Light, contrast), None).tokens
    };
    let t = pinned("#102a6b", Contrast::High);
    assert_eq!(t.component.list.selected, hex("#102a6b"));
    let kept = pinned("#dfe5f5", Contrast::Standard);
    assert_eq!((kept.component.list.selected, kept.component.list.selected_text), (hex("#dfe5f5"), kept.color(T::TextOnSelected)));

    let faint = pinned("#dfe5f5", Contrast::High);
    let moved = faint.component.list.selected;
    let (before, after) = (hex("#dfe5f5").to_oklch(), moved.to_oklch());
    assert!(after.l < before.l - 0.1, "{moved:?}");
    assert!((after.h - before.h).abs() < 5.0 && (after.c - before.c).abs() < 0.05, "{before:?} {after:?}");
    assert!(contrast_ratio(faint.component.list.selected_text, moved) >= 4.5);
}

/// A check box or radio button is identifiable by its fill or its outline on the panel and on a
/// popup. Where neither shows, the outline moves and `text.faint` stays; where the fill shows, the
/// outline is `text.faint` as it was.
#[test]
fn a_check_outline_moves_only_where_neither_fill_nor_outline_shows() {
    let faint = super::semantic_tests::pinned_faint();
    for mode in [Mode::Light, Mode::Dark] {
        for contrast in [Contrast::Standard, Contrast::High] {
            let t = resolve(&faint, app(mode, contrast), None).tokens;
            let pin = faint.pins.get(mode).get(T::TextFaint).unwrap();
            let outline = t.component.check.off.border.rest;
            let min = if contrast == Contrast::High { 4.5 } else { 3.0 };
            if contrast == Contrast::Standard {
                assert_eq!(t.color(T::TextFaint), pin);
                assert_ne!(outline, pin, "{mode:?}");
            }
            for bg in [T::SurfacePanel, T::SurfaceRaised] {
                let ratio = contrast_ratio(outline, t.color(bg));
                assert!(ratio >= min, "{mode:?} {contrast:?}: outline is {ratio:.2} on {bg:?}, needs {min}");
            }
        }
    }

    // A sunken fill 3:1 from both surfaces identifies the box: the outline stays.
    let mut shown = faint.clone();
    shown.pins.light = Pins::from_iter([(T::TextFaint, hex("#c8c8c8")), (T::SurfaceSunken, hex("#000000"))]);
    shown.pins.dark = Pins::from_iter([(T::TextFaint, hex("#404040")), (T::SurfaceSunken, hex("#ffffff"))]);
    for mode in [Mode::Light, Mode::Dark] {
        let t = resolve(&shown, app(mode, Contrast::Standard), None).tokens;
        assert_eq!(t.component.check.off.border.rest, t.color(T::TextFaint), "{mode:?}");
    }
}

/// The primary and secondary hover and pressed fills and the primary text states follow the final
/// rest colours, unless pinned, at the theme's opacities.
#[test]
fn button_states_follow_pinned_rest_colours() {
    let fill = |s| button(ButtonVariant::Primary, ButtonPart::Fill, s);
    let mut def = grey_blue();
    let (rest, text, hover) = (hex("#f6f6f8"), hex("#0c0c0e"), hex("#123456"));
    let pins = Pins::from_iter([
        (fill(State::Rest), rest),
        (button(ButtonVariant::Primary, ButtonPart::Text, State::Rest), text),
        (fill(State::Hover), hover),
    ]);
    def.pins.light = pins.clone();
    def.pins.dark = pins;
    def.components.insert(ComponentField::from_path("button.pressed_opacity").unwrap(), OverrideValue::Length(0.5));
    for mode in [Mode::Light, Mode::Dark] {
        let b = resolve(&def, app(mode, Contrast::Standard), None).tokens.component.button;
        assert_eq!(b.primary.fill.hover, hover, "a pinned hover fill stays");
        assert_eq!(b.primary.fill.pressed, rest.faded(0.5), "{mode:?}");
        assert_eq!([b.primary.text.hover, b.primary.text.pressed], [text; 2], "{mode:?}");
        assert_eq!(b.secondary.fill.pressed, b.secondary.fill.rest.faded(0.5), "{mode:?}");
    }
}

/// Text and number fields follow PhotoCraft (`text_field.rs:46-116`, `fields.rs:94-340`,
/// `theme.rs:824`): the edge doesn't change under the pointer, the unit is faint, and the text
/// selection is grey in the Pro themes (96 dark, 201 light) and `accent.soft` elsewhere.
#[test]
fn fields_follow_photocraft_edge_unit_and_selection() {
    for theme in builtins() {
        for mode in [Mode::Light, Mode::Dark] {
            let shown = theme.variants.mode_for(mode);
            let t = resolve(theme, app(mode, Contrast::Standard), None).tokens;
            let f = &t.component.field;
            if theme.id.as_str() != "craft.classic" {
                assert_eq!(f.border.hover, f.border.rest, "{} {mode:?}", theme.id);
            }
            assert_eq!(f.unit.rest, t.color(T::TextFaint), "{} {mode:?}", theme.id);
            let pro = ["craft.artcraft", "craft.pro", "craft.pro-light-gray"].contains(&theme.id.as_str());
            let expected = match (pro, shown) {
                (true, Mode::Dark) => Rgba::rgb(96, 96, 96),
                (true, Mode::Light) => Rgba::rgb(201, 201, 201),
                _ => t.color(T::AccentSoft),
            };
            assert_eq!(f.selection, expected, "{} {mode:?}", theme.id);
        }
    }
}

fn panel_rail_at(size: Size) -> (PanelTokens, RailTokens) {
    let appearance = Appearance { size, ..app(Mode::Dark, Contrast::Standard) };
    let c = resolve(&grey_blue(), appearance, None).tokens.component;
    (c.panel, c.rail)
}

/// The Compact sizes are PhotoCraft's (the tool rail 40 wide with 30 buttons and 20 point glyphs,
/// the panel rail 36 wide with 28 buttons and 15 point glyphs); Comfortable
/// scales them with the Size preset.
#[test]
fn panel_and_rail_sizes_follow_photocraft_compact_and_the_size_preset() {
    let (panel, rail) = panel_rail_at(Size::Compact);
    assert_eq!(panel.pad, 8.0);
    assert_eq!((rail.width, rail.button, rail.panel_width, rail.panel_button), (40.0, 30.0, 36.0, 28.0));
    assert_eq!((rail.icon, rail.panel_icon), (20.0, 15.0));
    assert_eq!((rail.gap, rail.row_gap, rail.panel_gap, rail.pad_y), (2.0, 3.0, 4.0, 8.0));
    assert_eq!((rail.flyout_row, rail.flyout_gap, rail.flyout_icon), (26.0, 6.0, 14.0));
    assert_eq!((rail.toggle_height, rail.toggle_icon), (14.0, 11.0));
    assert_eq!((rail.indicator_size, rail.indicator_inset, rail.marker_radius, rail.selected_border_width), (4.0, 2.0, 3.0, 1.0));
    assert_eq!(rail.radius, 3.0);

    let (panel, rail) = panel_rail_at(Size::Comfortable);
    assert_eq!(panel.pad, 12.0);
    assert_eq!((rail.width, rail.button, rail.panel_width, rail.panel_button), (50.0, 36.0, 44.0, 32.0));
    assert_eq!((rail.icon, rail.panel_icon), (24.0, 17.0));
    assert_eq!((rail.gap, rail.panel_gap, rail.pad_y), (3.0, 6.0, 12.0));
}

/// A rail draws on the window chrome, its rest glyph is PhotoCraft's faint text and its open
/// panel is a quiet fill in the selected outline.
#[test]
fn panel_and_rail_colours_come_from_the_semantic_roles() {
    let t = resolve(&grey_blue(), app(Mode::Dark, Contrast::Standard), None).tokens;
    let (p, r) = (&t.component.panel, &t.component.rail);
    assert_eq!((p.fill, p.header_line, p.footer_line), (t.color(T::SurfacePanel), t.color(T::LineDivider), t.color(T::LineDivider)));
    assert_eq!((r.fill, r.hover, r.edge), (t.color(T::SurfaceChrome), t.color(T::StateHover), t.color(T::LineDivider)));
    assert_eq!((r.selected, r.selected_border, r.selected_text), (t.color(T::StateSelected), t.color(T::AccentBorder), t.color(T::TextOnSelected)));
    assert_eq!((r.icon_color, r.panel_icon_color, r.icon_hover), (t.color(T::TextMuted), t.color(T::TextFaint), t.color(T::TextNormal)));
    assert_eq!((r.indicator_color, r.marker), (t.color(T::TextFaint), t.color(T::AccentSolid)));
}

#[test]
fn panel_and_rail_paths_round_trip_and_overrides_replace_the_defaults() {
    let ours: Vec<_> = ComponentField::ALL.iter().filter(|f| matches!(f, ComponentField::Panel(_) | ComponentField::Rail(_))).collect();
    assert_eq!(ours.len(), 3 + 12);
    for f in ours {
        assert_eq!(ComponentField::from_path(&f.path()), Some(*f), "{}", f.path());
    }
    assert_eq!(ComponentField::Rail(RailField::Radius).kind(), FieldKind::Length);
    assert_eq!(ComponentField::Rail(RailField::SelectedText).kind(), FieldKind::Color);
    for sized in ["panel.pad", "rail.width", "rail.panel_button", "rail.flyout_row", "rail.marker_radius"] {
        assert_eq!(ComponentField::from_path(sized), None, "{sized}");
    }

    let mut def = grey_blue();
    def.components.insert(ComponentField::Rail(RailField::Radius), OverrideValue::Length(5.0));
    def.components.insert(ComponentField::Rail(RailField::Edge), OverrideValue::Color(hex("#112233")));
    def.components.insert(ComponentField::Panel(PanelField::FooterLine), OverrideValue::Color(hex("#445566")));
    let c = resolve(&def, app(Mode::Light, Contrast::Standard), None).tokens.component;
    assert_eq!((c.rail.radius, c.rail.edge, c.panel.footer_line), (5.0, hex("#112233"), hex("#445566")));
}

/// The rail glyphs, the open-panel glyph and the marker meet the non-text minimum on what they
/// sit on, in every appearance.
#[test]
fn rail_pairs_meet_their_minimums_in_every_appearance() {
    let pairs = [
        (RailField::Icon, RailField::Fill),
        (RailField::PanelIcon, RailField::Fill),
        (RailField::IconHover, RailField::Hover),
        (RailField::SelectedText, RailField::Selected),
        (RailField::Marker, RailField::Fill),
    ];
    for def in [grey_blue(), violet()] {
        for a in appearances() {
            let r = resolve(&def, a, None);
            let ours = |t: &TokenRef| matches!(t, TokenRef::Component(ComponentField::Rail(_)));
            assert!(r.warnings.iter().all(|w| !ours(&w.fg)), "{} {a:?}: {:?}", def.name, r.warnings);
            let t = &r.tokens;
            let min = if a.contrast == Contrast::High { 4.5 } else { 3.0 };
            for (fg, bg) in pairs {
                let (fg, bg) = (ComponentField::Rail(fg), ComponentField::Rail(bg));
                let bg_color = over(t.component.color(bg).unwrap(), t.color(T::SurfacePanel));
                let ratio = contrast_ratio(over(t.component.color(fg).unwrap(), bg_color), bg_color);
                assert!(ratio >= min, "{} {a:?}: {} is {ratio:.2}, needs {min}", def.name, fg.path());
            }
        }
    }
}
