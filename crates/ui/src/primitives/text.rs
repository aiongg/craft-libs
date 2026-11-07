//! `TextRun`, the one description of UI text for measure, layout and paint, and `TextLayout`, its laid-out form.

use crate::runtime::egui::text_layout;
use crate::style::{StyleColor, TypeRole, UiTokens};
use craft_i18n::Direction;
use egui::epaint::text::{FontId, TextDirection};
use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Align, Color32, Context, Galley};
use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

/// How a run fits its room.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Wrap {
    /// One line at its natural width.
    #[default]
    Line,
    /// Wraps at the room's width.
    Wrap,
    /// One line, cut with an ellipsis at the end.
    Elide,
    /// One line, cut with an ellipsis in the middle, so the end of a path stays.
    ElidePath,
}

/// Where the lines of a multi-line run sit within the run's width.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum HAlign {
    /// At the start edge: left, right in a right-to-left interface.
    #[default]
    Start,
    /// Centred.
    Center,
    /// At the end edge: right, left in a right-to-left interface.
    End,
}

/// One styled stretch of a run.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Span<'a> {
    /// The text.
    pub text: Cow<'a, str>,
    /// Italic.
    pub italic: bool,
    /// Tabular figures.
    pub tabular: bool,
    /// The monospace face.
    pub mono: bool,
    /// The muted text colour.
    pub muted: bool,
    /// A shortcut hint: muted.
    pub shortcut: bool,
    /// A search match highlight.
    pub highlight: bool,
}

impl<'a> Span<'a> {
    /// A plain span.
    pub fn new(text: impl Into<Cow<'a, str>>) -> Self {
        Span { text: text.into(), italic: false, tabular: false, mono: false, muted: false, shortcut: false, highlight: false }
    }

    /// Italic.
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Tabular figures.
    pub fn tabular(mut self) -> Self {
        self.tabular = true;
        self
    }

    /// The monospace face.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    /// The muted colour.
    pub fn muted(mut self) -> Self {
        self.muted = true;
        self
    }

    /// A shortcut hint.
    pub fn shortcut(mut self) -> Self {
        self.shortcut = true;
        self
    }

    /// A match highlight.
    pub fn highlight(mut self) -> Self {
        self.highlight = true;
        self
    }
}

/// The one description of UI text.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextRun<'a> {
    /// The styled stretches, in reading order.
    pub spans: Vec<Span<'a>>,
    /// The type role.
    pub role: TypeRole,
    /// The colour; `None` takes the surface's ink.
    pub color: Option<StyleColor>,
    /// How the run fits its room.
    pub wrap: Wrap,
    /// Laid out left to right whatever the env's direction (a chord).
    pub ltr: bool,
    /// Where the lines sit within the run's width.
    pub align: HAlign,
}

impl<'a> TextRun<'a> {
    /// A one-span run in the label role.
    pub fn new(text: impl Into<Cow<'a, str>>) -> Self {
        TextRun { spans: vec![Span::new(text)], role: TypeRole::Label, color: None, wrap: Wrap::Line, ltr: false, align: HAlign::Start }
    }

    /// A run of `spans`.
    pub fn spans(spans: Vec<Span<'a>>) -> Self {
        TextRun { spans, role: TypeRole::Label, color: None, wrap: Wrap::Line, ltr: false, align: HAlign::Start }
    }

    /// Sets the type role.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }

    /// Sets the colour.
    pub fn color(mut self, color: StyleColor) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets how the run fits its room.
    pub fn wrap(mut self, wrap: Wrap) -> Self {
        self.wrap = wrap;
        self
    }

    /// Sets where the lines sit within the run's width.
    pub fn align(mut self, align: HAlign) -> Self {
        self.align = align;
        self
    }

    /// Cuts the run with an ellipsis at the end.
    pub fn elide(self) -> Self {
        self.wrap(Wrap::Elide)
    }

    /// The text of every span, joined.
    pub fn full_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_ref()).collect()
    }
}

/// The galley moved so its rect starts at x = 0: egui anchors a centred or right-aligned galley at its centre or
/// right edge, while every caller places a layout by its top-left corner.
fn anchored_at_start(mut galley: Arc<Galley>) -> Arc<Galley> {
    let shift = -galley.rect.min.x;
    if shift == 0.0 {
        return galley;
    }
    let galley_mut = Arc::make_mut(&mut galley);
    let by = egui::vec2(shift, 0.0);
    for placed in &mut galley_mut.rows {
        placed.pos.x += shift;
    }
    galley_mut.rect = galley_mut.rect.translate(by);
    galley_mut.mesh_bounds = galley_mut.mesh_bounds.translate(by);
    galley
}

/// What a layout reads from the env: the tokens, the direction and the ink colour.
#[derive(Clone, Copy)]
pub(crate) struct TextEnv<'a> {
    pub tokens: &'a UiTokens,
    pub direction: Direction,
    pub ink: Color32,
}

impl TextRun<'_> {
    /// The one text layout behind `MeasureCx::layout`, `PartsCx::layout`, `TextParts::layout` and `Draw::text`: the
    /// paragraph direction comes from the caller's env, so measure and paint agree. `max_width` is the room; a
    /// `Line` run ignores it.
    pub(crate) fn layout(&self, ctx: &Context, env: &TextEnv<'_>, max_width: f32) -> TextLayout {
        let full = self.full_text();
        let galley = match self.wrap {
            Wrap::ElidePath => self.layout_elided_path(ctx, env, max_width),
            _ => text_layout::layout(ctx, self.job(env, self.spans.iter().map(|s| (s, s.text.as_ref())), max_width)),
        };
        let elided = galley.elided || self.wrap == Wrap::ElidePath && galley.job.text != full;
        TextLayout::new(anchored_at_start(galley), elided, full)
    }

    fn job<'s>(&self, env: &TextEnv<'_>, parts: impl Iterator<Item = (&'s Span<'s>, &'s str)>, max_width: f32) -> LayoutJob {
        let elide = matches!(self.wrap, Wrap::Elide | Wrap::ElidePath);
        let direction = match (self.ltr, env.direction) {
            (false, Direction::Rtl) => TextDirection::RightToLeft,
            _ => TextDirection::LeftToRight,
        };
        let rtl = matches!(direction, TextDirection::RightToLeft);
        let mut job = LayoutJob {
            direction,
            halign: match (self.align, rtl) {
                (HAlign::Center, _) => Align::Center,
                (HAlign::End, false) => Align::RIGHT,
                _ => Align::LEFT,
            },
            // Start follows the paragraph's direction; an end-aligned run in a right-to-left interface puts every
            // line on the left.
            halign_min_is_start: !(self.align == HAlign::End && rtl),
            wrap: TextWrapping {
                max_width: if self.wrap == Wrap::Line { f32::INFINITY } else { max_width },
                max_rows: if elide { 1 } else { usize::MAX },
                break_anywhere: elide,
                ..TextWrapping::default()
            },
            ..LayoutJob::default()
        };
        let base = self.color.map_or(env.ink, StyleColor::egui);
        let text = &env.tokens.color.text;
        let to_color = |c: craft_theme::Rgba| Color32::from_rgba_unmultiplied(c.0[0], c.0[1], c.0[2], c.0[3]);
        for (span, text_part) in parts {
            let font_id: FontId = env.tokens.font(self.role, span.mono);
            let color = if span.muted {
                to_color(text.muted)
            } else if span.shortcut {
                to_color(text.faint)
            } else {
                base
            };
            let background = if span.highlight { to_color(env.tokens.color.state.r#match) } else { Color32::TRANSPARENT };
            job.append(text_part, 0.0, TextFormat { font_id, color, italics: span.italic, background, ..TextFormat::default() });
        }
        job
    }

    /// Cuts characters from the middle until the run fits on one row, keeping a longer tail than head.
    fn layout_elided_path(&self, ctx: &Context, env: &TextEnv<'_>, max_width: f32) -> Arc<Galley> {
        let full = self.full_text();
        let whole = text_layout::layout(ctx, self.job(env, self.spans.iter().map(|s| (s, s.text.as_ref())), f32::INFINITY));
        if whole.size().x <= max_width {
            return whole;
        }
        let chars: Vec<(usize, char)> = full.char_indices().collect();
        let n = chars.len();
        let fits = |cut: usize| {
            let galley = text_layout::layout(ctx, self.job_cut(env, &chars, &full, cut));
            (galley.size().x <= max_width, galley)
        };
        let (mut lo, mut hi) = (1, n);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if fits(mid).0 { hi = mid } else { lo = mid + 1 }
        }
        fits(lo).1
    }

    /// The job with `cut` characters removed from the middle and one ellipsis in their place, in the style of the
    /// span the cut starts in.
    fn job_cut(&self, env: &TextEnv<'_>, chars: &[(usize, char)], full: &str, cut: usize) -> LayoutJob {
        let n = chars.len();
        let keep = n.saturating_sub(cut);
        let head = keep / 3;
        let tail_start = (head + cut).min(n);
        let byte_at = |i: usize| chars.get(i).map_or(full.len(), |(b, _)| *b);
        let (cut_from, cut_to) = (byte_at(head), byte_at(tail_start));
        let mut pieces: Vec<(&Span<'_>, String)> = Vec::new();
        let mut at = 0;
        for span in &self.spans {
            let range: Range<usize> = at..at + span.text.len();
            let mut text = String::new();
            for (offset, c) in span.text.char_indices() {
                let b = range.start + offset;
                if b < cut_from || b >= cut_to {
                    text.push(c);
                }
                if b == cut_from {
                    text.push('\u{2026}');
                }
            }
            if cut_from == full.len() && range.end == full.len() {
                text.push('\u{2026}');
            }
            at = range.end;
            pieces.push((span, text));
        }
        self.job(env, pieces.iter().map(|(span, text)| (*span, text.as_str())), f32::INFINITY)
    }
}

/// One direction run of a single-row layout, owned.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct LayoutRun {
    /// The run's bytes in the text.
    pub bytes: Range<usize>,
    /// Whether the run reads right to left.
    pub rtl: bool,
    /// Each character's start offset in the row.
    pub positions: Vec<f32>,
    /// Each character's advance.
    pub widths: Vec<f32>,
}

/// Records that the galley of `layout` was painted, and its full text when the layout cut text.
pub(crate) fn note_painted(ctx: &Context, layout: &TextLayout) {
    if layout.elided {
        crate::runtime::egui::painted::note_elided(ctx, &layout.galley, &layout.full);
    }
}

/// A laid-out run.
#[derive(Clone)]
#[non_exhaustive]
pub struct TextLayout {
    /// The galley.
    pub galley: Arc<Galley>,
    /// The layout cut text.
    pub elided: bool,
    /// The full text, as the tooltip and accessible name of an elided run.
    pub full: String,
    /// The first row's baseline, from the layout's top.
    pub first_baseline: f32,
    /// The bidi runs of a single-row layout; empty for a wrapped one.
    pub runs: Vec<LayoutRun>,
}

impl TextLayout {
    pub(crate) fn new(galley: Arc<Galley>, elided: bool, full: String) -> TextLayout {
        let first_baseline = galley.rows.first().map_or(0.0, |placed| match placed.row.glyphs.first() {
            Some(glyph) => placed.pos.y + glyph.pos.y,
            None => placed.pos.y + placed.row.size.y,
        });
        let runs = if galley.rows.len() <= 1 { Self::build_row_runs(&galley, 0) } else { Vec::new() };
        TextLayout { galley, elided, full, first_baseline, runs }
    }

    /// One run per maximal stretch of glyphs with one direction; an empty row is one empty left-to-right run.
    fn build_row_runs(galley: &Galley, row: usize) -> Vec<LayoutRun> {
        let glyphs = galley.rows.get(row).map_or(&[][..], |placed| &placed.row.glyphs[..]);
        let mut runs: Vec<LayoutRun> = Vec::new();
        let mut at = 0;
        for glyph in glyphs {
            let len = glyph.chr.len_utf8();
            match runs.last_mut() {
                Some(run) if run.rtl == glyph.is_rtl() => {
                    run.bytes.end += len;
                    run.positions.push(glyph.pos.x);
                    run.widths.push(glyph.advance_width);
                }
                _ => {
                    runs.push(LayoutRun { bytes: at..at + len, rtl: glyph.is_rtl(), positions: vec![glyph.pos.x], widths: vec![glyph.advance_width] })
                }
            }
            at += len;
        }
        if runs.is_empty() {
            runs.push(LayoutRun { bytes: 0..0, rtl: false, positions: Vec::new(), widths: Vec::new() });
        }
        runs
    }

    /// Every row as its text, its rect from the galley's origin and its bidi runs, for the AccessKit run nodes.
    pub(crate) fn row_lines(&self) -> Vec<(String, egui::Rect, Vec<LayoutRun>)> {
        (0..self.galley.rows.len())
            .map(|at| {
                let placed = &self.galley.rows[at];
                let text = placed.row.glyphs.iter().map(|glyph| glyph.chr).collect();
                (text, egui::Rect::from_min_size(placed.pos, placed.row.size), Self::build_row_runs(&self.galley, at))
            })
            .collect()
    }

    /// The galley size.
    pub fn size(&self) -> egui::Vec2 {
        self.galley.size()
    }
}

/// A chord formatted for the OS, as one text run laid out as its own left-to-right run.
#[derive(Clone, Debug, PartialEq)]
pub struct ChordText {
    text: String,
}

impl ChordText {
    /// A chord already formatted by craft-keymap's pure function over `craft_keymap::layout`.
    pub fn new(formatted: impl Into<String>) -> Self {
        ChordText { text: formatted.into() }
    }

    /// The chord as a run laid out left to right in any direction.
    pub fn run(&self) -> TextRun<'_> {
        let mut run = TextRun::new(self.text.as_str());
        run.ltr = true;
        run
    }

    /// The chord isolated as a left-to-right run (LRI ... PDI) for use inside a longer label.
    pub fn isolated(&self) -> String {
        format!("\u{2066}{}\u{2069}", self.text)
    }
}
