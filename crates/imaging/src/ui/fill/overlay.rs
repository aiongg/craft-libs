//! The sampling tint and the fill area's ants over Content-Aware Fill's canvas (task WS-CA2).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::{AsIdSalt, Color32, ColorImage, Context, Id, IdSalt, Painter, Rect, TextureHandle, TextureOptions, Ui, pos2};

use craft_ui::canvas::kurbo::{self, Point};
use craft_ui::canvas::{CanvasView, Marquee, MarqueeLook};

use crate::trace::{CoverageGrid, GRID_EDGE_LIMIT, edges_of_grid};

/// A bit plane from the app (lead call 40): `size` texels of `texel × texel` document pixels from
/// document pixel `origin`, row by row. PhotoCraft computes them from its plan
/// (`content_aware_plan`, `paint_strokes`) at a proxy of `k` pixels per texel.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SamplingPlane {
    pub origin: [i32; 2],
    pub size: [u32; 2],
    pub texel: u32,
    pub bits: Vec<bool>,
}

/// The two marks Content-Aware Fill draws over the document:
///
/// - The **tint**: `color` at `opacity` on every texel where the sampling plane differs from
///   `excluded` and that is outside the fill area (Indicates › Sampling Area tints what is
///   sampled; Excluded Area what is not). Built into one texture per change of the planes, the
///   colour, the opacity or Indicates (a texel-scaled image, nearest filtering), drawn while Show
///   Sampling Area is on and ⌘H has not hidden it.
/// - The **ants** around the fill area: `craft_imaging::trace::edges_of_grid` over a
///   `CoverageGrid` of the fill plane (at most `GRID_EDGE_LIMIT` edges, else the bounds of its
///   cells), drawn as the library's marching ants (`Marquee::segments` in `MarqueeLook::Ants`: the
///   theme's dash length, speed and repaint interval, still with reduced motion). The edges are
///   kept while the plane stays the same.
///
/// The tint and the ants sit on the document: no AccessKit node.
pub struct FillOverlay<'a> {
    id_salt: IdSalt,
    sampling: Option<&'a SamplingPlane>,
    fill_area: Option<&'a SamplingPlane>,
    color: Color32,
    excluded: bool,
    tint: bool,
    document: Option<[u32; 2]>,
}

impl<'a> FillOverlay<'a> {
    pub fn new(id_salt: impl AsIdSalt, sampling: Option<&'a SamplingPlane>, fill_area: Option<&'a SamplingPlane>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), sampling, fill_area, color: Color32::TRANSPARENT, excluded: false, tint: true, document: None }
    }

    /// The tint's colour with the opacity as its alpha (the user's Color and Opacity).
    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    /// Indicates › Excluded Area.
    pub fn excluded(mut self, excluded: bool) -> Self {
        self.excluded = excluded;
        self
    }

    /// Whether the tint shows (Show Sampling Area, ⌘H).
    pub fn tint(mut self, tint: bool) -> Self {
        self.tint = tint;
        self
    }

    /// The document's size in pixels: the tint is cut at its edge (a plane's last texels may reach
    /// past it).
    pub fn document(mut self, size: [u32; 2]) -> Self {
        self.document = Some(size);
        self
    }

    /// Draws the tint and the ants in `ui`'s painter over `view`.
    pub fn show(self, ui: &Ui, view: &CanvasView) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:87-101 (the tint texels), :377-397 (tint
        // and ants), caf_ui.rs:60 `MAX_OUTLINE_EDGES`.
        let id = self.id(ui);
        self.paint(ui.ctx(), &ui.painter().clone(), view, id);
    }

    /// The id the caches are kept under.
    pub(super) fn id(&self, ui: &Ui) -> Id {
        ui.make_persistent_id(self.id_salt)
    }

    /// [`show`](Self::show) for a caller that has a painter and a context but no `Ui` (the canvas's
    /// `painter` closure), under the id [`id`](Self::id) gave.
    pub(super) fn paint(self, ctx: &Context, painter: &Painter, view: &CanvasView, id: Id) {
        let Self { sampling, fill_area, color, excluded, tint, document, .. } = self;
        let bounds = document.map(|[w, h]| kurbo::Rect::new(0.0, 0.0, f64::from(w), f64::from(h)));
        if tint && let Some(sampling) = sampling {
            paint_tint(ctx, painter, view, id, sampling, fill_area, color, excluded, bounds);
        }
        if let Some(fill_area) = fill_area {
            paint_ants(ctx, painter, view, id, fill_area);
        }
    }
}

/// The tint's texture, kept while its inputs stay the same.
#[derive(Clone)]
struct TintTexture {
    key: u64,
    texture: TextureHandle,
}

/// The ants of one fill area, kept while the plane stays the same.
#[derive(Clone)]
struct AntsOutline {
    key: u64,
    marquee: Arc<Marquee>,
}

fn plane_hash(plane: &SamplingPlane, hasher: &mut DefaultHasher) {
    plane.origin.hash(hasher);
    plane.size.hash(hasher);
    plane.texel.hash(hasher);
    plane.bits.hash(hasher);
}

/// The plane's cell at document pixel `(x, y)`: false off the plane.
fn bit_at(plane: &SamplingPlane, x: f64, y: f64) -> bool {
    let texel = f64::from(plane.texel.max(1));
    let cell = |at: f64, origin: i32, size: u32| {
        let c = ((at - f64::from(origin)) / texel).floor();
        (c >= 0.0 && c < f64::from(size)).then_some(c as usize)
    };
    match (cell(x, plane.origin[0], plane.size[0]), cell(y, plane.origin[1], plane.size[1])) {
        (Some(cx), Some(cy)) => plane.bits.get(cy * plane.size[0] as usize + cx).copied().unwrap_or(false),
        _ => false,
    }
}

/// The tint as an image of the sampling plane's texels.
pub(super) fn tint_image(sampling: &SamplingPlane, fill_area: Option<&SamplingPlane>, color: Color32, excluded: bool) -> ColorImage {
    let [width, height] = sampling.size;
    let texel = f64::from(sampling.texel.max(1));
    let mut pixels = Vec::with_capacity(sampling.bits.len());
    for index in 0..width as usize * height as usize {
        let (column, row) = ((index % width as usize) as f64, (index / width as usize) as f64);
        let centre = (f64::from(sampling.origin[0]) + (column + 0.5) * texel, f64::from(sampling.origin[1]) + (row + 0.5) * texel);
        let sampled = sampling.bits.get(index).copied().unwrap_or(false);
        let filled = fill_area.is_some_and(|plane| bit_at(plane, centre.0, centre.1));
        pixels.push(if !filled && sampled != excluded { color } else { Color32::TRANSPARENT });
    }
    ColorImage::new([width as usize, height as usize], pixels)
}

/// One texel of the sampling plane is tinted when it differs from `excluded` and its centre lies
/// outside the fill area (the fill area is never tinted).
#[expect(clippy::too_many_arguments, reason = "the planes and the look of one tint")]
fn paint_tint(
    ctx: &Context,
    painter: &Painter,
    view: &CanvasView,
    id: Id,
    sampling: &SamplingPlane,
    fill_area: Option<&SamplingPlane>,
    color: Color32,
    excluded: bool,
    bounds: Option<kurbo::Rect>,
) {
    // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:87-101.
    let [width, height] = sampling.size;
    if width == 0 || height == 0 {
        return;
    }
    let mut hasher = DefaultHasher::new();
    plane_hash(sampling, &mut hasher);
    if let Some(plane) = fill_area {
        plane_hash(plane, &mut hasher);
    }
    (color, excluded).hash(&mut hasher);
    let key = hasher.finish();
    let slot = id.with("tint");
    let cached = ctx.data(|d| d.get_temp::<TintTexture>(slot));
    let texture = match cached {
        Some(cached) if cached.key == key => cached.texture,
        previous => {
            let image = tint_image(sampling, fill_area, color, excluded);
            let texture = match previous {
                Some(mut previous) => {
                    previous.texture.set(image, TextureOptions::NEAREST);
                    previous.texture
                }
                None => ctx.load_texture(format!("fill-tint-{id:?}"), image, TextureOptions::NEAREST),
            };
            ctx.data_mut(|d| d.insert_temp(slot, TintTexture { key, texture: texture.clone() }));
            texture
        }
    };
    let texel = f64::from(sampling.texel.max(1));
    let origin = Point::new(f64::from(sampling.origin[0]), f64::from(sampling.origin[1]));
    let extent = kurbo::Rect::new(origin.x, origin.y, origin.x + f64::from(width) * texel, origin.y + f64::from(height) * texel);
    let corners = view.quad(extent);
    let on_screen = Rect::from_points(&corners);
    let clip = bounds.map_or(painter.clip_rect(), |bounds| Rect::from_points(&view.quad(bounds)).intersect(painter.clip_rect()));
    // The texture is not mirrored by a quad's corner order: the image covers the screen rect of its extent.
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.with_clip_rect(clip).image(texture.id(), on_screen, uv, Color32::WHITE); // design: allow(identity tint)
}

/// The fill area's edge as marching ants: `edges_of_grid` over the plane, or the bounds of its
/// cells when there are more than `GRID_EDGE_LIMIT` edges.
fn paint_ants(ctx: &Context, painter: &Painter, view: &CanvasView, id: Id, plane: &SamplingPlane) {
    // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:377-397, preview.rs:107 `outline`.
    let mut hasher = DefaultHasher::new();
    plane_hash(plane, &mut hasher);
    let key = hasher.finish();
    let slot = id.with("ants");
    let outline = match ctx.data(|d| d.get_temp::<AntsOutline>(slot)) {
        Some(cached) if cached.key == key => cached.marquee,
        _ => {
            let marquee = Arc::new(ants_of(plane));
            ctx.data_mut(|d| d.insert_temp(slot, AntsOutline { key, marquee: marquee.clone() }));
            marquee
        }
    };
    outline.paint(painter, view, MarqueeLook::Ants, ctx.input(|i| i.time));
}

fn ants_of(plane: &SamplingPlane) -> Marquee {
    let texel = i64::from(plane.texel.max(1));
    let to_doc = |cell: [i32; 2]| {
        Point::new((i64::from(plane.origin[0]) + i64::from(cell[0]) * texel) as f64, (i64::from(plane.origin[1]) + i64::from(cell[1]) * texel) as f64)
    };
    let grid = CoverageGrid::new([0, 0], plane.size, plane.bits.clone());
    match edges_of_grid(&grid, GRID_EDGE_LIMIT) {
        Some(edges) => Marquee::segments(edges.into_iter().map(|(a, b)| (to_doc(a), to_doc(b))), ""),
        None => {
            // Too many edges to draw: the bounds of the fill area, as PhotoCraft's `bounds`.
            let [width, height] = plane.size;
            let mut bounds: Option<[i32; 4]> = None;
            for y in 0..height as i32 {
                for x in 0..width as i32 {
                    if grid.inside(x, y) {
                        let b = bounds.get_or_insert([x, y, x + 1, y + 1]);
                        *b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
                    }
                }
            }
            let [x0, y0, x1, y1] = bounds.unwrap_or([0; 4]);
            let (a, b) = (to_doc([x0, y0]), to_doc([x1, y1]));
            Marquee::rect(kurbo::Rect::new(a.x, a.y, b.x, b.y), "")
        }
    }
}
