//! The stroke preview: Photoshop's S-curve test stroke as data ([`PreviewStroke`]) and the strip
//! that shows the app's rendering of it ([`BrushStrokeStrip`]).

use std::sync::Arc;

use craft_ui::canvas::textures::{Revision, TextureCache, TextureCacheOptions};
use craft_ui::localized::SharedLabels as _;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{AsIdSalt, Color32, ColorImage, IdSalt, Rect, Response, Role, Sense, StrokeKind, Ui, WidgetInfo, pos2, vec2};

use super::labels::BrushSettingsLabels;
use super::style::BrushSettingsStyle;

/// One point of the test stroke, in pixels of the strip image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewSample {
    /// Across, from the image's left edge.
    pub x: f64,
    /// Down, from the image's top edge.
    pub y: f64,
    /// The pen pressure, `PreviewStroke::PRESSURE_FLOOR` to 1.
    pub pressure: f32,
}

/// Photoshop's preview stroke for an image `width` × `height` pixels: an S-curve with the pressure
/// tapering in and out, so every engine draws the same test stroke.
///
/// [`SAMPLES`](Self::SAMPLES) points at `s = i / (SAMPLES − 1)`: `x = MARGIN + s × (width − 2 ×
/// MARGIN)`, `y = height / 2 − sin(2π s) × height × AMPLITUDE`, pressure `max(sin(π s),
/// PRESSURE_FLOOR)`. The engine draws it with the brush scaled by [`fit`](Self::fit), Build-up off
/// and a fixed seed (PhotoCraft 7), so the strip changes only when the settings do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewStroke {
    /// The image width in pixels.
    pub width: u32,
    /// The image height in pixels.
    pub height: u32,
}

impl PreviewStroke {
    /// The number of samples.
    pub const SAMPLES: usize = 65;
    /// Pixels kept clear at each end.
    pub const MARGIN: f64 = 10.0;
    /// The curve's swing up and down, as a fraction of the height.
    pub const AMPLITUDE: f64 = 0.22;
    /// The share of the height the brush diameter may take.
    pub const FIT: f64 = 0.55;
    /// The least pressure, at the stroke's ends.
    pub const PRESSURE_FLOOR: f32 = 0.05;

    /// The stroke for an image of this size.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// The samples, in drawing order.
    pub fn samples(&self) -> Vec<PreviewSample> {
        // pc: crates/ui-egui/src/panels/brush_preview.rs:119-127 (n = 64, x = 10 + s (w − 20), y = h/2 − sin(2πs) 0.22 h, pressure max(sin(πs), 0.05))
        let (w, h) = (f64::from(self.width), f64::from(self.height));
        let last = (Self::SAMPLES - 1) as f64;
        (0..Self::SAMPLES)
            .map(|i| {
                let s = i as f64 / last;
                PreviewSample {
                    x: Self::MARGIN + s * (w - 2.0 * Self::MARGIN),
                    y: h / 2.0 - (s * std::f64::consts::TAU).sin() * h * Self::AMPLITUDE,
                    pressure: ((s * std::f64::consts::PI).sin() as f32).max(Self::PRESSURE_FLOOR),
                }
            })
            .collect()
    }

    /// The scale for a brush `diameter` pixels across: `min(1, FIT × height / max(diameter, 1))`.
    /// The engine multiplies the brush size (and a dual brush's) by it, at least 1 pixel.
    pub fn fit(&self, diameter: f64) -> f64 {
        // pc: crates/ui-egui/src/panels/brush_preview.rs:111-113 (k = min(1, 0.55 h / size); sizes floored at 1)
        (Self::FIT * f64::from(self.height) / diameter.max(1.0)).min(1.0)
    }
}

/// The stroke preview strip: the app's rendering of the test stroke, cached in a
/// [`TextureCache`](craft_ui::canvas::textures::TextureCache) by its [`Revision`]. The app's
/// render closure runs only when the revision or the strip's size in pixels changes.
///
/// - **Look.** On `field.fill` with a 1 pt `field.border` and `radius.control` corners, the full width
///   it is given, `brush.strip_height` (76) high; the image fills it.
/// - **AccessKit.** An `Image` named "Stroke preview".
#[must_use = "show the strip with `show`"]
pub struct BrushStrokeStrip {
    id_salt: IdSalt,
    revision: Revision,
    labels: Option<BrushSettingsLabels>,
}

impl BrushStrokeStrip {
    /// A strip whose picture is at `revision` (`Revision::of(&settings_signature)`).
    pub fn new(id_salt: impl AsIdSalt, revision: Revision) -> Self {
        Self { id_salt: IdSalt::new(id_salt), revision, labels: None }
    }

    /// The words; the installed language's by default.
    pub fn labels(mut self, labels: BrushSettingsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the strip across the available width. `make` gets the strip's size in physical pixels
    /// (points × `pixels_per_point`, rounded) and returns the stroke image (the app's engine draws
    /// [`PreviewStroke`] into it); it runs only when the revision or that size changed.
    pub fn show(self, ui: &mut Ui, make: impl FnOnce([usize; 2]) -> ColorImage) -> Response {
        // pc: crates/ui-egui/src/panels/brush_panel.rs:388-391 (strip on `field` fill; replaced: `SigCache` by `TextureCache`)
        // pc: crates/ui-egui/src/panels/brush_preview.rs:16-40 `SigCache` (one slot per strip, re-render on a new signature)
        let Self { id_salt, revision, labels } = self;
        let labels = labels.map(Arc::new).unwrap_or_else(|| BrushSettingsLabels::shared(ui));
        let tokens = Tokens::of(ui);
        let style = BrushSettingsStyle::of(&tokens);
        let radius = tokens.radius.control;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), style.strip_height), Sense::hover());
        let ppp = ui.ctx().pixels_per_point();
        let pixels = [rect.width(), rect.height()].map(|side| (side * ppp).round().max(1.0) as usize);
        if ui.is_rect_visible(rect) {
            let cache = TextureCache::<()>::in_context(ui.ctx(), id_salt, TextureCacheOptions::frames());
            let picture = cache.get_or_put(ui.ctx(), (), Revision::of(&(revision, pixels)), || make(pixels));
            let painter = ui.painter();
            painter.rect_filled(rect, radius, style.strip_fill);
            painter.add(
                egui::epaint::RectShape::filled(rect, radius, Color32::WHITE)
                    .with_texture(picture.id, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))),
            );
            painter.rect_stroke(rect, radius, style.strip_border, StrokeKind::Inside);
        }
        let name = labels.stroke_preview.clone();
        response.widget_info(|| WidgetInfo::labeled(Role::Image, ui.is_enabled(), &name));
        response
    }
}
