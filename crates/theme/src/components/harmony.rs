//! Harmony panels: the Color Guide's base colour button, harmony strip and variation grid. The
//! sizes scale with the control height, and no theme overrides them. The harmony wheel's sizes are
//! `color_wheel.disc_*`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Harmony panel tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HarmonyTokens {
    /// Side of the base colour button: `harmony.base`.
    pub base: f32,
    /// Height of the harmony strip: `harmony.strip`.
    pub strip: f32,
    /// Height of a variation row, with the gap under it: `harmony.row`.
    pub row: f32,
    /// Height of a variation cell: `harmony.cell`.
    pub cell: f32,
}

/// Harmony panels have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HarmonyField {}

impl Family for HarmonyTokens {
    type Field = HarmonyField;
    const FIELDS: &'static [HarmonyField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { HarmonyTokens;
        "base" 0.0..=1024.0 => base;
        "strip" 0.0..=1024.0 => strip;
        "row" 0.0..=1024.0 => row;
        "cell" 0.0..=1024.0 => cell;
    };

    /// The Compact sizes (VectorCraft's), scaled by the control height of the size and rounded to
    /// whole points.
    fn defaults(s: &Semantic<'_>) -> HarmonyTokens {
        let k = s.size.control.md / values::COLOR_PICKER_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        HarmonyTokens {
            base: scale(values::HARMONY_BASE),
            strip: scale(values::HARMONY_STRIP),
            row: scale(values::HARMONY_ROW),
            cell: scale(values::HARMONY_CELL),
        }
    }

    fn path(f: HarmonyField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<HarmonyField> {
        None
    }

    fn kind(f: HarmonyField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: HarmonyField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: HarmonyField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
