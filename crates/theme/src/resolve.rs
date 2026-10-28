//! Resolution: a theme and an appearance to concrete tokens.

use std::collections::BTreeMap;

use super::components::{ComponentField, ComponentTokens, Semantic};
use super::derive::{self, ColorSteps, ContrastWarning, Measured};
use super::ramp::Mode;
use super::scale::Elevation;
use super::theme::{Appearance, Contrast, ContrastIntent, PinLayer, ThemeDef};
use super::tokens::{ElevationTokens, Motion, MotionTokens, ShapeTokens, Tokens};
use super::values;
use crate::AppId;

/// The tokens for one theme and appearance, with the contrast pairs that fail.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    /// Every semantic and component token.
    pub tokens: Tokens,
    /// Pairs below their minimum, semantic then component: pinned colours in standard contrast
    /// (component pairs only where a component colour is overridden), or pairs that no lightness
    /// move could fix.
    pub warnings: Vec<ContrastWarning>,
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
    /// Component tokens after the overrides, before their contrast pass.
    pub component_before_pass: ComponentTokens,
    /// Every component pair on the final tokens, failing or not.
    pub component_pairs: Vec<Measured>,
}

/// [`resolve`], and the steps on the way.
pub(super) fn resolve_with_steps(theme: &ThemeDef, appearance: Appearance, app: Option<&AppId>) -> (Resolved, Steps) {
    let mode = theme.variants.mode_for(appearance.mode);
    let (color, color_steps) = derive::colors(theme, app, mode, appearance.contrast);
    let mut warnings: Vec<ContrastWarning> = color_steps.warnings().collect();
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

    let motion = |d: super::scale::Duration| Motion { millis: if appearance.reduced_motion { 0 } else { d.millis() }, easing: values::MOTION_EASING };
    let (hover, open, move_) = values::MOTION_DURATIONS;

    let shape = ShapeTokens { radius, edge };
    let mut component = ComponentTokens::defaults(&Semantic { color: &color, shape: &shape, space: &preset.space, size: &preset.size });
    let defaults = component.clone();
    let high = appearance.contrast == Contrast::High;
    let component_pins = derive::component_overrides(&mut component, theme, app, mode, high, &color);
    let component_before_pass = component.clone();
    let (component_pairs, component_warnings) = derive::component_colors(&mut component, &defaults, &color, high);
    warnings.extend(component_warnings);

    let tokens = Tokens {
        color,
        shape,
        space: preset.space,
        size: preset.size,
        typography: preset.typography.scaled(values::text_size_factor(appearance.text_size), values::TYPE_ROUNDING),
        elevation: ElevationTokens { popup: shadow(popup), dialog: shadow(dialog), floating: shadow(floating) },
        motion: MotionTokens { hover: motion(hover), open: motion(open), r#move: motion(move_) },
        component,
    };
    let resolved = Resolved { tokens, warnings, contrast_intent: theme.contrast_intent, variant_used: mode };
    (resolved, Steps { color: color_steps, component_pins, component_before_pass, component_pairs })
}
