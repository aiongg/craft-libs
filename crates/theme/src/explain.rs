//! Why each colour has its value: the steps of [`resolve`] for one theme and appearance, with
//! every contrast pair the rules measure. The gallery's token browser shows them.

use super::color::Rgba;
use super::components::{ComponentField, ComponentTokens, Semantic};
use super::derive::{ContrastCheck, Ramps};
use super::resolve::{self, Resolved};
use super::theme::{Appearance, PinLayer, Size, ThemeDef};
use super::tokens::{ColorToken, ColorTokens, ShapeTokens, TokenRef};
use super::values::{self, ColorSource};
use crate::AppId;

/// The steps of [`resolve`] for one theme and appearance.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct ColorOrigin {
    /// The token.
    pub token: ColorToken,
    /// Its derivation rule in the mode used.
    pub source: ColorSource,
    /// The value `source` gives without this token's pin, before the contrast pass. A token
    /// defined by other tokens takes their pinned values when it is not pinned itself, so the
    /// value explains what it shows, and their unpinned values when it is.
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
#[non_exhaustive]
pub struct FieldOrigin {
    /// The field.
    pub field: ComponentField,
    /// The semantic token the default takes, when the default is one token unchanged; `None`
    /// for a computed default (a constant or a mix).
    pub reads: Option<ColorToken>,
    /// The layer whose pin overrides the field, if any.
    pub pin: Option<PinLayer>,
    /// The value before any contrast pass, when a pass changed it: the component pass, or the
    /// semantic pass through the token the default reads.
    pub contrast_move: Option<Rgba>,
    /// The final value.
    pub value: Rgba,
}

/// One measured contrast pair. Translucent backgrounds are measured over `surface.panel`;
/// pairs on `overlay.halo` report the lower of the ratios over black and over white.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PairReport {
    /// The pair on the final colours.
    pub pair: ContrastCheck,
    /// The ratio before any contrast pass.
    pub ratio_before: f64,
    /// Whether a contrast pass, semantic or component, moved the foreground or the background.
    pub moved_by_pass: bool,
    /// Whether the pair failed before the passes and passes after them.
    pub fixed_by_pass: bool,
}

impl PairReport {
    /// Whether `token` is this pair's foreground or background.
    pub fn involves(&self, token: TokenRef) -> bool {
        self.pair.fg == token || self.pair.bg == token
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
                contrast_move: changed(steps.component_before.color(field)?, value),
                value,
            })
        })
        .collect();

    let moved = |t: TokenRef| match t {
        TokenRef::Color(t) => colors.iter().any(|o| o.token == t && o.contrast_move.is_some()),
        TokenRef::Component(f) => components.iter().any(|o| o.field == f && o.contrast_move.is_some()),
    };
    let before: Vec<&ContrastCheck> = steps.color.pairs_before.iter().chain(&steps.component_pairs_before).collect();
    let report = |pair: &ContrastCheck| {
        let ratio_before = before.iter().find(|b| b.fg == pair.fg && b.bg == pair.bg).map_or(pair.ratio, |b| b.ratio);
        PairReport {
            pair: *pair,
            ratio_before,
            moved_by_pass: moved(pair.fg) || moved(pair.bg),
            fixed_by_pass: ratio_before < pair.minimum && pair.passes(),
        }
    };
    let pairs = steps.color.pairs.iter().chain(&steps.component_pairs).map(report).collect();

    Explained { resolved, ramps: steps.color.ramps, colors, components, pairs }
}

/// Two ways to give each token its own opaque colour: channel = (index × step + offset) mod 256
/// for red, green and blue. The steps are odd, so each channel alone tells up to 256 tokens
/// apart; the spreads differ, so a mix that lands on another token's colour under one probe
/// doesn't under the other.
pub(super) const PROBES: [[usize; 6]; 2] = [[53, 97, 151, 11, 31, 67], [89, 163, 211, 17, 113, 7]];

/// Each component colour field with the semantic token its default takes, if it is one token:
/// the matches both [`PROBES`] agree on.
pub(super) fn default_reads() -> Vec<(ComponentField, Option<ColorToken>)> {
    agree(&probe_reads(PROBES[0]), &probe_reads(PROBES[1]))
}

/// The matches of `a` that `b` has too; any other field reads none.
pub(super) fn agree(
    a: &[(ComponentField, Option<ColorToken>)],
    b: &[(ComponentField, Option<ColorToken>)],
) -> Vec<(ComponentField, Option<ColorToken>)> {
    a.iter().map(|&(f, t)| (f, t.filter(|&t| b.contains(&(f, Some(t)))))).collect()
}

/// Component defaults are code (each family's `defaults(&Semantic)`), not a table, so the token
/// a field reads can't be looked up. Instead the defaults run on colours that all differ, and
/// each colour field's value is matched back to the token that had it. A field whose value
/// matches no token (a constant, a mix) reads none. One run serves every mode and size: the
/// defaults copy or combine semantic colours, and the Size preset feeds only non-colour fields.
pub(super) fn probe_reads(spread: [usize; 6]) -> Vec<(ComponentField, Option<ColorToken>)> {
    let [rm, gm, bm, ro, go, bo] = spread;
    let channel = |i: usize, step: usize, offset: usize| (i.wrapping_mul(step).wrapping_add(offset) % 256) as u8;
    let mut probe = ColorTokens::splat(Rgba::TRANSPARENT);
    for (i, &t) in ColorToken::ALL.iter().enumerate() {
        // Opaque, so no probe colour equals a translucent constant such as `Rgba::TRANSPARENT`.
        probe.set(t, Rgba([channel(i, rm, ro), channel(i, gm, go), channel(i, bm, bo), 0xff]));
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
