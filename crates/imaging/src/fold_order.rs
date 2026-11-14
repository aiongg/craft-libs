//! The cell order of a warp: one order for the preview and the render, without egui or kurbo.

/// The order the cells of a warp mesh row `row` are drawn in: turned-over cells last, then by increasing displacement,
/// a non-finite vertex under, a stable sort. `pos` are the displaced points and `rest` the points at rest, as
/// `[x, y]` pairs.
pub fn cell_order(pos: &[[f64; 2]], rest: &[[f64; 2]], row: usize) -> Vec<usize> {
    let _ = (pos, rest, row);
    todo!("tl.T13")
}
