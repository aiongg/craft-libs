//! The renderer-neutral display list. Coordinates are logical points in the node's own space; the
//! frame tree supplies each node's offset, clip and opacity.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use craft_theme::Rgba;
use emath::{Pos2, Rect, Vec2};
use kurbo::{BezPath, Shape};

use crate::geom::{CornerRadii, RectU, SizeU};
use crate::text::TextHandle;

/// Where a stroke sits relative to the shape's edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StrokeAlign {
    #[default]
    Inside,
    Middle,
    Outside,
}

/// A dash pattern along a stroke, in logical points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dash {
    pub on: f32,
    pub off: f32,
    /// How far into the pattern the stroke starts.
    pub offset: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub color: Rgba,
    pub align: StrokeAlign,
    /// Solid when `None`.
    pub dash: Option<Dash>,
}

impl Stroke {
    pub const fn new(width: f32, color: Rgba) -> Stroke {
        Stroke { width, color, align: StrokeAlign::Inside, dash: None }
    }

    pub const fn aligned(mut self, align: StrokeAlign) -> Stroke {
        self.align = align;
        self
    }

    pub const fn dashed(mut self, dash: Dash) -> Stroke {
        self.dash = Some(dash);
        self
    }

    /// How far the stroke reaches outside the shape's edge.
    fn outset(&self) -> f32 {
        match self.align {
            StrokeAlign::Inside => 0.0,
            StrokeAlign::Middle => self.width / 2.0,
            StrokeAlign::Outside => self.width,
        }
    }
}

/// How a path's inside is decided.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// A path fill. `convex` promises the path is one convex contour, which lets a backend whose
/// tessellator fills only convex shapes (epaint) take its fast path; a concave or multi-contour
/// path must leave it false.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fill {
    pub color: Rgba,
    pub rule: FillRule,
    pub convex: bool,
}

impl Fill {
    /// Any shape, filled by the non-zero rule.
    pub const fn new(color: Rgba) -> Fill {
        Fill { color, rule: FillRule::NonZero, convex: false }
    }

    pub const fn convex(color: Rgba) -> Fill {
        Fill { color, rule: FillRule::NonZero, convex: true }
    }
}

/// An image the app registered in the [`ImageStore`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageKey(pub u64);

/// A surface the app draws itself (a document canvas): the renderer asks the producer registered
/// for the key to draw it inside the item's rect. Registration is the renderer's API, since what a
/// producer gets (a wgpu pass, CPU tiles) depends on the backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfaceKey(pub u64);

/// One vertex of a [`Mesh`]: position in the node's space, texture coordinate (normalised; any
/// value when the mesh has no texture) and a straight-alpha sRGB colour that multiplies the
/// texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub pos: Pos2,
    pub uv: Pos2,
    pub color: Rgba,
}

/// Triangles with vertex colours: colour wheels, ramps, anything the other items can't say.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    /// Three per triangle.
    pub indices: Vec<u32>,
    pub texture: Option<ImageKey>,
}

impl Mesh {
    /// An empty mesh sampling `texture`.
    pub fn with_texture(texture: ImageKey) -> Mesh {
        Mesh { texture: Some(texture), ..Mesh::default() }
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty() || self.vertices.is_empty()
    }

    /// Adds an untextured vertex of `color`.
    pub fn colored_vertex(&mut self, pos: Pos2, color: Rgba) {
        self.vertices.push(Vertex { pos, uv: Pos2::ZERO, color });
    }

    pub fn add_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend([a, b, c]);
    }

    /// Adds `rect` filled with `color` as two triangles.
    pub fn add_colored_rect(&mut self, rect: Rect, color: Rgba) {
        let base = self.vertices.len() as u32;
        for p in [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()] {
            self.colored_vertex(p, color);
        }
        self.add_triangle(base, base + 1, base + 2);
        self.add_triangle(base, base + 2, base + 3);
    }

    /// Adds `other`'s triangles (its texture must be this mesh's).
    pub fn append(&mut self, other: Mesh) {
        let base = self.vertices.len() as u32;
        self.vertices.extend(other.vertices);
        self.indices.extend(other.indices.into_iter().map(|i| base + i));
    }

    pub fn bounds(&self) -> Option<Rect> {
        let mut it = self.vertices.iter();
        let first = it.next()?;
        Some(it.fold(Rect::from_min_max(first.pos, first.pos), |r, v| r.union(Rect::from_min_max(v.pos, v.pos))))
    }
}

/// A clip: a rect with optional rounded corners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clip {
    pub rect: Rect,
    pub radii: CornerRadii,
}

impl From<Rect> for Clip {
    fn from(rect: Rect) -> Clip {
        Clip { rect, radii: CornerRadii::ZERO }
    }
}

#[derive(Clone, Debug)]
pub enum Item {
    /// A rectangle with optional rounded corners; a fully transparent fill draws no fill.
    Rect {
        rect: Rect,
        radii: CornerRadii,
        fill: Rgba,
        stroke: Option<Stroke>,
    },
    Circle {
        center: Pos2,
        radius: f32,
        fill: Rgba,
        stroke: Option<Stroke>,
    },
    Line {
        from: Pos2,
        to: Pos2,
        stroke: Stroke,
    },
    /// kurbo's path, in the node's space (f64, as kurbo has it).
    Path {
        path: Arc<BezPath>,
        fill: Option<Fill>,
        stroke: Option<Stroke>,
    },
    Mesh(Arc<Mesh>),
    /// A soft shadow of a rounded rect: `rect` moved by `offset`, grown by `spread`, blurred over
    /// `blur` (the Gaussian's extent, as CSS box-shadow's blur radius).
    Shadow {
        rect: Rect,
        radii: CornerRadii,
        offset: Vec2,
        blur: f32,
        spread: f32,
        color: Rgba,
    },
    /// `uv` is the normalised source rect; `tint` multiplies the image.
    Image {
        key: ImageKey,
        rect: Rect,
        uv: Rect,
        tint: Rgba,
    },
    /// `color` painted through the image's alpha (an icon mask in any colour).
    ImageMask {
        key: ImageKey,
        rect: Rect,
        uv: Rect,
        color: Rgba,
    },
    /// A vertical linear gradient over `rect`, from `top` at its top edge to `bottom` at its
    /// bottom edge, interpolated in premultiplied sRGB.
    VGradient {
        rect: Rect,
        top: Rgba,
        bottom: Rgba,
    },
    /// A laid-out text at `origin` (the layout's top left), `size` its layout size. `color`
    /// replaces the layout's colour; `highlight` fills a range of it behind its glyphs. `angle`
    /// turns the text about `origin`, in radians, clockwise on screen (0 for level text).
    Text {
        layout: TextHandle,
        origin: Pos2,
        size: Vec2,
        color: Option<Rgba>,
        highlight: Option<TextHighlight>,
        angle: f32,
    },
    /// The app-drawn surface `key` inside `rect`.
    Surface {
        key: SurfaceKey,
        rect: Rect,
    },
    /// A place in the list for items filled later by code with no painter ([`crate::BuildCx::fill_slot`]): the
    /// node keeps slot `key`'s items beside its display lists and paints them here. ui-rt puts them in place
    /// before a list reaches a renderer, so renderers never see a slot.
    Slot(u64),
    PushClip(Clip),
    PopClip,
    /// Items up to the matching [`Item::PopOpacity`] composite as one group at `opacity`. Nothing
    /// in the group paints outside `bounds`, so a backend may composite it through a layer of
    /// that size.
    PushOpacity {
        opacity: f32,
        bounds: Rect,
    },
    PopOpacity,
}

/// A filled range of a text (a selection), painted with the text: line by line, each line's full
/// height, where the renderer puts the glyphs, so a renderer that snaps text to pixels snaps the
/// fill with it.
#[derive(Clone, Debug, PartialEq)]
pub struct TextHighlight {
    /// Byte range into the layout's text.
    pub range: Range<usize>,
    pub fill: Rgba,
}

/// Room around every item's geometry for anti-aliasing, in logical points.
const AA_MARGIN: f32 = 1.0;

/// Room around a text's layout box for glyphs that reach past it, as a share of its height.
const GLYPH_OVERHANG: f32 = 0.25;

/// How far a shadow's blur reaches past its grown rect, as a multiple of `blur`; a Gaussian with
/// sigma `blur / 2` is below 1/255 there.
const SHADOW_REACH: f32 = 1.5;

fn kurbo_rect(r: kurbo::Rect) -> Rect {
    Rect::from_min_max(emath::pos2(r.x0 as f32, r.y0 as f32), emath::pos2(r.x1 as f32, r.y1 as f32))
}

impl Item {
    /// What the item can paint, in local coordinates: geometry, stroke and anti-aliasing. Clips
    /// inside the list are not applied, so the bounds may be larger than the pixels drawn.
    pub fn ink(&self) -> Option<Rect> {
        let outset = |s: &Option<Stroke>| s.map_or(0.0, |s| s.outset());
        let r = match self {
            Item::Rect { rect, stroke, .. } => rect.expand(outset(stroke)),
            Item::Circle { center, radius, stroke, .. } => {
                let r = radius + outset(stroke);
                Rect::from_center_size(*center, Vec2::splat(r * 2.0))
            }
            Item::Line { from, to, stroke } => Rect::from_two_pos(*from, *to).expand(stroke.width),
            Item::Path { path, stroke, .. } => kurbo_rect(path.bounding_box()).expand(stroke.map_or(0.0, |s| s.width)),
            Item::Mesh(mesh) => mesh.bounds()?,
            Item::Shadow { rect, offset, blur, spread, .. } => rect.translate(*offset).expand(spread.max(0.0) + blur * SHADOW_REACH),
            Item::Image { rect, .. } | Item::ImageMask { rect, .. } | Item::VGradient { rect, .. } | Item::Surface { rect, .. } => *rect,
            Item::Text { origin, size, angle, .. } => {
                let reach = size.y * GLYPH_OVERHANG;
                if *angle == 0.0 {
                    Rect::from_min_size(*origin, *size).expand(reach)
                } else {
                    // The turned box's corners, about the origin.
                    let rot = emath::Rot2::from_angle(*angle);
                    let corners = [Vec2::ZERO, Vec2::new(size.x, 0.0), *size, Vec2::new(0.0, size.y)].map(|c| *origin + rot * c);
                    Rect::from_points(&corners).expand(reach)
                }
            }
            Item::Slot(_) | Item::PushClip(_) | Item::PopClip | Item::PushOpacity { .. } | Item::PopOpacity => return None,
        };
        // Rects built from flipped corners are normalised before the margin goes on.
        Some(Rect::from_two_pos(r.min, r.max).expand(AA_MARGIN))
    }

    /// The image or surface the item shows, with its rect: updates to it damage the item.
    pub fn media(&self) -> Option<(Media, Rect)> {
        match self {
            Item::Image { key, rect, .. } | Item::ImageMask { key, rect, .. } => Some((Media::Image(*key), *rect)),
            Item::Mesh(mesh) => Some((Media::Image(mesh.texture?), mesh.bounds()?)),
            Item::Surface { key, rect } => Some((Media::Surface(*key), *rect)),
            _ => None,
        }
    }
}

/// Content a display list names by key, which changes without the list being re-recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Media {
    Image(ImageKey),
    Surface(SurfaceKey),
}

#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    pub items: Vec<Item>,
}

impl DisplayList {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The union of the items' [`Item::ink`]; `None` when nothing paints.
    pub fn ink(&self) -> Option<Rect> {
        self.items.iter().filter_map(Item::ink).reduce(|a, b| a.union(b))
    }
}

/// An sRGB image with straight alpha, rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    pub size: SizeU,
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    pub fn new(size: SizeU) -> RgbaImage {
        RgbaImage { size, pixels: vec![0; size.width as usize * size.height as usize * 4] }
    }

    /// Copies `src` (tightly packed rows of `rect`'s size) into `rect`. Returns false, changing
    /// nothing, when the rect is outside the image or the pixel count does not match.
    pub fn write(&mut self, rect: RectU, src: &[u8]) -> bool {
        let row = rect.width() as usize * 4;
        if rect.is_empty() || !rect.within(RectU::of_size(self.size)) || src.len() != row * rect.height() as usize {
            return false;
        }
        let stride = self.size.width as usize * 4;
        for (i, y) in (rect.y0..rect.y1).enumerate() {
            let at = y as usize * stride + rect.x0 as usize * 4;
            self.pixels[at..at + row].copy_from_slice(&src[i * row..(i + 1) * row]);
        }
        true
    }
}

/// How a renderer samples an image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Filter {
    #[default]
    Linear,
    /// Pixel art and zoomed-in document pixels.
    Nearest,
}

/// What a renderer samples outside the image's normalised `0..1` coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Wrap {
    /// The edge pixels.
    #[default]
    Clamp,
    /// The image again, tiled: a pattern (the transparency checkerboard) painted as one image.
    Repeat,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ImageOptions {
    /// When the image is drawn larger than its pixels.
    pub magnify: Filter,
    /// When it is drawn smaller.
    pub minify: Filter,
    /// Build mipmaps (thumbnails drawn far below their size).
    pub mipmaps: bool,
    /// Outside the image's coordinates.
    pub wrap: Wrap,
}

/// One change to the [`ImageStore`] since the renderers last synced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageChange {
    /// New or replaced: upload the whole image with its options.
    Set(ImageKey),
    /// Pixels inside the rect changed: upload that part.
    Pixels(ImageKey, RectU),
    Removed(ImageKey),
}

#[derive(Clone)]
struct StoredImage {
    image: Arc<RgbaImage>,
    options: ImageOptions,
}

/// Images by key, shared by every window. Renderers apply [`ImageStore::changes`] once per frame
/// (`Renderer::sync_images`) and then draw the frame's windows.
///
/// A frame uploads at most [`ImageStore::upload_budget`] bytes of new images: an insert past it
/// waits, invisible to [`ImageStore::contains`], and enters the store in a later frame (the
/// runtime asks for that frame itself), so a burst of thumbnails never spikes one frame.
pub struct ImageStore {
    images: HashMap<ImageKey, StoredImage>,
    changes: Vec<ImageChange>,
    /// Keys whose pixels changed without a re-recorded display list: the runtime damages their
    /// draws.
    pub(crate) touched: Vec<ImageKey>,
    budget: usize,
    /// Bytes of whole images set since the renderers last synced.
    sent: usize,
    /// Inserts past this frame's budget, oldest first.
    waiting: Vec<(ImageKey, StoredImage)>,
}

/// The bytes of new images one frame uploads by default.
pub const UPLOAD_BUDGET: usize = 16 * 1024 * 1024;

impl Default for ImageStore {
    fn default() -> Self {
        ImageStore { images: HashMap::new(), changes: Vec::new(), touched: Vec::new(), budget: UPLOAD_BUDGET, sent: 0, waiting: Vec::new() }
    }
}

impl ImageStore {
    pub fn insert(&mut self, key: ImageKey, image: Arc<RgbaImage>) {
        self.insert_with(key, image, ImageOptions::default());
    }

    /// Stores `image` under `key` for the renderers to upload, in this frame when the frame's
    /// upload budget has room (the first image of a frame always does), else in the first frame
    /// that has room; a replaced image shows until then.
    pub fn insert_with(&mut self, key: ImageKey, image: Arc<RgbaImage>, options: ImageOptions) {
        let stored = StoredImage { image, options };
        if let Some(at) = self.waiting.iter().position(|(k, _)| *k == key) {
            self.waiting[at].1 = stored;
        } else if self.waiting.is_empty() && self.fits(stored.image.pixels.len()) {
            self.set(key, stored);
        } else {
            self.waiting.push((key, stored));
        }
    }

    fn set(&mut self, key: ImageKey, stored: StoredImage) {
        self.sent = self.sent.saturating_add(stored.image.pixels.len());
        self.images.insert(key, stored);
        self.changes.retain(|c| !matches!(c, ImageChange::Pixels(k, _) if *k == key));
        self.changes.push(ImageChange::Set(key));
        self.touched.push(key);
    }

    /// Whether `bytes` more fit this frame's budget; the first image of a frame always does.
    fn fits(&self, bytes: usize) -> bool {
        self.sent == 0 || self.sent.saturating_add(bytes) <= self.budget
    }

    /// Whether this frame takes more images: nothing waits and the frame has sent less than its
    /// budget. A component that makes pictures on the UI thread makes no more in a frame without
    /// room.
    pub fn has_room(&self) -> bool {
        self.waiting.is_empty() && self.sent < self.budget
    }

    /// The bytes of new images one frame uploads.
    pub fn upload_budget(&self) -> usize {
        self.budget
    }

    /// Sets the bytes of new images one frame uploads.
    pub fn set_upload_budget(&mut self, bytes: usize) {
        self.budget = bytes;
    }

    /// How many inserts wait for a later frame.
    pub fn waiting(&self) -> usize {
        self.waiting.len()
    }

    /// Writes `pixels` (tightly packed rows) into `rect` of image `key`; renderers upload that
    /// part only. Returns false when there is no such image or the rect does not fit.
    pub fn update(&mut self, key: ImageKey, rect: RectU, pixels: &[u8]) -> bool {
        if let Some((_, stored)) = self.waiting.iter_mut().find(|(k, _)| *k == key) {
            return Arc::make_mut(&mut stored.image).write(rect, pixels);
        }
        let Some(stored) = self.images.get_mut(&key) else {
            return false;
        };
        if !Arc::make_mut(&mut stored.image).write(rect, pixels) {
            return false;
        }
        let set = self.changes.contains(&ImageChange::Set(key));
        if !set {
            self.changes.push(ImageChange::Pixels(key, rect));
        }
        self.touched.push(key);
        true
    }

    pub fn set_options(&mut self, key: ImageKey, options: ImageOptions) {
        if let Some(stored) = self.images.get_mut(&key)
            && stored.options != options
        {
            stored.options = options;
            self.changes.push(ImageChange::Set(key));
            self.touched.push(key);
        }
    }

    pub fn remove(&mut self, key: ImageKey) {
        self.waiting.retain(|(k, _)| *k != key);
        if self.images.remove(&key).is_some() {
            self.changes.retain(|c| !matches!(c, ImageChange::Set(k) | ImageChange::Pixels(k, _) if *k == key));
            self.changes.push(ImageChange::Removed(key));
        }
    }

    pub fn contains(&self, key: ImageKey) -> bool {
        self.images.contains_key(&key)
    }

    pub fn get(&self, key: ImageKey) -> Option<&Arc<RgbaImage>> {
        self.images.get(&key).map(|s| &s.image)
    }

    pub fn options(&self, key: ImageKey) -> ImageOptions {
        self.images.get(&key).map(|s| s.options).unwrap_or_default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ImageKey, &Arc<RgbaImage>)> {
        self.images.iter().map(|(k, v)| (*k, &v.image))
    }

    /// Changes since the renderers last synced, in order.
    pub fn changes(&self) -> &[ImageChange] {
        &self.changes
    }

    /// Ends the frame's uploads: the next frame's budget starts, and the inserts that waited
    /// enter the store, oldest first, while it has room. Each one entered is touched, so the
    /// runtime runs a frame for it.
    pub(crate) fn clear_changes(&mut self) {
        self.changes.clear();
        self.sent = 0;
        let mut waiting = std::mem::take(&mut self.waiting).into_iter().peekable();
        while let Some((key, stored)) = waiting.next_if(|(_, stored)| self.fits(stored.image.pixels.len())) {
            self.set(key, stored);
        }
        self.waiting.extend(waiting);
    }
}
