//! [`RasterDisplay`]: the CPU raster path of a canvas. The app gives the document's composite as
//! RGBA for a region ([`RasterSource`]); the display uploads it as egui textures, draws it through
//! the [`CanvasView`] over a [`Checkerboard`], and re-reads only what the app marks damaged.
//!
//! A document larger than the device's texture limit ([`MAX_TEXTURE`], or less where the GPU
//! allows less) is uploaded reduced; zoomed in, the part in view is added at the screen's
//! resolution as a detail texture ([`DetailPlan`]), so a large document shows its own pixels at
//! 100 % and the cost follows the window, not the document. Pixels are drawn sharp (nearest)
//! from 100 % up and filtered below.
//!
//! PhotoCraft's GPU canvas is a separate path (step 8); both draw the same checkerboard and
//! pixel grid ([`PixelGrid`](super::PixelGrid)).
//!
//! ```no_run
//! use craft_ui::canvas::{CanvasView, Checkerboard, RasterDisplay, RasterSource};
//! # fn demo(ui: &mut egui::Ui, view: &CanvasView, doc: &dyn RasterSource) {
//! let shown = RasterDisplay::new("doc-1").checkerboard(Checkerboard::default()).show(ui, view, doc);
//! # let _ = shown;
//! # }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Color32, ColorImage, Context, Id, IdSalt, Painter, Ui};

use super::view::CanvasView;

/// The largest texture side the display uploads.
pub const MAX_TEXTURE: u32 = 4096;

/// A rectangle of whole document pixels, `x0..x1` by `y0..y1` (exclusive ends).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PixelRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl PixelRect {
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    /// The whole of a document of `size` pixels.
    pub fn of_size(size: [u32; 2]) -> Self {
        todo!()
    }

    pub fn width(&self) -> i32 {
        (self.x1 - self.x0).max(0)
    }

    pub fn height(&self) -> i32 {
        (self.y1 - self.y0).max(0)
    }

    pub fn is_empty(&self) -> bool {
        self.width() == 0 || self.height() == 0
    }

    /// The part inside both; empty when they do not overlap.
    pub fn intersect(&self, other: &PixelRect) -> PixelRect {
        todo!()
    }

    /// The smallest rect holding both (an empty one adds nothing).
    pub fn union(&self, other: &PixelRect) -> PixelRect {
        todo!()
    }

    /// The document pixels `view` shows (its bounding box when turned), rounded outward and
    /// clipped to a document of `size`.
    // pc: crates/ui-egui/src/canvas/textures.rs:633 `visible_doc_rect`.
    pub fn visible(view: &CanvasView, size: [u32; 2]) -> PixelRect {
        todo!()
    }
}

/// The document's composite, as the display reads it. The app implements it over its document
/// (PhotoCraft's compositor, a PDF page renderer, a video frame).
pub trait RasterSource {
    /// The document's size in pixels.
    fn size(&self) -> [u32; 2];

    /// Changes whenever the pixels or the size change. A new revision with no
    /// [`RasterDisplay::damage`] since the last pass re-reads everything.
    fn revision(&self) -> u64;

    /// Write the composite of `region` into `out` (sRGBA, unmultiplied), one texel per `factor`
    /// pixels each way (`factor` is a power of two; 1 is full resolution). `out` is already
    /// `region / factor` texels, rounded up.
    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage);
}

/// The texture side on this device: [`MAX_TEXTURE`], or the device's smaller limit (2048 on some
/// older GPUs and WebGL); past it egui asserts and the backend fails.
// pc: crates/ui-egui/src/canvas/textures.rs:15 `texture_side`, :20 `clamp_texture_side`.
pub fn texture_side(ctx: &Context) -> u32 {
    todo!()
}

/// How many document pixels per side one texel of the base texture holds for a document of
/// `size` with textures at most `side` per side: 1 when it fits.
// pc: crates/ui-egui/src/canvas/textures.rs:26 `texture_factor`.
pub fn texture_factor(size: [u32; 2], side: u32) -> u32 {
    todo!()
}

/// The detail texture a zoomed-in view of a reduced document needs: the part in view (`region`,
/// snapped outward to 256-texel steps so a small pan reuses it) at one texel per `factor`
/// pixels, a power of two no coarser than the screen and capped by the texture side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailPlan {
    pub factor: u32,
    pub region: PixelRect,
}

impl DetailPlan {
    /// The plan for a view at `device_zoom` (device pixels per document pixel) over `visible`,
    /// or `None` when the base texture is already as fine as the screen or nothing is in view.
    // pc: crates/ui-egui/src/canvas/textures.rs:50 `detail_plan`.
    pub fn of(size: [u32; 2], side: u32, device_zoom: f64, visible: PixelRect) -> Option<DetailPlan> {
        todo!()
    }
}

/// Photoshop's Transparency grid size (Preferences › Transparency & Gamut).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CheckerSize {
    /// No checkerboard: transparency shows white.
    None,
    Small,
    #[default]
    Medium,
    Large,
}

impl CheckerSize {
    /// A square's side in screen points: 4, 8, 16; `None` draws no checkerboard.
    // pc: crates/engine/src/prefs.rs:736 `TransparencyAndGamut::square`.
    pub fn square(self) -> Option<f32> {
        match self {
            CheckerSize::None => None,
            CheckerSize::Small => Some(4.0),
            CheckerSize::Medium => Some(8.0),
            CheckerSize::Large => Some(16.0),
        }
    }
}

/// Photoshop's Transparency grid colours.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CheckerColors {
    #[default]
    Light,
    Medium,
    Dark,
    Red,
    Orange,
    Green,
    Blue,
    Purple,
    /// The user's two colours (light, dark).
    Custom(Color32, Color32),
}

impl CheckerColors {
    /// The two square colours, light then dark.
    // pc: crates/engine/src/prefs.rs:745 `TransparencyAndGamut::colors`.
    pub fn colors(self) -> [Color32; 2] {
        let rgb = |[r, g, b]: [u8; 3]| Color32::from_rgb(r, g, b);
        match self {
            CheckerColors::Light => [rgb([255, 255, 255]), rgb([204, 204, 204])],
            CheckerColors::Medium => [rgb([153, 153, 153]), rgb([102, 102, 102])],
            CheckerColors::Dark => [rgb([102, 102, 102]), rgb([51, 51, 51])],
            CheckerColors::Red => [rgb([255, 255, 255]), rgb([255, 204, 204])],
            CheckerColors::Orange => [rgb([255, 255, 255]), rgb([255, 229, 204])],
            CheckerColors::Green => [rgb([255, 255, 255]), rgb([204, 255, 204])],
            CheckerColors::Blue => [rgb([255, 255, 255]), rgb([204, 229, 255])],
            CheckerColors::Purple => [rgb([255, 255, 255]), rgb([229, 204, 255])],
            CheckerColors::Custom(light, dark) => [light, dark],
        }
    }
}

/// The checkerboard drawn under transparent pixels: squares of a fixed screen size at any zoom,
/// in the user's colours. A user preference, not a theme token.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Checkerboard {
    pub size: CheckerSize,
    pub colors: CheckerColors,
}

impl Checkerboard {
    /// Fill the document's area as `view` shows it (a quad when turned) with the checkerboard,
    /// squares aligned to the canvas's top-left so they hold still while the document scrolls
    /// under them, as Photoshop. White with [`CheckerSize::None`].
    // pc: crates/ui-egui/src/canvas/display.rs:4 `checker` (a 2 × 2 repeating texture).
    pub fn paint(&self, painter: &Painter, view: &CanvasView, size: [u32; 2]) {
        todo!()
    }
}

/// The CPU raster display of one canvas. Built each pass; its textures live in egui memory under
/// its id until [`forget`](Self::forget).
pub struct RasterDisplay {
    id_salt: IdSalt,
    checker: Option<Checkerboard>,
}

/// What the display did in a pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterResponse {
    /// The display's id, for [`RasterDisplay::damage`] and [`RasterDisplay::forget`].
    pub id: Id,
    /// The pixels read from the source this pass (damage, a new detail region, the first pass).
    pub read: Option<PixelRect>,
    /// The detail texture in use this pass, if any.
    pub detail: Option<DetailPlan>,
}

impl RasterDisplay {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), checker: None }
    }

    /// Draw `checker` under the document. Without one the document draws straight on the
    /// canvas (an opaque document, a video frame).
    pub fn checkerboard(mut self, checker: Checkerboard) -> Self {
        self.checker = Some(checker);
        self
    }

    /// Read what changed from `source`, upload it and draw the document through `view`, clipped
    /// to the canvas rect. Nothing is read when the revision and the view's detail region are
    /// unchanged and no damage is pending.
    // pc: crates/ui-egui/src/canvas/textures.rs:115 `ensure_texture_in_view`, :251 `update_detail`, :292 `render_detail`.
    pub fn show(self, ui: &mut Ui, view: &CanvasView, source: &dyn RasterSource) -> RasterResponse {
        todo!()
    }

    /// Mark `region` of the display `id` changed: the next [`show`](Self::show) re-reads it only
    /// (merged with other damage since the last pass).
    // pc: crates/ui-egui/src/canvas/textures.rs:322 `damage_since`.
    pub fn damage(ctx: &Context, id: Id, region: PixelRect) {
        todo!()
    }

    /// Free the display's textures (the document closed).
    // pc: crates/ui-egui/src/canvas/textures.rs:667 `retain_gpu_documents` (the CPU side).
    pub fn forget(ctx: &Context, id: Id) {
        todo!()
    }
}
