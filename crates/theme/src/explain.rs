//! Why each colour has its value: the steps of [`resolve`] for one theme and appearance, with
//! every contrast pair the rules measure. The gallery's token browser shows them.

use super::color::Rgba;
use super::components::{ComponentField, ComponentTokens, Semantic};
use super::derive::{Measured, Ramps};
use super::resolve::{self, Resolved};
use super::theme::{Appearance, PinLayer, Size, ThemeDef};
use super::tokens::{ColorToken, ColorTokens, ShapeTokens, TokenRef};
use super::values::{self, ColorSource};
use crate::AppId;

/// The steps of [`resolve`] for one theme and appearance.
#[derive(Clone, Debug, PartialEq)]
pub struct Explained {
    /// Equal to `resolve(theme, appearance, app)`.
    pub resolved: Resolved,
    /// The ramps of the mode used ([`Resolved::variant_used`]).
    pub ramps: Ramps,
    /// One per [`ColorToken::ALL`], in that order.
    pub colors: Vec<ColorOrigin>,
    /// One per component colour field, in [`ComponentField::ALL`] order.
    pub components: Vec<FieldOrigin>,
    /// Every pair the contrast rules measure, failing or not: semantic pairs in rule order, then
    /// component pairs.
    pub pairs: Vec<PairReport>,
}

/// How a colour token got its value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorOrigin {
    /// The token.
    pub token: ColorToken,
    /// Its derivation rule in the mode used.
    pub source: ColorSource,
    /// The value `source` gives, before pins and the contrast pass. Tokens defined by other
    /// tokens take those tokens' derived values.
    pub derived: Rgba,
    /// The layer whose pin the token takes, if it is pinned.
    pub pin: Option<PinLayer>,
    /// The value before the contrast pass, when the pass changed it. Tokens defined by other
    /// tokens count as changed when they follow a token the pass moved.
    pub contrast_move: Option<Rgba>,
    /// The final value.
    pub value: Rgba,
}

/// How a component colour field got its value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldOrigin {
    /// The field.
    pub field: ComponentField,
    /// The semantic token the default takes, when the default is one token unchanged; `None`
    /// for a computed default (a constant or a mix).
    pub reads: Option<ColorToken>,
    /// The layer whose pin overrides the field, if any.
    pub pin: Option<PinLayer>,
    /// The value before the component contrast pass, when the pass changed it.
    pub contrast_move: Option<Rgba>,
    /// The final value.
    pub value: Rgba,
}

/// One measured contrast pair. Translucent backgrounds are measured over `surface.panel`;
/// pairs on `overlay.halo` report the lower of the ratios over black and over white.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PairReport {
    /// The foreground token or field.
    pub fg: TokenRef,
    /// The background token or field.
    pub bg: TokenRef,
    /// The ratio on the final colours.
    pub ratio: f64,
    /// The required ratio in this appearance's contrast.
    pub minimum: f64,
    /// Whether a contrast pass moved the foreground or the background.
    pub fixed_by_pass: bool,
}

impl PairReport {
    /// Whether the ratio reaches the minimum.
    pub fn passes(&self) -> bool {
        self.ratio >= self.minimum
    }

    /// Whether `token` is this pair's foreground or background.
    pub fn involves(&self, token: TokenRef) -> bool {
        self.fg == token || self.bg == token
    }
}

impl Explained {
    /// The origin of `token`.
    pub fn color(&self, token: ColorToken) -> Option<&ColorOrigin> {
        self.colors.iter().find(|o| o.token == token)
    }

    /// The origin of a component colour field; `None` for fields that are not colours.
    pub fn field(&self, field: ComponentField) -> Option<&FieldOrigin> {
        self.components.iter().find(|o| o.field == field)
    }

    /// Every pair `token` takes part in, as foreground or background.
    pub fn pairs_of(&self, token: TokenRef) -> impl Iterator<Item = &PairReport> + '_ {
        self.pairs.iter().filter(move |p| p.involves(token))
    }
}

/// Resolves `theme` for `appearance` as [`resolve`] does, and reports how every colour got its
/// value and every contrast pair the rules measure.
pub fn explain(theme: &ThemeDef, appearance: Appearance, app: Option<&AppId>) -> Explained {
    let (resolved, steps) = resolve::resolve_with_steps(theme, appearance, app);
    let mode = resolved.variant_used;
    let final_colors = &resolved.tokens.color;
    let final_components = &resolved.tokens.component;
    let changed = |before: Rgba, after: Rgba| (before != after).then_some(before);

    let colors: Vec<ColorOrigin> = ColorToken::ALL
        .iter()
        .map(|&token| {
            let value = final_colors.get(token);
            ColorOrigin {
                token,
                source: values::source(token, mode),
                derived: steps.color.derived.get(token),
                pin: steps.color.pins.get(&token).copied(),
                contrast_move: changed(steps.color.before_pass.get(token), value),
                value,
            }
        })
        .collect();

    let reads = default_reads();
    let components: Vec<FieldOrigin> = ComponentField::ALL
        .iter()
        .filter_map(|&field| {
            let value = final_components.color(field)?;
            Some(FieldOrigin {
                field,
                reads: reads.iter().find(|(f, _)| *f == field).and_then(|&(_, t)| t),
                pin: steps.component_pins.get(&field).copied(),
                contrast_move: changed(steps.component_before_pass.color(field)?, value),
                value,
            })
        })
        .collect();

    let moved = |t: TokenRef| match t {
        TokenRef::Color(t) => colors.iter().any(|o| o.token == t && o.contrast_move.is_some()),
        TokenRef::Component(f) => components.iter().any(|o| o.field == f && o.contrast_move.is_some()),
    };
    let report = |m: &Measured| PairReport { fg: m.fg, bg: m.bg, ratio: m.ratio, minimum: m.minimum, fixed_by_pass: moved(m.fg) || moved(m.bg) };
    let pairs = steps.color.pairs.iter().chain(&steps.component_pairs).map(report).collect();

    Explained { resolved, ramps: steps.color.ramps, colors, components, pairs }
}

/// Each component colour field with the semantic token its default takes, if it is one token.
///
/// Component defaults are code (each family's `defaults(&Semantic)`), not a table, so the token
/// can't be looked up. Instead the defaults run once on colours that all differ, and each field's
/// value is matched back to the token that had it. A field whose value matches no token (a
/// constant, a mix) reads none. Colours are mode-independent here because the defaults only
/// copy or combine semantic colours; the Size preset only feeds non-colour fields.
fn default_reads() -> Vec<(ComponentField, Option<ColorToken>)> {
    let mut probe = ColorTokens::splat(Rgba::TRANSPARENT);
    for (i, &t) in ColorToken::ALL.iter().enumerate() {
        // Opaque, so no probe colour equals a translucent constant such as `Rgba::TRANSPARENT`.
        let [hi, lo] = u16::try_from(i).unwrap_or(u16::MAX).to_be_bytes();
        probe.set(t, Rgba([hi, lo, 0x5b, 0xff]));
    }
    let preset = values::preset(Size::Compact);
    let shape = ShapeTokens { radius: preset.radius, edge: preset.edge };
    let defaults = ComponentTokens::defaults(&Semantic { color: &probe, shape: &shape, space: &preset.space, size: &preset.size });
    ComponentField::ALL
        .iter()
        .filter_map(|&f| {
            let v = defaults.color(f)?;
            Some((f, ColorToken::ALL.iter().copied().find(|&t| probe.get(t) == v)))
        })
        .collect()
}
