//! Full-window editors: the side column, the canvas region, the zoom bar and the progress bar. The sizes are PhotoCraft's, the same at every Size.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Full-window editors: the side column, canvas region, zoom bar and progress bar.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FullWindowTokens {
    /// A standard side column: `full_window.side.width`.
    pub side_width: f32,
    /// A wide side column: `full_window.side.width_wide`.
    pub side_width_wide: f32,
    /// The padding inside a side column, across: `full_window.side.pad_x`.
    pub side_pad_x: f32,
    /// The padding inside a side column, along: `full_window.side.pad_y`.
    pub side_pad_y: f32,
    /// The gap between footer buttons and above the footer row: `full_window.footer.gap`.
    pub footer_gap: f32,
    /// The room kept on each side of a fitted image: `full_window.canvas.margin`.
    pub canvas_margin: f32,
    /// The zoom bar's height: `full_window.zoom_bar.height`.
    pub zoom_bar_height: f32,
    /// The zoom bar's padding, across: `full_window.zoom_bar.pad_x`.
    pub zoom_bar_pad_x: f32,
    /// The zoom bar's padding, along: `full_window.zoom_bar.pad_y`.
    pub zoom_bar_pad_y: f32,
    /// The zoom bar's progress bar width: `full_window.progress.width`.
    pub progress_width: f32,
    /// The progress bar's height: `full_window.progress.height`.
    pub progress_height: f32,
    /// A side column's fill: `full_window.side.fill`, `surface.panel`.
    pub side_fill: Rgba,
    /// The line on a side column's inner edge: `full_window.side.divider`, `line.divider`.
    pub side_divider: Rgba,
    /// The canvas region's fill: `full_window.canvas.fill`, `surface.canvas`.
    pub canvas_fill: Rgba,
    /// The outline outside the image: `full_window.canvas.outline`, `line.divider`.
    pub canvas_outline: Rgba,
}

impl FullWindowTokens {
    fn color_mut(&mut self, f: FullWindowField) -> &mut Rgba {
        match f {
            FullWindowField::SideFill => &mut self.side_fill,
            FullWindowField::SideDivider => &mut self.side_divider,
            FullWindowField::CanvasFill => &mut self.canvas_fill,
            FullWindowField::CanvasOutline => &mut self.canvas_outline,
        }
    }
}

/// An overridable full window token. Paths: `side.fill`, `side.divider`, `canvas.fill`, `canvas.outline`, e.g. `full_window.side.fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FullWindowField {
    /// `full_window.side.fill`
    SideFill,
    /// `full_window.side.divider`
    SideDivider,
    /// `full_window.canvas.fill`
    CanvasFill,
    /// `full_window.canvas.outline`
    CanvasOutline,
}

impl FullWindowField {
    const fn path(self) -> &'static str {
        match self {
            FullWindowField::SideFill => "side.fill",
            FullWindowField::SideDivider => "side.divider",
            FullWindowField::CanvasFill => "canvas.fill",
            FullWindowField::CanvasOutline => "canvas.outline",
        }
    }
}

impl Family for FullWindowTokens {
    type Field = FullWindowField;
    const FIELDS: &'static [FullWindowField] =
        &[FullWindowField::SideFill, FullWindowField::SideDivider, FullWindowField::CanvasFill, FullWindowField::CanvasOutline];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { FullWindowTokens;
        "side.width" 0.0..=1024.0 => side_width;
        "side.width_wide" 0.0..=1024.0 => side_width_wide;
        "side.pad_x" 0.0..=128.0 => side_pad_x;
        "side.pad_y" 0.0..=128.0 => side_pad_y;
        "footer.gap" 0.0..=64.0 => footer_gap;
        "canvas.margin" 0.0..=256.0 => canvas_margin;
        "zoom_bar.height" 0.0..=256.0 => zoom_bar_height;
        "zoom_bar.pad_x" 0.0..=128.0 => zoom_bar_pad_x;
        "zoom_bar.pad_y" 0.0..=128.0 => zoom_bar_pad_y;
        "progress.width" 0.0..=1024.0 => progress_width;
        "progress.height" 0.0..=128.0 => progress_height;
    };

    fn defaults(s: &Semantic<'_>) -> FullWindowTokens {
        let c = s.color;
        FullWindowTokens {
            side_width: values::FULL_WINDOW_SIDE_WIDTH,
            side_width_wide: values::FULL_WINDOW_SIDE_WIDTH_WIDE,
            side_pad_x: values::FULL_WINDOW_SIDE_PAD_X,
            side_pad_y: values::FULL_WINDOW_SIDE_PAD_Y,
            footer_gap: s.space.gap.inline,
            canvas_margin: values::FULL_WINDOW_CANVAS_MARGIN,
            zoom_bar_height: values::FULL_WINDOW_ZOOM_BAR_HEIGHT,
            zoom_bar_pad_x: values::FULL_WINDOW_ZOOM_BAR_PAD_X,
            zoom_bar_pad_y: values::FULL_WINDOW_ZOOM_BAR_PAD_Y,
            progress_width: values::FULL_WINDOW_PROGRESS_WIDTH,
            progress_height: values::FULL_WINDOW_PROGRESS_HEIGHT,
            side_fill: c.surface.panel,
            side_divider: c.line.divider,
            canvas_fill: c.surface.canvas,
            canvas_outline: c.line.divider,
        }
    }

    fn path(f: FullWindowField) -> String {
        f.path().to_owned()
    }

    fn from_path(path: &str) -> Option<FullWindowField> {
        Self::FIELDS.iter().copied().find(|f| f.path() == path)
    }

    fn kind(_: FullWindowField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: FullWindowField) -> OverrideValue {
        let mut copy = *self;
        OverrideValue::Color(*copy.color_mut(f))
    }

    fn set(&mut self, f: FullWindowField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            *self.color_mut(f) = c;
        }
    }

    // No contrast pairs: these are fills and lines on the document or beside it.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
