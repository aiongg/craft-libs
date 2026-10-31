//! Color Sampler points (F04.08): up to [`MAX_SAMPLERS`] numbered points pinned to canvas
//! pixels, each read in the Info panel ([`SamplerReadout`]), so a fixed place can be compared
//! instead of chasing the pointer.
//!
//! The points are per-document view state: not saved with the document, no History step, no
//! pixels changed. The app keeps one [`ColorSamplers`] per document.

#![expect(dead_code, reason = "skeleton: removed with the canvas tool")]

use craft_cms::convert::CmykConverter;
use craft_theme::Srgba;
use craft_theme::TypeRole;
use craft_ui::canvas::CanvasView;
use craft_ui::canvas::kurbo::Point;
use craft_ui::controls::Button;
use craft_ui::localized::SharedLabels;
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use egui::{Align2, Color32, Context, Id, Painter, Stroke, Ui, vec2};

craft_ui::words! {
    /// The Color Sampler's words, English by default.
    #[non_exhaustive]
    pub struct SamplerLabels {
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
            for (width, color) in [(3.0, Color32::from_black_alpha(170)), (1.0, Color32::WHITE)] {
                let stroke = Stroke::new(width, color);
                painter.circle_stroke(at, sampler.marker_radius, stroke);
                for direction in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                    painter.line_segment([at + direction * sampler.tick_start, at + direction * sampler.tick_end], stroke);
                }
            }
            // Upper right, so several points stay readable when they overlap.
            let anchor = at + vec2(7.0, -7.0);
            let number = (index + 1).to_string();
            painter.text(anchor + vec2(1.0, 1.0), Align2::LEFT_BOTTOM, &number, font.clone(), Color32::from_black_alpha(180));
            painter.text(anchor, Align2::LEFT_BOTTOM, &number, font.clone(), Color32::WHITE);
        }
    }
}

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
// needs: S3-canvas tool_host — `impl CanvasTool for ColorSamplerTool` (`press` with the hit
// tolerance `SAMPLER_HIT / view zoom`, `release` commits the move, `cursor` is the sampler
// crosshair).
// pc: crates/ui-egui/src/tools/analysis_ui.rs:338 (Down), :357 (Up), :210 `tolerance`.
pub struct ColorSamplerTool<'a> {
    samplers: &'a mut ColorSamplers,
    size: [u32; 2],
    moving: Option<(usize, [f64; 2])>,
    error: Option<SamplerError>,
}

impl<'a> ColorSamplerTool<'a> {
    /// The tool on a document `size` pixels large.
    pub fn new(samplers: &'a mut ColorSamplers, size: [u32; 2]) -> ColorSamplerTool<'a> {
        ColorSamplerTool { samplers, size, moving: None, error: None }
    }

    /// The error of the last press, once.
    pub fn take_error(&mut self) -> Option<SamplerError> {
        self.error.take()
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
    let labels = SamplerLabels::shared(ui);
    let counted = craft_i18n::fmt(&labels.count, &[("n", &count.to_string()), ("max", &MAX_SAMPLERS.to_string())]);
    ui.add(Text::new(counted));
    OptionsBar::divider(ui);
    let clear = ui.add(Button::new(labels.clear_all.clone()).enabled(count > 0)).clicked();
    OptionsBar::divider(ui);
    ui.add(Text::new(labels.hint.clone()).content(Content::Muted));
    clear
}

/// The readouts of `samplers`, cached until the document changes: the app passes its document
/// revision, and the readouts are computed again only when it differs (PhotoCraft #4528).
// pc: crates/ui-egui/src/panels/info.rs:124 `sampler_readouts`.
pub fn sampler_readouts(
    ctx: &Context,
    revision: u64,
    samplers: &ColorSamplers,
    read: impl Fn([f64; 2]) -> Srgba,
    cmyk: &CmykConverter,
) -> std::sync::Arc<Vec<SamplerReadout>> {
    type Cached = (u64, Vec<[f64; 2]>, std::sync::Arc<Vec<SamplerReadout>>);
    let id = Id::unique("craft_color::sampler_readouts");
    // The points are part of the key so that another document at the same revision does not
    // reuse these readouts.
    if let Some((cached_revision, points, readouts)) = ctx.data(|d| d.get_temp::<Cached>(id))
        && cached_revision == revision
        && points == samplers.points
    {
        return readouts;
    }
    let readouts: std::sync::Arc<Vec<SamplerReadout>> =
        std::sync::Arc::new(samplers.points.iter().enumerate().map(|(i, p)| SamplerReadout::new(i, *p, read(*p), cmyk)).collect());
    ctx.data_mut(|d| d.insert_temp::<Cached>(id, (revision, samplers.points.clone(), readouts.clone())));
    readouts
}
