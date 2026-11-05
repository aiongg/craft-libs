//! [`SelectionOverlay`].

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasView, Marquee, MarqueeLook};
use craft_ui::localized::{Labels, LocaleNumbers, Unit, Units};
use egui::accesskit::Role;
use egui::epaint::Vertex;
use egui::{
    AsIdSalt, Color32, ColorImage, Id, IdSalt, Mesh, Rect, Response, Sense, Shape, TextureHandle, TextureOptions, Ui, UiBuilder, WidgetInfo, pos2,
};

use crate::selection::{PreviewMode, QuickMaskLook};
use crate::trace::{MaskRect, MaskSource, Segment, bounded_step, outline, quantised_region, trace_step};

/// The longest side of the preview texture, in texels: a larger document is sampled every `k`-th
/// pixel (`k = ceil(long side / 1024)`).
// pc: crates/ui-egui/src/dialogs/color_range_ui.rs:47 `CANVAS_PREVIEW` (1024).
pub(super) const PREVIEW_TEXTURE_MAX: u32 = 1024;

craft_ui::words! {
    /// The accessible name and value of the selection overlay, English by default.
    #[non_exhaustive]
    pub struct SelectionOverlayLabels {
        /// The graphics object's name.
        name: "selectionOverlay.name" = "Selection";
        /// The value with nothing selected.
        nothing: "selectionOverlay.nothing" = "No selection";
        /// The value: the content bounds, each number with the document's unit.
        bounds: "selectionOverlay.bounds" = "{width} × {height} at {x}, {y}";
        /// Appended to the value while a preview shows: `{value}, {mode}`.
        with_mode: "selectionOverlay.withMode" = "{value}, {mode}";
        grayscale: "selectionOverlay.grayscale" = "Grayscale";
        black_matte: "selectionOverlay.blackMatte" = "Black Matte";
        white_matte: "selectionOverlay.whiteMatte" = "White Matte";
        quick_mask: "selectionOverlay.quickMask" = "Quick Mask";
    }
}

/// Where the overlay's outline comes from.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum OutlineSource<'a> {
    /// The app's mask: the overlay traces it ([`crate::trace::outline`]) and caches the
    /// segments, and builds the preview texture from it.
    Mask(&'a dyn MaskSource),
    /// Segments the app traced itself (in a job, PhotoCraft's Rayon trace), in document pixels of
    /// the full-size document, with the content `bounds` for the accessible value. Drawn as they
    /// are: no cache, no preview texture.
    Segments {
        #[allow(missing_docs)]
        segments: &'a [Segment],
        #[allow(missing_docs)]
        bounds: MaskRect,
    },
}

impl<'a, M: MaskSource> From<&'a M> for OutlineSource<'a> {
    fn from(mask: &'a M) -> Self {
        OutlineSource::Mask(mask)
    }
}

/// What [`SelectionOverlay::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectionOverlayResponse {
    /// The overlay's id (its cache and its AccessKit node).
    pub id: Id,
    /// The selection's content bounds in full-size document pixels, `None` when nothing is
    /// selected.
    pub bounds: Option<MaskRect>,
    /// The trace ran this pass (the mask's revision, the step, the region or the reduction
    /// changed); `false` when the cached segments were drawn. For a performance readout and tests.
    pub retraced: bool,
}

/// The selection on the canvas, drawn once per pass before the tool's paint.
///
/// ```ignore
/// let mask = SurfaceMask(&doc.selection);                       // the app's MaskSource
/// SelectionOverlay::new(("selection", doc.id), &mask)
///     .visible(show.selection_edges && !polygon.replaces_selection())
///     .offset(floating_offset)                                  // a floating piece, a selection drag
///     .preview(PreviewMode::None, QuickMaskLook::default())
///     .show(ui, &canvas, &view);
/// ```
///
/// What it draws, bottom first:
/// - The preview ([`preview`](Self::preview)), when the mode is not `None` and the source is a
///   mask: one texture of [`PreviewMode::pixel`] per texel over the whole document, at most
///   1024 texels on the long side, rebuilt only when the mask's revision, the mode, the look or the
///   reduction changes, drawn over the document's rectangle with linear filtering.
/// - The ants ([`MarqueeLook::Ants`](craft_ui::canvas::MarqueeLook::Ants), the `marquee.*`
///   tokens) along the selection's boundary, when [`visible`](Self::visible). Hidden ants keep the
///   preview.
///
/// The trace, for a mask source: step [`trace_step`](crate::trace::trace_step) of the view's
/// device zoom (`view.zoom · pixels_per_point`) and the [`reduction`](Self::reduction); region
/// [`quantised_region`](crate::trace::quantised_region) of the view's visible bounds shifted back by
/// the [`offset`](Self::offset), so a small
/// pan reuses the trace; bounds the mask's content bounds (the whole canvas when
/// [`MaskSource::outside`] is above 0) within the region. With a [`mapped`](Self::mapped) outline
/// the whole selection is traced instead (a Free Transform may carry parts from outside the view
/// into it) with [`bounded_step`](crate::trace::bounded_step). Segments are scaled up by the
/// reduction. The overlay keeps one trace per id in context memory, keyed by the mask's revision,
/// the step, the region and the reduction, and retraces only when one of them changes.
///
/// Drawing: each segment end moves by the [`offset`](Self::offset), then through the
/// [`mapped`](Self::mapped) point map, then through the view, so the ants are crisp on device
/// pixels at any zoom and rotation. While the ants show, the overlay asks for a repaint every
/// `marquee.tick_ms` (100 ms) unless reduced motion stops the march.
///
/// AccessKit: one graphics object named "Selection", valued with the content bounds in the
/// document's unit ("120 × 80 at 40, 20", with [`value_in`](Self::value_in)) or "No selection";
/// a preview adds its mode's name. No focus; the ants are decoration.
pub struct SelectionOverlay<'a> {
    id_salt: IdSalt,
    source: OutlineSource<'a>,
    visible: bool,
    offset: [i32; 2],
    reduction: u32,
    map: Option<&'a dyn Fn(Point) -> Point>,
    preview: PreviewMode,
    look: QuickMaskLook,
    value_in: Option<(&'a LocaleNumbers, &'a Unit)>,
    labels: Option<SelectionOverlayLabels>,
}

impl<'a> SelectionOverlay<'a> {
    /// The selection of `source` (a `&impl MaskSource`, or [`OutlineSource::Segments`]) under
    /// `id_salt`: ants shown, no offset, no reduction, no map, no preview.
    pub fn new(id_salt: impl AsIdSalt, source: impl Into<OutlineSource<'a>>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            source: source.into(),
            visible: true,
            offset: [0, 0],
            reduction: 1,
            map: None,
            preview: PreviewMode::None,
            look: QuickMaskLook::default(),
            value_in: None,
            labels: None,
        }
    }

    /// Whether the ants show (View › Show › Selection Edges, and not while a New Polygonal or
    /// Magnetic Lasso outline replaces the selection). Hidden ants keep the preview. Default on.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Draw the ants `offset` whole document pixels from where the selection is: a floating piece,
    /// a selection drag in progress ([`SelectionDrag::delta`](crate::ui::select::SelectionDrag)),
    /// layers moved together with the selection.
    pub fn offset(mut self, offset: [i32; 2]) -> Self {
        self.offset = offset;
        self
    }

    /// The source is reduced `k` times (Select and Mask's preview proxy): its pixels are `k`
    /// document pixels each way. Default 1; 0 counts as 1.
    pub fn reduction(mut self, k: u32) -> Self {
        self.reduction = k.max(1);
        self
    }

    /// Draw the outline through `map` (document point to document point): where a Free Transform
    /// box puts the selection (`|p| homography.apply(p)` from `TransformSession::homography`) or
    /// the app's warp map in the Warp phase. The whole selection is traced, not only the visible
    /// part.
    pub fn mapped(mut self, map: &'a dyn Fn(Point) -> Point) -> Self {
        self.map = Some(map);
        self
    }

    /// The Selection Preview (Color Range's modes, `look` for its Quick Mask) or, with
    /// [`PreviewMode::QuickMask`] and the user's Quick Mask Options, the Quick Mask tint (Edit in
    /// Quick Mask Mode, the Type Mask tools). Default [`PreviewMode::None`].
    pub fn preview(mut self, mode: PreviewMode, look: QuickMaskLook) -> Self {
        self.preview = mode;
        self.look = look;
        self
    }

    /// Format the accessible value's numbers with `numbers` in `unit` (the rulers' unit). Without
    /// it the value is in pixels with the context's numbers.
    pub fn value_in(mut self, numbers: &'a LocaleNumbers, unit: &'a Unit) -> Self {
        self.value_in = Some((numbers, unit));
        self
    }

    /// Translated words; without them [`SelectionOverlayLabels::of`] the context.
    pub fn labels(mut self, labels: SelectionOverlayLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the overlay over `canvas` (the canvas's response, for the clip rect and the AccessKit
    /// parent) with `view`.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> SelectionOverlayResponse {
        let id = canvas.id.with("craft_imaging::ui::SelectionOverlay").with_salt(self.id_salt);
        let k = self.reduction.max(1);
        let labels = self.labels.clone().unwrap_or_else(|| SelectionOverlayLabels::of(ui));
        let mut retraced = false;
        let bounds = match self.source {
            OutlineSource::Mask(mask) => content_bounds(mask, k),
            OutlineSource::Segments { bounds, .. } => (!bounds.is_empty()).then_some(bounds),
        };

        if let OutlineSource::Mask(mask) = self.source
            && self.preview != PreviewMode::None
        {
            self.paint_preview(ui, id, view, mask, k);
        }

        // pc: canvas.rs:236-244 draws the ants only when the selection has an outline to draw.
        let has_outline = match self.source {
            OutlineSource::Mask(_) => bounds.is_some(),
            OutlineSource::Segments { segments, .. } => !segments.is_empty(),
        };
        if self.visible && has_outline {
            let traced;
            let segments: &[Segment] = match self.source {
                OutlineSource::Mask(mask) => {
                    let (segments, ran) = self.traced(ui, id, view, mask, k);
                    (traced, retraced) = (segments, ran);
                    &traced
                }
                OutlineSource::Segments { segments, .. } => segments,
            };
            // pc: canvas.rs:309-313 (offset), :318 and `Marquee::paint` (repaint every tick).
            let [dx, dy] = self.offset.map(f64::from);
            let place = |p: [i32; 2]| {
                let p = Point::new(f64::from(p[0]) + dx, f64::from(p[1]) + dy);
                self.map.map_or(p, |map| map(p))
            };
            let lines = segments.iter().map(|&(a, b)| (place(a), place(b))).collect::<Vec<_>>();
            let time = ui.input(|i| i.time);
            Marquee::segments(lines, "").paint(&ui.painter_at(view.rect), view, MarqueeLook::Ants, time);
        }

        self.describe(ui, canvas, view, id, bounds, &labels);
        SelectionOverlayResponse { id, bounds, retraced }
    }

    /// The cached trace of the visible region (the whole selection under a map), and whether it ran
    /// this pass.
    fn traced(&self, ui: &Ui, id: Id, view: &CanvasView, mask: &dyn MaskSource, k: u32) -> (Arc<Vec<Segment>>, bool) {
        // pc: canvas.rs:248 (step in device pixels), :261-278 (region), :266-281 (whole trace).
        let device_zoom = view.zoom * f64::from(ui.ctx().pixels_per_point());
        let mut step = trace_step(device_zoom, k);
        let whole = self.map.is_some();
        let region = if whole {
            let far = 1 << 30;
            MaskRect::new(-far, -far, far, far)
        } else {
            // The mask lies `offset` from where it is drawn, so the part of it in view is the
            // visible area shifted back.
            let v = view.visible();
            let [dx, dy] = self.offset.map(f64::from);
            quantised_region([v.x0 - dx, v.y0 - dy, v.x1 - dx, v.y1 - dy], step, k)
        };
        let content = if mask.outside() > 0.0 { MaskRect::of_size(mask.size()) } else { mask.content_bounds() };
        let bounds = content.intersect(&region);
        if whole {
            step = bounded_step(bounds, step);
        }
        let revision = mask.revision();
        let key = (revision, step, region, k);
        let cached = ui.ctx().data(|d| d.get_temp::<(TraceKey, Arc<Vec<Segment>>)>(id));
        if let Some((cached_key, segments)) = cached
            && cached_key == key
        {
            return (segments, false);
        }
        // pc: canvas.rs:298-302 scales the segments up by `k`.
        let mut segments = outline(mask, bounds, step);
        if k > 1 {
            let s = i32::try_from(k).unwrap_or(i32::MAX);
            let up = |p: [i32; 2]| [p[0].saturating_mul(s), p[1].saturating_mul(s)];
            segments.iter_mut().for_each(|(a, b)| (*a, *b) = (up(*a), up(*b)));
        }
        let segments = Arc::new(segments);
        ui.ctx().data_mut(|d| d.insert_temp(id, (key, segments.clone())));
        (segments, true)
    }

    /// The preview texture over the document's rectangle.
    fn paint_preview(&self, ui: &Ui, id: Id, view: &CanvasView, mask: &dyn MaskSource, k: u32) {
        // pc: color_range_ui.rs:592-636 (texture, reuse by key, drawn over the image rect).
        let size = mask.size();
        if size[0] == 0 || size[1] == 0 {
            return;
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (mask.revision(), self.preview, k, size).hash(&mut hasher);
        let look = &self.look;
        (look.color.r.to_bits(), look.color.g.to_bits(), look.color.b.to_bits(), look.opacity.to_bits(), look.indicates as u8).hash(&mut hasher);
        let key = hasher.finish();
        let texture_id = id.with("preview");
        let cached = ui.ctx().data(|d| d.get_temp::<(u64, TextureHandle)>(texture_id));
        let texture = match cached {
            Some((cached_key, texture)) if cached_key == key => texture,
            _ => {
                let texture = ui.ctx().load_texture("craft-selection-preview", self.preview_image(mask, size), TextureOptions::LINEAR);
                ui.ctx().data_mut(|d| d.insert_temp(texture_id, (key, texture.clone())));
                texture
            }
        };
        let side = |n: u32| f64::from(n) * f64::from(k);
        let corners = view.quad(craft_ui::canvas::kurbo::Rect::new(0.0, 0.0, side(size[0]), side(size[1])));
        let uvs = [pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(1.0, 1.0), pos2(0.0, 1.0)];
        let mut mesh = Mesh::with_texture(texture.id());
        for (pos, uv) in corners.into_iter().zip(uvs) {
            mesh.vertices.push(Vertex { pos, uv, color: Color32::WHITE });
        }
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        ui.painter_at(view.rect).add(Shape::mesh(mesh));
    }

    /// One texel per `p`-th source pixel (`p = ceil(long side / 1024)`), each [`PreviewMode::pixel`].
    fn preview_image(&self, mask: &dyn MaskSource, size: [u32; 2]) -> ColorImage {
        let p = size[0].max(size[1]).div_ceil(PREVIEW_TEXTURE_MAX).max(1);
        let (w, h) = (size[0].div_ceil(p) as usize, size[1].div_ceil(p) as usize);
        let mut row = vec![0.0_f32; w];
        let mut pixels = Vec::with_capacity(w * h);
        for ty in 0..h {
            mask.read_row(i32::try_from(ty as u64 * u64::from(p)).unwrap_or(i32::MAX), 0, p, &mut row);
            pixels.extend(row.iter().map(|&c| {
                let [r, g, b, a] = self.preview.pixel(c, &self.look).0;
                Color32::from_rgba_unmultiplied(r, g, b, a)
            }));
        }
        ColorImage::new([w, h], pixels)
    }

    /// The AccessKit node: one graphics object inside the canvas node, hover-only.
    fn describe(&self, ui: &mut Ui, canvas: &Response, view: &CanvasView, id: Id, bounds: Option<MaskRect>, labels: &SelectionOverlayLabels) {
        let value = self.value(ui, bounds, labels);
        let rect = bounds
            .map(|b| {
                let side = |v: i32| f64::from(v);
                let [dx, dy] = self.offset.map(f64::from);
                let r = craft_ui::canvas::kurbo::Rect::new(side(b.x0) + dx, side(b.y0) + dy, side(b.x1) + dx, side(b.y1) + dy);
                Rect::from_points(&view.quad(r)).intersect(view.rect)
            })
            .filter(|r| r.is_positive())
            .unwrap_or_else(|| Rect::from_center_size(view.rect.center(), egui::Vec2::ZERO));
        let nodes = ui.new_child(UiBuilder::new().max_rect(view.rect).accessibility_parent(canvas.id));
        let response = nodes.interact(rect, id, Sense::hover());
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::GraphicsObject, nodes.is_enabled(), &labels.name);
            info.current_text_value = Some(value.clone());
            info
        });
    }

    /// The accessible value: the bounds in the unit (or "No selection"), plus the preview's name.
    fn value(&self, ui: &Ui, bounds: Option<MaskRect>, l: &SelectionOverlayLabels) -> String {
        let units = Units::of(ui);
        let base = match bounds {
            None => l.nothing.clone(),
            Some(b) => {
                let n = |v: f64| match self.value_in {
                    Some((numbers, unit)) => format!("{}{}{}", unit.prefix(), numbers.format(v, 0..=2), unit.suffix()),
                    None => units.pixels(v),
                };
                craft_i18n::fmt(
                    &l.bounds,
                    &[
                        ("width", &n(f64::from(b.width()))),
                        ("height", &n(f64::from(b.height()))),
                        ("x", &n(f64::from(b.x0))),
                        ("y", &n(f64::from(b.y0))),
                    ],
                )
            }
        };
        let mode = match self.preview {
            PreviewMode::None => return base,
            PreviewMode::Grayscale => &l.grayscale,
            PreviewMode::BlackMatte => &l.black_matte,
            PreviewMode::WhiteMatte => &l.white_matte,
            PreviewMode::QuickMask => &l.quick_mask,
        };
        craft_i18n::fmt(&l.with_mode, &[("value", &base), ("mode", mode)])
    }
}

/// The cache key of a trace: the mask's revision, the step, the region and the reduction.
type TraceKey = (u64, u32, MaskRect, u32);

/// The selection's content bounds in full-size document pixels (the whole canvas when the mask is
/// selected outside its content); `None` when nothing is selected.
fn content_bounds(mask: &dyn MaskSource, k: u32) -> Option<MaskRect> {
    let b = if mask.outside() > 0.0 { MaskRect::of_size(mask.size()) } else { mask.content_bounds() };
    let up = |v: i32| v.saturating_mul(i32::try_from(k).unwrap_or(i32::MAX));
    (!b.is_empty()).then(|| MaskRect::new(up(b.x0), up(b.y0), up(b.x1), up(b.y1)))
}
