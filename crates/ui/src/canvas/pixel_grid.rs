//! [`PixelGrid`]: View › Show › Pixel Grid. Shown above 500 %, one device pixel wide, lightening
//! a dark pixel by about a quarter and darkening a light one, only over pixels that have content
//! (never over the checkerboard). The line colour fades from white to black across a band of
//! luma instead of switching at one cut, so mid-tone images get an even grid.
//!
//! The grid follows the composite of the part in view: [`PixelGrid::show`] reads it through the
//! same [`RasterSource`] as the [`RasterDisplay`](super::RasterDisplay) and caches the merged
//! line segments per revision and padded region, so panning within the padding costs nothing
//! and the work is bounded by the viewport. Beside [`CanvasGrid`](super::CanvasGrid), which is the
//! document grid of Preferences › Guides, Grid & Slices.

use std::sync::Arc;

use egui::{Color32, Context, Id, Painter, Stroke};
use kurbo::Point;

use super::crisp_shape;
use super::raster::{PixelRect, RasterSource};
use super::view::CanvasView;

/// One grid line piece in document pixels, axis-aligned, with its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSegment {
    pub from: [i32; 2],
    pub to: [i32; 2],
    pub color: Color32,
}

/// The line is fully white at or below the first luma and fully black (at `LIGHT_SCALE`
/// strength) at or above the second; between them the two blend.
const FADE_LUMA: [f32; 2] = [0.35, 0.75];
/// The share of black in the line is kept to this many steps, so noisy mid-tone pixels fall into
/// a few colours and equal-colour runs still merge into long segments.
const FADE_STEPS: f32 = 8.0;
/// Luminance of the checker under a see-through pixel when deciding light or dark; between the
/// default checker's two greys.
const CHECKER_LUMA: f32 = 0.9;

/// The grid of a region at one revision, kept in egui memory.
#[derive(Clone)]
struct GridCache {
    revision: u64,
    region: PixelRect,
    segments: Arc<Vec<GridSegment>>,
}

/// Merge consecutive cells of equal colour along one line into `emit(start, end, colour)` runs.
fn runs(len: i32, color_at: impl Fn(i32) -> Option<Color32>, mut emit: impl FnMut(i32, i32, Color32)) {
    let mut open: Option<(i32, Color32)> = None;
    for i in 0..len {
        let c = color_at(i);
        match (open, c) {
            (Some((_, oc)), Some(c)) if oc == c => {}
            _ => {
                if let Some((s, oc)) = open {
                    emit(s, i, oc);
                }
                open = c.map(|c| (i, c));
            }
        }
    }
    if let Some((s, oc)) = open {
        emit(s, len, oc);
    }
}

/// The pixel grid. Associated functions only: the cache lives in egui memory under the id the
/// app passes to [`show`](Self::show).
pub struct PixelGrid;

impl PixelGrid {
    /// The grid shows above this device zoom (500 %).
    pub const MIN_ZOOM: f64 = 5.0;
    /// How far a line moves its pixel toward white or black.
    pub const STRENGTH: f32 = 0.25;
    /// How much of [`STRENGTH`](Self::STRENGTH) a lightening line uses (white reads stronger).
    pub const LIGHT_SCALE: f32 = 0.64;
    /// The cached region snaps outward to multiples of this many pixels.
    pub const REGION_SNAP: i32 = 64;
    /// Segments drawn at most; past this the rest of the grid is left off.
    pub const MAX_SEGMENTS: usize = 60_000;

    /// Whether the grid shows at `device_zoom` (device pixels per document pixel): strictly
    /// above [`MIN_ZOOM`](Self::MIN_ZOOM).
    // pc: crates/ui-egui/src/canvas/pixel_grid.rs:45 `shows_at`.
    pub fn shows_at(device_zoom: f64) -> bool {
        device_zoom > Self::MIN_ZOOM
    }

    /// How light the line is over a pixel of `luma`: 0 a white line, 1 a black one; smoothstep
    /// across luma 0.35..0.75, `x = clamp((luma − 0.35) / 0.4, 0, 1)`, `round(x²(3 − 2x) · 8) / 8`.
    /// Not a finite number reads as 0.
    // pc: crates/ui-egui/src/canvas/pixel_grid.rs:51 `light_share`.
    pub fn light_share(luma: f32) -> f32 {
        let luma = if luma.is_finite() { luma } else { 0.0 };
        let x = ((luma - FADE_LUMA[0]) / (FADE_LUMA[1] - FADE_LUMA[0])).clamp(0.0, 1.0);
        (x * x * (3.0 - 2.0 * x) * FADE_STEPS).round() / FADE_STEPS
    }

    /// The line colour over a pixel of straight-alpha `rgba` (0..=1), or `None` where alpha is
    /// NaN or ≤ 0, or the result rounds to alpha 0. PhotoCraft's rule: `a = min(alpha, 1)`,
    /// `luma = 0.299 r + 0.587 g + 0.114 b`, `shown = luma·a + 0.9·(1 − a)` (the checker's luma),
    /// `t = light_share(shown)`, `white = STRENGTH·a·(1 − t)`, `black = STRENGTH·a·LIGHT_SCALE·t`;
    /// the colour is grey `round(white / (white + black) · 255)` at alpha
    /// `round((white + black) · 255)`, unmultiplied.
    // pc: crates/ui-egui/src/canvas/pixel_grid.rs:75 `cell_color`.
    pub fn cell_color(rgba: [f32; 4]) -> Option<Color32> {
        let a = rgba[3];
        if a.is_nan() || a <= 0.0 {
            return None;
        }
        let a = a.min(1.0);
        let luma = 0.299 * rgba[0] + 0.587 * rgba[1] + 0.114 * rgba[2];
        let shown = luma * a + CHECKER_LUMA * (1.0 - a);
        let t = Self::light_share(shown);
        // A white layer and a black layer over the pixel, as one grey with the combined alpha.
        let (white, black) = (Self::STRENGTH * a * (1.0 - t), Self::STRENGTH * a * Self::LIGHT_SCALE * t);
        let total = white + black;
        let alpha = (total * 255.0).round() as u8;
        if alpha == 0 {
            return None;
        }
        let grey = (white / total * 255.0).round() as u8;
        Some(Color32::from_rgba_unmultiplied(grey, grey, grey, alpha))
    }

    /// The merged segments of `rect`, whose pixels `px` are row by row: runs of one colour along
    /// each cell edge become one segment.
    // pc: crates/ui-egui/src/canvas/pixel_grid.rs:118 `segments`, :96 `runs`.
    pub fn segments(px: &[[f32; 4]], rect: PixelRect) -> Vec<GridSegment> {
        let (w, h) = (rect.width(), rect.height());
        let mut out = Vec::new();
        if w == 0 || h == 0 || px.len() < w as usize * h as usize {
            return out;
        }
        let at = |x: i32, y: i32| px.get(y as usize * w as usize + x as usize).and_then(|p| Self::cell_color(*p));
        // Each pixel owns its left and top edge, so the region's right and bottom edges carry no line.
        for x in 0..w {
            if out.len() >= Self::MAX_SEGMENTS {
                return out;
            }
            runs(h, |y| at(x, y), |s, e, color| out.push(GridSegment { from: [rect.x0 + x, rect.y0 + s], to: [rect.x0 + x, rect.y0 + e], color }));
        }
        for y in 0..h {
            if out.len() >= Self::MAX_SEGMENTS {
                return out;
            }
            runs(w, |x| at(x, y), |s, e, color| out.push(GridSegment { from: [rect.x0 + s, rect.y0 + y], to: [rect.x0 + e, rect.y0 + y], color }));
        }
        out.truncate(Self::MAX_SEGMENTS);
        out
    }

    /// The region to cache for a view showing `visible`: inside `bounds`, grown outward to
    /// [`REGION_SNAP`](Self::REGION_SNAP).
    // pc: crates/ui-egui/src/canvas/pixel_grid.rs:143 `cache_region`.
    pub fn cache_region(visible: PixelRect, bounds: PixelRect) -> PixelRect {
        let v = visible.intersect(&bounds);
        if v.is_empty() {
            return PixelRect::default();
        }
        let down = |n: i32| n.div_euclid(Self::REGION_SNAP) * Self::REGION_SNAP;
        let up = |n: i32| down(n.saturating_add(Self::REGION_SNAP - 1));
        PixelRect::new(down(v.x0), down(v.y0), up(v.x1), up(v.y1)).intersect(&bounds)
    }

    /// Draw the grid of `source` as `view` shows it, when [`shows_at`](Self::shows_at) the view's
    /// device zoom, reading the composite of the cached region when the revision changed or the
    /// view left it. Lines are one device pixel on whole device pixels (`crisp`).
    // pc: crates/ui-egui/src/canvas/pixel_grid.rs:167 `paint`.
    pub fn show(ctx: &Context, id: Id, painter: &Painter, view: &CanvasView, source: &dyn RasterSource) {
        let ppp = ctx.pixels_per_point();
        if !Self::shows_at(view.zoom.min(view.zoom_x()) * f64::from(ppp)) {
            return;
        }
        let size = source.size();
        let bounds = PixelRect::of_size(size);
        let visible = PixelRect::visible(view, size);
        let need = visible.intersect(&bounds);
        if need.is_empty() {
            return;
        }
        let key = id.with("pixel-grid");
        let revision = source.revision();
        let cached = ctx.data(|d| d.get_temp::<GridCache>(key)).filter(|c| c.revision == revision && c.region.intersect(&need) == need);
        let cache = cached.unwrap_or_else(|| {
            let region = Self::cache_region(visible, bounds);
            let mut image = egui::ColorImage::filled([region.width() as usize, region.height() as usize], Color32::TRANSPARENT);
            source.read(region, 1, &mut image);
            let px: Vec<[f32; 4]> = image.pixels.iter().map(|c| c.to_srgba_unmultiplied().map(|v| f32::from(v) / 255.0)).collect();
            let cache = GridCache { revision, region, segments: Arc::new(Self::segments(&px, region)) };
            ctx.data_mut(|d| d.insert_temp(key, cache.clone()));
            cache
        });
        let point = |p: [i32; 2]| view.to_screen(Point::new(f64::from(p[0]), f64::from(p[1])));
        for s in cache.segments.iter() {
            painter.add(crisp_shape(point(s.from), point(s.to), Stroke::new(1.0 / ppp, s.color), ppp));
        }
    }
}
