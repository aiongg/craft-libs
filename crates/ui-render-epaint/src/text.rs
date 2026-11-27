//! `TextEngine` on epaint's `Fonts`: the faces are the ones the app installs (craft's font stack
//! flattened to `FontDefinitions`), so glyphs, metrics and the atlas come out as in the egui
//! build, and a galley measured in layout is the galley the renderer draws.

use std::sync::Arc;

use craft_theme::FontSlot;
use craft_ui_rt::{Overflow, TextEngine, TextHandle, TextLayout, TextSpec, overflow_text};
use emath::{Pos2, Rect, Vec2};
use epaint::text::{FontDefinitions, FontFamily, FontId, Fonts, Galley, LayoutJob, TextFormat, TextOptions};
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
    spec: TextSpec,
    scale: f32,
    /// The atlas generation the galley's glyph coordinates belong to.
    generation: u64,
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
}

impl EpaintText {
    /// `definitions` hold the app's faces; the families named [`SEMIBOLD_FAMILY`] and
    /// [`CONDENSED_FAMILY`] serve the semibold and condensed slots when present.
    pub fn new(definitions: FontDefinitions, options: TextOptions) -> EpaintText {
        let has = |name: &str| definitions.families.contains_key(&FontFamily::Name(name.into()));
        let semibold = if has(SEMIBOLD_FAMILY) { FontFamily::Name(SEMIBOLD_FAMILY.into()) } else { FontFamily::Proportional };
        let condensed = if has(CONDENSED_FAMILY) { FontFamily::Name(CONDENSED_FAMILY.into()) } else { semibold.clone() };
        EpaintText { fonts: Fonts::new(options, definitions), semibold, condensed }
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
        self.fonts.atlas_generation()
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
        let generation = self.fonts.atlas_generation();
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
        Shaped { galley, text, spec: spec.clone(), scale, generation }
    }

    /// `shaped` shaped again when the atlas was cleared since it was made, `None` while it is
    /// current. Metrics are the same, only the glyph coordinates differ.
    pub(crate) fn reshaped(&mut self, shaped: &Shaped) -> Option<Shaped> {
        (shaped.generation != self.fonts.atlas_generation()).then(|| self.shape(&shaped.spec, shaped.scale))
    }
}

fn shaped(layout: &TextLayout) -> Option<&Shaped> {
    layout.handle.downcast_ref::<Shaped>()
}

impl TextEngine for EpaintText {
    fn layout(&mut self, spec: &TextSpec, scale: f32) -> TextLayout {
        let shaped = self.shape(spec, scale);
        let size = shaped.galley.rect.size();
        let baseline = shaped.galley.rows.first().map_or(0.0, |row| {
            let glyph_y = row.glyphs.first().map_or(row.size.y, |g| g.pos.y);
            row.pos.y + glyph_y
        });
        let lines = shaped.galley.rows.len();
        TextLayout { handle: TextHandle::new(Arc::new(shaped)), size, baseline, lines }
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
