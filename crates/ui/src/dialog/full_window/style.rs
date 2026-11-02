//! [`FullWindowStyle`]: the full-window regions' sizes and colours from the `full_window.*`
//! tokens (TK1), with the panel, canvas, divider and text colours they derive from.

use egui::{Color32, Stroke, Vec2};

use crate::theme::{ToEgui, UiTokens};

/// What the full-window regions paint with, from [`FullWindowStyle::of`]. Painting and layout
/// code reads sizes and colours from here and nowhere else.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FullWindowStyle {
    /// A [`SideWidth::Standard`](super::SideWidth::Standard) column: `full_window.side.width`
    /// (PhotoCraft 300; Liquify's 292 and Filter Gallery's 290 are replaced).
    pub side_width: f32,
    /// A [`SideWidth::Wide`](super::SideWidth::Wide) column: `full_window.side.width_wide`
    /// (Select and Mask's Properties, 350).
    pub side_width_wide: f32,
    /// A [`SideWidth::Narrow`](super::SideWidth::Narrow) column: `full_window.side.width_narrow`
    /// (Vanishing Point's, which carries only OK and Cancel).
    pub side_width_narrow: f32,
    /// The padding inside a side column, x then y: `full_window.side.pad_x`, `pad_y` (14, 12).
    pub side_pad: Vec2,
    /// A side column's fill: `full_window.side.fill` (`surface.panel`).
    pub side_fill: Color32,
    /// The 1 pt line on a side column's inner edge: `full_window.side.divider` (`line.divider`).
    pub side_divider: Stroke,
    /// The gap between footer buttons and above the footer row: `full_window.footer.gap`
    /// (`space.gap.inline`, 8).
    pub footer_gap: f32,
    /// The canvas region's fill: `full_window.canvas.fill` (`surface.canvas`).
    pub canvas_fill: Color32,
    /// The room kept on each side of a fitted image: `full_window.canvas.margin` (20).
    pub canvas_margin: f32,
    /// The 1 pt outline outside the image: `full_window.canvas.outline` (`line.divider`).
    pub canvas_outline: Stroke,
    /// The zoom bar's height: `full_window.zoom_bar.height` (30).
    pub zoom_bar_height: f32,
    /// The zoom bar's padding, x then y: `full_window.zoom_bar.pad` (10 × 3).
    pub zoom_bar_pad: Vec2,
    /// The zoom bar's fill: `full_window.side.fill`, as PhotoCraft fills it with the dock colour.
    pub zoom_bar_fill: Color32,
    /// The zoom bar's progress bar length: `full_window.progress.width` (90). Its thickness is
    /// the library `Progress`'s own, as PhotoCraft's 6 pt bar is.
    pub progress_width: f32,
    /// The zoom bar's note and other hints: `text.faint`.
    pub note: Color32,
    /// A preview's error line: `status.bad.fg`.
    pub error: Color32,
}

impl FullWindowStyle {
    /// The style for the installed theme.
    pub fn of(tokens: &UiTokens) -> Self {
        // new: the `full_window.*` family (step7-workspaces.md "Proposed tokens"); every field
        // above names its leaf. The note and error colours are the semantic `text.faint` and
        // `status.bad.fg`.
        let (w, c) = (&tokens.component.full_window, &tokens.color);
        let hairline = |colour: craft_theme::Rgba| Stroke::new(1.0, colour.to_egui());
        FullWindowStyle {
            side_width: w.side_width,
            side_width_wide: w.side_width_wide,
            side_width_narrow: w.side_width_narrow,
            side_pad: Vec2::new(w.side_pad_x, w.side_pad_y),
            side_fill: w.side_fill.to_egui(),
            side_divider: hairline(w.side_divider),
            footer_gap: w.footer_gap,
            canvas_fill: w.canvas_fill.to_egui(),
            canvas_margin: w.canvas_margin,
            canvas_outline: hairline(w.canvas_outline),
            zoom_bar_height: w.zoom_bar_height,
            zoom_bar_pad: Vec2::new(w.zoom_bar_pad_x, w.zoom_bar_pad_y),
            zoom_bar_fill: w.side_fill.to_egui(),
            progress_width: w.progress_width,
            note: c.text.faint.to_egui(),
            error: c.status.bad.fg.to_egui(),
        }
    }
}
