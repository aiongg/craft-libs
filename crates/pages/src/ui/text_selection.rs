//! [`TextSelectTool`] (N3.14): selecting text on a rendered page (Acrobat's Select tool, a
//! craft-ui canvas tool):
//! drag across glyphs in reading order, Alt-drag (or the app's Column Select tool) for the glyphs
//! inside a rectangle, double-click a word, triple-click a line, four clicks the page; the
//! selection highlighted, with find matches under it ([`paint_text_marks`]).
//!
//! The page's text comes from the app ([`PageTextSource`]: PdfCraft's extracted text layer); the
//! layer keeps no text.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::Range;

use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use craft_ui::theme::UiTokens;
use egui::Painter;
use kurbo::Rect;

use super::style::TextSelectStyle;
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
    pub page: usize,
    pub glyphs: Range<usize>,
    pub current: bool,
}

/// The text of the document's pages, for the tool: the pages whose text the app has (PdfCraft
/// extracts it in the background; a page without it yet selects nothing).
pub trait PageTexts {
    fn page(&self, page: usize) -> Option<&dyn PageTextSource>;
}

/// The tool's state, kept by the app per document view: the selection (view state, applied in
/// place, as craft-ui's `List` applies its selection) and the gesture in progress.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSelectState {
    pub selection: Option<TextSelection>,
    /// The drag in progress: the page and where it started (glyph, or page point for a column).
    gesture: Option<(usize, TextSpan)>,
}

/// Paint the selection and find marks of one placed page: the selection fill, the match fills,
/// the current match grown and rounded as the style says. The page view's overlays call it for
/// every page whatever tool is active (find marks show under the Hand tool too);
/// [`TextSelectTool::paint`](craft_ui::canvas::CanvasTool::paint) calls it for the selection.
// pdfcraft: crates/ui-egui/src/canvas.rs:1485 `paint_text_marks`.
pub fn paint_text_marks(
    painter: &Painter,
    placed: &PlacedPage,
    text: &dyn PageTextSource,
    selection: Option<&TextSelection>,
    marks: &[TextMark],
    style: &TextSelectStyle,
) {
    todo!()
}

/// Acrobat's Select tool on the page view, a craft-ui [`CanvasTool`] the app runs in its
/// `ToolHost` over [`PageViewResponse::canvas`](super::PageViewResponse::canvas); built per pass
/// over the pass's placed pages and the app's [`TextSelectState`].
///
/// - Press and drag with the primary button selects ([`TextSpan::Run`], anchored at the press's
///   glyph); with Alt held (or [`column`](Self::column) on: the Column Select tool) it selects
///   a column, its rectangle outlined while dragging. The press's click count is
///   [`ToolPointer::clicks`](craft_ui::canvas::ToolPointer::clicks) (the app-wide double-click
///   time): 2 the word, 3 the line, 4 the page ([`click_unit`]). The cursor is the I-beam over
///   glyphs, the arrow elsewhere ([`CanvasTool::cursor`]). A press off any glyph clears the
///   selection.
/// - Highlights through [`paint_text_marks`] with [`TextSelectStyle`] (the `text_selection.*`
///   tokens proposed in the spec).
/// - Keys while the canvas has the keyboard ([`CanvasTool::key`], taken only when used):
///   ⇧-arrows extend a run by a glyph (⇧⌥ by a word), Esc clears. Select All and Copy are the
///   app's commands (they read [`TextSelection::text`]).
/// - The selection is reported to AccessKit through the bridge ([`super::access`]).
pub struct TextSelectTool<'a> {
    pages: &'a [PlacedPage],
    texts: &'a dyn PageTexts,
    state: &'a mut TextSelectState,
    marks: &'a [TextMark],
    column: bool,
    changed: bool,
}

impl<'a> TextSelectTool<'a> {
    pub fn new(pages: &'a [PlacedPage], texts: &'a dyn PageTexts, state: &'a mut TextSelectState) -> Self {
        TextSelectTool { pages, texts, state, marks: &[], column: false, changed: false }
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

    /// Whether the selection changed this pass (read after `ToolHost::show`).
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// The tool's colours and sizes, from the tokens.
    pub fn style(tokens: &UiTokens) -> TextSelectStyle {
        TextSelectStyle::of(tokens)
    }
}

// pdfcraft: crates/ui-egui/src/canvas.rs:1470 `text_layer`, :1547 `text_pointer_input`.
impl CanvasTool for TextSelectTool<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        todo!()
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        todo!()
    }
}
