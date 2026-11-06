//! [`PresetStyle`]: the preset browser's sizes and colours from the semantic tokens.

use craft_theme::Rgba;
use egui::{Color32, CornerRadius, Stroke, Vec2};

use crate::controls::{ControlSize, Swatch, SwatchStyle};
use crate::theme::{ToEgui, UiTokens, corner_radius};

/// Width of the thumbnail size slider as a multiple of `size.control.md`.
const SLIDER_WIDTH_SCALE: f32 = 3.75;
/// Least thumbnail side in a list row.
const THUMB_LIST_MIN: f32 = 16.0;
/// Greatest thumbnail side in a list row.
const THUMB_LIST_MAX: f32 = 64.0;
/// Width of the named preset row's Name field as a multiple of `size.control.md`.
const NAME_WIDTH_SCALE: f32 = 6.0;
/// Greatest height of the picker's popup.
const POPUP_MAX_HEIGHT: f32 = 320.0;

/// What the browser paints with, from [`PresetStyle::from_tokens`]. Painting code reads sizes and
/// colours from here and nowhere else.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PresetStyle {
    /// Height of a group or folder header.
    pub header_height: f32,
    /// Between grid cells, unless the grid is flush.
    pub grid_gap: f32,
    /// Between the grid and the panel edge, unless flush.
    pub grid_inset: f32,
    /// Corners of a grid cell, unless flush.
    pub cell_radius: CornerRadius,
    /// Between a thumbnail and its caption.
    pub caption_gap: f32,
    /// Width of the thumbnail size slider.
    pub slider_width: f32,
    /// Least thumbnail side in a list row.
    pub thumb_list_min: f32,
    /// Greatest thumbnail side in a list row.
    pub thumb_list_max: f32,
    /// Side of the thumbnail in the drag ghost.
    pub ghost_thumb: f32,
    /// Between toolbar and footer buttons.
    pub toolbar_gap: f32,
    /// The 1 pt frame inside each cell that is not flush.
    pub border: Stroke,
    /// The ring outside a hovered cell.
    pub ring_hover: Stroke,
    /// The ring outside the selected cell.
    pub ring_selected: Stroke,
    /// The line outside the selected ring that keeps it visible over any neighbour.
    pub ring_halo: Stroke,
    /// A caption under a thumbnail.
    pub caption: Color32,
    /// The caption of the selected cell.
    pub caption_selected: Color32,
    /// The item count of a group.
    pub count: Color32,
    /// The label of a header.
    pub header_text: Color32,
    /// The folder icon of a header.
    pub header_icon: Color32,
    /// The insertion line and the outline of a drop into a group.
    pub insert: Color32,
    /// The outline under the insertion line.
    pub insert_halo: Color32,
    /// Width of the insertion line.
    pub insert_width: f32,
    /// The checkerboard behind a translucent swatch: light for the corners, dark for the other cells.
    pub checker: [Color32; 2],
    /// Side of a checkerboard cell.
    pub checker_cell: f32,
    /// A [`PickerThumb::Wide`](super::picker::PickerThumb::Wide) field's thumbnail (custom
    /// shapes).
    pub picker_wide: Vec2,
    /// A [`PickerThumb::Square`](super::picker::PickerThumb::Square) field's thumbnail
    /// (patterns, contours).
    pub picker_square: f32,
    /// The side of a cell in the picker's popup.
    pub picker_cell: f32,
    /// The width of the named preset row's Name field.
    pub name_width: f32,
    /// The greatest height of the picker's popup.
    pub popup_max_height: f32,
    /// The gap between the browser's list and its own footer.
    pub footer_room: f32,
    swatch: SwatchStyle,
}

impl PresetStyle {
    /// The style for the installed theme.
    pub fn from_tokens(tokens: &UiTokens) -> Self {
        let (c, space, size, stroke) = (&tokens.color, &tokens.space, &tokens.size, &tokens.stroke);
        Self {
            header_height: size.panel_header,
            grid_gap: space.gap.inline,
            grid_inset: space.pad.panel,
            cell_radius: corner_radius(tokens.shape.radius.control),
            caption_gap: space.gap.tight,
            slider_width: size.control.md * SLIDER_WIDTH_SCALE,
            thumb_list_min: THUMB_LIST_MIN,
            thumb_list_max: THUMB_LIST_MAX,
            ghost_thumb: size.thumb.sm,
            toolbar_gap: space.gap.inline,
            border: Stroke::new(stroke.thin, c.line.border.to_egui()),
            ring_hover: Stroke::new(stroke.thin, c.text.muted.to_egui()),
            ring_selected: Stroke::new(stroke.thick, c.accent.solid.to_egui()),
            ring_halo: Stroke::new(stroke.thin, c.overlay.halo.to_egui()),
            caption: c.text.muted.to_egui(),
            caption_selected: c.text.normal.to_egui(),
            count: c.text.faint.to_egui(),
            header_text: c.text.normal.to_egui(),
            header_icon: c.text.muted.to_egui(),
            insert: c.accent.solid.to_egui(),
            insert_halo: c.surface.panel.to_egui(),
            insert_width: stroke.thick,
            checker: [c.overlay.checker_light.to_egui(), c.overlay.checker_dark.to_egui()],
            checker_cell: size.checker,
            picker_wide: Vec2::new(size.thumb.md + space.pad.control, size.control.md),
            picker_square: size.thumb.md + 2.0 * space.pad.control,
            picker_cell: size.thumb.md + 2.0 * space.pad.control,
            name_width: size.control.md * NAME_WIDTH_SCALE,
            popup_max_height: POPUP_MAX_HEIGHT,
            footer_room: space.gap.inline,
            swatch: Swatch::style(tokens, ControlSize::Sm),
        }
    }

    /// A ring of black or white at partial alpha that stands out from `color` over the
    /// checkerboard: the inner edge of a swatch cell that is not flush.
    pub fn inner_edge(&self, color: Rgba) -> Color32 {
        self.swatch.inner_edge(Some(color))
    }
}
