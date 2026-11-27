//! `Canvas`: where `Draw` and the shape painters record a component's paint, as craft-ui-rt display items in the
//! node's local coordinates (docs/contract-rt.md sections 5 and 8). Painters build items in window coordinates
//! (`items`); the canvas moves them into the node's and applies the fade in force. Text arrives shaped by the frame's
//! `TextEngine`. `Services` is the frame's text engine and image store, which text layout and pictures go through.

use crate::fonts::document::Kind;
use crate::fonts::{FaceBytes, FaceState, FontRef, StackFacts};
use craft_theme::Rgba;
use craft_ui_rt::{
    Clip, DisplayList, FaceInstall, FontFace, ImageKey, ImageOptions, ImageStore, Item, RgbaImage, RichText, TextEngine, TextLayout as RtLayout,
    TextMetrics,
};
use craft_ui_rt::{Pos2, Rect, Vec2};
use egui::{Color32, CornerRadius};
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
    /// Registers what `make` builds under `key` as [`image`](Self::image) does, `make` giving `None` when there is
    /// nothing to draw (an icon layer with no part); returns whether `key` holds a picture to draw. A store that keeps
    /// no pictures has none.
    fn image_if(&self, _key: ImageKey, _options: ImageOptions, _make: &mut dyn FnMut() -> Option<Arc<RgbaImage>>) -> bool {
        false
    }
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
    /// `text` is `None` where no engine is lent (layouts come out empty). `fonts` are the window's stack facts
    /// (`fonts::stack_facts`, a recorded read).
    pub fn new(text: Option<&'f mut dyn TextEngine>, images: &'f mut ImageStore, scale: f32, fonts: StackFacts) -> Self {
        FrameServices { text: RefCell::new(text), images: RefCell::new(images), scale, fonts }
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

    fn image_if(&self, key: ImageKey, options: ImageOptions, make: &mut dyn FnMut() -> Option<Arc<RgbaImage>>) -> bool {
        let mut images = self.images.borrow_mut();
        if !images.contains(key) {
            // Nothing to draw is kept as one clear texel, so the next frame knows without building it again.
            let image = make().unwrap_or_else(|| Arc::new(RgbaImage::new(craft_ui_rt::SizeU::new(1, 1))));
            images.insert_with(key, image, options);
        }
        images.get(key).is_some_and(|image| !(image.size == craft_ui_rt::SizeU::new(1, 1) && image.pixels[3] == 0))
    }

    /// The face from `load`, installed in the engine the first time `key` is asked for, over the interface face.
    fn face_over_interface(&self, key: &str, load: &mut dyn FnMut() -> Option<FaceBytes>) -> FaceState {
        let family = Kind::OverInterface.family_name(key);
        self.install(&family, true, &mut || load())
    }

    /// The face the job loads, installed alone. The load runs here, in the frame: craft-ui-rt has no job queue for it
    /// yet (B9), so the first frame that asks waits for the bytes.
    fn face(&self, key: &str, job: FaceJob<'_>) -> FaceState {
        let family = Kind::Alone.family_name(key);
        let mut job = Some(job);
        self.install(&family, false, &mut || job.take().and_then(|job| job()()))
    }
}

impl FrameServices<'_> {
    /// Installs the face `load` gives as `family` unless the engine has it: then its fallbacks, its chain of document
    /// faces and, `over_interface`, the interface face.
    fn install(&self, family: &str, over_interface: bool, load: &mut dyn FnMut() -> Option<FaceBytes>) -> FaceState {
        let mut text = self.text.borrow_mut();
        let Some(engine) = text.as_deref_mut() else { return FaceState::Unavailable };
        if engine.has_face(family) {
            return FaceState::Ready(FontRef::named(family));
        }
        let Some(face) = load().filter(FaceBytes::parses) else { return FaceState::Unavailable };
        let mut fallback: Vec<FontFace> = face.chain_families().into_iter().map(|name| FontFace::Family(name.into())).collect();
        if over_interface {
            fallback.push(FontFace::Slot(craft_theme::FontSlot::Ui));
        }
        let install = FaceInstall { family: family.into(), bytes: Arc::clone(&face.bytes), index: face.index, coords: face.coords.clone(), fallback };
        if engine.install_face(install) { FaceState::Ready(FontRef::named(family)) } else { FaceState::Unavailable }
    }
}

/// No engine and no store: layouts come out empty.
pub(crate) struct NoServices;

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

/// A place in a canvas's record ([`Canvas::mark`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasMark {
    over: bool,
    at: usize,
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

/// The canvas a `Draw` records into: lent by the code that made it (a paint hook's canvas), or owned by the `Draw`
/// (`Ui::draw` in a build), which hands what it recorded to its sink when it drops.
pub(crate) enum CanvasRef<'a> {
    Lent(&'a Canvas<'a>),
    Owned(OwnedCanvas<'a>),
}

impl<'a> std::ops::Deref for CanvasRef<'a> {
    type Target = Canvas<'a>;

    fn deref(&self) -> &Canvas<'a> {
        match self {
            CanvasRef::Lent(canvas) => canvas,
            CanvasRef::Owned(owned) => &owned.canvas,
        }
    }
}

/// A canvas a build records into through `Ui::draw`, in window coordinates: when it drops, its lists join the build's
/// drawn lists (`Flow::drawn`, under and over the node's children), which the driver moves into the node's paint at the
/// end of the build.
pub(crate) struct OwnedCanvas<'a> {
    canvas: Canvas<'a>,
    sink: &'a mut (DisplayList, DisplayList),
    /// The canvas pushed a clip of its own, which it pops before handing its lists over.
    clipped: bool,
}

impl<'a> OwnedCanvas<'a> {
    /// `canvas` (origin at the window's corner) handing its lists to `sink`; with `clip`, what it records is cut to it.
    pub(crate) fn new(canvas: Canvas<'a>, clip: Option<Rect>, sink: &'a mut (DisplayList, DisplayList)) -> OwnedCanvas<'a> {
        if let Some(clip) = clip {
            canvas.push_clip(clip);
            let local = canvas.clip_rect().translate(-canvas.origin);
            canvas.over(|| canvas.push(Item::PushClip(Clip::from(local))));
        }
        OwnedCanvas { canvas, sink, clipped: clip.is_some() }
    }
}

impl Drop for OwnedCanvas<'_> {
    fn drop(&mut self) {
        if self.clipped {
            self.canvas.over(|| self.canvas.push(Item::PopClip));
            self.canvas.pop_clip();
        }
        let (content, overlay) = self.canvas.take();
        self.sink.0.items.extend(content.items);
        self.sink.1.items.extend(overlay.items);
    }
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

    /// The same canvas recording at `opacity` (a build cursor's fade).
    pub(crate) fn at_opacity(self, opacity: f32) -> Canvas<'a> {
        if opacity.is_finite() {
            self.opacity.set(opacity.clamp(0.0, 1.0));
        }
        self
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

    /// The place the next recorded item goes: [`take_since`](Self::take_since) takes what is recorded after it. A
    /// painter that must put its own items under what a body paints after it (a picture under the controls) marks
    /// first, lets the body paint, takes the body's items, records its own, and puts the body's back.
    pub fn mark(&self) -> CanvasMark {
        let over = self.over.get();
        let at = if over { self.overlay.borrow().items.len() } else { self.content.borrow().items.len() };
        CanvasMark { over, at }
    }

    /// Whether anything was recorded at `mark`'s level since it.
    pub(crate) fn painted_since(&self, mark: CanvasMark) -> bool {
        let list = if mark.over { &self.overlay } else { &self.content };
        list.borrow().items.len() > mark.at
    }

    /// Takes out the items recorded since `mark`, in window coordinates and the fade already applied: recording them
    /// again with [`extend`](Self::extend) puts them back at the end, as drawn.
    pub fn take_since(&self, mark: CanvasMark) -> Vec<Item> {
        let list = if mark.over { &self.overlay } else { &self.content };
        let mut list = list.borrow_mut();
        let at = mark.at.min(list.items.len());
        let mut taken = list.items.split_off(at);
        for item in &mut taken {
            super::items::translate(item, self.origin);
        }
        taken
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
        self.push_clip(rect);
        let out = f();
        self.pop_clip();
        out
    }

    /// Cuts what is recorded from here on to `rect` (window coordinates) inside the clip in force, until the matching
    /// [`pop_clip`](Self::pop_clip).
    pub(crate) fn push_clip(&self, rect: Rect) {
        let clip = rect.intersect(self.clip_rect());
        self.clips.borrow_mut().push(clip);
        self.push(Item::PushClip(Clip::from(clip.translate(-self.origin))));
    }

    /// Ends the clip of the last [`push_clip`](Self::push_clip).
    pub(crate) fn pop_clip(&self) {
        self.push(Item::PopClip);
        self.clips.borrow_mut().pop();
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

    /// A colour with the fade in force.
    fn faded_color(&self, color: Rgba) -> Rgba {
        let opacity = self.opacity.get();
        if opacity < 1.0 { super::items::fade(color, opacity) } else { color }
    }

    /// Records a laid-out text with its top-left corner at `pos` (window coordinates).
    pub fn text(&self, layout: &RtLayout, pos: Pos2) {
        self.text_turned(layout, pos, 0.0);
    }

    /// Records a laid-out text turned by `angle` radians (clockwise on screen) about its top-left corner `pos`.
    pub fn text_turned(&self, layout: &RtLayout, pos: Pos2, angle: f32) {
        self.add(Item::Text { layout: layout.handle.clone(), origin: pos, size: layout.size, color: None, highlight: None, angle });
    }

    /// Records `image` in `rect`, its part `uv`, multiplied by `tint`.
    pub fn image(&self, key: ImageKey, rect: Rect, uv: Rect, tint: Color32) {
        self.add(Item::Image { key, rect, uv, tint: super::items::rgba(tint) });
    }

    /// Records `color` through the alpha of `image` in `rect`.
    pub fn image_mask(&self, key: ImageKey, rect: Rect, uv: Rect, color: Color32) {
        self.add(Item::ImageMask { key, rect, uv, color: super::items::rgba(color) });
    }

    /// Records `item` (window coordinates) with the fade in force.
    pub fn add(&self, mut item: Item) {
        super::items::translate(&mut item, -self.origin);
        let opacity = self.opacity.get();
        if opacity >= 1.0 {
            self.push(item);
            return;
        }
        match item {
            // A text keeps its layout's colours: the fade composites it.
            Item::Text { color: None, .. } => {
                self.push(Item::PushOpacity { opacity, bounds: item.ink().unwrap_or(Rect::NOTHING) });
                self.push(item);
                self.push(Item::PopOpacity);
            }
            mut item => {
                super::items::map_colors(&mut item, |c| self.faded_color(c));
                self.push(item);
            }
        }
    }

    /// Records another canvas's lists (`take`), its content at the current level and its overlay over the children, in
    /// call order with what this canvas holds, under the fade in force.
    pub fn append(&self, (content, overlay): (DisplayList, DisplayList)) {
        self.extend(content.items);
        self.over(|| self.extend(overlay.items));
    }

    /// Records each of `items`.
    pub fn extend<I: IntoIterator<Item = Item>>(&self, items: I) {
        for item in items {
            self.add(item);
        }
    }

    /// A filled rounded rect.
    pub fn rect_filled(&self, rect: Rect, radius: impl Into<CornerRadius>, color: impl Into<Color32>) {
        self.add(super::items::rect_filled(rect, radius.into(), color.into()));
    }

    /// The egui context while the egui runtime runs the pipeline; `None` on craft-ui-rt.
    pub fn egui(&self) -> Option<&egui::Context> {
        self.services().egui()
    }

    /// The image for `key` in the frame's image store, made by `image` the first time `key` is asked for.
    pub fn cached_image<K: std::hash::Hash + 'static>(&self, key: K, image: impl FnOnce() -> egui::ColorImage) -> ImageKey {
        let id = image_key(&key);
        let mut image = Some(image);
        self.services().image(id, ImageOptions::default(), &mut || rgba_image(&(image.take().expect("made once"))()));
        id
    }

    /// A rounded rect's outline `width` wide, `kind` of it inside, centred on or outside the rect's edge.
    pub fn rect_stroke(&self, rect: Rect, radius: impl Into<CornerRadius>, width: f32, color: Color32, kind: egui::StrokeKind) {
        let align = match kind {
            egui::StrokeKind::Inside => craft_ui_rt::StrokeAlign::Inside,
            egui::StrokeKind::Middle => craft_ui_rt::StrokeAlign::Middle,
            egui::StrokeKind::Outside => craft_ui_rt::StrokeAlign::Outside,
        };
        self.add(super::items::rect_stroke(rect, radius.into(), width, color, align));
    }

    /// A circle's outline `width` wide, outside the radius.
    pub fn circle_stroke(&self, center: Pos2, radius: f32, width: f32, color: Color32) {
        self.add(super::items::circle_stroke(center, radius, width, color));
    }

    /// A filled circle.
    pub fn circle_filled(&self, center: Pos2, radius: f32, color: impl Into<Color32>) {
        self.add(super::items::circle_filled(center, radius, color.into()));
    }

    /// The open line through `points`, `width` wide, centred on it.
    pub fn line(&self, points: &[Pos2], width: f32, color: Color32) {
        self.extend(super::items::polyline(points, false, super::items::stroke(width, color, craft_ui_rt::StrokeAlign::Middle)));
    }

    /// The closed line through `points`, `width` wide, centred on it.
    pub fn closed_line(&self, points: &[Pos2], width: f32, color: Color32) {
        self.extend(super::items::polyline(points, true, super::items::stroke(width, color, craft_ui_rt::StrokeAlign::Middle)));
    }

    /// A line segment `width` wide.
    pub fn line_segment(&self, [from, to]: [Pos2; 2], width: f32, color: Color32) {
        self.extend(super::items::segment(from, to, width, color));
    }
}

/// The image key of a picture keyed by `key` in the app's or the framework's own key space.
pub fn image_key<K: std::hash::Hash + 'static>(key: &K) -> ImageKey {
    ImageKey(craft_ui_rt::hash_key(&(std::any::TypeId::of::<K>(), key)))
}

/// The display-list name of an egui texture, until the texture store sits on the frame's `ImageStore`: a managed id
/// keeps its number.
pub fn texture_key(id: egui::TextureId) -> ImageKey {
    match id {
        egui::TextureId::User(id) | egui::TextureId::Managed(id) => ImageKey(id),
    }
}

/// An egui colour image as display-list pixels (straight alpha).
pub fn rgba_image(image: &egui::ColorImage) -> Arc<RgbaImage> {
    let mut pixels = Vec::with_capacity(image.pixels.len() * 4);
    for p in &image.pixels {
        pixels.extend_from_slice(&p.to_srgba_unmultiplied());
    }
    Arc::new(RgbaImage { size: craft_ui_rt::SizeU::new(image.size[0] as u32, image.size[1] as u32), pixels })
}
