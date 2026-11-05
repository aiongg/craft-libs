//! [`TextSelectTool`] (N3.14): selecting text on a rendered page (Acrobat's Select tool, a
//! craft-ui canvas tool):
//! drag across glyphs in reading order, Alt-drag (or the app's Column Select tool) for the glyphs
//! inside a rectangle, double-click a word, triple-click a line, four clicks the page; the
//! selection highlighted, with find matches under it ([`paint_text_marks`]).
//!
//! The page's text comes from the app ([`PageTextSource`]: PdfCraft's extracted text layer); the
//! layer keeps no text.

use std::ops::Range;

use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use craft_ui::edit::CancelReason;
use craft_ui::theme::{UiTokens, corner_radius};
use egui::{CursorIcon, Key, Painter, Pos2, StrokeKind};
use kurbo::{Point, Rect};

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
        match self.span {
            TextSpan::Run { anchor, head } => text.line_rects(run_range(anchor, head)),
            TextSpan::Column { from, to } => {
                let glyphs = text.glyphs_in(Rect::from_points(from, to));
                runs(&glyphs).into_iter().flat_map(|run| text.line_rects(run)).collect()
            }
        }
    }

    /// The selected text, as Copy puts it on the clipboard.
    // pdfcraft: crates/ui-egui/src/canvas/selection.rs:37 `Selection::text`.
    pub fn text(&self, text: &dyn PageTextSource) -> String {
        match self.span {
            TextSpan::Run { anchor, head } => text.text_of(run_range(anchor, head)),
            TextSpan::Column { from, to } => text.column_text(&text.glyphs_in(Rect::from_points(from, to))),
        }
    }
}

/// The glyphs of a run, both ends included.
fn run_range(anchor: usize, head: usize) -> Range<usize> {
    anchor.min(head)..anchor.max(head).saturating_add(1)
}

/// `glyphs` as ranges of consecutive indices.
fn runs(glyphs: &[usize]) -> Vec<Range<usize>> {
    let mut runs: Vec<Range<usize>> = Vec::new();
    for &glyph in glyphs {
        match runs.last_mut() {
            Some(run) if run.end == glyph => run.end += 1,
            _ => runs.push(glyph..glyph + 1),
        }
    }
    runs
}

/// The glyphs `clicks` quick clicks select around `glyph`: 2 the word, 3 the line, 4 or more the
/// page.
// pdfcraft: crates/ui-egui/src/canvas/selection.rs:47 `click_unit`.
pub fn click_unit(text: &dyn PageTextSource, clicks: u32, glyph: usize) -> Option<Range<usize>> {
    match clicks {
        2 => text.word_at(glyph),
        3 => text.line_at(glyph),
        4.. => Some(0..text.glyph_count()).filter(|page| !page.is_empty()),
        _ => None,
    }
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
    /// The press's click count, and the selection from before the gesture (Esc brings it back).
    clicks: u32,
    before: Option<TextSelection>,
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
    let (top, bottom) = placed.band;
    let band =
        egui::Rect::from_min_max(egui::pos2(placed.rect.min.x, placed.rect.min.y + top), egui::pos2(placed.rect.max.x, placed.rect.min.y + bottom));
    let painter = painter.with_clip_rect(band.intersect(painter.clip_rect()));
    let screen = |r: Rect| egui::Rect::from_two_pos(placed.to_screen(Point::new(r.x0, r.y0)), placed.to_screen(Point::new(r.x1, r.y1)));
    for mark in marks.iter().filter(|mark| mark.page == placed.page) {
        let (fill, radius, grow) =
            if mark.current { (style.current, style.current_radius, style.current_grow) } else { (style.found, style.radius, 0.0) };
        for rect in text.line_rects(mark.glyphs.clone()) {
            painter.rect_filled(screen(rect).expand(grow), corner_radius(radius), fill);
        }
    }
    if let Some(selection) = selection.filter(|selection| selection.page == placed.page) {
        for rect in selection.rects(text) {
            painter.rect_filled(screen(rect), corner_radius(style.radius), style.selection);
        }
    }
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
        use craft_ui::theme::ToEgui as _;
        let t = &tokens.component.pages;
        TextSelectStyle {
            selection: t.text_selection_fill.to_egui(),
            found: t.find_match.to_egui(),
            current: t.find_current.to_egui(),
            radius: t.text_selection_radius,
            current_radius: t.find_current_radius,
            current_grow: t.find_current_grow,
            column_outline: egui::Stroke::new(t.text_selection_column_outline, tokens.color.accent.solid.to_egui()),
        }
    }
}

impl TextSelectTool<'_> {
    /// The shown part of the page under `at`.
    fn page_at(&self, at: Pos2) -> Option<&'_ PlacedPage> {
        let pages: &[PlacedPage] = self.pages;
        pages.iter().find(|placed| {
            let (top, bottom) = placed.band;
            placed.rect.contains(at) && at.y >= placed.rect.min.y + top && at.y <= placed.rect.min.y + bottom
        })
    }

    fn set(&mut self, selection: Option<TextSelection>) {
        if self.state.selection != selection {
            self.state.selection = selection;
            self.changed = true;
        }
    }

    fn end_gesture(&mut self) {
        self.state.gesture = None;
        self.state.clicks = 0;
        self.state.before = None;
    }
}

// pdfcraft: crates/ui-egui/src/canvas.rs:1470 `text_layer`, :1547 `text_pointer_input`.
impl CanvasTool for TextSelectTool<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        let Some(pointer) = cx.pointer else { return ToolCursor::System(CursorIcon::Default) };
        if self.column || cx.modifiers.alt {
            return ToolCursor::System(CursorIcon::Crosshair);
        }
        let over_glyph = self
            .page_at(pointer.screen)
            .and_then(|placed| self.texts.page(placed.page).map(|text| text.glyph_at(placed.to_page(pointer.screen))))
            .is_some_and(|glyph| glyph.is_some());
        ToolCursor::System(if over_glyph { CursorIcon::Text } else { CursorIcon::Default })
    }

    fn press(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        let texts = self.texts;
        let Some(placed) = self.page_at(pointer.screen).cloned() else {
            self.set(None);
            return ToolAction::Done;
        };
        let Some(text) = texts.page(placed.page) else {
            self.set(None);
            return ToolAction::Done;
        };
        let at = placed.to_page(pointer.screen);
        let before = self.state.selection;
        if self.column || pointer.modifiers.alt {
            self.state.gesture = Some((placed.page, TextSpan::Column { from: at, to: at }));
            self.state.clicks = 1;
            self.state.before = before;
            self.set(Some(TextSelection { page: placed.page, span: TextSpan::Column { from: at, to: at } }));
            return ToolAction::Claimed;
        }
        let Some(glyph) = text.glyph_at(at) else {
            self.set(None);
            return ToolAction::Done;
        };
        let clicks = pointer.clicks.max(1);
        let extended = before.filter(|selection| pointer.modifiers.shift && clicks < 2 && selection.page == placed.page);
        let (span, start) = match (clicks, extended) {
            (2.., _) => match click_unit(text, clicks, glyph).filter(|unit| !unit.is_empty()) {
                Some(unit) => (TextSpan::Run { anchor: unit.start, head: unit.end - 1 }, TextSpan::Run { anchor: glyph, head: glyph }),
                None => {
                    self.set(None);
                    return ToolAction::Done;
                }
            },
            (_, Some(TextSelection { span: TextSpan::Run { anchor, .. }, .. })) => {
                (TextSpan::Run { anchor, head: glyph }, TextSpan::Run { anchor, head: anchor })
            }
            _ => (TextSpan::Run { anchor: glyph, head: glyph }, TextSpan::Run { anchor: glyph, head: glyph }),
        };
        self.state.gesture = Some((placed.page, start));
        self.state.clicks = clicks;
        self.state.before = before;
        self.set(Some(TextSelection { page: placed.page, span }));
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        let (pages, texts) = (self.pages, self.texts);
        let Some((page, start)) = self.state.gesture else { return };
        let Some(selection) = self.state.selection.filter(|selection| selection.page == page) else { return };
        let (Some(placed), Some(text)) = (pages.iter().find(|placed| placed.page == page), texts.page(page)) else { return };
        let at = placed.to_page(pointer.screen);
        let span = match (selection.span, start) {
            (TextSpan::Column { from, .. }, _) => TextSpan::Column { from, to: at },
            (TextSpan::Run { anchor, .. }, TextSpan::Run { anchor: origin, .. }) => {
                let Some(hit) = text.glyph_at(at) else { return };
                let clicks = self.state.clicks;
                match click_unit(text, clicks, origin).filter(|unit| !unit.is_empty()) {
                    Some(unit) if clicks >= 2 => {
                        let reached = click_unit(text, clicks, hit).filter(|unit| !unit.is_empty()).unwrap_or(hit..hit + 1);
                        if reached.start < unit.start {
                            TextSpan::Run { anchor: unit.end - 1, head: reached.start }
                        } else {
                            TextSpan::Run { anchor: unit.start, head: (reached.end - 1).max(unit.end - 1) }
                        }
                    }
                    _ => TextSpan::Run { anchor, head: hit },
                }
            }
            (span, _) => span,
        };
        self.set(Some(TextSelection { page, span }));
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.drag(pointer, cx);
        if let Some(selection) = self.state.selection
            && matches!(selection.span, TextSpan::Column { .. })
            && self.texts.page(selection.page).is_none_or(|text| selection.text(text).is_empty())
        {
            self.set(None);
        }
        self.end_gesture();
    }

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {
        if self.state.gesture.is_some() {
            let before = self.state.before;
            self.set(before);
        }
        self.end_gesture();
    }

    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        let Some(selection) = self.state.selection.filter(|_| key.pressed) else { return false };
        if key.key == Key::Escape && !key.modifiers.any() {
            self.set(None);
            return true;
        }
        let (TextSpan::Run { anchor, head }, true) = (selection.span, key.modifiers.shift) else { return false };
        let Some(text) = self.texts.page(selection.page) else { return false };
        let count = text.glyph_count();
        let forward = match key.key {
            Key::ArrowRight | Key::ArrowDown => true,
            Key::ArrowLeft | Key::ArrowUp => false,
            _ => return false,
        };
        let moved = match key.key {
            Key::ArrowUp | Key::ArrowDown => line_step(text, head, forward),
            _ if key.modifiers.alt => word_step(text, head, forward, count),
            _ if forward => head + 1,
            _ => head.saturating_sub(1),
        };
        self.set(Some(TextSelection { page: selection.page, span: TextSpan::Run { anchor, head: moved.min(count.saturating_sub(1)) } }));
        true
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        let style = Self::style(cx.tokens);
        for placed in self.pages {
            let Some(text) = self.texts.page(placed.page) else { continue };
            paint_text_marks(painter, placed, text, self.state.selection.as_ref(), self.marks, &style);
        }
        if let (Some((page, _)), Some(TextSelection { page: selected, span: TextSpan::Column { from, to } })) =
            (self.state.gesture, self.state.selection)
            && page == selected
            && let Some(placed) = self.pages.iter().find(|placed| placed.page == page)
        {
            let rect = egui::Rect::from_two_pos(placed.to_screen(from), placed.to_screen(to));
            painter.rect_stroke(rect, 0.0, style.column_outline, StrokeKind::Middle);
        }
    }
}

/// The head one word on from `head`, or back.
fn word_step(text: &dyn PageTextSource, head: usize, forward: bool, count: usize) -> usize {
    if forward {
        (head + 1..count).find_map(|glyph| text.word_at(glyph).map(|word| word.end - 1).filter(|end| *end > head)).unwrap_or(head)
    } else {
        (0..head).rev().find_map(|glyph| text.word_at(glyph).map(|word| word.start).filter(|start| *start < head)).unwrap_or(head)
    }
}

/// The head on the next line (or the one before), at the same distance from the line's start.
fn line_step(text: &dyn PageTextSource, head: usize, forward: bool) -> usize {
    let Some(line) = text.line_at(head) else { return head };
    let other = if forward {
        (line.end < text.glyph_count()).then(|| text.line_at(line.end)).flatten()
    } else {
        line.start.checked_sub(1).and_then(|before| text.line_at(before))
    };
    other.filter(|other| !other.is_empty()).map_or(head, |other| (other.start + (head - line.start)).min(other.end - 1))
}
