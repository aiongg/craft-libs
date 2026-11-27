//! The text boundary. Shaping and metrics come with the renderer's backend; the runtime keeps
//! each node's layouts and hands them back to the renderer by handle.

use std::any::Any;
use std::ops::Range;
use std::sync::Arc;

use craft_theme::{FontSlot, Rgba};
use emath::{Pos2, Rect, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontSpec {
    pub slot: FontSlot,
    /// Size in logical points.
    pub size: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextSpec {
    pub text: Arc<str>,
    pub font: FontSpec,
    /// Line height in logical points; the font's own when `None`.
    pub line_height: Option<f32>,
    pub color: Rgba,
    /// The line's room in logical points; with [`Overflow::Wrap`] the width it wraps at. No
    /// limit when `None`.
    pub max_width: Option<f32>,
    pub overflow: Overflow,
}

/// What a text does when it is wider than `max_width`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overflow {
    /// Wraps onto further lines.
    #[default]
    Wrap,
    /// One line, cut after the last character that fits.
    Clip,
    /// One line, cut so that the most characters that fit are followed by "…".
    Ellipsis,
}

const ELLIPSIS: &str = "\u{2026}";

/// The text a one-line `spec` shows: `spec.text` when it fits `max_width` or does not cut, else
/// the longest prefix (whole characters, trailing white space dropped, at most one character
/// short of the whole) that fits, followed by the ellipsis in [`Overflow::Ellipsis`]; the
/// ellipsis alone when no prefix fits. `width` measures a candidate in logical points.
pub fn overflow_text(spec: &TextSpec, mut width: impl FnMut(&str) -> f32) -> Arc<str> {
    let suffix = match spec.overflow {
        Overflow::Wrap => return spec.text.clone(),
        Overflow::Clip => "",
        Overflow::Ellipsis => ELLIPSIS,
    };
    let Some(max) = spec.max_width else {
        return spec.text.clone();
    };
    if width(&spec.text) <= max {
        return spec.text.clone();
    }
    let text: &str = &spec.text;
    let ends: Vec<usize> = text.char_indices().map(|(at, _)| at).chain([text.len()]).collect();
    let cut = |count: usize| format!("{}{suffix}", text[..ends[count]].trim_end());
    let (mut lo, mut hi) = (0, ends.len().saturating_sub(2));
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if width(&cut(mid)) <= max {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    cut(lo).into()
}

/// The backend's shaped text. The engine that made it is the only one that reads it.
#[derive(Clone)]
pub struct TextHandle(Arc<dyn Any + Send + Sync>);

impl TextHandle {
    pub fn new(value: Arc<dyn Any + Send + Sync>) -> TextHandle {
        TextHandle(value)
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    pub fn ptr_eq(&self, other: &TextHandle) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for TextHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TextHandle({:p})", Arc::as_ptr(&self.0))
    }
}

/// A laid-out text: the handle plus the metrics layout needs. Logical points.
#[derive(Clone, Debug)]
pub struct TextLayout {
    pub handle: TextHandle,
    pub size: Vec2,
    /// Distance from the top to the first baseline.
    pub baseline: f32,
    pub lines: usize,
}

/// The face of a text section: a slot of the theme's font stack, or a family the app installed by
/// name (a weight or optical size of the interface face, a document face).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FontFace {
    Slot(FontSlot),
    Family(Arc<str>),
}

/// A font face an app installs while it runs (a document's face), for [`TextEngine::install_face`].
#[derive(Clone, Debug)]
pub struct FaceInstall {
    /// The family name text sets it with ([`FontFace::Family`]).
    pub family: Arc<str>,
    /// The font file.
    pub bytes: Arc<[u8]>,
    /// The face's index in the file.
    pub index: u32,
    /// The variation coordinates the face is drawn at.
    pub coords: Vec<([u8; 4], f32)>,
    /// The faces that draw what this one has no glyph for, in order.
    pub fallback: Vec<FontFace>,
}

/// An underline or strike-through line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLine {
    pub width: f32,
    pub color: Rgba,
}

/// How one section of a [`RichText`] is set.
#[derive(Clone, Debug, PartialEq)]
pub struct TextFormat {
    pub face: FontFace,
    /// Size in logical points.
    pub size: f32,
    /// Line height in logical points; the face's row height rounded up to whole points when `None`.
    pub line_height: Option<f32>,
    pub color: Rgba,
    /// Filled behind the section's glyphs; nothing when fully transparent.
    pub background: Rgba,
    pub italic: bool,
    pub underline: Option<TextLine>,
    pub strikethrough: Option<TextLine>,
    /// Extra space after each character, in logical points.
    pub letter_spacing: f32,
    /// OpenType features and their values (`tnum` = 1).
    pub features: Vec<([u8; 4], u32)>,
    /// Variation axis coordinates (`opsz`).
    pub axes: Vec<([u8; 4], f32)>,
}

impl TextFormat {
    pub fn new(face: FontFace, size: f32, color: Rgba) -> TextFormat {
        TextFormat {
            face,
            size,
            line_height: None,
            color,
            background: Rgba([0; 4]),
            italic: false,
            underline: None,
            strikethrough: None,
            letter_spacing: 0.0,
            features: Vec::new(),
            axes: Vec::new(),
        }
    }
}

/// A byte range of a [`RichText`] and its format.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSection {
    pub bytes: Range<usize>,
    pub format: TextFormat,
    /// Space in logical points before the section (a first-line indent).
    pub indent: f32,
}

/// Where a [`RichText`] breaks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextWrap {
    /// The width rows wrap at; `f32::INFINITY` for none.
    pub max_width: f32,
    /// Rows past this many are cut, the last one ending in `overflow`.
    pub max_rows: usize,
    /// Rows may break inside a word.
    pub break_anywhere: bool,
    /// Ends a row cut by `max_rows` or `max_width`; `None` cuts without a mark.
    pub overflow: Option<char>,
}

impl Default for TextWrap {
    fn default() -> TextWrap {
        TextWrap { max_width: f32::INFINITY, max_rows: usize::MAX, break_anywhere: false, overflow: Some('\u{2026}') }
    }
}

/// A paragraph of sections with their own faces, sizes and colours: what a UI toolkit's text
/// runs lay out (mixed weights, links, highlights, mnemonic underlines, document faces).
#[derive(Clone, Debug, PartialEq)]
pub struct RichText {
    pub text: Arc<str>,
    /// Cover `text` in order, without gaps.
    pub sections: Vec<TextSection>,
    pub wrap: TextWrap,
    /// The paragraph runs right to left.
    pub rtl: bool,
    /// The share of a row's free room (the widest row's width less its own) placed before it: 0
    /// keeps every row at the left edge, 0.5 centres, 1 right-aligns. The shift is rounded to whole
    /// pixels at the layout's scale.
    pub row_align: f32,
}

/// One glyph of a laid-out row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphPlace {
    pub chr: char,
    /// From the row's left edge.
    pub x: f32,
    pub advance: f32,
    pub rtl: bool,
}

/// One row of a laid-out text, in layout coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRow {
    pub rect: Rect,
    pub glyphs: Vec<GlyphPlace>,
}

/// The geometry of a laid-out text a toolkit reads after layout: rows and glyphs (bidi runs, row
/// hits, caret positions), the ink bounds and whether the engine cut the text.
#[derive(Clone, Debug, PartialEq)]
pub struct TextMetrics {
    /// The text as laid out: the source text, or what the engine kept of it.
    pub text: Arc<str>,
    pub rows: Vec<TextRow>,
    /// The union of the glyphs' ink; `Rect::NOTHING` when there are none.
    pub ink: Rect,
    /// The engine cut the text (`max_rows`, a cut row).
    pub elided: bool,
}

/// Text shaping and metrics, supplied by the renderer's backend (epaint `Fonts`, skparagraph), so
/// that a layout measured in the layout phase is the one the renderer draws. One engine serves
/// every window; layouts are shaped per scale, so windows at different scales shape their own.
pub trait TextEngine {
    /// Shapes `spec` for drawing at `scale` (glyph positions snap to that scale's pixels).
    fn layout(&mut self, spec: &TextSpec, scale: f32) -> TextLayout;
    /// The byte index of the caret position nearest `pos` (layout coordinates).
    fn hit_index(&mut self, layout: &TextLayout, pos: Pos2) -> usize;
    /// The caret rect before byte `index`, in layout coordinates.
    fn caret_rect(&mut self, layout: &TextLayout, index: usize) -> Rect;

    /// Shapes a paragraph of sections. An engine without rich text sets the whole text in the
    /// first section's slot, size and colour.
    fn layout_rich(&mut self, text: &RichText, scale: f32) -> TextLayout {
        let format = text.sections.first().map(|s| &s.format);
        let slot = match format.map(|f| &f.face) {
            Some(FontFace::Slot(slot)) => *slot,
            _ => FontSlot::Ui,
        };
        let overflow = if text.wrap.max_rows == 1 && text.wrap.max_width.is_finite() {
            if text.wrap.overflow.is_some() { Overflow::Ellipsis } else { Overflow::Clip }
        } else {
            Overflow::Wrap
        };
        let spec = TextSpec {
            text: text.text.clone(),
            font: FontSpec { slot, size: format.map_or(13.0, |f| f.size) },
            line_height: format.and_then(|f| f.line_height),
            color: format.map_or(Rgba([0, 0, 0, 255]), |f| f.color),
            max_width: text.wrap.max_width.is_finite().then_some(text.wrap.max_width),
            overflow,
        };
        self.layout(&spec, scale)
    }

    /// Installs `face`, so text in its family sets in it. Returns false when the engine installs no
    /// faces while it runs, or cannot read the face; text in the family then falls back as the
    /// engine sets an unknown family.
    fn install_face(&mut self, face: FaceInstall) -> bool {
        let _ = face;
        false
    }

    /// Whether the family `family` was installed ([`install_face`](Self::install_face)).
    fn has_face(&self, family: &str) -> bool {
        let _ = family;
        false
    }

    /// The rows, glyphs and ink of `layout`. An engine without glyph metrics gives one row of the
    /// layout's size and no glyphs.
    fn metrics(&mut self, layout: &TextLayout) -> TextMetrics {
        let rect = Rect::from_min_size(Pos2::ZERO, layout.size);
        TextMetrics { text: Arc::from(""), rows: vec![TextRow { rect, glyphs: Vec::new() }], ink: rect, elided: false }
    }
}
