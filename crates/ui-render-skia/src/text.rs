//! `TextEngine` on Skia's paragraph layout (skparagraph: HarfBuzz shaping, ICU line breaking).
//!
//! Faces come from font bytes: the suite's Inter is embedded, and every other face is registered
//! with [`SkiaText::register_face`]. A text style names its family first and then the registered
//! fallback families in order; skparagraph resolves each glyph against that list, so scripts the
//! interface face lacks (CJK, Arabic) use the fallback faces the app registered. Anything still
//! missing goes to the system's font manager ([`SkiaText::new`]) or stays unresolved
//! ([`SkiaText::isolated`], for tests that must not depend on the machine's fonts).
//!
//! A stack's faces are registered at their variation coordinates. Its sources are asked before a
//! text is shaped, once per family and character no face of the family maps, and a face they find
//! is registered and appended to the family, so the character sets in it in the same layout. Draw
//! tweaks (scale, offsets) are not applied: skparagraph sets every face at the text size.
//!
//! A `Paragraph` is neither `Send` nor `Sync` in skia-safe, and a `TextHandle` must be both, so the
//! paragraph lives only during `layout`: its glyph runs become one `TextBlob` (immutable and
//! thread-safe in Skia), and each character's box is kept for hit testing and carets.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;

use craft_theme::{FontSlot, Rgba};
use craft_ui_rt::{
    FaceInstall, FallbackAsk, FontFace as StackFamily, FontStack, Overflow, StackFace, TextEngine, TextHandle, TextLayout, TextSpec, overflow_text,
};
use emath::{Pos2, Rect, pos2, vec2};
use skia_safe::font_arguments::{VariationPosition, variation_position::Coordinate};
use skia_safe::font_style::{Slant, Weight, Width};
use skia_safe::textlayout::{
    FontCollection, Paragraph, ParagraphBuilder, ParagraphStyle, RectHeightStyle, RectWidthStyle, TextDirection, TextStyle, TypefaceFontProvider,
};
use skia_safe::{Data, FontArguments, FontMgr, FontStyle, FourByteTag, TextBlob, TextBlobBuilder, Typeface};

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
    /// The faces `install_face` added, registered again over every new stack.
    installed: Vec<FaceInstall>,
    /// The stack `set_stack` gave last, and how many it gave. With a stack, each slot sets in its
    /// family's faces, each registered under its own name.
    stack: Option<FontStack>,
    stack_generation: u64,
    /// The typefaces registered by name: the stack's faces and those its sources found.
    typefaces: HashMap<String, Typeface>,
    /// The faces the stack's sources found, per family, after the family's own.
    found: HashMap<StackFamily, Vec<String>>,
    /// The families and characters the sources were asked for under this stack.
    asked: HashSet<(StackFamily, char)>,
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
            installed: Vec::new(),
            stack: None,
            stack_generation: 0,
            typefaces: HashMap::new(),
            found: HashMap::new(),
            asked: HashSet::new(),
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
        self.add_at(family, bytes, 0);
    }

    fn add_at(&mut self, family: &str, bytes: &[u8], index: usize) -> bool {
        match self.mgr.new_from_data(Data::new_copy(bytes), Some(index)) {
            Some(face) => {
                self.provider.register_typeface(face, family);
                true
            }
            None => {
                log::error!("skia could not read the {family} face");
                false
            }
        }
    }

    /// Registers `face` under its name at its variation coordinates.
    fn add_stack_face(&mut self, face: &StackFace) -> bool {
        let Some(typeface) = self.mgr.new_from_data(Data::new_copy(face.data()), Some(face.index as usize)) else {
            log::error!("skia could not read the {} face", face.name);
            return false;
        };
        let coordinates: Vec<Coordinate> =
            face.coords.iter().map(|(tag, value)| Coordinate { axis: FourByteTag::new(u32::from_be_bytes(*tag)), value: *value }).collect();
        let typeface = if coordinates.is_empty() {
            typeface
        } else {
            let arguments = FontArguments::new().set_variation_design_position(VariationPosition { coordinates: &coordinates });
            typeface.clone_with_arguments(&arguments).unwrap_or(typeface)
        };
        self.provider.register_typeface(typeface.clone(), face.name.as_ref());
        self.typefaces.insert(face.name.to_string(), typeface);
        true
    }

    /// Registers again every face of the stack, those its sources found and the installed faces,
    /// in a new provider.
    fn register_all(&mut self) {
        self.provider = TypefaceFontProvider::new();
        self.typefaces.clear();
        let mut faces: Vec<StackFace> = Vec::new();
        if let Some(stack) = &self.stack {
            for (family, own) in &stack.families {
                faces.extend(own.iter().cloned());
                faces.extend(stack.source_faces(family));
            }
        }
        for face in faces {
            if !self.typefaces.contains_key(face.name.as_ref()) {
                self.add_stack_face(&face);
            }
        }
        for face in std::mem::take(&mut self.installed) {
            self.add_at(&face.family, &face.bytes, face.index as usize);
            self.installed.push(face);
        }
        self.sync();
    }

    /// Asks the stack's sources for each character of `text` that no face of `slot`'s family maps,
    /// once per family and character, and registers what they find.
    fn find_fallbacks(&mut self, slot: FontSlot, text: &str) {
        let sources = self.stack.as_ref().map(|stack| stack.fallback.clone()).unwrap_or_default();
        if sources.is_empty() {
            return;
        }
        let family = StackFamily::Slot(slot);
        let mut names: Option<Vec<String>> = None;
        let mut added = false;
        for (at, c) in text.char_indices() {
            if c.is_whitespace() || c.is_control() || self.asked.contains(&(family.clone(), c)) {
                continue;
            }
            let names = names.get_or_insert_with(|| self.families(slot).into_iter().map(str::to_owned).collect());
            let covered = names.iter().filter_map(|name| self.typefaces.get(name)).any(|typeface| typeface.unichar_to_glyph(c as i32) != 0);
            if covered {
                continue;
            }
            self.asked.insert((family.clone(), c));
            let end = text[at..].char_indices().nth(1).map_or(text.len(), |(next, _)| at + next);
            let ask = FallbackAsk { family: &family, cluster: &text[at..end] };
            let Some(face) = sources.iter().find_map(|source| source.face_for(&ask)) else { continue };
            if !self.typefaces.contains_key(face.name.as_ref()) && !self.add_stack_face(&face) {
                continue;
            }
            let found = self.found.entry(family.clone()).or_default();
            if !found.iter().any(|name| **name == *face.name) {
                found.push(face.name.to_string());
                names.push(face.name.to_string());
                added = true;
            }
        }
        if added {
            self.sync();
        }
    }

    fn sync(&mut self) {
        self.fonts.set_asset_font_manager(Some(self.provider.clone().into()));
    }

    /// The faces of the stack family that sets `slot`: its own, else the next lighter slot's.
    fn stack_faces(&self, slot: FontSlot) -> Option<Vec<&str>> {
        let stack = self.stack.as_ref()?;
        let chain: &[FontSlot] = match slot {
            FontSlot::Ui => &[FontSlot::Ui],
            FontSlot::Mono => &[FontSlot::Mono],
            FontSlot::UiSemibold => &[FontSlot::UiSemibold, FontSlot::Ui],
            FontSlot::UiCondensed => &[FontSlot::UiCondensed, FontSlot::UiSemibold, FontSlot::Ui],
        };
        let faces = chain.iter().find_map(|slot| stack.family(&StackFamily::Slot(*slot)))?;
        Some(faces.iter().map(|face| face.name.as_ref()).collect())
    }

    fn families(&self, slot: FontSlot) -> Vec<&str> {
        if let Some(faces) = self.stack_faces(slot) {
            let found = self.found.get(&StackFamily::Slot(slot)).into_iter().flatten().map(String::as_str);
            return faces.into_iter().chain(found).chain(self.fallback.iter().map(String::as_str)).collect();
        }
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
    fn install_face(&mut self, face: FaceInstall) -> bool {
        if !self.add_at(&face.family, &face.bytes, face.index as usize) {
            return false;
        }
        self.sync();
        self.installed.retain(|kept| kept.family != face.family);
        self.installed.push(face);
        true
    }

    fn has_face(&self, family: &str) -> bool {
        self.installed.iter().any(|face| &*face.family == family)
    }

    /// Registers the faces again without the family: Skia's provider takes no face out.
    fn remove_face(&mut self, family: &str) -> bool {
        let before = self.installed.len();
        self.installed.retain(|face| &*face.family != family);
        let removed = self.installed.len() != before;
        if removed {
            self.register_all();
        }
        removed
    }

    /// Registers every face of `stack` under its own name at its variation coordinates, in place
    /// of the faces registered before, and the installed faces again. The sources' answers of the
    /// stack before are forgotten, so they are asked again.
    fn set_stack(&mut self, stack: &FontStack) {
        self.stack = Some(stack.clone());
        self.found.clear();
        self.asked.clear();
        self.register_all();
        self.stack_generation += 1;
    }

    fn stack_generation(&self) -> u64 {
        self.stack_generation
    }

    fn stack(&self) -> Option<&FontStack> {
        self.stack.as_ref()
    }

    /// Layouts are in logical points at every scale: Skia rasterises glyphs under the canvas
    /// matrix at the target's pixel size, so the scale changes no metric here.
    fn layout(&mut self, spec: &TextSpec, _scale: f32) -> TextLayout {
        self.find_fallbacks(spec.font.slot, &spec.text);
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
