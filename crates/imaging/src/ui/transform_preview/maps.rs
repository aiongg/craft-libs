//! [`PreviewMap`]: where a transform preview's source goes, and the mesh it is drawn with (task
//! TR-T10).

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T10)")]

use kurbo::{Point, Rect};

use craft_ui::canvas::quad::Homography;

use crate::ui::warp::WarpMesh;
use crate::ui::warp::pin::PinMeshView;

/// Where the preview's source goes, in document units. The texture covers the source frame; each
/// map says where every point of it lands.
#[derive(Clone, Copy)]
pub enum PreviewMap<'a> {
    /// Free Transform: the source frame's corners onto a box's corners (clockwise from the top
    /// left), through the homography; a `QUAD_CELLS` × `QUAD_CELLS` mesh keeps perspective
    /// straight, cells with a corner past the horizon left out.
    Quad([Point; 4]),
    /// Warp: the source frame's unit square through a warp mesh's surface, sampled finer where the
    /// warp pulls hard (about every `SEGMENT_PX` screen points, `MIN_PER_CELL` to `MAX_PER_CELL`
    /// per patch, at most `MAX_PER_AXIS` per axis).
    Mesh(&'a WarpMesh),
    /// Puppet Warp: the solver's triangles, each vertex at its deformed position and its texel at
    /// its rest position within `source` (the layer's bounds), drawn in the solver's order.
    Pins { mesh: PinMeshView<'a>, source: Rect },
    /// Perspective Warp and other engine maps: a `cells` × `cells` grid over `source` pushed
    /// through the app's function (PhotoCraft's piecewise homography, LC20).
    Function { source: Rect, cells: usize, map: &'a dyn Fn(Point) -> Point },
}

impl std::fmt::Debug for PreviewMap<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreviewMap::Quad(quad) => f.debug_tuple("Quad").field(quad).finish(),
            PreviewMap::Mesh(mesh) => f.debug_tuple("Mesh").field(mesh).finish(),
            PreviewMap::Pins { mesh, source } => f.debug_struct("Pins").field("mesh", mesh).field("source", source).finish(),
            PreviewMap::Function { source, cells, .. } => {
                f.debug_struct("Function").field("source", source).field("cells", cells).finish_non_exhaustive()
            }
        }
    }
}

impl<'a> PreviewMap<'a> {
    /// A box: the source frame's corners onto `quad`.
    pub fn quad(quad: [Point; 4]) -> Self {
        PreviewMap::Quad(quad)
    }

    /// A warp mesh over the source frame.
    pub fn mesh(mesh: &'a WarpMesh) -> Self {
        PreviewMap::Mesh(mesh)
    }

    /// A pin solver's mesh over `source`, the layer's bounds.
    pub fn pins(mesh: PinMeshView<'a>, source: Rect) -> Self {
        PreviewMap::Pins { mesh, source }
    }

    /// The app's map over a `cells` × `cells` grid of `source` (Perspective Warp: 40,
    /// `PreviewQuality::PLANE_CELLS`).
    pub fn function(source: Rect, cells: usize, map: &'a dyn Fn(Point) -> Point) -> Self {
        PreviewMap::Function { source, cells, map }
    }

    /// How much the map enlarges a source pixel, for the texture level: the quad's largest side
    /// ratio to the source frame for a box, 1 for the other maps (PhotoCraft `quad_scale`).
    pub fn source_scale(&self, source: [usize; 2]) -> f32 {
        // pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:9 (`quad_scale`).
        todo!()
    }

    /// The mesh to draw at `zoom` screen points per document unit: positions, rest positions,
    /// texture coordinates within `uv` (the level's extent covering the source) and triangles in
    /// drawing order.
    pub(super) fn build(&self, zoom: f64, uv: [f32; 2]) -> PreviewMesh {
        // pc: crates/ui-egui/src/tools/transform_tool/draw.rs:113 (quad), :83 (`front_vertices`);
        // crates/ui-egui/src/tools/warp_preview.rs:20 (`samples`), :58 (`triangle_order`);
        // crates/ui-egui/src/tools/puppet_ui.rs:348 (pins); perspective_ui.rs:263 (function).
        todo!()
    }
}

/// A preview mesh in document units, before it is clipped and mapped to the screen.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct PreviewMesh {
    /// Where each vertex is drawn.
    pub points: Vec<Point>,
    /// Where each vertex sits with no transform (the displacement order reads it).
    pub rest: Vec<Point>,
    pub uv: Vec<[f32; 2]>,
    /// Triangles, three indices each, in drawing order.
    pub indices: Vec<u32>,
}

/// The sample positions `(us, vs)` along a warp mesh's two axes at `zoom` screen points per
/// document unit: every knot included, increasing, from 0 to 1. Each patch gets
/// `ceil(extent × zoom / SEGMENT_PX)` segments clamped to `MIN_PER_CELL..=MAX_PER_CELL`, where
/// `extent` is the larger side of its control points' bounding box; an axis over `MAX_PER_AXIS`
/// is scaled down together (each interval keeping at least 2). A zoom that is not finite or not
/// positive counts as 1.
pub(super) fn samples(mesh: &WarpMesh, zoom: f64) -> (Vec<f64>, Vec<f64>) {
    // pc: crates/ui-egui/src/tools/warp_preview.rs:20 (`samples`), :84 (`axis`).
    todo!()
}

/// The point of a warp mesh's surface at mesh parameters `(s, t)` (bicubic Bézier patches,
/// `WarpMesh::locate`).
pub(super) fn surface_point(mesh: &WarpMesh, s: f64, t: f64) -> Point {
    // new: the library's `WarpMesh` has no evaluation yet; PhotoCraft's is `BezierMesh::eval` in
    // `photocraft-geom` (engine), the de Casteljau patch evaluation.
    todo!()
}

/// Triangle indices for a `row`-wide vertex grid, two per cell, cells ordered from the most
/// displaced (mean of its four corners' distance from rest) to the least. Empty when `row < 2` or
/// the lengths differ.
pub(super) fn triangle_order(points: &[Point], rest: &[Point], row: usize) -> Vec<u32> {
    // pc: crates/ui-egui/src/tools/warp_preview.rs:58 (`triangle_order`).
    todo!()
}

/// For an `n` × `n` grid of cells over the unit square mapped by `h` (`(n + 1)²` vertices, row by
/// row): whether each vertex lands on the same side of the horizon as the unit square (its
/// homogeneous `w` has the sign of the square's, beyond 1e-9). A Distort dragged past the horizon
/// sends the rest of the image to the far side, where its cells would wrap round infinity.
pub(super) fn front_vertices(h: &Homography, n: usize) -> Vec<bool> {
    // pc: crates/ui-egui/src/tools/transform_tool/draw.rs:83 (`front_vertices`).
    todo!()
}

/// `mesh` with every triangle cut to the document `canvas` (0, 0)–(width, height) in document
/// space, so the cut follows a turned view.
pub(super) fn clip_to_canvas(mesh: PreviewMesh, canvas: [u32; 2]) -> PreviewMesh {
    // pc: crates/ui-egui/src/tools/transform_tool/draw.rs:395 (`clip_to_canvas`).
    todo!()
}
