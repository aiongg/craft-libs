//! Color Sampler points (F04.08): up to [`MAX_SAMPLERS`] numbered points pinned to canvas
//! pixels, each read in the Info panel ([`SamplerReadout`]), so a fixed place can be compared
//! instead of chasing the pointer.
//!
//! The points are per-document view state: not saved with the document, no History step, no
//! pixels changed. The app keeps one [`ColorSamplers`] per document.

use std::hash::Hash;
use std::sync::Arc;

use craft_cms::convert::CmykConverter;
use craft_theme::Srgba;
use craft_theme::TypeRole;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasTool, CanvasView, ToolAction, ToolContext, ToolCursor, ToolPointer};
use craft_ui::composites::{StatusLine, Tone};
use craft_ui::controls::Button;
use craft_ui::edit::CancelReason;
use craft_ui::localized::SharedLabels;
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use egui::{Align2, Color32, Context, Id, Painter, Stroke, Ui, vec2};

craft_ui::words! {
    /// The Color Sampler's words, English by default.
    #[non_exhaustive]
    pub struct ColorSamplerLabels {
        /// The options bar count; `{n}` placed, `{max}` the limit.
        count: "colorSampler.count" = "{n} of {max} sample points";
        clear_all: "colorSampler.clearAll" = "Clear All";
        hint: "colorSampler.hint" = "Click to place · Alt-click or drag off-canvas to remove";
        /// The error when a point is placed past the limit; `{max}` is the limit.
        limit: "colorSampler.limit" = "Color Sampler limit reached: at most {max} points per document";
        /// A readout row's heading; `{n}` is the point's number ("#1").
        point: "colorSampler.point" = "#{n}";
    }
}

/// Photoshop's limit: at most 10 points per document.
pub const MAX_SAMPLERS: usize = 10;

/// How far from a point a press still hits it: 7 screen points, in document pixels at the view's
/// zoom.
pub const SAMPLER_HIT: f32 = 7.0;

/// Why a point was not placed or moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplerError {
    /// [`MAX_SAMPLERS`] points are placed already.
    Full,
    /// The point is outside the canvas.
    Outside,
    /// No point has that index.
    NoSuchPoint,
}

/// A document's sample points, in placing order (point 1 first). Each sits on a whole pixel: a
/// position floors to the pixel it lands on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ColorSamplers {
    points: Vec<[f64; 2]>,
}

impl ColorSamplers {
    pub fn new() -> ColorSamplers {
        ColorSamplers::default()
    }

    /// The points, number `i + 1` at index `i`.
    pub fn points(&self) -> &[[f64; 2]] {
        &self.points
    }

    /// Places a point at `pos` in a canvas `size` pixels large; its index.
    // pc: crates/engine/src/cmd/sampler_cmds.rs:91 `add` (with :63 `pixel`: floor, inside the
    // bounds; the limit message at :98).
    pub fn add(&mut self, pos: [f64; 2], size: [u32; 2]) -> Result<usize, SamplerError> {
        let pixel = pixel(pos, size).ok_or(SamplerError::Outside)?;
        if self.points.len() >= MAX_SAMPLERS {
            return Err(SamplerError::Full);
        }
        self.points.push(pixel);
        Ok(self.points.len() - 1)
    }

    /// Moves point `index` to `pos`; dropped outside the canvas it is removed instead (Photoshop).
    // pc: crates/engine/src/cmd/sampler_cmds.rs:106 `move_point`; crates/ui-egui/src/tools/analysis_ui.rs:357
    // (outside: `delete`).
    pub fn move_point(&mut self, index: usize, pos: [f64; 2], size: [u32; 2]) -> Result<(), SamplerError> {
        if index >= self.points.len() {
            return Err(SamplerError::NoSuchPoint);
        }
        match pixel(pos, size) {
            Some(pixel) => self.points[index] = pixel,
            None => {
                self.points.remove(index);
            }
        }
        Ok(())
    }

    /// Removes point `index`; the later points move up one number.
    pub fn remove(&mut self, index: usize) -> Result<(), SamplerError> {
        if index >= self.points.len() {
            return Err(SamplerError::NoSuchPoint);
        }
        self.points.remove(index);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }

    /// The nearest point within `tolerance` document pixels of `at`.
    // pc: crates/ui-egui/src/tools/analysis_ui.rs:379 `nearest_sampler`.
    pub fn hit(&self, at: [f64; 2], tolerance: f64) -> Option<usize> {
        let distance = |p: &[f64; 2]| (p[0] - at[0]).hypot(p[1] - at[1]);
        self.points
            .iter()
            .enumerate()
            .filter(|(_, p)| distance(p) <= tolerance)
            .min_by(|a, b| distance(a.1).total_cmp(&distance(b.1)))
            .map(|(index, _)| index)
    }

    /// Draws every point over the canvas, whatever the tool: a reticle of radius 5 with four
    /// ticks from 6 to 8 points out, stroked 3 points black at alpha 170 then 1 point white; the
    /// number at (+7, −7), 11-point text in white over a black-alpha-180 shadow one point down
    /// right. The text size comes from `TypeRole::Caption` (the owner's font-size rule), the
    /// nearest to PhotoCraft's 11.
    // pc: crates/ui-egui/src/tools/analysis_ui.rs:468 (the marker loop).
    pub fn paint(&self, painter: &Painter, view: &CanvasView) {
        let tokens = craft_ui::theme::current(painter.ctx());
        let sampler = &tokens.component.sampler;
        let font = tokens.font(TypeRole::Caption);
        for (index, point) in self.points.iter().enumerate() {
            let at = view.to_screen(Point::new(point[0], point[1]));
            // Double-stroked so the reticle reads on any pixel.
            for (width, color) in [(RETICLE_OUTLINE_WIDTH, Color32::from_black_alpha(RETICLE_OUTLINE_ALPHA)), (RETICLE_WIDTH, Color32::WHITE)] {
                let stroke = Stroke::new(width, color);
                painter.circle_stroke(at, sampler.marker_radius, stroke);
                for direction in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                    painter.line_segment([at + direction * sampler.tick_start, at + direction * sampler.tick_end], stroke);
                }
            }
            // Upper right, so several points stay readable when they overlap.
            let anchor = at + NUMBER_OFFSET;
            let number = (index + 1).to_string();
            painter.text(anchor + NUMBER_SHADOW_OFFSET, Align2::LEFT_BOTTOM, &number, font.clone(), Color32::from_black_alpha(NUMBER_SHADOW_ALPHA));
            painter.text(anchor, Align2::LEFT_BOTTOM, &number, font.clone(), Color32::WHITE);
        }
    }
}

/// The reticle's dark outline: 3 points, black at alpha 170, under the 1 point white stroke
/// (PhotoCraft's, so it reads on any pixel).
const RETICLE_OUTLINE_WIDTH: f32 = 3.0;
const RETICLE_OUTLINE_ALPHA: u8 = 170;
const RETICLE_WIDTH: f32 = 1.0;
/// The point number's anchor from the point (upper right, so overlapping points stay readable)
/// and its shadow's offset and alpha.
const NUMBER_OFFSET: egui::Vec2 = vec2(7.0, -7.0);
const NUMBER_SHADOW_OFFSET: egui::Vec2 = vec2(1.0, 1.0);
const NUMBER_SHADOW_ALPHA: u8 = 180;

/// The pixel that document position `pos` lands on, if it is inside a canvas `size` pixels large.
fn pixel(pos: [f64; 2], size: [u32; 2]) -> Option<[f64; 2]> {
    let [x, y] = pos.map(f64::floor);
    (x >= 0.0 && y >= 0.0 && x < f64::from(size[0]) && y < f64::from(size[1])).then_some([x, y])
}

/// The Color Sampler tool on the canvas: a click inside the canvas places a point; a press on a
/// point and a drag moves it (committed on release; a release within half a pixel of the press
/// is a click and does nothing); dropped off the canvas it is removed; Alt-click on a point
/// removes it; Alt-click on empty space and a click outside the canvas do nothing. A press past
/// the limit reports [`SamplerError::Full`] through [`take_error`](Self::take_error), shown in
/// the status bar, not as a fault.
// pc: crates/ui-egui/src/tools/analysis_ui.rs:338 (Down), :357 (Up), :210 `tolerance`.
pub struct ColorSamplerTool<'a> {
    samplers: &'a mut ColorSamplers,
    size: [u32; 2],
    moving: Option<(usize, [f64; 2])>,
    /// The last press came past the limit.
    full: bool,
}

impl<'a> ColorSamplerTool<'a> {
    /// The tool on a document `size` pixels large.
    pub fn new(samplers: &'a mut ColorSamplers, size: [u32; 2]) -> ColorSamplerTool<'a> {
        ColorSamplerTool { samplers, size, moving: None, full: false }
    }

    #[cfg(test)]
    pub(super) fn samplers(&self) -> &ColorSamplers {
        self.samplers
    }

    /// The status-bar message for the last press that failed, once: the limit message when a
    /// press came past [`MAX_SAMPLERS`].
    pub fn take_error(&mut self, ctx: &Context) -> Option<StatusLine> {
        let labels = ColorSamplerLabels::shared_ctx(ctx);
        std::mem::take(&mut self.full)
            .then(|| StatusLine { text: craft_i18n::fmt(&labels.limit, &[("max", &MAX_SAMPLERS.to_string())]), tone: Tone::Bad })
    }
}

impl CanvasTool for ColorSamplerTool<'_> {
    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        ToolCursor::Crosshair { length: 8.0, gap: 2.0 }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        let at = [pointer.doc.x, pointer.doc.y];
        let tolerance = f64::from(SAMPLER_HIT) / cx.view.zoom.max(f64::EPSILON);
        self.moving = None;
        match self.samplers.hit(at, tolerance) {
            // Alt-click on a point removes it.
            Some(index) if pointer.modifiers.alt => {
                let _ = self.samplers.remove(index);
                ToolAction::Done
            }
            // A plain press on a point starts a move, committed on release.
            Some(index) => {
                self.moving = Some((index, at));
                ToolAction::Claimed
            }
            // Alt-click on empty space does nothing.
            None if pointer.modifiers.alt => ToolAction::Done,
            // A click outside the canvas places nothing; past the limit reports Full.
            None => {
                self.full = self.samplers.add(at, self.size) == Err(SamplerError::Full);
                ToolAction::Done
            }
        }
    }

    fn drag(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn release(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        let Some((index, from)) = self.moving.take() else { return };
        let to = [pointer.doc.x, pointer.doc.y];
        // Within half a pixel of the press it was a click on the point, not a drag.
        if (to[0] - from[0]).hypot(to[1] - from[1]) <= 0.5 {
            return;
        }
        // Dropped off the canvas the point is removed, on it moved.
        let _ = self.samplers.move_point(index, to, self.size);
    }

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {
        self.moving = None;
    }

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}

    fn autoscrolls(&self) -> bool {
        false
    }
}

/// One point's Info panel reading: RGB 0–255 and colour-managed CMYK percentages through the
/// document's converter (the same conversion as the Color Picker, so both agree). One composite
/// pixel, no averaging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SamplerReadout {
    /// The point's number, from 1.
    pub number: usize,
    pub position: [i64; 2],
    pub rgb: [u8; 3],
    pub cmyk: [u8; 4],
}

impl SamplerReadout {
    /// The reading of point `index` at `position` whose pixel is `color`. Channels that are not
    /// finite read as 0; each is clamped before converting.
    // pc: crates/engine/src/cmd/sampler_cmds.rs:39 `readout`, :31 `device_cmyk`.
    pub fn new(index: usize, position: [f64; 2], color: Srgba, cmyk: &CmykConverter) -> SamplerReadout {
        let unit = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        let rgb = [color.r, color.g, color.b].map(unit);
        SamplerReadout {
            number: index + 1,
            position: position.map(|v| v.floor() as i64),
            rgb: rgb.map(|v| (v * 255.0).round() as u8),
            cmyk: cmyk.from_srgb(rgb).map(|v| (unit(v) * 100.0).round() as u8),
        }
    }
}

/// The options bar of the Color Sampler tool: "{n} of {max} sample points", a divider, Clear All
/// (disabled with no points), a divider, the hint in muted text. Returns `true` when Clear All
/// was pressed.
// pc: crates/ui-egui/src/tools/analysis_ui.rs:593.
pub fn sampler_options(ui: &mut Ui, count: usize) -> bool {
    let labels = ColorSamplerLabels::shared(ui);
    let counted = craft_i18n::fmt(&labels.count, &[("n", &count.to_string()), ("max", &MAX_SAMPLERS.to_string())]);
    ui.add(Text::new(counted));
    OptionsBar::divider(ui);
    let clear = ui.add(Button::new(labels.clear_all.clone()).enabled(count > 0)).clicked();
    OptionsBar::divider(ui);
    ui.add(Text::new(labels.hint.clone()).content(Content::Muted));
    clear
}

/// The Info panel readouts of a document's [`ColorSamplers`], cached until something they depend
/// on changes (PhotoCraft #4528). The cache is kept per [`document`](Self::document); a reading
/// is computed again when the document's [`revision`](Self::revision), the points or the
/// [`cmyk`](Self::cmyk) converter's profile differ from the cached ones.
// pc: crates/ui-egui/src/panels/info.rs:124 `sampler_readouts`.
pub struct SamplerReadouts<'a, F> {
    samplers: &'a ColorSamplers,
    read: F,
    revision: u64,
    document: u64,
    cmyk: Option<&'a CmykConverter>,
}

impl<'a, F: Fn([f64; 2]) -> Srgba> SamplerReadouts<'a, F> {
    /// The readouts of `samplers`, `read` giving the colour at a document point.
    pub fn new(samplers: &'a ColorSamplers, read: F) -> SamplerReadouts<'a, F> {
        SamplerReadouts { samplers, read, revision: 0, document: 0, cmyk: None }
    }

    /// The document's revision; the readouts are computed again when it differs.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }

    /// Which document the samplers belong to (any hashable key): documents at the same revision
    /// with samplers at the same points do not share readouts.
    pub fn document(mut self, document: impl Hash) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        document.hash(&mut hasher);
        self.document = std::hash::Hasher::finish(&hasher);
        self
    }

    /// The document's CMYK converter, as the Color Picker's; the built-in coated one without it.
    pub fn cmyk(mut self, converter: &'a CmykConverter) -> Self {
        self.cmyk = Some(converter);
        self
    }

    /// The readouts, from the cache when nothing changed.
    pub fn get(self, ctx: &Context) -> Arc<Vec<SamplerReadout>> {
        type Cached = (u64, usize, Vec<[f64; 2]>, Arc<Vec<SamplerReadout>>);
        let cmyk = self.cmyk.unwrap_or_else(|| CmykConverter::coated());
        let profile = Arc::as_ptr(cmyk.profile()) as usize;
        let id = Id::unique(("craft_color::sampler_readouts", self.document));
        if let Some((revision, cached_profile, points, readouts)) = ctx.data(|d| d.get_temp::<Cached>(id))
            && revision == self.revision
            && cached_profile == profile
            && points == self.samplers.points
        {
            return readouts;
        }
        let readouts: Arc<Vec<SamplerReadout>> =
            Arc::new(self.samplers.points.iter().enumerate().map(|(i, p)| SamplerReadout::new(i, *p, (self.read)(*p), cmyk)).collect());
        ctx.data_mut(|d| d.insert_temp::<Cached>(id, (self.revision, profile, self.samplers.points.clone(), readouts.clone())));
        readouts
    }
}
