//! Style of the `dialog` family.

use craft_theme::{Rgba, Tokens};

const BUTTON_MIN: f32 = 84.0;
const SIDE_MIN: f32 = 96.0;
const WIDTH_COMPACT: f32 = 260.0;
const WIDTH_MIN: f32 = 380.0;
const WIDTH_STANDARD: f32 = 440.0;
const WIDTH_WIDE: f32 = 600.0;
const WIDTH_ALERT: f32 = 420.0;
const VIEWPORT_SIZE: [f32; 2] = [480.0, 360.0];
const PREVIEW_PANE: [f32; 2] = [140.0, 150.0];
const VIEWPORT_MIN: [f32; 2] = [240.0, 160.0];
const RESIZE_MARGIN: f32 = 20.0;
const KEEP_VISIBLE: f32 = 100.0;
const SCROLL_MIN: f32 = 120.0;
const ANCHOR_MARK_RADIUS: f32 = 1.0;

/// Sizes and colours of dialogs that the semantic tokens don't name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DialogStyle {
    /// Least width of a footer button.
    pub button_min: f32,
    /// Least width of a side-column button.
    pub side_min: f32,
    /// Greatest content width of a compact dialog.
    pub width_compact: f32,
    /// Least content width of a standard or wide dialog.
    pub width_min: f32,
    /// Greatest content width of a standard dialog.
    pub width_standard: f32,
    /// Greatest content width of a wide dialog.
    pub width_wide: f32,
    /// Content width of an alert or a prompt.
    pub width_alert: f32,
    /// Default size of a secondary viewport's window.
    pub viewport_size: [f32; 2],
    /// A preview pane's least width and its height.
    pub preview_pane: [f32; 2],
    /// Least size of a secondary viewport's window.
    pub viewport_min: [f32; 2],
    /// Space kept between a resizable dialog's greatest size and the window edge.
    pub resize_margin: f32,
    /// Width of a dragged dialog that must stay on screen.
    pub keep_visible: f32,
    /// Least height of a scrolling body.
    pub scroll_min: f32,
    /// Corner radius of the mark in the chosen anchor cell.
    pub anchor_mark_radius: f32,
    /// The well of a preview pane.
    pub preview_fill: Rgba,
}

impl DialogStyle {
    /// The dialog style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        Self {
            button_min: BUTTON_MIN,
            side_min: SIDE_MIN,
            width_compact: WIDTH_COMPACT,
            width_min: WIDTH_MIN,
            width_standard: WIDTH_STANDARD,
            width_wide: WIDTH_WIDE,
            width_alert: WIDTH_ALERT,
            viewport_size: VIEWPORT_SIZE,
            preview_pane: PREVIEW_PANE,
            viewport_min: VIEWPORT_MIN,
            resize_margin: RESIZE_MARGIN,
            keep_visible: KEEP_VISIBLE,
            scroll_min: SCROLL_MIN,
            anchor_mark_radius: ANCHOR_MARK_RADIUS,
            preview_fill: tokens.color.surface.canvas,
        }
    }
}
