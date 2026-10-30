//! Preset browsers: a library of named presets shown as a grid of thumbnails or a list of rows.
//! The family holds the sizes of the grid and its headers and the colours the browser draws
//! itself. Rows, selection fills, the drop marker and the disclosure triangle are the list
//! family's; the selected ring is `swatch.selected`.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// Preset browser tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PresetTokens {
    /// Height of a group or folder header: `size.preset.header_height`.
    pub header_height: f32,
    /// Between grid cells, unless the grid is flush: `size.preset.grid_gap`.
    pub grid_gap: f32,
    /// Between the grid and the panel edge, unless the grid is flush: `size.preset.grid_inset`.
    pub grid_inset: f32,
    /// Corner radius of a grid cell, unless the grid is flush: `size.preset.cell_radius`.
    pub cell_radius: f32,
    /// Between a thumbnail and its caption: `size.preset.caption_gap`.
    pub caption_gap: f32,
    /// Width of the thumbnail size slider: `size.preset.slider_width`.
    pub slider_width: f32,
    /// Least thumbnail side in a list row: [`values::PRESET_THUMB_LIST_MIN`].
    pub thumb_list_min: f32,
    /// Greatest thumbnail side in a list row: [`values::PRESET_THUMB_LIST_MAX`].
    pub thumb_list_max: f32,
    /// Side of the thumbnail in the drag ghost: [`values::PRESET_GHOST_THUMB`].
    pub ghost_thumb: f32,
    /// Between toolbar and footer buttons: `gap.inline`.
    pub toolbar_gap: f32,
    /// Width of the ring on a hovered cell, outside it: [`values::PRESET_RING_HOVER_WIDTH`].
    pub ring_hover_width: f32,
    /// Width of the contrast line outside the selected ring: [`values::PRESET_RING_HALO_WIDTH`].
    pub ring_halo_width: f32,
    /// The 1 point frame inside each cell that is not flush: `preset.border`, `line.border`. A
    /// design target with no contrast minimum.
    pub border: Rgba,
    /// The ring outside a hovered cell: `preset.ring.hover`, `text.muted`. A design target.
    pub ring_hover: Rgba,
    /// The line outside the selected ring, which keeps it visible over any neighbouring
    /// thumbnail: `overlay.halo`. The ring itself is `swatch.selected`.
    pub ring_halo: Rgba,
    /// A caption under a thumbnail: `preset.caption`, `text.muted`.
    pub caption: Rgba,
    /// The caption of the selected cell: `text.normal`.
    pub caption_selected: Rgba,
    /// The item count of a group: `preset.count`, `text.faint`.
    pub count: Rgba,
    /// The label of a header: `text.normal`.
    pub header_text: Rgba,
    /// The folder icon of a header: `text.muted`.
    pub header_icon: Rgba,
}

/// An overridable preset browser field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PresetField {
    /// `preset.border`
    Border,
    /// `preset.ring.hover`
    RingHover,
    /// `preset.caption`
    Caption,
    /// `preset.count`
    Count,
}

impl PresetField {
    const ALL: [PresetField; 4] = [PresetField::Border, PresetField::RingHover, PresetField::Caption, PresetField::Count];
}

impl Family for PresetTokens {
    type Field = PresetField;
    const FIELDS: &'static [PresetField] = &PresetField::ALL;

    fn defaults(s: &Semantic<'_>) -> PresetTokens {
        let (c, p) = (s.color, s.size.preset);
        PresetTokens {
            header_height: p.header_height,
            grid_gap: p.grid_gap,
            grid_inset: p.grid_inset,
            cell_radius: p.cell_radius,
            caption_gap: p.caption_gap,
            slider_width: p.slider_width,
            thumb_list_min: values::PRESET_THUMB_LIST_MIN,
            thumb_list_max: values::PRESET_THUMB_LIST_MAX,
            ghost_thumb: values::PRESET_GHOST_THUMB,
            toolbar_gap: s.space.gap.inline,
            ring_hover_width: values::PRESET_RING_HOVER_WIDTH,
            ring_halo_width: values::PRESET_RING_HALO_WIDTH,
            border: c.line.border,
            ring_hover: c.text.muted,
            ring_halo: c.overlay.halo,
            caption: c.text.muted,
            caption_selected: c.text.normal,
            count: c.text.faint,
            header_text: c.text.normal,
            header_icon: c.text.muted,
        }
    }

    fn path(f: PresetField) -> String {
        match f {
            PresetField::Border => "border",
            PresetField::RingHover => "ring.hover",
            PresetField::Caption => "caption",
            PresetField::Count => "count",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<PresetField> {
        Self::FIELDS.iter().copied().find(|f| Self::path(*f) == path)
    }

    fn kind(_: PresetField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: PresetField) -> OverrideValue {
        OverrideValue::Color(match f {
            PresetField::Border => self.border,
            PresetField::RingHover => self.ring_hover,
            PresetField::Caption => self.caption,
            PresetField::Count => self.count,
        })
    }

    fn set(&mut self, f: PresetField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            match f {
                PresetField::Border => self.border = c,
                PresetField::RingHover => self.ring_hover = c,
                PresetField::Caption => self.caption = c,
                PresetField::Count => self.count = c,
            }
        }
    }

    /// The caption and the count are text: 4.5:1 (7:1 with high contrast) against the panel. The
    /// selected ring is `swatch.selected`, whose pair against the panel is the swatch family's;
    /// the border and the hover ring are design targets.
    fn pairs() -> Vec<ContrastPair> {
        [PresetField::Caption, PresetField::Count]
            .into_iter()
            .map(|f| ContrastPair {
                fg: TokenRef::Component(ComponentField::Preset(f)),
                bg: TokenRef::Color(ColorToken::SurfacePanel),
                minimum: values::TEXT_MIN,
                alternate: None,
            })
            .collect()
    }
}
