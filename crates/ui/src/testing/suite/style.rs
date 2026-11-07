//! Style cases: a listed variant looks unlike the default, and every flag look of the style is reachable by some
//! state of its type. They read the style of a type's sample instance, so a type with no sample (`#[component(sample =
//! ..)]`) has no style to read here.

use super::{ANY, Case, Subject};
use crate::style::{StyleColor, SurfaceLook};
use crate::testing::style::{tokens, with_cx};
use craft_behavior::Input;
use craft_theme::{Contrast, ControlSize, Flags, Mode, Stateful};

pub(super) const CASES: &[Case] =
    &[Case::new("variants-differ", ANY, listed_variants_differ_from_the_default), Case::new("looks-reachable", ANY, every_flag_look_is_reachable)];

/// The resolved looks of the sample of `subject` under `variant`; `None` for a type with no sample.
fn looks_under(subject: &Subject, variant: craft_theme::Variant) -> Option<crate::testing::style::StyleLooks> {
    let make = subject.probe.sample?;
    let instance = make();
    let ctx = egui::Context::default();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    with_cx(&ctx, &tokens, variant, ControlSize::Md, |cx| (subject.probe.looks)(&*instance, cx))
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

/// The flags some state of the type can set: those every component takes from its props and scope, and those its
/// input, value, popup and shape add.
fn reachable(subject: &Subject) -> Flags {
    let probe = subject.probe;
    let mut flags = Flags::READ_ONLY | Flags::SELECTED | Flags::BUSY;
    if !matches!(probe.input, Input::Display) {
        flags |= Flags::FOCUSED | Flags::FOCUS_VISIBLE;
    }
    if probe.carries_value() {
        flags |= Flags::MIXED | Flags::CHECKED | Flags::INVALID | Flags::WARN;
    }
    if probe.overlay {
        flags |= Flags::OPEN;
    }
    if matches!(probe.shape, crate::contract::ShapeKind::Items) {
        flags |= Flags::LEAD | Flags::DRAGGING | Flags::DROP_TARGET | Flags::EXPANDED;
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
