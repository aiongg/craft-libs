//! `TextEngine` on epaint's `Fonts`: the faces are the ones the app installs (craft's font stack
//! flattened to `FontDefinitions`), so glyphs, metrics and the atlas come out as in the egui
//! build, and a galley measured in layout is the galley the renderer draws.

use std::sync::Arc;

use craft_theme::FontSlot;
use craft_ui_rt::{
    FaceInstall, FontFace, GlyphPlace, Overflow, RichText, TextEngine, TextHandle, TextLayout, TextMetrics, TextRow, TextSpec, overflow_text,
};
use emath::{Pos2, Rect, Vec2};
use epaint::text::{FontDefinitions, FontFamily, FontId, Fonts, Galley, LayoutJob, TextFormat, TextOptions, TextWrapping};
use epaint::{Color32, ImageDelta};

/// The family name craft's font stack registers for the semibold face.
pub const SEMIBOLD_FAMILY: &str = "semibold";
/// The family name craft's font stack registers for the condensed tab-title face.
pub const CONDENSED_FAMILY: &str = "tab-title";

/// What a `TextHandle` holds: the galley, the text its character indices refer to, and what is
/// needed to shape it again when the glyph atlas was reset since.
pub(crate) struct Shaped {
    pub galley: Arc<Galley>,
    text: Arc<str>,
    source: Source,
    scale: f32,
    /// The atlas generation the galley's glyph coordinates belong to.
    generation: u64,
}

/// What a layout was shaped from, to shape it again after an atlas reset.
#[derive(Clone)]
enum Source {
    Plain(TextSpec),
    Rich(RichText),
}

fn color(c: craft_theme::Rgba) -> Color32 {
    let [r, g, b, a] = c.0;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

impl Shaped {
    /// The character index of byte `index` of the text.
    pub(crate) fn char_index(&self, index: usize) -> usize {
        let index = self.text.floor_char_boundary(index.min(self.text.len()));
        self.text[..index].chars().count()
    }
}

pub struct EpaintText {
    fonts: Fonts,
    semibold: FontFamily,
    condensed: FontFamily,
    /// The atlas generations of the `Fonts` an installed face replaced, so a generation never
    /// repeats across them.
    generation_base: u64,
}

impl EpaintText {
    /// `definitions` hold the app's faces; the families named [`SEMIBOLD_FAMILY`] and
    /// [`CONDENSED_FAMILY`] serve the semibold and condensed slots when present.
    pub fn new(definitions: FontDefinitions, options: TextOptions) -> EpaintText {
        let has = |name: &str| definitions.families.contains_key(&FontFamily::Name(name.into()));
        let semibold = if has(SEMIBOLD_FAMILY) { FontFamily::Name(SEMIBOLD_FAMILY.into()) } else { FontFamily::Proportional };
        let condensed = if has(CONDENSED_FAMILY) { FontFamily::Name(CONDENSED_FAMILY.into()) } else { semibold.clone() };
        EpaintText { fonts: Fonts::new(options, definitions), semibold, condensed, generation_base: 0 }
    }

    fn family(&self, slot: FontSlot) -> FontFamily {
        match slot {
            FontSlot::Ui => FontFamily::Proportional,
            FontSlot::UiSemibold => self.semibold.clone(),
            FontSlot::Mono => FontFamily::Monospace,
            FontSlot::UiCondensed => self.condensed.clone(),
        }
    }

    /// Counts the times the glyph atlas was cleared because it was full. Layouts made under an
    /// older generation hold stale glyph coordinates; the renderer shapes those again when it
    /// draws them (`Shaped::generation`), and callers that cache galleys of their own compare it.
    pub fn atlas_generation(&self) -> u64 {
        self.generation_base + self.fonts.atlas_generation()
    }

    pub(crate) fn atlas_size(&self) -> [usize; 2] {
        self.fonts.texture_atlas().size()
    }

    /// The glyph atlas change since the last call; the renderer uploads it before drawing.
    pub(crate) fn atlas_delta(&mut self) -> Option<ImageDelta> {
        self.fonts.font_image_delta()
    }

    /// Flushes the galley cache and handles a full atlas. Called once per frame, between layout
    /// and drawing.
    pub(crate) fn begin_pass(&mut self) {
        let options = *self.fonts.options();
        self.fonts.begin_pass(options, epaint::text::ViewportKey::default());
    }

    pub(crate) fn tessellator(&self, scale: f32, options: epaint::TessellationOptions) -> epaint::Tessellator {
        let atlas = self.fonts.texture_atlas();
        epaint::Tessellator::new(scale, options, atlas.size(), atlas.prepared_discs())
    }

    fn shape(&mut self, spec: &TextSpec, scale: f32) -> Shaped {
        let [r, g, b, a] = spec.color.0;
        let mut format = TextFormat::simple(FontId::new(spec.font.size, self.family(spec.font.slot)), Color32::from_rgba_unmultiplied(r, g, b, a));
        let wraps = spec.overflow == Overflow::Wrap;
        let max_width = if wraps { spec.max_width.unwrap_or(f32::INFINITY) } else { f32::INFINITY };
        let generation = self.atlas_generation();
        let mut fonts = self.fonts.with_pixels_per_point(scale);
        // A run without its own line height takes the font's row height rounded up to whole
        // points, as craft's text layout sets it (`text_layout::layout_with`).
        format.line_height = Some(spec.line_height.unwrap_or_else(|| fonts.row_height(&format.font_id).ceil()));
        let mut layout = |text: &str| {
            let mut job = LayoutJob::single_section(text.to_owned(), format.clone());
            job.wrap.max_width = max_width;
            fonts.layout_job(job)
        };
        let text = overflow_text(spec, |t| layout(t).rect.width());
        let galley = layout(&text);
        Shaped { galley, text, source: Source::Plain(spec.clone()), scale, generation }
    }

    fn face(&self, face: &FontFace) -> FontFamily {
        match face {
            FontFace::Slot(slot) => self.family(*slot),
            FontFace::Family(name) => FontFamily::Name(name.as_ref().into()),
        }
    }

    /// The job craft's text runs lay out: one section per format, rows at the left edge (then
    /// placed by `row_align`), the paragraph direction for bidi.
    fn shape_rich(&mut self, rich: &RichText, scale: f32) -> Shaped {
        let generation = self.atlas_generation();
        let sections: Vec<(std::ops::Range<usize>, f32, TextFormat)> = rich
            .sections
            .iter()
            .map(|section| {
                let f = &section.format;
                let mut format = TextFormat {
                    font_id: FontId::new(f.size, self.face(&f.face)),
                    color: color(f.color),
                    background: color(f.background),
                    italics: f.italic,
                    extra_letter_spacing: f.letter_spacing,
                    line_height: f.line_height,
                    ..TextFormat::default()
                };
                if let Some(line) = f.underline {
                    format.underline = epaint::Stroke::new(line.width, color(line.color));
                }
                if let Some(line) = f.strikethrough {
                    format.strikethrough = epaint::Stroke::new(line.width, color(line.color));
                }
                for &(tag, value) in &f.axes {
                    format.coords.push(tag, value);
                }
                if !f.features.is_empty() {
                    format.font_features = epaint::text::FontFeatures::new(f.features.iter().copied());
                }
                (section.bytes.clone(), section.indent, format)
            })
            .collect();
        let mut fonts = self.fonts.with_pixels_per_point(scale);
        let mut job = LayoutJob {
            text: rich.text.to_string(),
            direction: if rich.rtl { epaint::text::TextDirection::RightToLeft } else { epaint::text::TextDirection::LeftToRight },
            halign: emath::Align::LEFT,
            halign_min_is_start: false,
            wrap: TextWrapping {
                max_width: rich.wrap.max_width,
                max_rows: rich.wrap.max_rows,
                break_anywhere: rich.wrap.break_anywhere,
                overflow_character: rich.wrap.overflow,
            },
            ..LayoutJob::default()
        };
        for (bytes, indent, mut format) in sections {
            // A section without its own line height takes its face's row height rounded up to whole
            // points, so a wrapped text keeps its height at every scale.
            if format.line_height.is_none() {
                format.line_height = Some(fonts.row_height(&format.font_id).ceil());
            }
            job.sections.push(epaint::text::LayoutSection {
                leading_space: indent,
                byte_range: epaint::text::ByteIndex(bytes.start)..epaint::text::ByteIndex(bytes.end),
                format,
            });
        }
        let galley = aligned_rows(anchored_at_start(fonts.layout_job(job)), rich.row_align, scale);
        let text: Arc<str> = galley.job.text.as_str().into();
        Shaped { galley, text, source: Source::Rich(rich.clone()), scale, generation }
    }

    /// `shaped` shaped again when the atlas was cleared since it was made, `None` while it is
    /// current. Metrics are the same, only the glyph coordinates differ.
    pub(crate) fn reshaped(&mut self, shaped: &Shaped) -> Option<Shaped> {
        (shaped.generation != self.atlas_generation()).then(|| match &shaped.source {
            Source::Plain(spec) => self.shape(spec, shaped.scale),
            Source::Rich(rich) => self.shape_rich(rich, shaped.scale),
        })
    }
}

/// The galley moved so its rect starts at x = 0, so every layout is placed by its top-left corner.
fn anchored_at_start(mut galley: Arc<Galley>) -> Arc<Galley> {
    let shift = -galley.rect.min.x;
    if shift == 0.0 {
        return galley;
    }
    let g = Arc::make_mut(&mut galley);
    for placed in &mut g.rows {
        placed.pos.x += shift;
    }
    g.rect = g.rect.translate(Vec2::new(shift, 0.0));
    g.mesh_bounds = g.mesh_bounds.translate(Vec2::new(shift, 0.0));
    galley
}

/// Each row placed within the widest by `share` of its free room, rounded to whole pixels.
fn aligned_rows(mut galley: Arc<Galley>, share: f32, scale: f32) -> Arc<Galley> {
    let width = galley.rect.width();
    if share == 0.0 || galley.rows.iter().all(|placed| placed.size.x >= width) {
        return galley;
    }
    let g = Arc::make_mut(&mut galley);
    for placed in &mut g.rows {
        let free = (width - placed.size.x).max(0.0);
        placed.pos.x = (free * share * scale).round() / scale;
    }
    galley
}

fn layout_of(shaped: Shaped) -> TextLayout {
    let size = shaped.galley.rect.size();
    let baseline = shaped.galley.rows.first().map_or(0.0, |row| {
        let glyph_y = row.glyphs.first().map_or(row.size.y, |g| g.pos.y);
        row.pos.y + glyph_y
    });
    let lines = shaped.galley.rows.len();
    TextLayout { handle: TextHandle::new(Arc::new(shaped)), size, baseline, lines }
}

fn shaped(layout: &TextLayout) -> Option<&Shaped> {
    layout.handle.downcast_ref::<Shaped>()
}

impl TextEngine for EpaintText {
    fn layout(&mut self, spec: &TextSpec, scale: f32) -> TextLayout {
        layout_of(self.shape(spec, scale))
    }

    fn layout_rich(&mut self, text: &RichText, scale: f32) -> TextLayout {
        layout_of(self.shape_rich(text, scale))
    }

    /// Makes the fonts again with the face added: epaint takes faces only through its
    /// definitions. Layouts shaped before carry an older atlas generation and are shaped again
    /// when drawn.
    fn install_face(&mut self, face: FaceInstall) -> bool {
        if skrifa::FontRef::from_index(&face.bytes, face.index).is_err() {
            return false;
        }
        let mut definitions = self.fonts.definitions().clone();
        let name = face.family.to_string();
        let blob: epaint::text::Blob = Arc::new(Arc::clone(&face.bytes));
        let tweak = epaint::text::FontTweak { coords: epaint::text::VariationCoords::new(face.coords.iter().copied()), ..Default::default() };
        definitions.font_data.insert(name.clone(), Arc::new(epaint::text::FontData::from_blob(blob, face.index).tweak(tweak)));
        let mut list = vec![name.clone()];
        for fallback in &face.fallback {
            let family = self.face(fallback);
            for font in definitions.families.get(&family).into_iter().flatten() {
                if !list.contains(font) {
                    list.push(font.clone());
                }
            }
        }
        definitions.families.insert(FontFamily::Name(name.into()), list);
        let options = *self.fonts.options();
        self.generation_base += self.fonts.atlas_generation() + 1;
        self.fonts = Fonts::new(options, definitions);
        true
    }

    fn has_face(&self, family: &str) -> bool {
        self.fonts.definitions().families.contains_key(&FontFamily::Name(family.into()))
    }

    fn metrics(&mut self, layout: &TextLayout) -> TextMetrics {
        let Some(s) = shaped(layout) else {
            return TextMetrics { text: Arc::from(""), rows: Vec::new(), ink: Rect::NOTHING, elided: false };
        };
        let rows = s
            .galley
            .rows
            .iter()
            .map(|placed| TextRow {
                rect: Rect::from_min_size(placed.pos, placed.row.size),
                glyphs: placed.row.glyphs.iter().map(|g| GlyphPlace { chr: g.chr, x: g.pos.x, advance: g.advance_width, rtl: g.is_rtl() }).collect(),
            })
            .collect();
        let ink = if s.galley.mesh_bounds.is_finite() { s.galley.mesh_bounds } else { Rect::NOTHING };
        TextMetrics { text: s.text.clone(), rows, ink, elided: s.galley.elided }
    }

    fn hit_index(&mut self, layout: &TextLayout, pos: Pos2) -> usize {
        let Some(s) = shaped(layout) else { return 0 };
        let cursor = s.galley.cursor_from_pos(Vec2::new(pos.x, pos.y));
        s.text.char_indices().nth(cursor.index.0).map_or(s.text.len(), |(i, _)| i)
    }

    fn caret_rect(&mut self, layout: &TextLayout, index: usize) -> Rect {
        let Some(s) = shaped(layout) else {
            return Rect::ZERO;
        };
        s.galley.pos_from_cursor(epaint::text::cursor::CCursor::new(s.char_index(index)))
    }
}
