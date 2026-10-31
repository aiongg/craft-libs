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

use egui::epaint::{Mesh, Vertex};
use egui::{AsIdSalt, Color32, ColorImage, Context, Id, IdSalt, Painter, Pos2, Shape, TextureHandle, TextureId, TextureOptions, Ui, pos2};

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
        Self::new(0, 0, i32::try_from(size[0]).unwrap_or(i32::MAX), i32::try_from(size[1]).unwrap_or(i32::MAX))
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
        let r = PixelRect::new(self.x0.max(other.x0), self.y0.max(other.y0), self.x1.min(other.x1), self.y1.min(other.y1));
        if r.is_empty() { PixelRect::default() } else { r }
    }

    /// The smallest rect holding both (an empty one adds nothing).
    pub fn union(&self, other: &PixelRect) -> PixelRect {
        match (self.is_empty(), other.is_empty()) {
            (true, true) => PixelRect::default(),
            (true, false) => *other,
            (false, true) => *self,
            (false, false) => PixelRect::new(self.x0.min(other.x0), self.y0.min(other.y0), self.x1.max(other.x1), self.y1.max(other.y1)),
        }
    }

    /// The document pixels `view` shows (its bounding box when turned), rounded outward with a
    /// margin of 2 pixels for filtering and clipped to a document of `size`.
    // pc: crates/ui-egui/src/canvas/textures.rs:633 `visible_doc_rect`.
    pub fn visible(view: &CanvasView, size: [u32; 2]) -> PixelRect {
        let v = view.visible();
        let c = |n: f64| n.clamp(-1e9, 1e9) as i32;
        let seen = PixelRect::new(c(v.x0.floor()) - 2, c(v.y0.floor()) - 2, c(v.x1.ceil()) + 2, c(v.y1.ceil()) + 2);
        seen.intersect(&PixelRect::of_size(size))
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
    u32::try_from(ctx.input(|i| i.max_texture_side)).unwrap_or(MAX_TEXTURE).clamp(1, MAX_TEXTURE)
}

/// How many document pixels per side one texel of the base texture holds for a document of
/// `size` with textures at most `side` per side: 1 when it fits.
// pc: crates/ui-egui/src/canvas/textures.rs:26 `texture_factor`.
pub fn texture_factor(size: [u32; 2], side: u32) -> u32 {
    size[0].max(size[1]).div_ceil(side.max(1)).max(1)
}

/// The detail texture a zoomed-in view of a reduced document needs: the part in view (`region`,
/// snapped outward to 256-texel steps so a small pan reuses it) at one texel per `factor`
/// pixels, a power of two no coarser than the screen and capped by the texture side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailPlan {
    pub factor: u32,
    pub region: PixelRect,
}

/// The detail region snaps outward to multiples of this many texels.
const DETAIL_QUANTUM: i32 = 256;

/// The largest power of two, up to 64, whose texels are still no coarser than a device pixel at
/// `device_zoom`.
// pc: crates/ui-egui/src/dialogs/adjust_preview.rs:363 `proxy_factor`.
fn proxy_factor(device_zoom: f64) -> u32 {
    let mut k = 1u32;
    while k < 64 && f64::from(k * 2) * device_zoom <= 1.0 {
        k *= 2;
    }
    k
}

impl DetailPlan {
    /// The plan for a view at `device_zoom` (device pixels per document pixel) over `visible`,
    /// or `None` when the base texture is already as fine as the screen or nothing is in view.
    // pc: crates/ui-egui/src/canvas/textures.rs:50 `detail_plan`.
    pub fn of(size: [u32; 2], side: u32, device_zoom: f64, visible: PixelRect) -> Option<DetailPlan> {
        if !device_zoom.is_finite() || device_zoom <= 0.0 {
            return None;
        }
        let base = texture_factor(size, side);
        let mut factor = proxy_factor(device_zoom);
        let v = visible.intersect(&PixelRect::of_size(size));
        if v.is_empty() {
            return None;
        }
        let bounds = PixelRect::of_size(size);
        while factor < base {
            let q = DETAIL_QUANTUM.saturating_mul(i32::try_from(factor).unwrap_or(i32::MAX));
            let down = |x: i32| x.div_euclid(q).saturating_mul(q);
            let up = |x: i32| x.div_euclid(q).saturating_add(i32::from(x.rem_euclid(q) != 0)).saturating_mul(q);
            let region = PixelRect::new(down(v.x0), down(v.y0), up(v.x1), up(v.y1)).intersect(&bounds);
            // A reduced region's texel count can round one up.
            let fits = |n: i32| n.unsigned_abs().div_ceil(factor).saturating_add(1) <= side;
            // Snapped when that fits, else just what is in view (a window near the device's limit).
            for region in [region, v] {
                if !region.is_empty() && fits(region.width()) && fits(region.height()) {
                    return Some(DetailPlan { factor, region });
                }
            }
            factor = factor.saturating_mul(2);
        }
        None
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
        let quad = view.quad(kurbo::Rect::new(0.0, 0.0, f64::from(size[0]), f64::from(size[1])));
        let Some(square) = self.size.square() else {
            painter.add(Shape::mesh(quad_mesh(TextureId::default(), quad, [egui::epaint::WHITE_UV; 4], Color32::WHITE)));
            return;
        };
        let texture = checker_texture(painter.ctx(), self.colors);
        // One texel per square, two texels per repeat.
        let origin = view.rect.min;
        let uv = quad.map(|p| pos2((p.x - origin.x) / (2.0 * square), (p.y - origin.y) / (2.0 * square)));
        painter.add(Shape::mesh(quad_mesh(texture.id(), quad, uv, Color32::WHITE)));
    }
}

/// A display's textures, kept in egui memory under its id.
#[derive(Clone)]
struct State {
    size: [u32; 2],
    revision: u64,
    base: TextureHandle,
    detail: Option<Detail>,
}

#[derive(Clone)]
struct Detail {
    plan: DetailPlan,
    texture: TextureHandle,
}

/// The region marked by [`RasterDisplay::damage`] since the last pass.
#[derive(Clone, Copy, Default)]
struct Pending(PixelRect);

fn state_key(id: Id) -> Id {
    id.with("raster-state")
}

fn damage_key(id: Id) -> Id {
    id.with("raster-damage")
}

/// Sharp from 100 % up on a full-resolution texture; a reduced texture's texels are averages of
/// several pixels, so it stays filtered.
// pc: crates/ui-egui/src/canvas/textures.rs:112 `canvas_texture_options`.
fn texture_options(factor: u32) -> TextureOptions {
    if factor > 1 { TextureOptions::LINEAR } else { TextureOptions { magnification: egui::TextureFilter::Nearest, ..TextureOptions::LINEAR } }
}

/// `region` grown outward to whole texels of `factor` pixels.
fn snap(region: PixelRect, factor: u32) -> PixelRect {
    let f = i32::try_from(factor).unwrap_or(i32::MAX);
    let up = |n: i32| n.div_euclid(f).saturating_add(i32::from(n.rem_euclid(f) != 0)).saturating_mul(f);
    PixelRect::new(region.x0.div_euclid(f).saturating_mul(f), region.y0.div_euclid(f).saturating_mul(f), up(region.x1), up(region.y1))
}

/// The texel size of `region` at `factor`, rounded up.
fn texels(region: PixelRect, factor: u32) -> [usize; 2] {
    [region.width() as usize, region.height() as usize].map(|n| n.div_ceil(factor as usize))
}

/// Where `region` starts in a texture that holds `origin` at one texel per `factor` pixels.
fn texel_at(region: PixelRect, origin: PixelRect, factor: u32) -> [usize; 2] {
    [(region.x0 - origin.x0) as usize / factor as usize, (region.y0 - origin.y0) as usize / factor as usize]
}

fn read_region(source: &dyn RasterSource, region: PixelRect, factor: u32) -> ColorImage {
    let mut image = ColorImage::filled(texels(region, factor), Color32::TRANSPARENT);
    source.read(region, factor, &mut image);
    image
}

/// Draw `texture`, which holds `region` of the document from its top-left at one texel per
/// `factor` pixels, through `view`.
fn draw(painter: &Painter, view: &CanvasView, texture: &TextureHandle, factor: u32, region: PixelRect) {
    let [tw, th] = texture.size();
    let (w, h) = (region.width() as f32 / (tw as f32 * factor as f32), region.height() as f32 / (th as f32 * factor as f32));
    let corners = view.quad(kurbo::Rect::new(f64::from(region.x0), f64::from(region.y0), f64::from(region.x1), f64::from(region.y1)));
    let uv = [pos2(0.0, 0.0), pos2(w, 0.0), pos2(w, h), pos2(0.0, h)];
    painter.add(Shape::mesh(quad_mesh(texture.id(), corners, uv, Color32::WHITE)));
}

/// The 2 × 2 repeating texture of `colors`, kept in egui memory.
fn checker_texture(ctx: &Context, colors: CheckerColors) -> TextureHandle {
    let key = Id::NULL.with(("craft-ui-checker", colors));
    if let Some(t) = ctx.data(|d| d.get_temp::<TextureHandle>(key)) {
        return t;
    }
    let [a, b] = colors.colors();
    let options = TextureOptions {
        magnification: egui::TextureFilter::Nearest,
        minification: egui::TextureFilter::Nearest,
        wrap_mode: egui::TextureWrapMode::Repeat,
        mipmap_mode: None,
    };
    let texture = ctx.load_texture("craft-ui-checker", ColorImage::new([2, 2], vec![a, b, b, a]), options);
    ctx.data_mut(|d| d.insert_temp(key, texture.clone()));
    texture
}

/// A textured quad with corners `pos` and texture coordinates `uv`, in the same order.
fn quad_mesh(texture: TextureId, pos: [Pos2; 4], uv: [Pos2; 4], color: Color32) -> Mesh {
    let mut mesh = Mesh::with_texture(texture);
    mesh.vertices.extend((0..4).map(|i| Vertex { pos: pos[i], uv: uv[i], color }));
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    mesh
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
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.id_salt);
        let size = source.size();
        let revision = source.revision();
        let bounds = PixelRect::of_size(size);
        let mut response = RasterResponse { id, read: None, detail: None };
        if bounds.is_empty() {
            Self::forget(&ctx, id);
            return response;
        }
        let side = texture_side(&ctx);
        let factor = texture_factor(size, side);
        let pending = ctx.data_mut(|d| d.remove_temp::<Pending>(damage_key(id))).map_or_else(PixelRect::default, |p| p.0).intersect(&bounds);
        let old = ctx.data(|d| d.get_temp::<State>(state_key(id))).filter(|s| s.size == size);
        let mut read = PixelRect::default();

        // The base texture: the whole document at one texel per `factor` pixels.
        let (base, base_fresh) = match &old {
            Some(s) => (s.base.clone(), false),
            None => {
                let image = read_region(source, bounds, factor);
                read = bounds;
                (ctx.load_texture("craft-ui-raster", image, texture_options(factor)), true)
            }
        };
        let changed = old.as_ref().is_none_or(|s| s.revision != revision);
        if !base_fresh && (changed || !pending.is_empty()) {
            let mut base = base.clone();
            if pending.is_empty() {
                base.set(read_region(source, bounds, factor), texture_options(factor));
                read = bounds;
            } else {
                let region = snap(pending, factor).intersect(&bounds);
                base.set_partial(texel_at(region, bounds, factor), read_region(source, region, factor), texture_options(factor));
                read = read.union(&region);
            }
        }

        // The detail texture: the part in view at the screen's resolution.
        let ppp = ctx.pixels_per_point();
        let device_zoom = view.zoom.max(view.zoom_x()) * f64::from(ppp);
        let plan = DetailPlan::of(size, side, device_zoom, PixelRect::visible(view, size));
        let kept = old.as_ref().and_then(|s| s.detail.clone());
        let detail = plan.map(|plan| {
            let options = texture_options(plan.factor);
            let reuse = kept.filter(|d| d.plan == plan);
            match reuse {
                Some(d) if !changed && pending.is_empty() => d,
                Some(mut d) if !pending.is_empty() => {
                    let region = snap(pending.intersect(&plan.region), plan.factor).intersect(&plan.region);
                    if !region.is_empty() {
                        d.texture.set_partial(texel_at(region, plan.region, plan.factor), read_region(source, region, plan.factor), options);
                        read = read.union(&region);
                    }
                    d
                }
                other => {
                    let image = read_region(source, plan.region, plan.factor);
                    read = read.union(&plan.region);
                    let texture = match other {
                        Some(mut d) if d.texture.size() == image.size => {
                            d.texture.set(image, options);
                            d.texture
                        }
                        _ => ctx.load_texture("craft-ui-raster-detail", image, options),
                    };
                    Detail { plan, texture }
                }
            }
        });
        response.detail = detail.as_ref().map(|d| d.plan);
        response.read = (!read.is_empty()).then_some(read);

        let painter = ui.painter_at(view.rect);
        if let Some(checker) = &self.checker {
            checker.paint(&painter, view, size);
        }
        draw(&painter, view, &base, factor, bounds);
        if let Some(d) = &detail {
            draw(&painter, view, &d.texture, d.plan.factor, d.plan.region);
        }
        ctx.data_mut(|d| d.insert_temp(state_key(id), State { size, revision, base, detail }));
        response
    }

    /// Mark `region` of the display `id` changed: the next [`show`](Self::show) re-reads it only
    /// (merged with other damage since the last pass).
    // pc: crates/ui-egui/src/canvas/textures.rs:322 `damage_since`.
    pub fn damage(ctx: &Context, id: Id, region: PixelRect) {
        ctx.data_mut(|d| {
            let pending = d.get_temp_mut_or_default::<Pending>(damage_key(id));
            pending.0 = pending.0.union(&region);
        });
    }

    /// Free the display's textures (the document closed).
    // pc: crates/ui-egui/src/canvas/textures.rs:667 `retain_gpu_documents` (the CPU side).
    pub fn forget(ctx: &Context, id: Id) {
        ctx.data_mut(|d| {
            d.remove::<State>(state_key(id));
            d.remove_temp::<Pending>(damage_key(id));
        });
    }
}
