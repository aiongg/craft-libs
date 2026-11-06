//! [`FullWindowStyle`]: the full-window regions' sizes and colours, computed from the semantic
//! tokens and the regions' own constants.

use egui::{Color32, Stroke, Vec2};

use crate::theme::{ToEgui, UiTokens};

/// What the full-window regions paint with, from [`FullWindowStyle::of`]. Painting and layout
/// code reads sizes and colours from here and nowhere else.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FullWindowStyle {
    /// A [`SideWidth::Standard`](super::SideWidth::Standard) column.
    pub side_width: f32,
    /// A [`SideWidth::Wide`](super::SideWidth::Wide) column.
    pub side_width_wide: f32,
    /// A [`SideWidth::Narrow`](super::SideWidth::Narrow) column, which carries only OK and Cancel.
    pub side_width_narrow: f32,
    /// The padding inside a side column, x then y.
    pub side_pad: Vec2,
    /// A side column's fill.
    pub side_fill: Color32,
    /// The line on a side column's inner edge.
    pub side_divider: Stroke,
    /// The gap between footer buttons and above the footer row.
    pub footer_gap: f32,
    /// The canvas region's fill.
    pub canvas_fill: Color32,
    /// The room kept on each side of a fitted image.
    pub canvas_margin: f32,
    /// The outline outside the image.
    pub canvas_outline: Stroke,
    /// The zoom bar's height.
    pub zoom_bar_height: f32,
    /// The zoom bar's padding, x then y.
    pub zoom_bar_pad: Vec2,
    /// The zoom bar's fill.
    pub zoom_bar_fill: Color32,
    /// The zoom bar's progress bar length. Its thickness is the library `Progress`'s own.
    pub progress_width: f32,
    /// The zoom bar's note and other hints.
    pub note: Color32,
    /// A preview's error line.
    pub error: Color32,
}

impl FullWindowStyle {
    /// The style for the installed theme.
    pub fn of(tokens: &UiTokens) -> Self {
        let (t, c) = (tokens, &tokens.color);
        let dialog = crate::style::dialog::DialogStyle::new(tokens);
        let hairline = |colour: craft_theme::Rgba| Stroke::new(t.stroke.thin, colour.to_egui());
        FullWindowStyle {
            side_width: dialog.width_standard * 2.0 / 3.0,
            side_width_wide: dialog.width_standard * 4.0 / 5.0,
            side_width_narrow: dialog.width_compact * 5.0 / 6.0,
            side_pad: Vec2::splat(t.space.pad.panel),
            side_fill: c.surface.panel.to_egui(),
            side_divider: hairline(c.line.divider),
            footer_gap: t.space.gap.inline,
            canvas_fill: c.surface.canvas.to_egui(),
            canvas_margin: t.space.pad.dialog,
            canvas_outline: hairline(c.line.divider),
            zoom_bar_height: t.size.control.md,
            zoom_bar_pad: Vec2::new(t.space.gap.stack, t.space.gap.tight),
            zoom_bar_fill: c.surface.panel.to_egui(),
            progress_width: t.size.control.md * 3.0,
            note: c.text.faint.to_egui(),
            error: c.status.bad.fg.to_egui(),
        }
    }
}
