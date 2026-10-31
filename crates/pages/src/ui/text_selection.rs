//! [`TextSelectionLayer`] (N3.14): selecting text on a rendered page (Acrobat's Select tool):
//! drag across glyphs in reading order, Alt-drag (or the app's Column Select tool) for the glyphs
//! inside a rectangle, double-click a word, triple-click a line, four clicks the page; the
//! selection highlighted, with find matches under it.
//!
//! The page's text comes from the app ([`PageTextSource`]: PdfCraft's extracted text layer); the
//! layer keeps no text.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::Range;

use egui::{Response, Ui};
use kurbo::Rect;

use super::view::PlacedPage;

/// A page's text as the selection reads it, glyph by glyph in reading order. Rectangles are page
/// points (origin top-left).
pub trait PageTextSource {
    fn glyph_count(&self) -> usize;
    /// The glyph nearest `p` within a line's height, if any.
    fn glyph_at(&self, p: kurbo::Point) -> Option<usize>;
    /// One rectangle per line piece of `glyphs`.
    fn line_rects(&self, glyphs: Range<usize>) -> Vec<Rect>;
    /// The glyphs whose boxes `rect` holds, in reading order.
    fn glyphs_in(&self, rect: Rect) -> Vec<usize>;
    /// The word and the line around a glyph, as glyph ranges.
    fn word_at(&self, glyph: usize) -> Option<Range<usize>>;
    fn line_at(&self, glyph: usize) -> Option<Range<usize>>;
    /// The text of `glyphs` as Copy puts it on the clipboard (lines joined with newlines).
    fn text_of(&self, glyphs: Range<usize>) -> String;
    /// The text of a column selection, one line per row.
    fn column_text(&self, glyphs: &[usize]) -> String;
}

/// What is selected on one page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextSpan {
    /// Reading order, between two glyphs (inclusive), anchor where the drag started.
    Run { anchor: usize, head: usize },
    /// The glyphs inside a rectangle (page points), drawn from the corner the drag started at.
    Column { from: kurbo::Point, to: kurbo::Point },
}

/// A text selection on one page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelection {
    pub page: usize,
    pub span: TextSpan,
}

impl TextSelection {
    /// The selection as rectangles, one per line piece (for highlighting and markup quads).
    // pdfcraft: crates/ui-egui/src/canvas/selection.rs:29 `Selection::rects`.
    pub fn rects(&self, text: &dyn PageTextSource) -> Vec<Rect> {
        todo!()
    }

    /// The selected text, as Copy puts it on the clipboard.
    // pdfcraft: crates/ui-egui/src/canvas/selection.rs:37 `Selection::text`.
    pub fn text(&self, text: &dyn PageTextSource) -> String {
        todo!()
    }
}

/// The glyphs `clicks` quick clicks select around `glyph`: 2 the word, 3 the line, 4 or more the
/// page.
// pdfcraft: crates/ui-egui/src/canvas/selection.rs:47 `click_unit`.
pub fn click_unit(text: &dyn PageTextSource, clicks: u32, glyph: usize) -> Option<Range<usize>> {
    todo!()
}

/// A highlight drawn with the selection: find matches, the current one stronger.
#[derive(Clone, Debug, PartialEq)]
pub struct TextMark {
    pub glyphs: Range<usize>,
    pub current: bool,
}

/// The layer on one placed page, built per pass after the page is drawn.
///
/// - Drag with the primary button selects ([`TextSpan::Run`]); with Alt held (or
///   [`column`](Self::column) on) it selects a column, its rectangle outlined 1 pt while
///   dragging. The cursor is the I-beam over glyphs.
/// - Highlights: selection 70/255 of `selection.text_fill` (PdfCraft: #3A7BF0 at 70), corner
///   radius 1; matches yellow at 90 and the current one orange at 110, 1 pt larger, radius 2
///   (PdfCraft's find marks); the colours are tokens the fill task proposes.
/// - Keys with the page view focused: ⇧-arrows extend a run by a glyph (⇧⌥ by a word), ⌘/Ctrl-A
///   selects the page (or, continuous, the document: the app's Select All), Esc clears.
/// - The selection is reported to AccessKit through the bridge ([`super::access`]).
pub struct TextSelectionLayer<'a> {
    id_salt: egui::IdSalt,
    page: &'a PlacedPage,
    text: &'a dyn PageTextSource,
    selection: &'a mut Option<TextSelection>,
    marks: &'a [TextMark],
    column: bool,
}

impl<'a> TextSelectionLayer<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, page: &'a PlacedPage, text: &'a dyn PageTextSource, selection: &'a mut Option<TextSelection>) -> Self {
        TextSelectionLayer { id_salt: egui::IdSalt::new(id_salt), page, text, selection, marks: &[], column: false }
    }

    pub fn marks(mut self, marks: &'a [TextMark]) -> Self {
        self.marks = marks;
        self
    }

    /// The Column Select tool is active.
    pub fn column(mut self, on: bool) -> Self {
        self.column = on;
        self
    }

    // pdfcraft: crates/ui-egui/src/canvas.rs:1470 `text_layer`, :1485 `paint_text_marks`, :1547
    // `text_pointer_input`.
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
