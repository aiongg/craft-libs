//! [`PresetStyle`]: the preset browser's sizes and colours from the `preset` tokens, with the
//! selected ring from `swatch`, and the insertion line from `list`.

use craft_theme::{Rgba, SwatchTokens};
use egui::{Color32, CornerRadius, Stroke};

use crate::theme::{ToEgui, UiTokens, corner_radius};

/// What the browser paints with, from [`PresetStyle::from_tokens`]. Painting code reads sizes and
/// colours from here and nowhere else.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PresetStyle {
    /// Height of a group or folder header: `preset.header_height`.
    pub header_height: f32,
    /// Between grid cells, unless the grid is flush: `preset.grid_gap`.
    pub grid_gap: f32,
    /// Between the grid and the panel edge, unless flush: `preset.grid_inset`.
    pub grid_inset: f32,
    /// Corners of a grid cell, unless flush: `preset.cell_radius`.
    pub cell_radius: CornerRadius,
    /// Between a thumbnail and its caption: `preset.caption_gap`.
    pub caption_gap: f32,
    /// Width of the thumbnail size slider: `preset.slider_width`.
    pub slider_width: f32,
    /// Least thumbnail side in a list row: `preset.thumb_list_min`.
    pub thumb_list_min: f32,
    /// Greatest thumbnail side in a list row: `preset.thumb_list_max`.
    pub thumb_list_max: f32,
    /// Side of the thumbnail in the drag ghost: `preset.ghost_thumb`.
    pub ghost_thumb: f32,
    /// Between toolbar and footer buttons: `preset.toolbar_gap`.
    pub toolbar_gap: f32,
    /// The 1 pt frame inside each cell that is not flush: `preset.border`.
    pub border: Stroke,
    /// The ring outside a hovered cell: `preset.ring.hover`, `preset.ring_hover_width` wide.
    pub ring_hover: Stroke,
    /// The ring outside the selected cell: `swatch.selected`, `swatch.selected_width` wide.
    pub ring_selected: Stroke,
    /// The line outside the selected ring that keeps it visible over any neighbour:
    /// `preset.ring.halo`.
    pub ring_halo: Stroke,
    /// A caption under a thumbnail: `preset.caption`.
    pub caption: Color32,
    /// The caption of the selected cell: `preset.caption.selected`.
    pub caption_selected: Color32,
    /// The item count of a group: `preset.count`.
    pub count: Color32,
    /// The label of a header: `preset.header.text`.
    pub header_text: Color32,
    /// The folder icon of a header: `preset.header.icon`.
    pub header_icon: Color32,
    /// The insertion line and the outline of a drop into a group: `list.drop`.
    pub insert: Color32,
    /// The outline under the insertion line: `list.drop_halo`.
    pub insert_halo: Color32,
    /// Width of the insertion line: `list.drop_width`.
    pub insert_width: f32,
    /// The checkerboard behind a translucent swatch: `swatch.checker.a` holds the corners,
    /// `swatch.checker.b` the other cells.
    pub checker: [Color32; 2],
    /// Side of a checkerboard cell: `swatch.checker_cell`.
    pub checker_cell: f32,
    swatch: SwatchTokens,
}

impl PresetStyle {
    /// The style for the installed theme.
    pub fn from_tokens(tokens: &UiTokens) -> Self {
        let p = &tokens.component.preset;
        let (s, l) = (&tokens.component.swatch, &tokens.component.list);
        Self {
            header_height: p.header_height,
            grid_gap: p.grid_gap,
            grid_inset: p.grid_inset,
            cell_radius: corner_radius(p.cell_radius),
            caption_gap: p.caption_gap,
            slider_width: p.slider_width,
            thumb_list_min: p.thumb_list_min,
            thumb_list_max: p.thumb_list_max,
            ghost_thumb: p.ghost_thumb,
            toolbar_gap: p.toolbar_gap,
            border: Stroke::new(1.0, p.border.to_egui()),
            ring_hover: Stroke::new(p.ring_hover_width, p.ring_hover.to_egui()),
            ring_selected: Stroke::new(s.selected_width, s.selected.to_egui()),
            ring_halo: Stroke::new(p.ring_halo_width, p.ring_halo.to_egui()),
            caption: p.caption.to_egui(),
            caption_selected: p.caption_selected.to_egui(),
            count: p.count.to_egui(),
            header_text: p.header_text.to_egui(),
            header_icon: p.header_icon.to_egui(),
            insert: l.drop.to_egui(),
            insert_halo: l.drop_halo.to_egui(),
            insert_width: l.drop_width,
            checker: [s.checker_a.to_egui(), s.checker_b.to_egui()],
            checker_cell: s.checker_cell,
            swatch: *s,
        }
    }

    /// A ring of black or white at partial alpha that stands out from `color` over the
    /// checkerboard: the inner edge of a swatch cell that is not flush.
    pub fn inner_edge(&self, color: Rgba) -> Color32 {
        self.swatch.inner_edge(color).to_egui()
    }
}
