//! [`FullWindowStyle`]: the full-window regions' sizes and colours from the `full_window.*`
//! tokens (TK1), with the panel, canvas, divider and text colours they derive from.

use egui::{Color32, Stroke, Vec2};

use crate::theme::UiTokens;

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
    /// The zoom bar's progress bar: `full_window.progress.width` × `.height` (90 × 6).
    pub progress: Vec2,
    /// The zoom bar's note and other hints: `text.faint`.
    pub note: Color32,
    /// A preview's error line: `status.bad.fg`.
    pub error: Color32,
}

impl FullWindowStyle {
    /// The style for the installed theme.
    #[expect(clippy::todo, reason = "skeleton: TK1 adds the full_window tokens and fills this")]
    pub fn of(tokens: &UiTokens) -> Self {
        // new: the `full_window.*` family (step7-workspaces.md "Proposed tokens"); every field
        // above names its leaf. The note and error colours are the semantic `text.faint` and
        // `status.bad.fg`.
        let _ = tokens;
        todo!()
    }
}
