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

/// How a [`FontStack`] face draws: adjustments of its glyphs that leave the layout's metrics alone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceTweak {
    /// Glyphs drawn at this many times the text size.
    pub scale: f32,
    /// Glyphs moved down by this share of the text size.
    pub y_offset_factor: f32,
    /// Glyphs moved down by this many logical points.
    pub y_offset: f32,
    /// Where the face is a fallback (not its family's first face), its glyphs sit on the row's
    /// baseline; centred in the row otherwise (emoji and symbols).
    pub baseline_fallback: bool,
}

impl Default for FaceTweak {
    fn default() -> FaceTweak {
        FaceTweak { scale: 1.0, y_offset_factor: 0.0, y_offset: 0.0, baseline_fallback: false }
    }
}

/// A font file's bytes, shared without copying: a file read at run time, a face embedded in the
/// binary (`Arc::new(bytes)` over a `&'static [u8]`) or a system font's mapped file.
pub type FontBytes = Arc<dyn AsRef<[u8]> + Send + Sync>;

/// One face of a [`FontStack`] family.
#[derive(Clone)]
pub struct StackFace {
    /// The face's name: a face listed by several families has the same name in each, and the
    /// engine reads its file once. A face set at another weight or tweak has a name of its own.
    pub name: Arc<str>,
    /// The font file.
    pub bytes: FontBytes,
    /// The face's index in the file.
    pub index: u32,
    /// The variation coordinates the face is drawn at (`wght`, `opsz`).
    pub coords: Vec<([u8; 4], f32)>,
    pub tweak: FaceTweak,
}

impl StackFace {
    /// Face `index` of `bytes`, at its default coordinates, untweaked.
    pub fn new(name: impl Into<Arc<str>>, bytes: FontBytes, index: u32) -> StackFace {
        StackFace { name: name.into(), bytes, index, coords: Vec::new(), tweak: FaceTweak::default() }
    }

    /// Face `index` of a file embedded in the binary, shared without a copy.
    pub fn from_static(name: impl Into<Arc<str>>, bytes: &'static [u8], index: u32) -> StackFace {
        StackFace::new(name, Arc::new(bytes), index)
    }

    /// The font file's bytes.
    pub fn data(&self) -> &[u8] {
        (*self.bytes).as_ref()
    }

    /// The face under `name`, the rest unchanged.
    #[must_use]
    pub fn named(mut self, name: impl Into<Arc<str>>) -> StackFace {
        self.name = name.into();
        self
    }

    /// The coordinate set for `tag`, if any.
    pub fn coord(&self, tag: [u8; 4]) -> Option<f32> {
        self.coords.iter().find(|(t, _)| *t == tag).map(|(_, value)| *value)
    }
}

impl std::fmt::Debug for StackFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StackFace")
            .field("name", &self.name)
            .field("bytes", &self.data().len())
            .field("index", &self.index)
            .field("coords", &self.coords)
            .field("tweak", &self.tweak)
            .finish()
    }
}

/// What a [`FaceSource`] is asked when no face of a family has a character.
#[derive(Clone, Copy, Debug)]
pub struct FallbackAsk<'a> {
    /// The family the text is set in.
    pub family: &'a FontFace,
    /// The grapheme cluster that starts with the character: the face returned must have a glyph for
    /// its first character, and should have the cluster's other visible characters (a variation
    /// selector picks between emoji and symbol faces).
    pub cluster: &'a str,
}

/// Faces a [`FontStack`] finds while text is shaped, after its families' own faces: the installed
/// system fonts, faces fetched or read when a character first needs them, faces kept for scripts
/// beyond the interface's. Engines ask the sources of a stack in order during shaping, so a face a
/// source finds sets the character in the same layout.
pub trait FaceSource: Send + Sync {
    /// Faces to put after `family`'s own, up front. Asked once per family per stack.
    fn faces_for(&self, family: &FontFace) -> Vec<StackFace> {
        let _ = family;
        Vec::new()
    }

    /// A face with a glyph for the first character of `ask.cluster`, after every face of the family
    /// (its own and those found before) missed it. Asked at most once per family and character per
    /// stack generation, also when it answers `None`: a source that finds the face later (a fetch, a
    /// file read in the background) has the host set the stack again
    /// ([`TextEngine::set_stack`]), which asks again. The face is appended to the family.
    fn face_for(&self, ask: &FallbackAsk<'_>) -> Option<StackFace> {
        let _ = ask;
        None
    }
}

/// The faces text is set in, for every renderer: each family's faces in the order a character
/// falls back through them, then the sources asked for a character none of them has. The theme's
/// slots ([`FontFace::Slot`]) and the families an app names ([`FontFace::Family`]) are families
/// alike. An engine takes it with [`TextEngine::set_stack`].
#[derive(Clone, Default)]
pub struct FontStack {
    pub families: Vec<(FontFace, Vec<StackFace>)>,
    /// Asked in order after a family's faces, per family and character.
    pub fallback: Vec<Arc<dyn FaceSource>>,
}

impl std::fmt::Debug for FontStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontStack").field("families", &self.families).field("fallback", &self.fallback.len()).finish()
    }
}

impl FontStack {
    /// The faces of `family`, `None` when the stack has no such family.
    pub fn family(&self, family: &FontFace) -> Option<&[StackFace]> {
        self.families.iter().find(|(f, _)| f == family).map(|(_, faces)| faces.as_slice())
    }

    /// Sets the faces of `family`, in place of the ones it had.
    pub fn set_family(&mut self, family: FontFace, faces: Vec<StackFace>) {
        match self.families.iter_mut().find(|(f, _)| *f == family) {
            Some((_, kept)) => *kept = faces,
            None => self.families.push((family, faces)),
        }
    }

    /// The faces every source puts after `family`'s own, in source order.
    pub fn source_faces(&self, family: &FontFace) -> Vec<StackFace> {
        self.fallback.iter().flat_map(|source| source.faces_for(family)).collect()
    }

    /// The first source's face for `ask`, the sources asked in order.
    pub fn find_fallback(&self, ask: &FallbackAsk<'_>) -> Option<StackFace> {
        self.fallback.iter().find_map(|source| source.face_for(ask))
    }
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

    /// Takes out the family `family` that [`install_face`](Self::install_face) added, to free its
    /// file; text set in it from the next layout on falls back as an unknown family does. Layouts
    /// made before keep their shaping, and the stack generation does not move: nothing still shows
    /// the family when its owner takes it out. Returns whether the engine had it.
    fn remove_face(&mut self, family: &str) -> bool {
        let _ = family;
        false
    }

    /// Sets text in `stack` from the next layout on, in place of the faces set before; the faces
    /// [`install_face`](Self::install_face) added stay. Layouts made before keep the old faces'
    /// shaping, so every call moves [`stack_generation`](Self::stack_generation) and the runtime
    /// lays out again what laid text out. An engine with fixed faces ignores it.
    fn set_stack(&mut self, stack: &FontStack) {
        let _ = stack;
    }

    /// Moves by one at each [`set_stack`](Self::set_stack); 0 before the first.
    fn stack_generation(&self) -> u64 {
        0
    }

    /// The stack [`set_stack`](Self::set_stack) gave last, `None` before the first.
    fn stack(&self) -> Option<&FontStack> {
        None
    }

    /// The rows, glyphs and ink of `layout`. An engine without glyph metrics gives one row of the
    /// layout's size and no glyphs.
    fn metrics(&mut self, layout: &TextLayout) -> TextMetrics {
        let rect = Rect::from_min_size(Pos2::ZERO, layout.size);
        TextMetrics { text: Arc::from(""), rows: vec![TextRow { rect, glyphs: Vec::new() }], ink: rect, elided: false }
    }
}
