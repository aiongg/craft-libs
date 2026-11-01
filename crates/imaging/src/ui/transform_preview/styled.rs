//! The styled transform preview: a layer with a style (Stroke, Drop Shadow, …) composited by the
//! app at the resolution it is shown at, within a time budget (task TR-T11).
//!
//! A style is not transformed with its layer: it is drawn again from the layer's new shape (a
//! stroke keeps its width, a shadow its angle). So the moving texels are resampled through the box
//! by the library ([`StylePlan`], the homography), the app's compositor lays the style over them
//! ([`StyledSource::compose`]), and the result is drawn in place of the mesh. The work follows
//! the screen, never finer than the document, and shrinks to fit [`StyledBudget`]; it is rebuilt
//! only when its key changes (box, zoom, document revision, display key, texture level, filter).
//! The composite is drawn over transparency with Normal blending, like the mesh it replaces.

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T11)")]

use std::sync::Mutex;

use egui::{ColorImage, Context, TextureHandle, TextureId};
use kurbo::{Point, Rect};

use craft_ui::canvas::PixelRect;
use craft_ui::canvas::quad::Homography;

use super::{PreviewDisplay, PreviewTextures};

/// How long a styled rebuild may take (LC25, LC26): the work pixels a composite may have are the
/// time over the source's [cost per pixel](StyledSource::pixel_cost_ns), clamped. Separate from
/// S7-adjust's live-preview damage tracking (different inputs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyledBudget {
    /// Milliseconds a rebuild may take while the pointer is down (a drag rebuilds every pass).
    pub drag_ms: f64,
    /// Milliseconds once the box rests.
    pub rest_ms: f64,
    /// The fewest work pixels a budget asks for, however costly the style.
    pub min_pixels: f64,
    /// The most work pixels in a composite.
    pub max_pixels: f64,
}

impl StyledBudget {
    /// PhotoCraft's drag time, 30 ms.
    pub const DRAG_MS: f64 = 30.0;
    /// PhotoCraft's rest time, 120 ms.
    pub const REST_MS: f64 = 120.0;
    /// 0.25 million work pixels.
    pub const MIN_PIXELS: f64 = 0.25e6;
    /// 8 million work pixels.
    pub const MAX_PIXELS: f64 = 8.0e6;

    /// The work pixels for a style costing `cost_ns` per pixel: `ms × 10⁶ / cost_ns` with the drag
    /// or rest time, clamped to `min_pixels..=max_pixels`.
    pub fn pixels(&self, cost_ns: f64, dragging: bool) -> f64 {
        // pc: crates/ui-egui/src/tools/transform_fx.rs:94 (`budget`).
        todo!()
    }
}

impl Default for StyledBudget {
    fn default() -> Self {
        Self { drag_ms: Self::DRAG_MS, rest_ms: Self::REST_MS, min_pixels: Self::MIN_PIXELS, max_pixels: Self::MAX_PIXELS }
    }
}

/// Where and how finely a styled preview is composited. Rectangles are in work pixels: document
/// pixels times `scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StylePlan {
    /// Work pixels per document pixel, at most 1.
    pub scale: f64,
    /// The box: the layer's bounds after the transform.
    pub bounds: PixelRect,
    /// What is composited and drawn: `bounds` grown by the style's reach, inside the canvas.
    pub shown: PixelRect,
    /// The moving texels resampled for it: `shown` grown by twice the reach, so every effect over
    /// `shown` sees all of the shape it depends on, also past the canvas edge.
    pub content: PixelRect,
}

impl StylePlan {
    /// The plan for a box `quad` (document units, clockwise from the top left) in a document of
    /// `canvas` pixels shown at `zoom` screen pixels per document pixel, within `pixels` work
    /// pixels. The scale starts at `min(zoom, 1)` and shrinks by `sqrt(pixels / area) × 0.98` until
    /// `content`'s area fits, in at most 6 rounds; the reach is the source's at each scale.
    /// `None` when the box is not finite, nothing of it is on the canvas, or it does not fit after
    /// 6 rounds. Work coordinates are clamped to ±5 × 10⁸ so rectangle arithmetic cannot overflow.
    pub fn new(quad: [Point; 4], canvas: [u32; 2], zoom: f32, pixels: f64, reach: impl Fn(f64) -> i32) -> Option<StylePlan> {
        // pc: crates/ui-egui/src/tools/transform_fx.rs:117 (`plan`).
        todo!()
    }

    /// `shown` in document units: where the composite is drawn.
    pub fn document_rect(&self) -> Rect {
        // pc: crates/ui-egui/src/tools/transform_fx.rs:67 (`doc_rect`).
        todo!()
    }
}

/// The moving texels resampled over a work rectangle: straight-alpha RGBA floats, row by row,
/// `rect.width() × rect.height()` of them. The edges of the source fade over one texel, as a
/// resampled layer's do.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledContent {
    pub rect: PixelRect,
    pub texels: Vec<[f32; 4]>,
}

/// The app's side of a styled preview: its compositor, which the library cannot have (the layer's
/// style and the document are the app's).
pub trait StyledSource {
    /// Changes whenever the document or the moving layer does (its style, opacity, fill).
    fn revision(&self) -> u64;

    /// Rough cost of compositing one work pixel with the layer's style, in nanoseconds (PhotoCraft
    /// measures it per effect, `pixel_cost_ns`).
    fn pixel_cost_ns(&self) -> f64;

    /// How far the style reaches past the layer's pixels at `scale` work pixels per document
    /// pixel, in work pixels (PhotoCraft `effects::margin` of the scaled style).
    fn reach(&self, scale: f64) -> i32;

    /// The layer with its style at `plan.scale`, over `content`, composited over `plan.shown`:
    /// premultiplied texels, `shown.width() × shown.height()`. `None` when it cannot be made.
    fn compose(&self, plan: &StylePlan, content: &StyledContent) -> Option<ColorImage>;
}

/// The moving texels (`image`, premultiplied, of which `uv` covers the source frame) resampled
/// over `rect`. `to_unit` takes a work pixel to the unit square of the source frame and `inside`
/// is a work point within the box; `nearest` keeps texels whole.
pub(super) fn resample(image: &ColorImage, uv: [f32; 2], to_unit: &Homography, inside: Point, rect: PixelRect, nearest: bool) -> StyledContent {
    // pc: crates/ui-egui/src/tools/transform_fx.rs:164 (`resample`).
    todo!()
}

/// What a styled composite is made from this pass (built by
/// [`TransformPreview::show`](super::TransformPreview::show)).
pub(super) struct StyledFrame<'a> {
    pub textures: &'a PreviewTextures,
    pub quad: [Point; 4],
    /// How much the box enlarges its source (1 is the same size).
    pub quad_scale: f32,
    pub nearest: bool,
    /// Screen pixels per document pixel.
    pub zoom: f32,
    pub dragging: bool,
    pub canvas: [u32; 2],
    pub budget: StyledBudget,
}

/// What a composite depends on: the box (bit for bit), the plan, the revision, the display key,
/// the texture level and the filter.
#[derive(Clone, Debug, PartialEq)]
struct StyledKey {
    revision: u64,
    display_key: u64,
    quad: [u64; 8],
    plan: StylePlan,
    level: u32,
    nearest: bool,
    crisp: bool,
}

struct Shown {
    key: StyledKey,
    texture: TextureHandle,
    rect: Rect,
}

#[derive(Default)]
struct StyledState {
    shown: Option<Shown>,
    builds: u64,
}

/// The styled composite of a session's moving layer, kept until its key changes. The app keeps one
/// per session while the layer has a style.
pub struct StyledPreview {
    name: String,
    state: Mutex<StyledState>,
}

impl StyledPreview {
    /// `name` keys the texture in egui.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), state: Mutex::new(StyledState::default()) }
    }

    /// How many times the composite was made: once per change of its key.
    pub fn builds(&self) -> u64 {
        // pc: crates/ui-egui/src/tools/transform_fx.rs:386.
        todo!()
    }

    /// The composite for `frame` and the document rectangle it covers, compositing again through
    /// `source` when the key changed. Crisp (nearest) when the plan is at full scale and the zoom
    /// is at least `CRISP_ZOOM`. `None` when there is nothing to draw this way; the caller draws
    /// the plain mesh.
    pub(super) fn texture(
        &self,
        ctx: &Context,
        frame: &StyledFrame<'_>,
        source: &dyn StyledSource,
        display: Option<&PreviewDisplay>,
    ) -> Option<(TextureId, Rect)> {
        // pc: crates/ui-egui/src/tools/transform_fx.rs:393 (`texture`), :335 (`Frame::plan`).
        todo!()
    }
}
