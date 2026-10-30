//! Toolbars: a row or column of icon-button groups. Sizes only (no colours): a vertical strip
//! draws in `surface.panel` with a `line.divider` hairline on its inner edge, and the selected
//! button is the icon button's toggle look.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};

/// Toolbar tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToolbarTokens {
    /// Side of a horizontal toolbar's button: `toolbar.button`.
    pub button: f32,
    /// Side of a vertical strip's button: `toolbar.button_large`.
    pub button_large: f32,
    /// Between the buttons of a group: `toolbar.gap`.
    pub gap: f32,
    /// Between the groups of a vertical strip: `toolbar.group_gap`.
    pub group_gap: f32,
    /// Inset of a vertical strip, across and along: `toolbar.pad`.
    pub pad: [f32; 2],
    /// Width of a vertical strip: `toolbar.width`.
    pub width: f32,
    /// Width of the overflow button: `toolbar.overflow`.
    pub overflow: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolbarField {}

impl Family for ToolbarTokens {
    type Field = ToolbarField;
    const FIELDS: &'static [ToolbarField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ToolbarTokens;
        "button" 0.0..=1024.0 => button;
        "button_large" 0.0..=1024.0 => button_large;
        "gap" 0.0..=64.0 => gap;
        "group_gap" 0.0..=64.0 => group_gap;
        "pad.0" 0.0..=64.0 => pad[0];
        "pad.1" 0.0..=64.0 => pad[1];
        "width" 0.0..=1024.0 => width;
        "overflow" 0.0..=1024.0 => overflow;
    };

    /// PhotoCraft's Liquify strip and the horizontal 26 point toolbar at Compact.
    fn defaults(s: &Semantic<'_>) -> ToolbarTokens {
        ToolbarTokens {
            button: s.set.toolbar_button,
            button_large: s.set.toolbar_button_large,
            gap: s.set.toolbar_gap,
            group_gap: s.set.toolbar_group_gap,
            pad: s.set.toolbar_pad,
            width: s.set.toolbar_width,
            overflow: s.set.toolbar_overflow,
        }
    }

    fn path(f: ToolbarField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ToolbarField> {
        None
    }

    fn kind(f: ToolbarField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ToolbarField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ToolbarField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
