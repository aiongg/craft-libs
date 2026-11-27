//! `Canvas`: where `Draw` and the shape painters record a component's paint, as craft-ui-rt display items in the
//! node's local coordinates (docs/contract-rt.md sections 5 and 8). Painters keep composing epaint shapes; the canvas
//! lowers each to the display item that paints the same pixels, and text arrives already shaped by the frame's
//! `TextEngine`. `Services` is the frame's text engine and image store, which text layout and pictures go through.

use crate::fonts::{FaceBytes, FaceState, StackFacts};
use craft_theme::Rgba;
use craft_ui_rt::{
    Clip, CornerRadii, DisplayList, Fill, ImageKey, ImageOptions, ImageStore, Item, Mesh, RgbaImage, RichText, Stroke as RtStroke, StrokeAlign,
    TextEngine, TextLayout as RtLayout, TextMetrics, Vertex,
};
use egui::epaint::{CircleShape, PathShape, RectShape, Shape, StrokeKind, TextureId};
use egui::{Color32, CornerRadius, Pos2, Rect, Stroke, Vec2};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

/// The frame's text engine and image store, as the code that records a paint or measures text reaches them.
pub trait Services {
    /// The window's scale.
    fn scale(&self) -> f32;
    /// What text layout reads of the installed font stack.
    fn fonts(&self) -> StackFacts;
    /// Shapes `text` at the window's scale.
    fn layout(&self, text: &RichText) -> RtLayout;
    /// The rows, glyphs and ink of `layout`.
    fn metrics(&self, layout: &RtLayout) -> TextMetrics;
    /// Registers the pixels `make` builds under `key`, sampled by `options`, unless `key` is already there.
    fn image(&self, key: ImageKey, options: ImageOptions, make: &mut dyn FnMut() -> Arc<RgbaImage>);
    /// The document face `key` over the interface face, its bytes from `load` the first time (`DocumentFaces`). An
    /// engine that installs no faces at run time leaves it unavailable.
    fn face_over_interface(&self, _key: &str, _load: &mut dyn FnMut() -> Option<FaceBytes>) -> FaceState {
        FaceState::Unavailable
    }
    /// The document face `key`, loaded by the job `job` gives (`DocumentFaces::face`).
    fn face(&self, _key: &str, _job: FaceJob<'_>) -> FaceState {
        FaceState::Unavailable
    }
    /// The bytes of the document face `key`, when it is installed.
    fn face_bytes(&self, _key: &str) -> Option<FaceBytes> {
        None
    }
    /// The largest image side the renderer uploads, in pixels.
    fn max_image_side(&self) -> u32 {
        8192
    }
    /// The egui context, while the egui runtime runs the pipeline: the texture store and thumbnails still load and
    /// wake through it. `None` on craft-ui-rt.
    fn egui(&self) -> Option<&egui::Context> {
        None
    }
}

/// What starts a document face's load: it returns the load, which runs off the frame.
pub type FaceJob<'a> = Box<dyn FnOnce() -> Box<dyn FnOnce() -> Option<FaceBytes> + Send> + 'a>;

/// [`Services`] over the frame's engine and store, lent to one run.
pub struct FrameServices<'f> {
    text: RefCell<Option<&'f mut dyn TextEngine>>,
    images: RefCell<&'f mut ImageStore>,
    scale: f32,
    fonts: StackFacts,
}

impl<'f> FrameServices<'f> {
    /// `text` is `None` where no engine is lent (layouts come out empty).
    pub fn new(text: Option<&'f mut dyn TextEngine>, images: &'f mut ImageStore, scale: f32) -> Self {
        FrameServices { text: RefCell::new(text), images: RefCell::new(images), scale, fonts: StackFacts::default() }
    }
}

fn empty_layout() -> RtLayout {
    RtLayout { handle: craft_ui_rt::TextHandle::new(Arc::new(())), size: Vec2::ZERO, baseline: 0.0, lines: 0 }
}

impl Services for FrameServices<'_> {
    fn scale(&self) -> f32 {
        self.scale
    }

    fn fonts(&self) -> StackFacts {
        self.fonts
    }

    fn layout(&self, text: &RichText) -> RtLayout {
        match self.text.borrow_mut().as_deref_mut() {
            Some(engine) => engine.layout_rich(text, self.scale),
            None => empty_layout(),
        }
    }

    fn metrics(&self, layout: &RtLayout) -> TextMetrics {
        match self.text.borrow_mut().as_deref_mut() {
            Some(engine) => engine.metrics(layout),
            None => TextMetrics { text: Arc::from(""), rows: Vec::new(), ink: Rect::NOTHING, elided: false },
        }
    }

    fn image(&self, key: ImageKey, options: ImageOptions, make: &mut dyn FnMut() -> Arc<RgbaImage>) {
        let mut images = self.images.borrow_mut();
        if !images.contains(key) {
            images.insert_with(key, make(), options);
        }
    }
}

/// No engine and no store: layouts come out empty.
struct NoServices;

impl Services for NoServices {
    fn scale(&self) -> f32 {
        1.0
    }

    fn fonts(&self) -> StackFacts {
        StackFacts::default()
    }

    fn layout(&self, _: &RichText) -> RtLayout {
        empty_layout()
    }

    fn metrics(&self, _: &RtLayout) -> TextMetrics {
        TextMetrics { text: Arc::from(""), rows: Vec::new(), ink: Rect::NOTHING, elided: false }
    }

    fn image(&self, _: ImageKey, _: ImageOptions, _: &mut dyn FnMut() -> Arc<RgbaImage>) {}
}

/// One component's paint as display items: the list under its children and the one over them, in the node's
/// coordinates (window coordinates less `origin`).
pub struct Canvas<'a> {
    content: RefCell<DisplayList>,
    overlay: RefCell<DisplayList>,
    over: Cell<bool>,
    /// The node's top-left corner in window coordinates; painters draw in window coordinates.
    origin: Vec2,
    /// The clips in force, innermost last, in window coordinates.
    clips: RefCell<Vec<Rect>>,
    /// The fade in force: every colour recorded is multiplied by it, as egui's painter opacity.
    opacity: Cell<f32>,
    services: ServicesRef<'a>,
}

/// The services a canvas records with: lent by the run, or owned (the egui runtime's context, a cheap clone).
enum ServicesRef<'a> {
    Lent(&'a dyn Services),
    Owned(Box<dyn Services + 'a>),
}

impl<'a> Canvas<'a> {
    /// A canvas for a node whose top-left corner is `origin`, painting inside `clip` (window coordinates).
    pub fn new(services: &'a dyn Services, origin: Pos2, clip: Rect) -> Canvas<'a> {
        Canvas {
            content: RefCell::new(DisplayList::default()),
            overlay: RefCell::new(DisplayList::default()),
            over: Cell::new(false),
            origin: origin.to_vec2(),
            clips: RefCell::new(vec![clip]),
            opacity: Cell::new(1.0),
            services: ServicesRef::Lent(services),
        }
    }

    /// A canvas owning its services.
    pub fn owned(services: impl Services + 'a, origin: Pos2, clip: Rect) -> Canvas<'a> {
        Canvas { services: ServicesRef::Owned(Box::new(services)), ..Canvas::new(&NoServices, origin, clip) }
    }

    /// The frame's text engine and image store.
    pub fn services(&self) -> &dyn Services {
        match &self.services {
            ServicesRef::Lent(services) => *services,
            ServicesRef::Owned(services) => services.as_ref(),
        }
    }

    /// The window's scale.
    pub fn pixels_per_point(&self) -> f32 {
        self.services().scale()
    }

    /// The clip in force, in window coordinates.
    pub fn clip_rect(&self) -> Rect {
        self.clips.borrow().last().copied().unwrap_or(Rect::EVERYTHING)
    }

    /// The recorded lists: under and over the node's children.
    pub fn finish(self) -> (DisplayList, DisplayList) {
        (self.content.into_inner(), self.overlay.into_inner())
    }

    /// Takes what was recorded so far, leaving the canvas empty.
    pub fn take(&self) -> (DisplayList, DisplayList) {
        (std::mem::take(&mut *self.content.borrow_mut()), std::mem::take(&mut *self.overlay.borrow_mut()))
    }

    /// Records `item` (local coordinates) at the current level.
    pub fn push(&self, item: Item) {
        if self.over.get() { self.overlay.borrow_mut().items.push(item) } else { self.content.borrow_mut().items.push(item) }
    }

    /// Records what `f` paints over the node's children.
    pub fn over<R>(&self, f: impl FnOnce() -> R) -> R {
        let saved = self.over.replace(true);
        let out = f();
        self.over.set(saved);
        out
    }

    /// Records what `f` paints cut to `rect` (window coordinates) inside the clip in force.
    pub fn clipped<R>(&self, rect: Rect, f: impl FnOnce() -> R) -> R {
        let clip = rect.intersect(self.clip_rect());
        self.clips.borrow_mut().push(clip);
        self.push(Item::PushClip(Clip::from(clip.translate(-self.origin))));
        let out = f();
        self.push(Item::PopClip);
        self.clips.borrow_mut().pop();
        out
    }

    /// Records what `f` paints `share` as opaque (stacking with the fade in force).
    pub fn faded<R>(&self, share: f32, f: impl FnOnce() -> R) -> R {
        let saved = self.opacity.get();
        if share.is_finite() {
            self.opacity.set(saved * share.clamp(0.0, 1.0));
        }
        let out = f();
        self.opacity.set(saved);
        out
    }

    /// A colour with the fade in force, as a display-list colour.
    fn color(&self, color: Color32) -> Rgba {
        let opacity = self.opacity.get();
        let color = if opacity < 1.0 { color.gamma_multiply(opacity) } else { color };
        Rgba(color.to_srgba_unmultiplied())
    }

    fn stroke(&self, stroke: Stroke, kind: StrokeKind) -> Option<RtStroke> {
        if stroke.is_empty() {
            return None;
        }
        let align = match kind {
            StrokeKind::Inside => StrokeAlign::Inside,
            StrokeKind::Middle => StrokeAlign::Middle,
            StrokeKind::Outside => StrokeAlign::Outside,
        };
        Some(RtStroke::new(stroke.width, self.color(stroke.color)).aligned(align))
    }

    fn local(&self, pos: Pos2) -> Pos2 {
        pos - self.origin
    }

    /// Records a laid-out text with its top-left corner at `pos` (window coordinates).
    pub fn text(&self, layout: &RtLayout, pos: Pos2) {
        let origin = self.local(pos);
        let opacity = self.opacity.get();
        let item = Item::Text { layout: layout.handle.clone(), origin, size: layout.size, color: None, highlight: None };
        if opacity < 1.0 {
            self.push(Item::PushOpacity { opacity, bounds: Rect::from_min_size(origin, layout.size) });
            self.push(item);
            self.push(Item::PopOpacity);
        } else {
            self.push(item);
        }
    }

    /// Records `image` in `rect`, its part `uv`, multiplied by `tint`.
    pub fn image(&self, key: ImageKey, rect: Rect, uv: Rect, tint: Color32) {
        self.push(Item::Image { key, rect: rect.translate(-self.origin), uv, tint: self.color(tint) });
    }

    /// Records `color` through the alpha of `image` in `rect`.
    pub fn image_mask(&self, key: ImageKey, rect: Rect, uv: Rect, color: Color32) {
        self.push(Item::ImageMask { key, rect: rect.translate(-self.origin), uv, color: self.color(color) });
    }

    /// Records `shape` (window coordinates) as the display items that paint it.
    pub fn add(&self, shape: impl Into<Shape>) {
        self.lower(shape.into());
    }

    /// Records each of `shapes`.
    pub fn extend<I: IntoIterator<Item = Shape>>(&self, shapes: I) {
        for shape in shapes {
            self.lower(shape);
        }
    }

    /// A filled rounded rect.
    pub fn rect_filled(&self, rect: Rect, radius: impl Into<CornerRadius>, color: impl Into<Color32>) {
        self.add(RectShape::filled(rect, radius, color));
    }

    /// A filled circle.
    pub fn circle_filled(&self, center: Pos2, radius: f32, color: impl Into<Color32>) {
        self.add(CircleShape::filled(center, radius, color));
    }

    /// A line segment.
    pub fn line_segment(&self, points: [Pos2; 2], stroke: impl Into<Stroke>) {
        self.add(Shape::line_segment(points, stroke));
    }

    fn lower(&self, shape: Shape) {
        match shape {
            Shape::Noop => {}
            Shape::Vec(shapes) => self.extend(shapes),
            Shape::Rect(r) => self.lower_rect(&r),
            Shape::Circle(c) => {
                let fill = self.color(c.fill);
                let stroke = self.stroke(c.stroke, StrokeKind::Outside);
                // egui strokes a circle centred on its radius.
                let stroke = stroke.map(|s| s.aligned(StrokeAlign::Middle));
                self.push(Item::Circle { center: self.local(c.center), radius: c.radius, fill, stroke });
            }
            Shape::Ellipse(e) => {
                let n = 64;
                let points: Vec<Pos2> = (0..n)
                    .map(|i| {
                        let a = std::f32::consts::TAU * i as f32 / n as f32;
                        e.center + Vec2::new(e.radius.x * a.cos(), e.radius.y * a.sin())
                    })
                    .collect();
                self.lower(Shape::Path(PathShape { points, closed: true, fill: e.fill, stroke: e.stroke.into() }));
            }
            Shape::LineSegment { points, stroke } => {
                if let Some(stroke) = self.stroke(stroke, StrokeKind::Middle) {
                    self.push(Item::Line { from: self.local(points[0]), to: self.local(points[1]), stroke });
                }
            }
            Shape::Path(p) => {
                if p.points.len() < 2 {
                    return;
                }
                let mut path = craft_behavior::kurbo::BezPath::new();
                let at = |q: Pos2| {
                    let q = self.local(q);
                    craft_behavior::kurbo::Point::new(f64::from(q.x), f64::from(q.y))
                };
                path.move_to(at(p.points[0]));
                for &q in &p.points[1..] {
                    path.line_to(at(q));
                }
                if p.closed {
                    path.close_path();
                }
                let fill = (p.closed && p.fill != Color32::TRANSPARENT).then(|| Fill::convex(self.color(p.fill)));
                let stroke = match &p.stroke.color {
                    egui::epaint::ColorMode::Solid(color) => self.stroke(Stroke::new(p.stroke.width, *color), p.stroke.kind),
                    egui::epaint::ColorMode::UV(_) => None,
                };
                if fill.is_some() || stroke.is_some() {
                    self.push(Item::Path { path: Arc::new(path), fill, stroke });
                }
            }
            Shape::Mesh(mesh) => {
                let texture = match mesh.texture_id {
                    TextureId::User(id) => Some(ImageKey(id)),
                    TextureId::Managed(_) => None,
                };
                let opacity = self.opacity.get();
                let vertices = mesh
                    .vertices
                    .iter()
                    .map(|v| {
                        let color = if opacity < 1.0 { v.color.gamma_multiply(opacity) } else { v.color };
                        Vertex { pos: self.local(v.pos), uv: v.uv, color: Rgba(color.to_srgba_unmultiplied()) }
                    })
                    .collect();
                self.push(Item::Mesh(Arc::new(Mesh { vertices, indices: mesh.indices.clone(), texture })));
            }
            // Text reaches the canvas shaped by the frame's engine (`Canvas::text`); quadratic and cubic curves and
            // callbacks are not painted by the shape painters.
            Shape::Text(_) | Shape::Band(_) | Shape::QuadraticBezier(_) | Shape::CubicBezier(_) | Shape::Callback(_) => {}
        }
    }

    fn lower_rect(&self, r: &RectShape) {
        let radii = radii(r.corner_radius);
        let rect = r.rect.translate(-self.origin);
        if r.blur_width > 0.0 {
            self.push(Item::Shadow { rect, radii, offset: Vec2::ZERO, blur: r.blur_width, spread: 0.0, color: self.color(r.fill) });
            return;
        }
        if let Some(brush) = &r.brush {
            if let TextureId::User(id) = brush.fill_texture_id {
                self.push(Item::Image { key: ImageKey(id), rect, uv: brush.uv, tint: self.color(r.fill) });
                return;
            }
        }
        self.push(Item::Rect { rect, radii, fill: self.color(r.fill), stroke: self.stroke(r.stroke, r.stroke_kind) });
    }
}

/// egui's whole-point corner radii as display-list radii.
pub(crate) fn radii(r: CornerRadius) -> CornerRadii {
    CornerRadii { top_left: f32::from(r.nw), top_right: f32::from(r.ne), bottom_right: f32::from(r.se), bottom_left: f32::from(r.sw) }
}

/// The image key of a picture keyed by `key` in the app's or the framework's own key space.
pub fn image_key<K: std::hash::Hash + 'static>(key: &K) -> ImageKey {
    ImageKey(craft_ui_rt::hash_key(&(std::any::TypeId::of::<K>(), key)))
}

/// An egui colour image as display-list pixels (straight alpha).
pub fn rgba_image(image: &egui::ColorImage) -> Arc<RgbaImage> {
    let mut pixels = Vec::with_capacity(image.pixels.len() * 4);
    for p in &image.pixels {
        pixels.extend_from_slice(&p.to_srgba_unmultiplied());
    }
    Arc::new(RgbaImage { size: craft_ui_rt::SizeU::new(image.size[0] as u32, image.size[1] as u32), pixels })
}
