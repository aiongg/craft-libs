//! The Slice tool's overlay: outlines, dashes, badges and the click slop. The same at every Size (they sit on the document). Its handles are the `handle` family's.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The Slice tool's overlay: outlines, dashes, badges and the click slop. The same at every Size (they sit on the document). Its handles are the `handle` family's.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliceTokens {
    /// A slice's outline width: `slice.outline`.
    pub outline: f32,
    /// The selected slice's outline width: `slice.selected_outline`.
    pub selected_outline: f32,
    /// An auto slice's dash: `slice.auto_dash`.
    pub auto_dash: f32,
    /// An auto slice's gap: `slice.auto_gap`.
    pub auto_gap: f32,
    /// An auto slice's alpha over the slice colour (a ratio, not a size): `slice.auto_alpha`.
    pub auto_alpha: f32,
    /// A new slice preview's dash: `slice.drag_dash`.
    pub drag_dash: f32,
    /// A new slice preview's gap: `slice.drag_gap`.
    pub drag_gap: f32,
    /// The badge's offset from the slice's corner: `slice.badge_offset`.
    pub badge_offset: f32,
    /// The badge's padding around the number, horizontal then vertical: `slice.badge_pad`.
    pub badge_pad: [f32; 2],
    /// The badge's corner radius: `slice.badge_radius`.
    pub badge_radius: f32,
    /// The smallest slice on screen that shows a badge, width then height: `slice.badge_min`.
    pub badge_min: [f32; 2],
    /// A drag no wider and no taller than this many document pixels is a click: `slice.click_slop`.
    pub click_slop: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SliceField {}

impl Family for SliceTokens {
    type Field = SliceField;
    const FIELDS: &'static [SliceField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SliceTokens;
        "outline" 0.0..=16.0 => outline;
        "selected_outline" 0.0..=16.0 => selected_outline;
        "auto_dash" 0.0..=64.0 => auto_dash;
        "auto_gap" 0.0..=64.0 => auto_gap;
        "auto_alpha" 0.0..=1.0 => auto_alpha;
        "drag_dash" 0.0..=64.0 => drag_dash;
        "drag_gap" 0.0..=64.0 => drag_gap;
        "badge_offset" 0.0..=64.0 => badge_offset;
        "badge_pad.0" 0.0..=64.0 => badge_pad[0];
        "badge_pad.1" 0.0..=64.0 => badge_pad[1];
        "badge_radius" 0.0..=32.0 => badge_radius;
        "badge_min.0" 0.0..=256.0 => badge_min[0];
        "badge_min.1" 0.0..=256.0 => badge_min[1];
        "click_slop" 0.0..=64.0 => click_slop;
    };

    fn defaults(_: &Semantic<'_>) -> SliceTokens {
        SliceTokens {
            outline: values::SLICE_OUTLINE,
            selected_outline: values::SLICE_SELECTED_OUTLINE,
            auto_dash: values::SLICE_AUTO_DASH,
            auto_gap: values::SLICE_AUTO_GAP,
            auto_alpha: values::SLICE_AUTO_ALPHA,
            drag_dash: values::SLICE_DRAG_DASH,
            drag_gap: values::SLICE_DRAG_GAP,
            badge_offset: values::SLICE_BADGE_OFFSET,
            badge_pad: values::SLICE_BADGE_PAD,
            badge_radius: values::SLICE_BADGE_RADIUS,
            badge_min: values::SLICE_BADGE_MIN,
            click_slop: values::SLICE_CLICK_SLOP,
        }
    }

    fn path(f: SliceField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SliceField> {
        None
    }

    fn kind(f: SliceField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SliceField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SliceField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
