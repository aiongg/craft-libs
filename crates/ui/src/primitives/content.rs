//! Content painters and document text.

use super::text::TextLayout;
use crate::fonts::FontRef;
use crate::runtime::egui::text_layout;
use crate::style::{DataColor, UiTokens};
use craft_i18n::Direction;
use egui::epaint::text::FontId;
use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Painter};

/// The raw painter of an allowlisted content painter, clipped to its rect.
#[non_exhaustive]
pub struct ContentPainterCx<'a> {
    /// The painter.
    pub painter: Painter,
    pub(super) tokens: &'a UiTokens,
    pub(super) direction: Direction,
}

impl<'a> ContentPainterCx<'a> {
    /// A raw painter for a content painter, with the tokens and direction the content-paint kit reads.
    pub(crate) fn new(painter: Painter, tokens: &'a UiTokens, direction: Direction) -> Self {
        ContentPainterCx { painter, tokens, direction }
    }

    /// The one sanctioned text layout for content painters: document runs with their own font, size and colour,
    /// laid out by the call `TextRun::layout` uses.
    pub fn layout(&self, text: &DocText<'_>) -> TextLayout {
        let mut job = LayoutJob::default();
        job.wrap.max_width = text.wrap.unwrap_or(f32::INFINITY);
        job.halign = text.align;
        let mut full = String::new();
        for run in text.runs {
            full.push_str(run.text);
            job.append(
                run.text,
                0.0,
                TextFormat { font_id: FontId::new(run.size, run.font.family()), color: run.color.0, italics: run.italic, ..TextFormat::default() },
            );
        }
        let galley = text_layout::layout(self.painter.ctx(), job);
        let elided = galley.elided;
        TextLayout::new(galley, elided, full)
    }
}

/// Document text: runs with their own font, size and colour. On `TextRun` these fields would let any component
/// bypass tokens for UI text, so only `ContentPainterCx::layout` takes them.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct DocText<'a> {
    /// The runs.
    pub runs: &'a [DocRun<'a>],
    /// The wrap width.
    pub wrap: Option<f32>,
    /// The horizontal alignment.
    pub align: Align,
}

impl<'a> DocText<'a> {
    /// Unwrapped document text of `runs`.
    pub const fn new(runs: &'a [DocRun<'a>]) -> Self {
        DocText { runs, wrap: None, align: Align::Min }
    }
}

/// One run of document text.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DocRun<'a> {
    /// The text.
    pub text: &'a str,
    /// The font.
    pub font: FontRef,
    /// The size in points.
    pub size: f32,
    /// The weight, for engines that have one; egui's fonts have one weight per family.
    pub weight: u16,
    /// Italic.
    pub italic: bool,
    /// The colour, from data.
    pub color: DataColor,
}

impl<'a> DocRun<'a> {
    /// A regular-weight upright run.
    pub fn new(text: &'a str, font: FontRef, size: f32, color: DataColor) -> Self {
        DocRun { text, font, size, weight: 400, italic: false, color }
    }
}
