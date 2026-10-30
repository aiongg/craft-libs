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
    // One thread per theme: an explanation resolves again with its steps.
    std::thread::scope(|scope| {
        for theme in builtins() {
            scope.spawn(move || {
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
            });
        }
    });
}

#[test]
fn pins_report_their_layer() {
    let photo = AppId::parse("photocraft").unwrap();
    let grey = Rgba::rgb(0x40, 0x40, 0x40);
    let mut def = unpinned();
    def.pins.dark = Pins::from_iter([(T::LineDivider, grey), (T::SurfaceRaised, grey)]);
    def.pins.dark.colors.insert(TokenRef::Component(field("button.ghost.border.rest")), grey);
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
    assert!(!pairs.is_empty() && pairs.iter().all(|p| p.moved_by_pass && p.pair.passes()));
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
    assert_eq!(reads("button.secondary.border.rest"), Some(T::LineControlEdge));
    assert_eq!(reads("button.ghost.fill.rest"), None, "transparent is a constant");
    assert!(e.field(field("button.radius")).is_none(), "only colour fields are listed");
}

#[test]
fn readers_are_the_fields_that_read_the_token() {
    let e = explain(&graphite(), appearance(Mode::Light, Contrast::Standard, Size::Compact), None);
    assert!(e.readers(T::AccentSolid).contains(&field("button.primary.fill.rest")));
    for &t in ColorToken::ALL.iter() {
        for f in e.readers(t) {
            assert_eq!(e.field(f).unwrap().reads, Some(t));
        }
    }
    let listed: usize = ColorToken::ALL.iter().map(|&t| e.readers(t).len()).sum();
    assert_eq!(listed, e.components.iter().filter(|o| o.reads.is_some()).count(), "every reading field is listed once");
    let order = |fs: Vec<ComponentField>| fs.iter().map(|f| ComponentField::ALL.iter().position(|a| a == f).unwrap()).collect::<Vec<_>>();
    let idx = order(e.readers(T::AccentSolid));
    assert!(idx.windows(2).all(|w| w[0] < w[1]), "ComponentField order");
    let unread = ColorToken::ALL.iter().find(|&&t| e.readers(t).is_empty());
    assert!(unread.is_some(), "some token is read by no field");
}

#[test]
fn every_warning_is_a_failing_pair_with_its_ratio() {
    let a = appearance(Mode::Dark, Contrast::Standard, Size::Compact);
    let mut def = unpinned();
    let panel = resolve(&def, a, None).tokens.color(T::SurfacePanel);
    def.pins.dark = Pins::from_iter([(TokenRef::Color(T::TextNormal), panel)]);
    def.pins.dark.colors.insert(TokenRef::Component(field("button.primary.fill.rest")), Rgba::rgb(0x80, 0x80, 0x80));
    def.pins.dark.colors.insert(TokenRef::Component(field("button.primary.text.rest")), Rgba::rgb(0x88, 0x88, 0x88));

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
            let found =
                e.pairs.iter().find(|p| p.pair.fg == w.fg && p.pair.bg == w.bg).unwrap_or_else(|| panic!("{} {a:?}: {w:?} not in pairs", theme.id));
            assert_eq!(found.pair, *w);
            assert!(!found.pair.passes() && !found.fixed_by_pass);
            match w.fg {
                TokenRef::Color(_) => semantic += 1,
                TokenRef::Component(_) => component += 1,
            }
        }
    }
    assert!(semantic > 0 && component > 0, "the pinned theme has warnings of both layers");
}

#[test]
fn high_contrast_moves_a_pinned_token() {
    let pin = Rgba::rgb(0x55, 0x55, 0x55);
    let mut def = unpinned();
    def.pins.dark = Pins::from_iter([(T::TextFaint, pin)]);
    let standard = explain(&def, appearance(Mode::Dark, Contrast::Standard, Size::Compact), None);
    let faint = standard.color(T::TextFaint).unwrap();
    assert_eq!((faint.pin, faint.contrast_move, faint.value), (Some(PinLayer::Theme), None, pin));
    let high = explain(&def, appearance(Mode::Dark, Contrast::High, Size::Compact), None);
    let faint = high.color(T::TextFaint).unwrap();
    assert_eq!((faint.pin, faint.contrast_move), (Some(PinLayer::Theme), Some(pin)));
    assert_ne!(faint.value, pin);
}

#[test]
fn aliases_derive_from_their_targets_pins() {
    let a = appearance(Mode::Dark, Contrast::Standard, Size::Compact);
    let plain = explain(&unpinned(), a, None);
    let pin = Rgba::rgb(0xa0, 0xa0, 0xa0);
    let mut def = unpinned();
    def.pins.dark = Pins::from_iter([(T::TextFaint, pin)]);
    let e = explain(&def, a, None);
    let inactive = e.color(T::CaptionInactive).unwrap();
    assert_eq!((inactive.source, inactive.pin), (ColorSource::Same(T::TextFaint), None));
    assert_eq!((inactive.derived, inactive.contrast_move, inactive.value), (pin, None, pin));

    // A pinned alias derives from its target's unpinned value.
    def.pins.dark = Pins::from_iter([(T::CaptionInactive, pin)]);
    let e = explain(&def, a, None);
    let inactive = e.color(T::CaptionInactive).unwrap();
    assert_eq!((inactive.pin, inactive.value), (Some(PinLayer::Theme), pin));
    assert_eq!(inactive.derived, plain.color(T::TextFaint).unwrap().derived);

    // On-solid text follows a pinned solid.
    let light = appearance(Mode::Light, Contrast::Standard, Size::Compact);
    let mut def = unpinned();
    def.pins.light = Pins::from_iter([(T::AccentSolid, Rgba::rgb(0x20, 0x20, 0x40))]);
    assert_eq!(explain(&unpinned(), light, None).color(T::TextOnAccent).unwrap().derived, Rgba::BLACK);
    let e = explain(&def, light, None);
    let on_accent = e.color(T::TextOnAccent).unwrap();
    let before_pass = on_accent.contrast_move.unwrap_or(on_accent.value);
    assert_eq!((on_accent.pin, on_accent.derived, before_pass), (None, Rgba::WHITE, Rgba::WHITE));
}

#[test]
fn fixed_and_moved_pairs_differ() {
    let pair = |e: &Explained, fg: ColorToken, bg: ColorToken| *e.pairs.iter().find(|p| p.pair.fg == fg.into() && p.pair.bg == bg.into()).unwrap();
    let high = appearance(Mode::Dark, Contrast::High, Size::Compact);
    let e = explain(&graphite(), high, None);

    let fixed = pair(&e, T::TextFaint, T::SurfacePanel);
    assert!(fixed.ratio_before < fixed.pair.minimum && fixed.pair.passes());
    assert!(fixed.moved_by_pass && fixed.fixed_by_pass);

    let moved = pair(&e, T::TextMuted, T::SurfaceBackground);
    assert!(moved.ratio_before >= moved.pair.minimum && moved.ratio_before != moved.pair.ratio);
    assert!(moved.moved_by_pass && !moved.fixed_by_pass);

    // The semantic pass moved `accent.solid` and `text.on_accent`, which the button reads.
    let button = e.pairs.iter().find(|p| p.pair.fg == field("button.primary.text.rest").into()).unwrap();
    assert_eq!(button.pair.bg, field("button.primary.fill.rest").into());
    assert!(button.moved_by_pass && button.fixed_by_pass);

    // A pinned panel no text can reach 7:1 on: `text.normal` moves and still fails.
    let mut def = unpinned();
    def.high_contrast.dark = Pins::from_iter([(T::SurfacePanel, Rgba::rgb(0x77, 0x77, 0x77))]);
    let e = explain(&def, high, None);
    let failing = pair(&e, T::TextNormal, T::SurfacePanel);
    assert!(!failing.pair.passes() && failing.ratio_before < failing.pair.minimum);
    assert!(failing.moved_by_pass && !failing.fixed_by_pass);
}

#[test]
fn reads_are_what_both_probes_agree_on() {
    use super::explain::{PROBES, agree, default_reads, probe_reads};
    let (a, b) = (probe_reads(PROBES[0]), probe_reads(PROBES[1]));
    // The hover fill of lists is `state.hover` moved only when it is too close
    // to the row beside it, so whether a probe reads it depends on the probe's colours.
    let moved_when_close = [ComponentField::List(ListField::Hover)];
    // Selected fills and their text move in lightness to show on what they sit on, so what they
    // read depends on the probe's colours too.
    let moved_to_show = |f: &ComponentField| {
        matches!(
            f,
            ComponentField::List(
                ListField::Selected | ListField::SelectedHover | ListField::SelectedText | ListField::SelectedDimmed | ListField::SelectedDetail
            ) | ComponentField::IconButton(_)
                | ComponentField::Segmented(SegmentedField::Selected { .. })
                | ComponentField::Dock(DockField::IconSelected | DockField::IconSelectedText)
                | ComponentField::Button(ButtonField::Color { variant: ButtonVariant::Secondary, state: State::Pressed, .. })
                | ComponentField::Check(CheckField::Off { part: CheckOffPart::Border, state: State::Rest })
        )
    };
    let steady = |reads: &[(ComponentField, Option<T>)]| {
        reads.iter().filter(|(f, _)| !moved_when_close.contains(f) && !moved_to_show(f)).copied().collect::<Vec<_>>()
    };
    assert_eq!(steady(&a), steady(&b), "today's defaults read the same tokens under either probe");
    let reads = default_reads();
    assert_eq!(steady(&reads), steady(&a));
    assert!(reads.iter().filter(|(f, _)| moved_when_close.contains(f)).all(|&(_, t)| t.is_none() || t == Some(T::StateHover)));
    let f = field("button.primary.fill.rest");
    assert_eq!(agree(&[(f, Some(T::AccentSolid))], &[(f, Some(T::AccentHover))]), [(f, None)]);
    assert_eq!(agree(&[(f, Some(T::AccentSolid))], &[(f, None)]), [(f, None)]);
}

#[test]
fn a_pulled_anchored_token_names_its_target_and_the_pair() {
    let mut def = unpinned();
    def.pins.dark = Pins::from_iter([(T::SurfacePanel, Rgba::from_hex("#555555").unwrap())]);
    let e = explain(&def, appearance(Mode::Dark, Contrast::High, Size::Compact), None);
    let raised = e.color(T::SurfaceRaised).unwrap();
    assert_eq!(raised.source, ColorSource::Anchored(4));
    assert_eq!(raised.pass_move, Some(PassMove::Pulled(T::TextNormal)));
    assert_eq!(raised.contrast_move, Some(raised.derived));
    assert_eq!(raised.source_words(), "neutral 4, anchored on surface.panel, pulled toward it for text.normal");
    assert_eq!(e.color(T::LineDivider).unwrap().source_words(), "neutral 6, anchored on surface.panel");
    // Text the pull can't serve moves the fill away instead, and the explanation names that move.
    let selected_hover = e.color(T::StateSelectedHover).unwrap();
    assert_eq!(selected_hover.pass_move, Some(PassMove::Relaxed(T::TextOnSelected)));
    assert_eq!(selected_hover.source_words(), "neutral 7, anchored on surface.panel, moved away from text.on_selected for high contrast");
    assert_eq!(e.color(T::EdgeLight).unwrap().source_words(), "surface.panel lit by 0.15");
    let anchoring = raised.anchoring.unwrap();
    assert_eq!((anchoring.on, anchoring.steps.step(4)), (T::SurfacePanel, raised.derived), "the strip holds the target");
}

#[test]
fn explained_sizes_name_the_layer_that_set_each_value() {
    let sizes =
        AppSizes::parse(br#"{ "all": { "size.control.md": 28, "typography.body.size": 15 }, "comfortable": { "toolbar.button": 40 } }"#).unwrap();
    let theme = unpinned();
    let origin = |e: &Explained, path: &str| e.sizes.iter().find(|o| o.path == path).cloned().unwrap_or_else(|| panic!("no size {path}"));

    let plain = explain(&theme, appearance(Mode::Dark, Contrast::Standard, Size::Comfortable), None);
    assert!(plain.sizes.iter().all(|o| !o.layer.from_app()), "no app layer, no app origin");
    assert_eq!(origin(&plain, "size.control.md").layer, SizeLayer::SizeSet);
    assert_eq!(origin(&plain, "list.row_height").layer, SizeLayer::Derived);

    let comfortable = explain_sized(&theme, appearance(Mode::Dark, Contrast::Standard, Size::Comfortable), None, &sizes);
    assert_eq!(comfortable.resolved, resolve_sized(&theme, appearance(Mode::Dark, Contrast::Standard, Size::Comfortable), None, &sizes));
    let control = origin(&comfortable, "size.control.md");
    assert_eq!((control.layer, control.value, control.range), (SizeLayer::App, SizeValue::Number(28.0), (16.0, 48.0)));
    assert_eq!(origin(&comfortable, "list.row_height").layer, SizeLayer::Derived, "derived from the override, not set by it");
    assert_eq!(origin(&comfortable, "list.row_height").value, SizeValue::Number(28.0));
    assert_eq!(origin(&comfortable, "toolbar.button").layer, SizeLayer::AppPin);
    assert_eq!(origin(&comfortable, "space.gap.row").layer, SizeLayer::SizeSet);

    let compact = explain_sized(&theme, appearance(Mode::Dark, Contrast::Standard, Size::Compact), None, &sizes);
    assert_eq!(origin(&compact, "toolbar.button").layer, SizeLayer::Derived, "a pin for the other set does not apply");
}
