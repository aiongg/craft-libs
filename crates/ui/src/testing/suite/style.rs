//! Style cases: a listed variant looks unlike the default, and every flag look of the style is reachable by some
//! state of its type. They read the style of a type's sample instance, so a type with no sample (`#[component(sample =
//! ..)]`) has no style to read here.

use super::{ANY, Case, Subject};
use crate::style::UiTokens;
use crate::style::{StyleColor, SurfaceLook};
use crate::testing::style::{themes_at, tokens, with_cx};
use craft_behavior::Input;
use craft_theme::{Contrast, ControlSize, Flags, Mode, Resolve, Rgba, State, StateMix, Stateful, contrast_ratio, text_contrast_min};

pub(super) const CASES: &[Case] = &[
    Case::new("variants-differ", ANY, listed_variants_differ_from_the_default),
    Case::new("looks-reachable", ANY, every_flag_look_is_reachable),
    Case::new("text-contrast", ANY, text_and_icons_meet_the_text_minimum),
];

/// The resolved looks of the sample of `subject` under `variant` and `tokens`; `None` for a type with no sample.
fn looks_in(subject: &Subject, tokens: &UiTokens, variant: craft_theme::Variant) -> Option<crate::testing::style::StyleLooks> {
    let make = subject.probe.sample?;
    let instance = make();
    let ctx = egui::Context::default();
    with_cx(&ctx, tokens, variant, ControlSize::Md, |cx| (subject.probe.looks)(&*instance, cx))
}

fn looks_under(subject: &Subject, variant: craft_theme::Variant) -> Option<crate::testing::style::StyleLooks> {
    looks_in(subject, &tokens(Mode::Light, Contrast::Standard), variant)
}

fn listed_variants_differ_from_the_default(subject: &Subject) -> Result<(), String> {
    let Some(listed) = looks_under(subject, craft_theme::Variant::SECONDARY) else {
        return Ok(());
    };
    let Some((&default, others)) = listed.variants.split_first() else {
        return Ok(());
    };
    let base = looks_under(subject, default);
    for &variant in others {
        if let (Some(base), Some(looks)) = (&base, looks_under(subject, variant))
            && looks.same_as(base)
        {
            return Err(format!("the variant {variant:?} resolves to the style of {default:?}"));
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
    let Some(looks) = looks_under(subject, craft_theme::Variant::SECONDARY) else {
        return Ok(());
    };
    let reachable = reachable(subject);
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

/// Text and icon colours against the surface fill they are painted on, in every state the type reaches (disabled
/// controls are exempt, as in WCAG 1.4.3), every variant it lists and every built-in theme at both contrasts. A
/// style with no surface fill paints on what lies behind it, so it has nothing to measure here.
fn text_and_icons_meet_the_text_minimum(subject: &Subject) -> Result<(), String> {
    if subject.probe.sample.is_none() {
        return Ok(());
    }
    let reachable = reachable(subject);
    for contrast in [Contrast::Standard, Contrast::High] {
        let min = text_contrast_min(contrast);
        for (theme, tokens) in themes_at(contrast) {
            let Some(first) = looks_in(subject, &tokens, craft_theme::Variant::SECONDARY) else {
                return Ok(());
            };
            let variants = if first.variants.is_empty() { &[craft_theme::Variant::SECONDARY][..] } else { first.variants };
            for &variant in variants {
                let Some(looks) = looks_in(subject, &tokens, variant) else {
                    continue;
                };
                let Some(SurfaceLook { fill, .. }) = &looks.surface else {
                    continue;
                };
                let inks = looks.texts.iter().map(|text| ("text", &text.color)).chain(looks.icons.iter().map(|icon| ("icon", &icon.color)));
                for (what, ink) in inks {
                    for flags in flag_sets([fill, ink].into_iter(), reachable) {
                        for (state, t) in READABLE {
                            let (surface, ink) = (fill.resolve(state, flags, t).egui(), ink.resolve(state, flags, t).egui());
                            if surface.a() == 0 {
                                continue;
                            }
                            let ratio = contrast_ratio(Rgba::from(ink), Rgba::from(surface));
                            if ratio < min {
                                return Err(format!(
                                    "{theme} {contrast:?} {variant:?}: {what} {ink:?} on {surface:?} in {state:?} with {flags:?} has contrast {ratio:.2}, under {min}"
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
