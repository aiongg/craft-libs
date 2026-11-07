//! Style cases: a listed variant looks unlike the default, every flag look of the style is reachable by some state
//! of its type, and text and icons read on what they sit on. They measure the styles the pipeline resolved while
//! the type's stories ran (settled in each mode and contrast, then driven), resolved again for every listed variant
//! and built-in theme. A type with no story is measured through its `#[component(sample = ..)]`. A type with
//! neither fails, since its style is never measured.

use super::drive::{run, settled, variants};
use super::{ANY, Case, Subject};
use crate::style::UiTokens;
use crate::style::{StyleColor, SurfaceKind, SurfaceLook};
use crate::testing::fixture::Fixture;
use crate::testing::style::{StyleLooks, StyleSeen, themes_at, tokens, with_cx};
use craft_behavior::Input;
use craft_theme::{Contrast, ControlSize, Flags, Mode, Resolve, Rgba, State, StateMix, Stateful, contrast_ratio, text_contrast_min};
use std::sync::Arc;

pub(super) const CASES: &[Case] = &[
    Case::new("variants-differ", ANY, listed_variants_differ_from_the_default),
    Case::new("looks-reachable", ANY, every_flag_look_is_reachable),
    Case::new("text-contrast", ANY, text_and_icons_meet_the_text_minimum),
];

/// A style of the subject that the cases measure: one the pipeline resolved for a story, or the one of the type's
/// sample.
enum Source {
    Seen(Arc<StyleSeen>),
    Sample,
}

/// The styles of `subject` the stories resolve: each story settled in every mode and contrast, then each of its
/// drives run once in the default mode. One entry per style key, variant and size.
fn sources(subject: &Subject) -> Result<Vec<Source>, String> {
    let name = subject.probe.type_name();
    let mut seen: Vec<Arc<StyleSeen>> = Vec::new();
    let mut take = |fixture: &Fixture| {
        for style in fixture.styles_of(name) {
            if !seen.iter().any(|known| known.inputs == style.inputs && known.variant == style.variant && known.size == style.size) {
                seen.push(style);
            }
        }
    };
    for story in &subject.stories {
        for mode in [Mode::Light, Mode::Dark] {
            for contrast in [Contrast::Standard, Contrast::High] {
                let mut fixture = Fixture::story(story);
                fixture.theme(mode, contrast);
                fixture.settle();
                take(&fixture);
            }
        }
        let (_, plan) = settled(story, subject.probe);
        for variant in variants(plan, subject.probe) {
            let (mut fixture, _) = settled(story, subject.probe);
            run(&mut fixture, plan, variant);
            take(&fixture);
        }
    }
    if !seen.is_empty() {
        return Ok(seen.into_iter().map(Source::Seen).collect());
    }
    if subject.probe.sample.is_some() {
        return Ok(vec![Source::Sample]);
    }
    Err(format!("{name} has a style and neither a story that shows it nor a sample, so no measurement reads its style"))
}

/// The resolved looks of `source` under `variant` and `tokens`.
fn looks_in(subject: &Subject, source: &Source, tokens: &UiTokens, variant: craft_theme::Variant) -> Option<StyleLooks> {
    let ctx = egui::Context::default();
    match source {
        Source::Seen(seen) => Some(with_cx(&ctx, tokens, variant, seen.size, |cx| seen.resolve_in(cx))),
        Source::Sample => {
            let make = subject.probe.sample?;
            let instance = make();
            with_cx(&ctx, tokens, variant, ControlSize::Md, |cx| (subject.probe.looks)(&*instance, cx))
        }
    }
}

fn looks_under(subject: &Subject, source: &Source, variant: craft_theme::Variant) -> Option<StyleLooks> {
    looks_in(subject, source, &tokens(Mode::Light, Contrast::Standard), variant)
}

fn listed_variants_differ_from_the_default(subject: &Subject) -> Result<(), String> {
    for source in sources(subject)? {
        let Some(listed) = looks_under(subject, &source, craft_theme::Variant::SECONDARY) else {
            continue;
        };
        let Some((&default, others)) = listed.variants.split_first() else {
            continue;
        };
        let base = looks_under(subject, &source, default);
        for &variant in others {
            if let (Some(base), Some(looks)) = (&base, looks_under(subject, &source, variant))
                && looks.same_as(base)
            {
                return Err(format!("the variant {variant:?} resolves to the style of {default:?}"));
            }
        }
    }
    Ok(())
}

/// The flags some state of the type can set (core 4.1, "Reachable flags"): those step 12 takes from any component's
/// props, scope and bound command or from its `flags()`, and those its input, value, popup and shape add.
fn reachable(subject: &Subject) -> Flags {
    let probe = subject.probe;
    let mut flags = Flags::READ_ONLY | Flags::SELECTED | Flags::CHECKED | Flags::BUSY | Flags::EMPHASIZED;
    if !matches!(probe.input, Input::Display) {
        flags |= Flags::FOCUSED | Flags::FOCUS_VISIBLE;
    }
    if probe.carries_value() {
        flags |= Flags::MIXED | Flags::INVALID | Flags::WARN;
    }
    if probe.overlay {
        flags |= Flags::OPEN;
    }
    if matches!(probe.shape, crate::contract::ShapeKind::Items) {
        flags |= Flags::LEAD | Flags::DRAGGING | Flags::DROP_TARGET;
    }
    if !matches!(probe.shape, crate::contract::ShapeKind::Leaf) {
        flags |= Flags::EXPANDED;
    }
    flags
}

fn unreachable_in(look: &Stateful<StyleColor>, reachable: Flags) -> Option<String> {
    for (mask, _) in &look.when {
        if !reachable.contains(mask.0) {
            return Some(format!("a `when` entry needs {:?}, which no state of the type sets", mask.0 - reachable));
        }
    }
    if look.on.is_some() && !reachable.intersects(Flags::ON) {
        return Some("an `on` look, and no state of the type sets selected, checked or mixed".to_owned());
    }
    None
}

fn every_flag_look_is_reachable(subject: &Subject) -> Result<(), String> {
    let reachable = reachable(subject);
    for source in sources(subject)? {
        let Some(looks) = looks_under(subject, &source, craft_theme::Variant::SECONDARY) else {
            continue;
        };
        if let Some(SurfaceLook { fill, border, .. }) = &looks.surface {
            if let Some(problem) = unreachable_in(fill, reachable) {
                return Err(format!("the surface fill has {problem}"));
            }
            if let Some(problem) = border.as_ref().and_then(|border| unreachable_in(border, reachable)) {
                return Err(format!("the surface border has {problem}"));
            }
        }
        if looks.focus.is_some() && !reachable.intersects(Flags::FOCUSED | Flags::FOCUS_VISIBLE) {
            return Err("the style has a focus ring and the type takes no focus".to_owned());
        }
    }
    Ok(())
}

/// The flag sets worth resolving a state in: none, and each `when` mask of `looks` that the type can reach.
fn flag_sets<'a>(looks: impl Iterator<Item = &'a Stateful<crate::style::StyleColor>>, reachable: Flags) -> Vec<Flags> {
    let mut sets = vec![Flags::empty()];
    for look in looks {
        sets.extend(look.when.iter().map(|(mask, _)| mask.0).filter(|mask| reachable.contains(*mask)));
    }
    sets
}

/// The states that stay readable under WCAG: rest, hover and pressed, each with the `on` axis off and on.
const READABLE: [(State, StateMix); 6] = [
    (State::Rest, StateMix::new(0.0, 0.0, 0.0)),
    (State::Hover, StateMix::new(1.0, 0.0, 0.0)),
    (State::Pressed, StateMix::new(1.0, 1.0, 0.0)),
    (State::Rest, StateMix::new(0.0, 0.0, 1.0)),
    (State::Hover, StateMix::new(1.0, 0.0, 1.0)),
    (State::Pressed, StateMix::new(1.0, 1.0, 1.0)),
];

/// The opaque surfaces a component without a fill of its own can sit on.
const GROUNDS: [SurfaceKind; 6] =
    [SurfaceKind::Background, SurfaceKind::Canvas, SurfaceKind::Chrome, SurfaceKind::Panel, SurfaceKind::Raised, SurfaceKind::Sunken];

/// Text and icon colours against what they are painted on, in every state the type reaches (disabled controls are
/// exempt, as in WCAG 1.4.3), every variant it lists and every built-in theme at both contrasts. A style with a
/// surface fill is measured against that fill; a style without one against each of `GROUNDS`.
fn text_and_icons_meet_the_text_minimum(subject: &Subject) -> Result<(), String> {
    let reachable = reachable(subject);
    for source in sources(subject)? {
        measure_contrast(subject, &source, reachable)?;
    }
    Ok(())
}

fn measure_contrast(subject: &Subject, source: &Source, reachable: Flags) -> Result<(), String> {
    for contrast in [Contrast::Standard, Contrast::High] {
        let min = text_contrast_min(contrast);
        for (theme, tokens) in themes_at(contrast) {
            let Some(first) = looks_in(subject, source, &tokens, craft_theme::Variant::SECONDARY) else {
                continue;
            };
            let variants = if first.variants.is_empty() { &[craft_theme::Variant::SECONDARY][..] } else { first.variants };
            for &variant in variants {
                let Some(looks) = looks_in(subject, source, &tokens, variant) else {
                    continue;
                };
                let fill = looks.surface.as_ref().map(|surface| &surface.fill);
                let grounds: Vec<(String, egui::Color32)> =
                    GROUNDS.iter().map(|&kind| (format!("{kind:?}"), crate::style::surface_style(&tokens, kind).fill)).collect();
                let inks = looks.texts.iter().map(|text| ("text", &text.color)).chain(looks.icons.iter().map(|icon| ("icon", &icon.color)));
                for (what, ink) in inks {
                    for flags in flag_sets(fill.into_iter().chain([ink]), reachable) {
                        for (state, t) in READABLE {
                            let paint = ink.resolve(state, flags, t).egui();
                            let under = match fill {
                                Some(fill) => vec![("surface fill".to_owned(), fill.resolve(state, flags, t).egui())],
                                None => grounds.clone(),
                            };
                            for (name, ground) in under {
                                if ground.a() == 0 {
                                    continue;
                                }
                                let ratio = contrast_ratio(Rgba::from(paint), Rgba::from(ground));
                                if ratio < min {
                                    return Err(format!(
                                        "{theme} {contrast:?} {variant:?}: {what} {paint:?} on {name} {ground:?} in {state:?} with {flags:?} has contrast {ratio:.2}, under {min}"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
