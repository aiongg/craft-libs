//! [`paint_mesh`]: the one painter of a deformed mesh's lines.

use egui::{Painter, Pos2, Shape};

use super::MeshStyle;

/// Draws the lines of a deformed mesh, each edge once, in [`MeshStyle::line`]: Puppet Warp's pin
/// mesh (the edges of [`PinMeshView::edges`](super::pin::PinMeshView::edges) at the deformed
/// positions) and Liquify's mesh (its grid lines). Decoration: no AccessKit node, no sense.
/// `edges` are screen points.
pub fn paint_mesh(painter: &Painter, edges: impl IntoIterator<Item = [Pos2; 2]>, style: &MeshStyle) {
    // pc: crates/ui-egui/src/tools/puppet_ui.rs:366 (each triangle edge once, deformed positions).
    for [a, b] in edges {
        painter.add(Shape::line_segment([a, b], style.line));
    }
}
