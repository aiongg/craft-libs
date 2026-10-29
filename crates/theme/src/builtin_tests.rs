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

/// Fails when a pair misses its minimum in high contrast. Standard contrast reports it instead
/// (see [`builtins_resolve_in_high_contrast_without_warnings`]).
fn assert_passes_in_high_contrast(theme: &ThemeDef, a: Appearance, p: &ContrastCheck) {
    if a.contrast == Contrast::High {
        assert!(p.passes(), "{} {a:?}: {} on {} is {:.2}, needs {}", theme.id, p.fg.path(), p.bg.path(), p.ratio, p.minimum);
    }
}

/// ArtCraft dark is PhotoCraft's ProMedium in Standard contrast, including the greys and the
/// accent whose pairs miss the minimums; those pairs are reported.
#[test]
fn artcraft_dark_standard_keeps_photocraft_values_and_reports_the_pairs_they_miss() {
    let theme = builtins().iter().find(|t| t.id.as_str() == "craft.artcraft").unwrap();
    let r = resolve(theme, semantic_tests::app(Mode::Dark, Contrast::Standard), None);
    let hex = |t| r.tokens.color(t).to_hex();
    assert_eq!(hex(ColorToken::StateHover), "#626262");
    assert_eq!(hex(ColorToken::StatePressed), "#707070");
    assert_eq!(hex(ColorToken::TextFaint), "#a0a0a0");
    assert_eq!(hex(ColorToken::AccentSolid), "#378ef0");
    // The pinned selected fill is PhotoCraft's `row_selected`, not a lightness the pass found.
    let c = &r.tokens.component;
    assert_eq!(
        (c.list.selected, c.segmented.selected.fill.rest, c.dock.icon_selected),
        (Rgba::rgb(0x6b, 0x6b, 0x6b), Rgba::rgb(0x6b, 0x6b, 0x6b), Rgba::rgb(0x6b, 0x6b, 0x6b))
    );
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
                assert_passes_in_high_contrast(theme, a, p);
            }
        }
    }
}

/// The text of a popup list's rows reads on the popup surface and on the active fill, and the check
/// mark shows on both, in every built-in, pins included. Standard contrast reports the pairs it misses.
#[test]
fn builtin_listbox_text_and_check_mark_meet_their_minimums() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Listbox(_)))).collect();
            assert_eq!(pairs.len(), 6, "{} {a:?}", theme.id);
            for p in pairs {
                assert_passes_in_high_contrast(theme, a, p);
            }
        }
    }
}

/// The active menu row's text reads on the accent fill and the shortcuts on the popup surface, in
/// every built-in and appearance, pins included. Standard contrast reports the pairs it misses.
#[test]
fn builtin_menu_text_meets_its_minimum() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Menu(_)))).collect();
            // The active text and the shortcut, and each hue's text and shortcut on its tint.
            assert_eq!(pairs.len(), 2 + 2 * MenuHue::ALL.len(), "{} {a:?}", theme.id);
            for p in pairs {
                assert_passes_in_high_contrast(theme, a, p);
            }
        }
    }
}

/// A list row's labels, dimmed labels and detail lines read on the panel and on the fills a row
/// takes (a hidden layer that is selected), the ghost's label on its fill, and the drop marker on
/// its halo, in every built-in, pins included. Standard contrast reports the pairs it misses.
#[test]
fn builtin_list_text_and_drop_marker_meet_their_minimums() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::List(_)))).collect();
            assert_eq!(pairs.len(), 15, "{} {a:?}", theme.id);
            for p in pairs {
                assert_passes_in_high_contrast(theme, a, p);
            }
        }
    }
}

/// The hover row of a list, tree or list box differs from the rest row and from the selected row
/// by the least ratio, and the label still reads on it, in every built-in and appearance.
#[test]
fn builtin_list_hover_is_visible_beside_the_rest_and_selected_rows() {
    let (min_std, min_high) = values::LIST_HOVER_MIN;
    for theme in builtins() {
        for a in appearances() {
            let min = if a.contrast == Contrast::High { min_high } else { min_std };
            let t = resolve(theme, a, None).tokens;
            let c = &t.component;
            let panel = t.color(ColorToken::SurfacePanel);
            let over_panel = |f: Rgba| crate::derive::over(f, panel);
            let list = c.color(ComponentField::List(ListField::Hover)).unwrap();
            let selected = c.color(ComponentField::List(ListField::Selected)).unwrap();
            let who = format!("{} {a:?}", theme.id);
            assert!(contrast_ratio(over_panel(list), panel) >= min, "{who}: list hover on the rest row");
            assert!(contrast_ratio(over_panel(list), over_panel(selected)) >= min, "{who}: list hover on the selected row");
            let popup = t.color(ColorToken::SurfaceRaised);
            let active = c.color(ComponentField::Listbox(ListboxField::ActiveFill)).unwrap();
            assert!(contrast_ratio(crate::derive::over(active, popup), popup) >= min, "{who}: list box active row");
        }
    }
}

/// Soft keeps its own rounder corners at every Size (7/10/14 at Comfortable); the other built-ins
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
    assert_eq!(radii(soft, Size::Comfortable), (7.0, 10.0, 14.0, 0.0, full, full));
    assert_eq!(radii(soft, Size::Compact), (4.0, 6.0, 8.0, 0.0, full, full));
    let pro = builtins().iter().find(|t| t.id.as_str() == "craft.pro").unwrap();
    assert_eq!(radii(pro, Size::Comfortable), (6.0, 8.0, 12.0, 0.0, full, full));
    assert_eq!(radii(pro, Size::Compact), (3.0, 4.0, 6.0, 0.0, full, full));
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
/// included; a disabled one that is on doesn't look off: its mark shows on its box (off has no
/// mark), whether or not the box differs from an off one's.
#[test]
fn builtin_check_marks_and_outlines_meet_the_non_text_minimum() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Check(_)))).collect();
            assert_eq!(pairs.len(), 7, "{} {a:?}", theme.id);
            for p in pairs {
                assert_passes_in_high_contrast(theme, a, p);
            }
            let c = resolve(theme, a, None).tokens.component.check;
            assert_ne!(c.on.mark.disabled, c.on.fill.disabled, "{} {a:?}: a disabled mark doesn't show", theme.id);
        }
    }
}

/// The labels of badges, the keys on keycaps, the progress fill, the swatch selection ring and the
/// mixed dash meet their minimums in every built-in, pins included. Standard contrast reports the pairs it misses.
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
            for p in pairs {
                assert_passes_in_high_contrast(theme, a, p);
            }
        }
    }
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
        assert!(lightness >= 0.93, "{mode:?}: surface.knob {} has lightness {lightness:.3}", knob.to_hex());
        assert_eq!(t.color(ColorToken::LineKnobEdge), Rgba::rgb(0x28, 0x28, 0x28), "{mode:?}: the Pro themes have PhotoCraft's dark ring");
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
fn data_channel_colours_reach_three_to_one_on_the_curve_fill() {
    for theme in builtins() {
        for a in appearances() {
            let t = resolve(theme, a, None).tokens;
            let fill = t.component.curve.fill;
            let d = t.color.data;
            for (name, c) in [("red", d.red), ("green", d.green), ("blue", d.blue), ("cyan", d.cyan), ("magenta", d.magenta), ("yellow", d.yellow)] {
                let ratio = contrast_ratio(c, fill);
                assert!(ratio >= 3.0, "{} {a:?}: data.{name} {} on curve.fill {} is {ratio:.2}", theme.id, c.to_hex(), fill.to_hex());
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

/// A tab's labels, details, ×, and drop marker meet their minimums in every built-in and
/// appearance, pins included. Standard contrast reports the pairs it misses.
#[test]
fn builtin_tab_text_close_and_drop_marker_meet_their_minimums() {
    for theme in builtins() {
        for a in appearances() {
            let (_, steps) = resolve::resolve_with_steps(theme, a, None);
            let pairs: Vec<_> = steps.component_pairs.iter().filter(|p| matches!(p.fg, TokenRef::Component(ComponentField::Tab(_)))).collect();
            assert_eq!(pairs.len(), 9, "{} {a:?}", theme.id);
            for p in pairs {
                assert_passes_in_high_contrast(theme, a, p);
            }
        }
    }
}

/// Bare canvas lines are the accent itself, as PhotoCraft draws them; `overlay.stroke`, drawn over
/// the halo, is lightened to read on it.
#[test]
fn builtin_overlay_line_is_the_plain_accent() {
    let theme = builtins().iter().find(|t| t.id.as_str() == "craft.pro").unwrap();
    let t = resolve(theme, semantic_tests::app(Mode::Dark, Contrast::Standard), None).tokens;
    assert_eq!(t.color(ColorToken::OverlayLine), Rgba::rgb(0x37, 0x8e, 0xf0));
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

/// Classic follows Windows 2000: white text on the navy selection, a black frame round the
/// default (primary) button whose grey face does not change on hover or press, and a black mark on
/// a white check box. No other built-in frames a button.
#[test]
fn classic_follows_windows_2000_and_no_other_theme_frames_a_button() {
    let black = Rgba::rgb(0, 0, 0);
    let white = Rgba::rgb(255, 255, 255);
    let face = Rgba::rgb(0xd4, 0xd0, 0xc8);
    for theme in builtins() {
        let t = resolve(theme, semantic_tests::app(Mode::Light, Contrast::Standard), None).tokens;
        let button = &t.component.button;
        if theme.id.as_str() != "craft.classic" {
            assert_eq!(button.default_frame, Rgba::TRANSPARENT, "{}", theme.id);
            continue;
        }
        assert_eq!(button.default_frame, black);
        assert_eq!([button.primary.fill.rest, button.primary.fill.hover, button.primary.fill.pressed], [face; 3]);
        assert_eq!(t.color(ColorToken::TextOnSelected), white);
        assert_eq!(t.component.check.off.fill.rest, white);
        assert_eq!([t.component.check.on.fill.rest, t.component.check.on.mark.rest], [white, black]);
    }
}

/// Classic's switch has a navy on track, its list box and select rows highlight in navy with
/// white text, its secondary buttons keep the face colour at rest and under the pointer, and its
/// popups are the menu grey. Other built-ins draw the switch track in the solid accent.
#[test]
fn classic_switch_listbox_and_secondary_button_follow_windows_2000() {
    let navy = Rgba::rgb(0x0a, 0x24, 0x6a);
    let white = Rgba::rgb(255, 255, 255);
    let face = Rgba::rgb(0xd4, 0xd0, 0xc8);
    for theme in builtins() {
        for a in appearances() {
            let t = resolve(theme, a, None).tokens;
            let c = &t.component;
            if theme.id.as_str() != "craft.classic" {
                // A switch is the check box's on fill, which a press makes less opaque only on a check box.
                let (switch, on) = (c.check.switch, c.check.on);
                assert_eq!(
                    (switch.fill.rest, switch.fill.hover, switch.fill.disabled),
                    (on.fill.rest, on.fill.hover, on.fill.disabled),
                    "{} {a:?}",
                    theme.id
                );
                assert_eq!(switch.fill, switch.border, "{} {a:?}", theme.id);
                continue;
            }
            if a.contrast == Contrast::High || a.mode != Mode::Light {
                continue;
            }
            assert_eq!([c.check.switch.fill.rest, c.check.switch.fill.hover, c.check.switch.fill.pressed], [navy; 3]);
            assert_eq!((c.listbox.active_fill, c.listbox.active_text), (navy, white));
            assert_eq!([c.button.secondary.fill.rest, c.button.secondary.fill.hover], [face; 2]);
            assert_eq!(c.button.secondary.border.pressed, Rgba::rgb(0x40, 0x40, 0x40));
            assert_eq!((t.color(ColorToken::SurfaceRaised), t.color(ColorToken::TextDisabled)), (face, Rgba::rgb(0x80, 0x80, 0x80)));
        }
    }
}

/// Classic's tooltip is Windows 2000's: InfoText on InfoWindow with a black edge. Every other
/// built-in keeps the popup surface.
#[test]
fn only_classic_gives_tooltips_a_look_of_their_own() {
    for theme in builtins() {
        for a in appearances() {
            let t = resolve(theme, a, None).tokens;
            let tip = &t.component.tooltip;
            if theme.id.as_str() == "craft.classic" {
                assert_eq!(
                    (tip.fill, tip.text, tip.border),
                    (Rgba::rgb(0xff, 0xff, 0xe1), t.color(ColorToken::TextNormal), t.color(ColorToken::TextNormal)),
                    "{a:?}"
                );
            } else {
                assert_eq!(
                    (tip.fill, tip.text, tip.border),
                    (t.color(ColorToken::SurfaceRaised), t.color(ColorToken::TextNormal), t.color(ColorToken::LineBorder)),
                    "{} {a:?}",
                    theme.id
                );
            }
            assert_eq!(tip.radius, t.shape.radius.popup, "{} {a:?}", theme.id);
        }
    }
}

/// The slider follows PhotoCraft (`slider.rs:104-289`, `theme.rs:831-839`) in every built-in: an
/// 18 point control with a 12 point knob and a 3 point track in the control edge colour at
/// Compact, two points more of knob and six more of height at Comfortable; the knob outline is
/// the Pro themes' #282828, the muted text colour in the other light themes and none in the
/// other dark ones; Classic has a 10 x 16 box knob.
#[test]
fn sliders_follow_photocraft() {
    let pro = ["craft.artcraft", "craft.pro", "craft.pro-light-gray"];
    for theme in builtins() {
        for (size, height, thumb) in [(Size::Compact, 18.0, 12.0), (Size::Comfortable, 24.0, 14.0)] {
            for mode in [Mode::Light, Mode::Dark] {
                let a = Appearance { size, ..semantic_tests::app(mode, Contrast::Standard) };
                let t = resolve(theme, a, None).tokens;
                let s = &t.component.slider;
                let id = theme.id.as_str();
                assert_eq!((s.height.md, s.track_height, s.gradient_height, s.track_radius), (height, 3.0, 5.0, 2.0), "{id} {a:?}");
                let classic = id == "craft.classic";
                let want = if classic { (10.0, 16.0) } else { (thumb, thumb) };
                assert_eq!((s.thumb_width.md, s.thumb.md), want, "{id} {a:?}");
                assert_eq!(s.track, t.color(ColorToken::LineControlEdge), "{id} {a:?}");
                if classic {
                    continue;
                }
                assert_eq!(t.elevation.knob, values::KNOB_SHADOW, "{id} {a:?}");
                let edge = t.color(ColorToken::LineKnobEdge);
                if pro.contains(&id) {
                    assert_eq!(edge, Rgba::rgb(0x28, 0x28, 0x28), "{id} {a:?}");
                } else if theme.variants.mode_for(mode) == Mode::Light {
                    assert_eq!(edge, t.color(ColorToken::TextMuted), "{id} {a:?}");
                } else {
                    assert_eq!(edge.a(), 0, "{id} {a:?}");
                }
            }
        }
    }
}

/// A check box or radio button is PhotoCraft's: 14 points at Compact, a 6 point label gap, radius
/// 2 and a 1.5 point outline in every theme; unchecked, the outline is the faint text colour at
/// rest and the muted text colour hovered, and the fill darkens when pressed; checked, the fill
/// is the solid accent at rest and hovered and a quarter less opaque when pressed.
#[test]
fn check_boxes_are_photocraft_boxes_in_every_theme() {
    for theme in builtins() {
        for a in appearances() {
            let r = resolve(theme, a, None).tokens;
            let (c, color) = (&r.component.check, &r.color);
            let compact = a.size == Size::Compact;
            let which = format!("{} {a:?}", theme.id);
            assert_eq!((c.box_size.md, c.box_gap), if compact { (14.0, 6.0) } else { (16.0, 8.0) }, "{which}");
            assert_eq!((c.radius, c.outline), (2.0, 1.5), "{which}");
            assert_eq!(c.mark.md, c.box_size.md * 0.125, "{which}");
            assert_eq!([c.off.border.hover, c.off.border.pressed], [color.text.muted; 2], "{which}");
            let [rest, pressed] = [c.off.fill.rest, c.off.fill.pressed].map(|f| f.0.iter().take(3).map(|&v| u32::from(v)).sum::<u32>());
            assert!(pressed < rest || rest == 0, "{which}: pressed darkens");
            if theme.id.as_str() == "craft.classic" {
                continue;
            }
            assert_eq!([c.on.fill.rest, c.on.fill.hover], [color.accent.solid; 2], "{which}");
            assert_eq!(c.on.fill.pressed, color.accent.solid.with_alpha(191), "{which}");
            assert_eq!(c.on.border.rest, Rgba::TRANSPARENT, "{which}: no outline on");
        }
    }
}

/// PhotoCraft's buttons: the Pro themes draw the secondary button as a transparent pill outlined
/// 1.5 points in `text.muted`, `text.normal` once hovered, filled with `state.hover` then and
/// pressed; the other themes fill it with `surface.sunken`, 1 point edged, and show it pressed at
/// 85% opacity. Studio and Soft take their own radius; the primary button shows 93% and 85% of its
/// fill when hovered and pressed (90% and 80% in the Pro themes).
#[test]
fn buttons_follow_photocraft() {
    for theme in builtins() {
        let id = theme.id.as_str();
        for mode in [Mode::Light, Mode::Dark] {
            if (mode == Mode::Dark && theme.variants == Variants::LightOnly) || (mode == Mode::Light && theme.variants == Variants::DarkOnly) {
                continue;
            }
            let a = semantic_tests::app(mode, Contrast::Standard);
            let t = resolve(theme, a, None).tokens;
            let b = &t.component.button;
            let secondary = &b.secondary;
            assert_eq!((b.height.md, b.pad_x.md), (28.0, 14.0), "{id}");
            let primary = b.primary.fill;
            let pro = ["craft.artcraft", "craft.pro", "craft.pro-light-gray"].contains(&id);
            let (hover, pressed) = if pro { (0.9, 0.8) } else { (0.93, 0.85) };
            if id != "craft.classic" {
                assert_eq!((b.hover_opacity, b.pressed_opacity), (hover, pressed), "{id}");
                assert_eq!(primary.hover, primary.rest.faded(hover), "{id} {mode:?}");
                assert_eq!(primary.pressed, primary.rest.faded(pressed), "{id} {mode:?}");
                assert_eq!([b.primary.text.hover, b.primary.text.pressed], [b.primary.text.rest; 2], "{id} {mode:?}");
            }
            if pro || ["craft.adwaita", "craft.breeze", "craft.soft"].contains(&id) {
                assert_eq!(t.color(ColorToken::TextOnAccent), Rgba::WHITE, "{id} {mode:?}: PhotoCraft draws white on its accent");
            }
            if pro {
                assert_eq!(b.border_width, 1.5, "{id}");
                assert_eq!(secondary.fill.rest, Rgba::TRANSPARENT, "{id} {mode:?}");
                assert_eq!([secondary.fill.hover, secondary.fill.pressed], [t.color(ColorToken::StateHover); 2], "{id} {mode:?}");
                assert_eq!(secondary.border.rest, t.color(ColorToken::TextMuted), "{id} {mode:?}");
                assert_eq!([secondary.border.hover, secondary.border.pressed], [t.color(ColorToken::TextNormal); 2], "{id} {mode:?}");
                assert_eq!(b.radius, 999.0, "{id}");
            } else {
                assert_eq!(b.border_width, 1.0, "{id}");
                if id != "craft.classic" {
                    let sunken = t.color(ColorToken::SurfaceSunken);
                    assert_eq!(secondary.fill.rest, sunken, "{id} {mode:?}");
                    assert_eq!(secondary.fill.pressed, sunken.faded(0.85), "{id} {mode:?}");
                    assert_eq!(secondary.fill.hover, t.color(ColorToken::StateHover), "{id} {mode:?}");
                    assert_eq!(secondary.border.rest, t.color(ColorToken::LineControlEdge), "{id} {mode:?}");
                }
            }
            match id {
                "craft.studio" => assert_eq!(b.radius, 6.0),
                "craft.soft" => assert_eq!(b.radius, 7.0),
                _ => {}
            }
        }
    }
}

/// Selected rows and icon buttons follow PhotoCraft (`theme.rs` `row_selected`, `accent_soft`,
/// `accent_border`, `accent_text`; `layer_row.rs:283-286`, `icons.rs:156-163`): the selected row
/// is `state.selected`, or `accent.soft` where PhotoCraft's `row_selected` is transparent; hover is
/// half-opacity `state.hover`; a selected icon button is `accent.soft` in a `accent.solid`
/// outline with PhotoCraft's `accent_text` glyph; the thumbnail frame is #141414 in the Pro themes.
#[test]
fn selected_rows_and_icon_buttons_follow_photocraft() {
    // (theme, mode, PhotoCraft accent_text)
    let accent_text = [
        ("craft.artcraft", Mode::Dark, "#ffffff"),
        ("craft.artcraft", Mode::Light, "#1062c4"),
        ("craft.pro", Mode::Dark, "#ffffff"),
        ("craft.pro", Mode::Light, "#1062c4"),
        ("craft.pro-light-gray", Mode::Light, "#004296"),
        ("craft.studio", Mode::Dark, "#d6d0ff"),
        ("craft.studio", Mode::Light, "#5040c8"),
        ("craft.adwaita", Mode::Dark, "#78aeed"),
        ("craft.adwaita", Mode::Light, "#1c71d8"),
        ("craft.breeze", Mode::Dark, "#50b8ee"),
        ("craft.breeze", Mode::Light, "#1e5e88"),
        ("craft.soft", Mode::Dark, "#bec8fc"),
        ("craft.soft", Mode::Light, "#2e3a92"),
        ("craft.solarized-dark", Mode::Dark, "#fdf6e3"),
        ("craft.classic", Mode::Light, "#ffffff"),
    ];
    for theme in builtins() {
        let id = theme.id.as_str();
        for mode in [Mode::Light, Mode::Dark] {
            if (mode == Mode::Dark && theme.variants == Variants::LightOnly) || (mode == Mode::Light && theme.variants == Variants::DarkOnly) {
                continue;
            }
            let t = resolve(theme, semantic_tests::app(mode, Contrast::Standard), None).tokens;
            let (list, icon) = (&t.component.list, &t.component.icon_button);
            let who = format!("{id} {mode:?}");
            let transparent = ["craft.studio", "craft.adwaita", "craft.breeze", "craft.soft", "craft.solarized-dark"].contains(&id);
            let selected = if transparent { t.color(ColorToken::AccentSoft) } else { t.color(ColorToken::StateSelected) };
            assert_eq!(list.selected, selected, "{who}");
            assert_eq!(list.hover.a(), 128, "{who}: state.hover at half opacity");
            assert_eq!(list.row_height, 24.0, "{who}");
            assert_eq!(
                (icon.selected.on_fill, icon.selected.on_border),
                (t.color(ColorToken::AccentSoft), t.color(ColorToken::AccentSolid)),
                "{who}"
            );
            assert_eq!((icon.size.md, icon.icon.md, icon.size.sm, icon.icon.sm), (24.0, 12.0, 20.0, 10.0), "{who}");
            if let Some((_, _, glyph)) = accent_text.iter().find(|(i, m, _)| *i == id && *m == mode) {
                assert_eq!(icon.selected.on_text, semantic_tests::hex(glyph), "{who}");
            }
            if ["craft.artcraft", "craft.pro", "craft.pro-light-gray"].contains(&id) {
                assert_eq!(
                    (list.thumb_border, list.thumb_border_selected),
                    (semantic_tests::hex("#141414"), semantic_tests::hex("#141414")),
                    "{who}"
                );
            } else {
                assert_eq!(list.thumb_border, t.color(ColorToken::LineControlEdge), "{who}");
            }
        }
    }
}
