//! A mask's boundary traced to outline segments, without egui.
//!
//! The selection overlay (`craft_imaging::ui::selection::SelectionOverlay`) draws marching ants
//! along these segments. The trace runs at display resolution: [`outline`] samples one pixel per
//! `step` × `step` block, so a 36 MP selection at 12 % zoom costs a 0.5 MP scan rather than a read
//! of the whole mask, and [`quantised_region`] keeps small pans on the cached trace.
//! [`CoverageGrid`] and [`edges_of_grid`] serve a boolean grid the app already holds (Content-Aware
//! Fill's fill area).
//!
//! The library has no raster type: the app implements [`MaskSource`] over its own mask (a
//! selection, a layer mask, a channel), as it implements `craft_ui::canvas::RasterSource` over its
//! composite. Every function here is serial; an app that wants the trace on another thread runs
//! [`outline`] in its own job and hands the overlay the segments.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

#[cfg(test)]
mod tests;

/// One boundary segment in document pixels, from one end to the other. Every segment is
/// horizontal or vertical.
pub type Segment = ([i32; 2], [i32; 2]);

/// The most cells a trace grid may have: [`bounded_step`] doubles the step until the grid fits.
// pc: crates/ui-egui/src/canvas/outline.rs:22 (1_048_576).
pub const TRACE_BUDGET: u64 = 1_048_576;

/// The most unit edges [`edges_of_grid`] returns before giving up.
// pc: crates/ui-egui/src/dialogs/caf_ui.rs:60 `MAX_OUTLINE_EDGES` (200_000).
pub const GRID_EDGE_LIMIT: usize = 200_000;

/// The finest and the coarsest trace step ([`trace_step`]).
// pc: crates/ui-egui/src/canvas.rs:248 (`clamp(1.0, 64.0)`).
pub const STEP_RANGE: (u32, u32) = (1, 64);

/// A rectangle of document pixels, `x0..x1` by `y0..y1` (the ends excluded): the egui-free twin of
/// `craft_ui::canvas::PixelRect`, in signed pixels because a mask's content may lie off the canvas
/// (a floating piece, a transformed selection).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MaskRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl MaskRect {
    /// The pixels `x0..x1` by `y0..y1`.
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    /// The whole canvas of `size` pixels, from the origin.
    pub fn of_size(size: [u32; 2]) -> Self {
        let side = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
        Self::new(0, 0, side(size[0]), side(size[1]))
    }

    /// No pixel inside.
    pub fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    /// The width in pixels, 0 when empty.
    pub fn width(&self) -> u32 {
        u32::try_from(i64::from(self.x1) - i64::from(self.x0)).unwrap_or(0)
    }

    /// The height in pixels, 0 when empty.
    pub fn height(&self) -> u32 {
        u32::try_from(i64::from(self.y1) - i64::from(self.y0)).unwrap_or(0)
    }

    /// Whether pixel `(x, y)` is inside.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }

    /// The part inside both; empty (the default) when they do not overlap.
    pub fn intersect(&self, other: &MaskRect) -> MaskRect {
        let r = MaskRect::new(self.x0.max(other.x0), self.y0.max(other.y0), self.x1.min(other.x1), self.y1.min(other.y1));
        if r.is_empty() { MaskRect::default() } else { r }
    }
}

/// A coverage mask the app owns: the selection, a layer mask, a channel, a proxy's refined
/// selection. Coverage is 0 (unselected) to 1 (selected); the trace's threshold is 0.5.
///
/// The app implements it over its own storage; PhotoCraft's is about 30 lines over its tiled
/// `Surface` (`sample_row_strided`, `sample_channel`).
pub trait MaskSource {
    /// The document's size in pixels: the canvas the mask belongs to.
    fn size(&self) -> [u32; 2];

    /// Changes whenever the coverage changes. The overlay retraces only when it, the step or the
    /// region changes, so an edit elsewhere in the document (a brush stroke on a layer) keeps the
    /// trace: key it on the mask, not on the document's revision (PhotoCraft fingerprints the
    /// mask's tiles).
    fn revision(&self) -> u64;

    /// The coverage everywhere outside [`content_bounds`](Self::content_bounds): 0 for a
    /// selection, 1 for an inverted one or a reveal-all layer mask.
    fn outside(&self) -> f32 {
        0.0
    }

    /// The bounds of the pixels whose coverage differs from [`outside`](Self::outside); empty for
    /// an empty mask. The trace reads nothing beyond it.
    fn content_bounds(&self) -> MaskRect;

    /// Writes the coverage of row `y` at `x0`, `x0 + step`, `x0 + 2 · step`, … into `out` (one
    /// value per element of `out`). Pixels outside the content read [`outside`](Self::outside).
    fn read_row(&self, y: i32, x0: i32, step: u32, out: &mut [f32]);

    /// The coverage of one pixel. Default: a one-element [`read_row`](Self::read_row).
    fn at(&self, x: i32, y: i32) -> f32 {
        let mut v = [0.0];
        self.read_row(y, x, 1, &mut v);
        v[0]
    }
}

/// The trace step for a view showing `device_zoom` device pixels per document pixel of a mask
/// reduced `reduction` times (Select and Mask's preview proxy): the largest power of two at or
/// below `1 / (device_zoom · reduction)`, within [`STEP_RANGE`]. Device pixels, not points, so a
/// one-pixel selection still traces at 100 % on a scaled display.
pub fn trace_step(device_zoom: f64, reduction: u32) -> u32 {
    // pc: crates/ui-egui/src/canvas.rs:248 (`(1 / max(zoom · k, 1e-3)).log2().floor().exp2()`,
    // clamped 1..=64).
    todo!()
}

/// The region of a `reduction`-times reduced mask to trace for the document rectangle `visible`
/// (`[x0, y0, x1, y1]` in full-size document units, the view's visible bounds): divided by
/// `reduction`, rounded out to multiples of `q = 256 · step` and padded by one `q` before and two
/// after, so pans of less than `q` keep the same region and the cached trace. Coordinates are
/// clamped to ±2³⁰ first (a view centre may be any finite number), so the padding cannot overflow.
pub fn quantised_region(visible: [f64; 4], step: u32, reduction: u32) -> MaskRect {
    // pc: crates/ui-egui/src/canvas.rs:261-277 (`q = 256 · step`; `floor(x0).div_euclid(q) · q −
    // q`, `(ceil(x1).div_euclid(q) + 2) · q`; the ±2³⁰ clamp at :265).
    todo!()
}

/// `step` doubled until the trace grid over `bounds` (rounded out to whole cells, plus the two
/// padding cells on each axis) has at most [`TRACE_BUDGET`] cells. A step below 1 counts as 1.
/// Used when the whole selection is traced (a Free Transform carries it, so parts outside the view
/// may come into it), never for an ordinary visible-region trace.
pub fn bounded_step(bounds: MaskRect, step: u32) -> u32 {
    // pc: crates/ui-egui/src/canvas/outline.rs:15-26 `bounded_step` (cells per axis
    // `ceil(hi / s) − floor(lo / s) + 2`, in i64; the product saturates).
    todo!()
}

/// The boundary of `mask` inside `bounds` (coverage above 0.5 is inside), sampling one pixel per
/// `step` × `step` block, as merged horizontal runs followed by merged vertical runs.
///
/// The grid is padded by one cell on every side, so edges on `bounds` are found. Each cell samples
/// its centre pixel; a row's sample `y` is clamped into `bounds`. A cell whose centre falls outside
/// `bounds` but which overlaps it samples the nearest pixel inside (one [`MaskSource::at`]), so a
/// one-pixel selection still shows at a coarse step; padding cells stay outside. Segment ends are
/// clamped to `bounds`, so a coarse cell never draws outside the selection. Empty `bounds` give no
/// segments.
pub fn outline<M: MaskSource + ?Sized>(mask: &M, bounds: MaskRect, step: u32) -> Vec<Segment> {
    // pc: crates/ui-egui/src/canvas/outline.rs:44-126 `outline_scaled_impl` (serial; the Rayon
    // branch is not ported, lead call 9).
    todo!()
}

/// A boolean grid of document pixels the app holds already: a cell is inside or not. Content-Aware
/// Fill's fill area and sampling area are such grids.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoverageGrid {
    origin: [i32; 2],
    size: [u32; 2],
    cells: Vec<bool>,
}

impl CoverageGrid {
    /// A grid of `size` cells whose top-left cell is document pixel `origin`; `cells` row by row,
    /// `size[0] · size[1]` of them. Missing cells are outside, extra ones are ignored.
    pub fn new(origin: [i32; 2], size: [u32; 2], cells: Vec<bool>) -> Self {
        Self { origin, size, cells }
    }

    /// The document pixel of the top-left cell.
    pub fn origin(&self) -> [i32; 2] {
        self.origin
    }

    /// The grid's width and height in cells.
    pub fn grid_size(&self) -> [u32; 2] {
        self.size
    }

    /// Whether document pixel `(x, y)` is inside; `false` off the grid.
    pub fn inside(&self, x: i32, y: i32) -> bool {
        // new: indexes `cells` from `origin`, row by row.
        todo!()
    }
}

/// A grid as a mask: inside cells read 1, the rest 0. Its revision is a hash of the cells, so an
/// overlay retraces only when they change.
impl MaskSource for CoverageGrid {
    fn size(&self) -> [u32; 2] {
        // new: the grid's far corner, `origin + grid_size`, clamped at 0.
        todo!()
    }

    fn revision(&self) -> u64 {
        // new: a hash of `origin`, `size` and `cells`.
        todo!()
    }

    fn content_bounds(&self) -> MaskRect {
        // new: the grid's own rectangle (not shrunk to the inside cells).
        todo!()
    }

    fn read_row(&self, y: i32, x0: i32, step: u32, out: &mut [f32]) {
        // new: `out[i] = 1.0` when `inside(x0 + i · step, y)`, else 0.
        todo!()
    }
}

/// Every unit edge between an inside cell of `grid` and an outside one (or the grid's border), in
/// document pixels, without merging into runs: per inside cell in row order, its left, right, top
/// and bottom edge where the neighbour is outside. `None` when there are more than `max_edges`
/// (callers pass [`GRID_EDGE_LIMIT`]); the caller then draws no outline.
pub fn edges_of_grid(grid: &CoverageGrid, max_edges: usize) -> Option<Vec<Segment>> {
    // pc: crates/ui-egui/src/dialogs/caf_ui/preview.rs:103-132 `outline` (the order left, right,
    // top, bottom per cell; `None` past the limit).
    todo!()
}
