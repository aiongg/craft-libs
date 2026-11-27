//! `TextEngine` on Skia's paragraph layout (skparagraph: HarfBuzz shaping, ICU line breaking).
//!
//! Faces come from font bytes: the suite's Inter is embedded, and every other face is registered
//! with [`SkiaText::register_face`]. A text style names its family first and then the registered
//! fallback families in order; skparagraph resolves each glyph against that list, so scripts the
//! interface face lacks (CJK, Arabic) use the fallback faces the app registered. Anything still
//! missing goes to the system's font manager ([`SkiaText::new`]) or stays unresolved
//! ([`SkiaText::isolated`], for tests that must not depend on the machine's fonts).
//!
//! A `Paragraph` is neither `Send` nor `Sync` in skia-safe, and a `TextHandle` must be both, so the
//! paragraph lives only during `layout`: its glyph runs become one `TextBlob` (immutable and
//! thread-safe in Skia), and each character's box is kept for hit testing and carets.

use std::ops::Range;
use std::sync::Arc;

use craft_theme::{FontSlot, Rgba};
use craft_ui_rt::{Overflow, TextEngine, TextHandle, TextLayout, TextSpec, overflow_text};
use emath::{Pos2, Rect, pos2, vec2};
use skia_safe::font_style::{Slant, Weight, Width};
use skia_safe::textlayout::{
    FontCollection, Paragraph, ParagraphBuilder, ParagraphStyle, RectHeightStyle, RectWidthStyle, TextDirection, TextStyle, TypefaceFontProvider,
};
use skia_safe::{Data, FontMgr, FontStyle, TextBlob, TextBlobBuilder};

// The suite's interface face: the bytes craft-ui-contract ships (`fonts/inter`). Other faces (the
// monospace face, scripts Inter lacks) come from the fonts set through `register_face`.
const INTER_REGULAR: &[u8] = include_bytes!("../../ui-contract/fonts/inter/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../ui-contract/fonts/inter/Inter-SemiBold.ttf");

const UI_FAMILY: &str = "Inter";
/// The monospace slot: a registered "Hack" first, else the system's monospace family.
const MONO_FAMILIES: [&str; 2] = ["Hack", "monospace"];

/// A font file to register with the engine under a family name.
#[derive(Clone)]
pub struct FontFace {
    pub family: String,
    pub bytes: Arc<[u8]>,
}

/// One character's box in layout coordinates (logical points), with the byte where it starts.
#[derive(Clone, Copy, Debug)]
struct CharBox {
    start: usize,
    rect: Rect,
    /// The character runs right to left.
    rtl: bool,
}

/// What a `TextHandle` holds. Glyph positions are in logical points; the renderer draws the blob
/// under the frame's scale, so glyphs rasterise at the target's pixel size.
pub(crate) struct Shaped {
    pub blob: Option<TextBlob>,
    pub color: Rgba,
    text_len: usize,
    boxes: Vec<CharBox>,
    height: f32,
    unresolved: usize,
}

pub struct SkiaText {
    fonts: FontCollection,
    provider: TypefaceFontProvider,
    mgr: FontMgr,
    /// Registered families that follow the primary family in every style, in order.
    fallback: Vec<String>,
    /// The families `install_face` added.
    installed: std::collections::HashSet<String>,
}

impl Default for SkiaText {
    fn default() -> Self {
        Self::new()
    }
}

impl SkiaText {
    /// Inter plus the machine's fonts as the last fallback (emoji and any script no registered
    /// face covers).
    pub fn new() -> SkiaText {
        let mut text = Self::isolated();
        text.fonts.set_default_font_manager(FontMgr::new(), None);
        text.fonts.enable_font_fallback();
        text
    }

    /// Inter and registered faces only: the result never depends on the machine's fonts.
    pub fn isolated() -> SkiaText {
        // FreeType without system fonts where Skia has it (Linux, Android); the platform's
        // manager elsewhere. Either only parses the bytes it is given here.
        let mgr = FontMgr::custom_empty().unwrap_or_else(FontMgr::new);
        let mut text = SkiaText {
            fonts: FontCollection::new(),
            provider: TypefaceFontProvider::new(),
            mgr,
            fallback: Vec::new(),
            installed: Default::default(),
        };
        for bytes in [INTER_REGULAR, INTER_SEMIBOLD] {
            text.add(UI_FAMILY, bytes);
        }
        text.sync();
        text.fonts.disable_font_fallback();
        text
    }

    /// Registers `face`'s bytes under its family. A later `set_fallback_families` can chain it.
    pub fn register_face(&mut self, face: &FontFace) {
        self.add(&face.family, &face.bytes);
        self.sync();
    }

    /// The families tried, in this order, for glyphs the style's own family lacks.
    pub fn set_fallback_families(&mut self, families: impl IntoIterator<Item = impl Into<String>>) {
        self.fallback = families.into_iter().map(Into::into).collect();
    }

    /// Glyphs of `layout` that no face covered (drawn as the missing-glyph box or not at all).
    pub fn unresolved_glyphs(&self, layout: &TextLayout) -> usize {
        layout.handle.downcast_ref::<Shaped>().map_or(0, |s| s.unresolved)
    }

    fn add(&mut self, family: &str, bytes: &[u8]) {
        match self.mgr.new_from_data(Data::new_copy(bytes), None) {
            Some(face) => {
                self.provider.register_typeface(face, family);
            }
            None => log::error!("skia could not read the {family} face"),
        }
    }

    fn sync(&mut self) {
        self.fonts.set_asset_font_manager(Some(self.provider.clone().into()));
    }

    fn families(&self, slot: FontSlot) -> Vec<&str> {
        let primary: &[&str] = match slot {
            FontSlot::Ui | FontSlot::UiSemibold | FontSlot::UiCondensed => &[UI_FAMILY],
            FontSlot::Mono => &MONO_FAMILIES,
        };
        primary.iter().copied().chain(self.fallback.iter().map(String::as_str)).collect()
    }

    fn paragraph(&self, spec: &TextSpec, text: &str) -> Paragraph {
        let mut style = TextStyle::new();
        style.set_font_families(&self.families(spec.font.slot));
        style.set_font_style(FontStyle::new(weight(spec.font.slot), Width::NORMAL, Slant::Upright));
        style.set_font_size(spec.font.size);
        let [r, g, b, a] = spec.color.0;
        style.set_color(skia_safe::Color::from_argb(a, r, g, b));
        if let Some(h) = spec.line_height {
            style.set_height(h / spec.font.size);
            style.set_height_override(true);
        }
        let mut para = ParagraphStyle::new();
        para.set_text_style(&style);
        // Flutter's width rounding is off: widths are the shaped advances.
        para.set_apply_rounding_hack(false);
        para.set_replace_tab_characters(true);
        let wrap = spec.overflow == Overflow::Wrap;
        if !wrap {
            para.set_max_lines(1);
        }
        let mut builder = ParagraphBuilder::new(&para, self.fonts.clone());
        builder.add_text(text);
        let mut paragraph = builder.build();
        let width = match (wrap, spec.max_width) {
            (true, Some(w)) => w,
            _ => f32::INFINITY,
        };
        paragraph.layout(width);
        paragraph
    }
}

fn weight(slot: FontSlot) -> Weight {
    match slot {
        FontSlot::Ui | FontSlot::Mono => Weight::NORMAL,
        // The condensed slot has no face of its own here and falls back to semibold.
        FontSlot::UiSemibold | FontSlot::UiCondensed => Weight::SEMI_BOLD,
    }
}

/// The width a one-line paragraph takes.
fn width(paragraph: &Paragraph) -> f32 {
    paragraph.max_intrinsic_width()
}

/// The paragraph's glyph runs as one blob, positions relative to the layout's top left.
fn blob(paragraph: &mut Paragraph) -> Option<TextBlob> {
    let mut builder = TextBlobBuilder::new();
    paragraph.visit(|_, info| {
        let Some(info) = info else { return };
        let origin = info.origin();
        let (glyphs, positions) = builder.alloc_run_pos(info.font(), info.count(), None);
        glyphs.copy_from_slice(info.glyphs());
        for (dst, src) in positions.iter_mut().zip(info.positions()) {
            *dst = *src + origin;
        }
    });
    builder.make()
}

/// Each character's box, from the UTF-16 ranges Skia's paragraph indexes by.
fn boxes(paragraph: &Paragraph, text: &str) -> Vec<CharBox> {
    let mut out = Vec::with_capacity(text.len());
    let mut u16_at = 0usize;
    for (start, ch) in text.char_indices() {
        let len = ch.len_utf16();
        let rects = paragraph.get_rects_for_range(u16_at..u16_at + len, RectHeightStyle::Max, RectWidthStyle::Tight);
        u16_at += len;
        if let Some(tb) = rects.first() {
            let r = tb.rect;
            out.push(CharBox { start, rect: Rect::from_min_max(pos2(r.left, r.top), pos2(r.right, r.bottom)), rtl: tb.direct == TextDirection::RTL });
        }
    }
    out
}

fn shaped(layout: &TextLayout) -> &Shaped {
    layout.handle.downcast_ref::<Shaped>().expect("text layout from another engine")
}

impl TextEngine for SkiaText {
    /// Registers the face under its family. Rich text is set by the trait's default here, which
    /// keeps every section in the interface face, so the family is not yet drawn.
    fn install_face(&mut self, face: craft_ui_rt::FaceInstall) -> bool {
        match self.mgr.new_from_data(Data::new_copy(&face.bytes), Some(face.index)) {
            Some(typeface) => {
                self.provider.register_typeface(typeface, face.family.as_ref());
                self.sync();
                self.installed.insert(face.family.to_string());
                true
            }
            None => false,
        }
    }

    fn has_face(&self, family: &str) -> bool {
        self.installed.contains(family)
    }

    /// Layouts are in logical points at every scale: Skia rasterises glyphs under the canvas
    /// matrix at the target's pixel size, so the scale changes no metric here.
    fn layout(&mut self, spec: &TextSpec, _scale: f32) -> TextLayout {
        let text = overflow_text(spec, |t| width(&self.paragraph(spec, t)));
        let mut paragraph = self.paragraph(spec, &text);
        let w = if spec.overflow == Overflow::Wrap && spec.max_width.is_some() { paragraph.longest_line() } else { width(&paragraph) };
        let size = vec2(w, paragraph.height());
        let baseline = paragraph.alphabetic_baseline();
        let lines = paragraph.line_number();
        let boxes = boxes(&paragraph, &text);
        let blob = blob(&mut paragraph);
        let unresolved = paragraph.unresolved_glyphs().unwrap_or(0);
        TextLayout {
            handle: TextHandle::new(Arc::new(Shaped { blob, color: spec.color, text_len: text.len(), boxes, height: size.y, unresolved })),
            size,
            baseline,
            lines,
        }
    }

    fn hit_index(&mut self, layout: &TextLayout, pos: Pos2) -> usize {
        let sh = shaped(layout);
        // The line under `pos` (the nearest one above or below the text), then the boundary
        // nearest to `pos.x` on it.
        let line_of = |b: &CharBox| (b.rect.min.y, b.rect.max.y);
        let Some(line) = sh.boxes.iter().map(line_of).min_by(|a, b| dist(pos.y, *a).total_cmp(&dist(pos.y, *b))) else {
            return 0;
        };
        // The box nearest to `pos.x` on that line; its leading half (the right half of a
        // right-to-left character) puts the caret before it, the other half after it.
        let gap = |b: &CharBox| (b.rect.min.x - pos.x).max(pos.x - b.rect.max.x).max(0.0);
        let Some(b) = sh.boxes.iter().filter(|b| line_of(b) == line).min_by(|a, b| gap(a).total_cmp(&gap(b))) else {
            return 0;
        };
        if (pos.x < b.rect.center().x) != b.rtl { b.start } else { next_start(sh, b.start) }
    }

    fn caret_rect(&mut self, layout: &TextLayout, index: usize) -> Rect {
        let sh = shaped(layout);
        // One logical point wide.
        let at = |x: f32, y0: f32, y1: f32| Rect::from_min_max(pos2(x, y0), pos2(x + 1.0, y1));
        if let Some(b) = sh.boxes.iter().find(|b| b.start >= index) {
            return at(b.rect.min.x, b.rect.min.y, b.rect.max.y);
        }
        match sh.boxes.last() {
            Some(b) => at(b.rect.max.x, b.rect.min.y, b.rect.max.y),
            None => at(0.0, 0.0, sh.height),
        }
    }
}

impl Shaped {
    /// The boxes of bytes `range` of the text: per line, from the first to the last selected
    /// character, at the line's full height.
    pub(crate) fn highlight(&self, range: Range<usize>) -> Vec<Rect> {
        let mut out: Vec<Rect> = Vec::new();
        for b in self.boxes.iter().filter(|b| range.contains(&b.start)) {
            match out.last_mut() {
                Some(r) if r.min.y == b.rect.min.y && r.max.y == b.rect.max.y => *r = r.union(b.rect),
                _ => out.push(b.rect),
            }
        }
        out
    }
}

fn dist(y: f32, (y0, y1): (f32, f32)) -> f32 {
    if y < y0 {
        y0 - y
    } else if y > y1 {
        y - y1
    } else {
        0.0
    }
}

/// The byte after the character starting at `start`.
fn next_start(sh: &Shaped, start: usize) -> usize {
    sh.boxes.iter().map(|b| b.start).find(|&s| s > start).unwrap_or(sh.text_len)
}

pub(crate) fn downcast(layout: &TextHandle) -> Option<&Shaped> {
    layout.downcast_ref::<Shaped>()
}
