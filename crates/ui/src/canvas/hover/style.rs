//! [`HoverStyle`]: the hover outline's look and timing, from tokens.

use std::time::Duration;

use egui::Color32;

use super::HoverOutline;
use crate::theme::{ToEgui, UiTokens};

const FILL_ALPHA: f32 = 0.42;
const EDGE_ALPHA: f32 = 0.95;
const DEBOUNCE_MS: u64 = 120;
const BUSY_MS: u64 = 40;

/// What [`HoverOutline`] draws and how [`HoverLookup`](super::HoverLookup) waits. Every size is a
/// token; the same at every Size (the outline sits on the document).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HoverStyle {
    /// The default colour of a rect, path and mask tint: `overlay.line` (the accent as a bare
    /// line, as PhotoCraft's rollover and Object Finder use it).
    pub color: Color32,
    /// The line width of a rect or path: `stroke.medium`.
    pub outline: f32,
    /// A mask texel's alpha per unit of coverage.
    pub fill_alpha: f32,
    /// A mask edge texel's alpha.
    pub edge_alpha: f32,
    /// How long the pointer rests before a lookup.
    pub debounce: Duration,
    /// The repaint interval while busy. The library repaints from the job queue's wake instead and
    /// keeps it only for a host without one.
    pub busy: Duration,
}

impl HoverOutline {
    /// The hover look in `tokens`.
    pub fn style(tokens: &UiTokens) -> HoverStyle {
        HoverStyle {
            color: tokens.color.overlay.line.to_egui(),
            outline: tokens.stroke.medium,
            fill_alpha: FILL_ALPHA,
            edge_alpha: EDGE_ALPHA,
            debounce: Duration::from_millis(DEBOUNCE_MS),
            busy: Duration::from_millis(BUSY_MS),
        }
    }
}
