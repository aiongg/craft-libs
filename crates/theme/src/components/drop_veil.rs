//! Drop veils: the scrim, frame and card that say what a release would do. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// DropVeil tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DropVeilTokens {
    /// Distance of the frame from the window's edge: `drop_veil.inset`.
    pub inset: f32,
    /// Stroke width of the frame: `drop_veil.frame_width`.
    pub frame_width: f32,
    /// Corner radius of the frame: `drop_veil.frame_radius`.
    pub frame_radius: f32,
    /// Corner radius of the card: `drop_veil.card_radius`.
    pub card_radius: f32,
    /// Between the card's lines: `drop_veil.card_gap`.
    pub card_gap: f32,
    /// Padding at the card's sides: `drop_veil.card_pad_x`.
    pub card_pad_x: f32,
    /// Padding at the card's top and bottom: `drop_veil.card_pad_y`.
    pub card_pad_y: f32,
    /// The lines wrap at the window's width less this: `drop_veil.wrap_margin`.
    pub wrap_margin: f32,
    /// The least width the lines wrap at: `drop_veil.min_wrap`.
    pub min_wrap: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DropVeilField {}

impl Family for DropVeilTokens {
    type Field = DropVeilField;
    const FIELDS: &'static [DropVeilField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { DropVeilTokens;
        "inset" 0.0..=256.0 => inset;
        "frame_width" 0.0..=16.0 => frame_width;
        "frame_radius" 0.0..=64.0 => frame_radius;
        "card_radius" 0.0..=64.0 => card_radius;
        "card_gap" 0.0..=64.0 => card_gap;
        "card_pad_x" 0.0..=128.0 => card_pad_x;
        "card_pad_y" 0.0..=128.0 => card_pad_y;
        "wrap_margin" 0.0..=1024.0 => wrap_margin;
        "min_wrap" 0.0..=1024.0 => min_wrap;
    };

    fn defaults(_: &Semantic<'_>) -> DropVeilTokens {
        DropVeilTokens {
            inset: values::DROP_VEIL_INSET,
            frame_width: values::DROP_VEIL_FRAME_WIDTH,
            frame_radius: values::DROP_VEIL_FRAME_RADIUS,
            card_radius: values::DROP_VEIL_CARD_RADIUS,
            card_gap: values::DROP_VEIL_CARD_GAP,
            card_pad_x: values::DROP_VEIL_CARD_PAD_X,
            card_pad_y: values::DROP_VEIL_CARD_PAD_Y,
            wrap_margin: values::DROP_VEIL_WRAP_MARGIN,
            min_wrap: values::DROP_VEIL_MIN_WRAP,
        }
    }

    fn path(f: DropVeilField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<DropVeilField> {
        None
    }

    fn kind(f: DropVeilField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: DropVeilField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: DropVeilField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
