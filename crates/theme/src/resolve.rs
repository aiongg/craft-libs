//! Resolution: a theme and an appearance to concrete tokens.

use std::collections::BTreeMap;

use super::components::{ComponentField, ComponentTokens, Semantic};
use super::derive::{self, ColorSteps, ContrastCheck};
use super::ramp::Mode;
use super::scale::Elevation;
use super::theme::{Appearance, Contrast, ContrastIntent, PinLayer, Pins, ThemeDef};
use super::tokens::{Edge, ElevationTokens, Motion, MotionTokens, ShapeTokens, TokenRef, Tokens};
use super::values;
use crate::AppId;

/// The tokens for one theme and appearance, with the contrast pairs that fail.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    /// Every semantic and component token.
    pub tokens: Tokens,
    /// Pairs below their minimum, semantic then component: in standard contrast, pairs that a
    /// pinned colour makes fail (a component pair also when the pin is a semantic colour its
    /// default reads), or pairs that no lightness move could fix.
    pub warnings: Vec<ContrastCheck>,
    /// Carried from the theme, for the picker's "Low contrast" label and the built-in check.
    pub contrast_intent: ContrastIntent,
    /// The mode whose variant was used; differs from the requested mode for single-variant
    /// themes.
    pub variant_used: Mode,
}

/// Resolves `theme` for `appearance`. Colours derive from the theme's seeds and pins for the
/// mode it has; sizes, spacing and type come from the Size preset and text size; shape comes
/// from the preset unless the theme has its own. Component tokens derive from the semantic ones,
/// then take the theme's component overrides and their own contrast pass. With `app`, that app's
/// overrides in the theme apply over the theme's own pins and components.
pub fn resolve(theme: &ThemeDef, appearance: Appearance, app: Option<&AppId>) -> Resolved {
    resolve_with_steps(theme, appearance, app).0
}

/// The intermediate values of [`resolve`], which [`explain`](crate::explain) reports.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Steps {
    pub color: ColorSteps,
    /// Each pinned component field and the layer whose pin it takes.
    pub component_pins: BTreeMap<ComponentField, PinLayer>,
    /// The component tokens before any contrast pass: defaults and overrides on the semantic
    /// colours before the semantic pass.
    pub component_before: ComponentTokens,
    /// Every component pair on `component_before`.
    pub component_pairs_before: Vec<ContrastCheck>,
    /// The same pairs on the final tokens.
    pub component_pairs: Vec<ContrastCheck>,
}

/// [`resolve`], and the steps on the way.
pub(super) fn resolve_with_steps(theme: &ThemeDef, appearance: Appearance, app: Option<&AppId>) -> (Resolved, Steps) {
    match resolve_references(theme, appearance, app) {
        Some(concrete) => resolve_concrete(&concrete, appearance, app),
        None => resolve_concrete(theme, appearance, app),
    }
}

/// `theme` with every reference that applies to `appearance` replaced by a colour pin holding the
/// referenced token's final value, or `None` when none applies. References resolve in rounds: one
/// resolution of the theme without them gives the values of the references to tokens that are not
/// references, the next those that read those, and so on. A reference to a token that another
/// layer pins by colour reads that pin, being the token's final value.
fn resolve_references(theme: &ThemeDef, appearance: Appearance, app: Option<&AppId>) -> Option<ThemeDef> {
    let mode = theme.variants.mode_for(appearance.mode);
    let high = appearance.contrast == Contrast::High;
    let has_app = app.is_some_and(|a| theme.apps.contains_key(a));
    let layers: Vec<PinLayer> = [
        Some(PinLayer::Theme),
        has_app.then_some(PinLayer::App),
        high.then_some(PinLayer::ThemeHighContrast),
        (has_app && high).then_some(PinLayer::AppHighContrast),
    ]
    .into_iter()
    .flatten()
    .collect();

    // The reference each token ends with: a later layer's pin or reference replaces an earlier one.
    let mut pending: Vec<(PinLayer, TokenRef, TokenRef)> = Vec::new();
    for &layer in &layers {
        let pins = layer_pins(theme, layer, app, mode)?;
        pending.retain(|p| !pins.colors.contains_key(&p.1) && !pins.refs.contains_key(&p.1));
        pending.extend(pins.refs.iter().map(|(&k, &t)| (layer, k, t)));
    }
    if pending.is_empty() {
        return None;
    }

    let mut work = theme.clone();
    for pins in all_pins_mut(&mut work) {
        pins.refs.clear();
    }
    while !pending.is_empty() {
        let tokens = resolve_concrete(&work, appearance, app).0.tokens;
        let (ready, rest): (Vec<_>, Vec<_>) = pending.iter().partition(|p| !pending.iter().any(|q| q.1 == p.2));
        // Without a ready reference the rest lead back to themselves, which a validated theme never does.
        if ready.is_empty() {
            break;
        }
        for &(layer, key, target) in ready {
            let value = match target {
                TokenRef::Color(t) => Some(tokens.color(t)),
                TokenRef::Component(f) => tokens.component.color(f),
            };
            if let (Some(value), Some(pins)) = (value, layer_pins_mut(&mut work, layer, app, mode)) {
                pins.colors.insert(key, value);
            }
        }
        pending = rest.into_iter().copied().collect();
    }
    Some(work)
}

fn layer_pins<'a>(theme: &'a ThemeDef, layer: PinLayer, app: Option<&AppId>, mode: Mode) -> Option<&'a Pins> {
    let app = app.and_then(|a| theme.apps.get(a));
    Some(match layer {
        PinLayer::Theme => theme.pins.get(mode),
        PinLayer::ThemeHighContrast => theme.high_contrast.get(mode),
        PinLayer::App => app?.pins.get(mode),
        PinLayer::AppHighContrast => app?.high_contrast.get(mode),
    })
}

fn layer_pins_mut<'a>(theme: &'a mut ThemeDef, layer: PinLayer, app: Option<&AppId>, mode: Mode) -> Option<&'a mut Pins> {
    let app = app.and_then(|a| theme.apps.get_mut(a));
    let modes = match layer {
        PinLayer::Theme => &mut theme.pins,
        PinLayer::ThemeHighContrast => &mut theme.high_contrast,
        PinLayer::App => &mut app?.pins,
        PinLayer::AppHighContrast => &mut app?.high_contrast,
    };
    Some(match mode {
        Mode::Light => &mut modes.light,
        Mode::Dark => &mut modes.dark,
    })
}

fn all_pins_mut(theme: &mut ThemeDef) -> impl Iterator<Item = &mut Pins> {
    let apps = theme.apps.values_mut().flat_map(|a| [&mut a.pins, &mut a.high_contrast]);
    [&mut theme.pins, &mut theme.high_contrast].into_iter().chain(apps).flat_map(|m| [&mut m.light, &mut m.dark])
}

fn resolve_concrete(theme: &ThemeDef, appearance: Appearance, app: Option<&AppId>) -> (Resolved, Steps) {
    let mode = theme.variants.mode_for(appearance.mode);
    let (color, color_steps) = derive::colors(theme, app, mode, appearance.contrast);
    let mut warnings: Vec<ContrastCheck> = color_steps.warnings().collect();
    let preset = values::preset(appearance.size);

    let seed = theme.shape;
    let radius = seed.and_then(|s| s.radius).unwrap_or(preset.radius);
    let edge = seed.and_then(|s| s.edge).map_or(preset.edge, |e| e.resolve(&color));

    let shadow = |level: usize| {
        let e = Elevation::level(level);
        match mode {
            Mode::Light => e,
            Mode::Dark => Elevation { alpha: (e.alpha * values::DARK_SHADOW_ALPHA_FACTOR).min(1.0), ..e },
        }
    };
    let (popup, dialog, floating) = values::ELEVATION_LEVELS;
    // A bevel is the relief of a raised control, so a bevelled knob lies flat.
    let knob = shadow(if matches!(edge, Edge::Bevel { .. }) { 0 } else { values::KNOB_ELEVATION });

    let elevation = ElevationTokens { popup: shadow(popup), dialog: shadow(dialog), floating: shadow(floating), knob };

    let motion = |d: super::scale::Duration| Motion { millis: if appearance.reduced_motion { 0 } else { d.millis() }, easing: values::MOTION_EASING };
    let (hover, open, move_) = values::MOTION_DURATIONS;

    let shape = ShapeTokens { radius, edge };
    let typography = preset.typography.scaled(values::text_size_factor(appearance.text_size), values::TYPE_ROUNDING);
    let mut component = ComponentTokens::defaults(&Semantic {
        color: &color,
        shape: &shape,
        space: &preset.space,
        size: &preset.size,
        elevation: &elevation,
        preset: appearance.size,
        typography: &typography,
    });
    let high = appearance.contrast == Contrast::High;
    let component_pins = derive::component_overrides(&mut component, theme, app, mode, high);
    let (component_pairs, component_warnings) = derive::component_colors(&mut component, &color, high);
    warnings.extend(component_warnings);

    let before = &color_steps.before_pass;
    let mut component_before = ComponentTokens::defaults(&Semantic {
        color: before,
        shape: &shape,
        space: &preset.space,
        size: &preset.size,
        elevation: &elevation,
        preset: appearance.size,
        typography: &typography,
    });
    derive::component_overrides(&mut component_before, theme, app, mode, high);
    let component_pairs_before = derive::measure_components(&component_before, before, high);

    let tokens = Tokens {
        color,
        shape,
        space: preset.space,
        size: preset.size,
        typography,
        elevation,
        motion: MotionTokens { hover: motion(hover), open: motion(open), r#move: motion(move_) },
        component,
    };
    let resolved = Resolved { tokens, warnings, contrast_intent: theme.contrast_intent, variant_used: mode };
    (resolved, Steps { color: color_steps, component_pins, component_before, component_pairs_before, component_pairs })
}
