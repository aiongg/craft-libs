//! Vanishing Point's drawing: planes, grids, nodes, the paste, the stamp.

use egui::epaint::Vertex;
use egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, pos2};

use craft_ui::canvas::CanvasView;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::quad::Homography;

use super::VanishingPointSource;
use super::editor::{VanishingPointEditor, VanishingPointSession, VanishingPointSettings, VanishingTool, paste_extent, paste_plane};
use super::planes::{MAX_GRID_LINES, PlaneCheck, VanishingPlane, VanishingScene};
use super::style::VanishingPointStyle;

/// The paste's mesh is this many cells a side.
const PASTE_CELLS: usize = 24;

/// Draws the editor over the image, in this order (a stamp preview is the canvas's image itself:
/// the workspace shows `VanishingPointSource::stamp_preview` instead of the layer while strokes
/// exist, lead call 47):
///
/// 1. The paste on the selected plane (else plane 0) while Paste is on, unless the stamp preview is
///    shown (the app paints the paste into it): the app's image mapped through the plane's frame
///    on a 24 × 24 mesh tinted white at `.paste_alpha`, cells outside the plane left out. It lies under the grids, as PhotoCraft
///    draws it, so a plane's lines stay visible across it.
/// 2. Each plane (not one the scene could not rebuild): an invalid plane only its outline; the
///    others their grid (cells of `grid_size` at the nearest edge, at most `MAX_GRID_LINES` a
///    direction, the grid at `.grid_alpha`) and outline (`.line`, the selected plane
///    `.line_selected`), in the check's color; the outline over the halo.
/// 3. The selected plane's nodes: corners and edge middles as `.node` white squares with a
///    `.line` outline in the plane's color; Create Plane's pending corners as `.pending` squares
///    with the rubber band to the pointer.
/// 4. Stamp: the source cross (`.source_arm` arms) and the foreshortened ring of the diameter
///    ([`VanishingScene::stamp_ring`]), each a `.ring_line` white line over the `.halo` at
///    `.halo_width`.
///
/// Decoration: the parts' AccessKit nodes are the host's (`ToolParts`).
#[expect(clippy::too_many_arguments, reason = "one painter over the editor's whole state")]
pub fn paint_editor(
    painter: &Painter,
    view: &CanvasView,
    editor: &VanishingPointEditor,
    session: &VanishingPointSession,
    settings: &VanishingPointSettings,
    scene: &dyn VanishingScene,
    source: &dyn VanishingPointSource,
    style: &VanishingPointStyle,
) {
    // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1078 `draw_content`, :1091 `draw` (colors, widths,
    // nodes :1091-1135), :1148 `draw_stamp`, :1189 `grid_steps`, :1205 `draw_paste`.
    let planes = &session.planes;
    let screen = |p: [f64; 2]| view.to_screen(Point::new(p[0], p[1]));
    let frame = |i: usize| {
        let (h, lu, lv) = scene.frame(planes, i)?;
        (lu.is_finite() && lv.is_finite() && lu > 1e-12 && lv > 1e-12).then_some((h, lu, lv))
    };
    let selected = editor.selected().filter(|i| *i < planes.len());
    let color_of = |check: PlaneCheck| match check {
        PlaneCheck::Valid => style.valid,
        PlaneCheck::Questionable => style.questionable,
        PlaneCheck::Invalid => style.invalid,
    };

    let previewed = !session.strokes.is_empty() && source.stamp_preview().is_some();
    if !previewed && let Some(index) = paste_plane(settings, source, planes.len(), selected) {
        paint_paste(painter, &screen, planes, index, session, scene, source, style, &frame);
    }

    for (i, plane) in planes.iter().enumerate() {
        if is_unset(plane) {
            continue;
        }
        let check = scene.check(planes, i);
        let color = color_of(check);
        if check != PlaneCheck::Invalid
            && let Some((h, lu, lv)) = frame(i)
        {
            let grid = Stroke::new(style.grid, color.gamma_multiply(style.grid_alpha));
            let at = |u: f64, v: f64| {
                let p = h.apply(Point::new(u, v));
                screen([p.x, p.y])
            };
            for u in grid_steps(plane, lu, lv, settings.grid_size, true) {
                painter.line_segment([at(u, 0.0), at(u, 1.0)], grid);
            }
            for v in grid_steps(plane, lu, lv, settings.grid_size, false) {
                painter.line_segment([at(0.0, v), at(1.0, v)], grid);
            }
        }
        let width = if selected == Some(i) { style.line_selected } else { style.line };
        let points: Vec<Pos2> = plane.corners.iter().map(|c| screen(*c)).collect();
        painter.add(Shape::closed_line(points.clone(), Stroke::new(halo_width(style, width), style.halo)));
        painter.add(Shape::closed_line(points, Stroke::new(width, color)));
    }

    if let Some(i) = selected.filter(|i| !is_unset(&planes[*i])) {
        let plane = &planes[i];
        let color = color_of(scene.check(planes, i));
        for centre in plane.corners.iter().copied().chain(plane.edge_nodes()).map(screen) {
            let square = Rect::from_center_size(centre, Vec2::splat(style.node));
            painter.rect_filled(square, 0.0, style.node_fill);
            painter.rect_stroke(square, 0.0, Stroke::new(style.line, color), StrokeKind::Inside);
        }
    }

    if !editor.pending().is_empty() {
        let mut points: Vec<Pos2> = editor.pending().iter().map(|c| screen(*c)).collect();
        let corners = points.len();
        if let Some(pointer) = editor.pointer() {
            points.push(screen(pointer));
        }
        painter.add(Shape::line(points.clone(), Stroke::new(style.line, style.valid)));
        for c in points.iter().take(corners) {
            painter.rect_filled(Rect::from_center_size(*c, Vec2::splat(style.pending)), 0.0, style.valid);
        }
    }

    if editor.tool() == VanishingTool::Stamp {
        paint_stamp(painter, &screen, editor, session, settings, scene, style);
    }
}

/// A line of `width` over its halo: the halo reaches as far past the line as it does past the
/// source cross and the ring (`halo_width` over `line`).
fn halo_width(style: &VanishingPointStyle, width: f32) -> f32 {
    width + (style.halo_width - style.line).max(0.0)
}

/// A torn plane the scene could not rebuild has all-zero corners.
fn is_unset(plane: &VanishingPlane) -> bool {
    plane.corners.iter().flatten().all(|v| *v == 0.0)
}

/// Plane coordinates (0..1, exclusive) of the grid lines across `u` (or `v`) for cells of `size`
/// pixels at the plane's nearest edge, square in the plane's metric.
fn grid_steps(plane: &VanishingPlane, lu: f64, lv: f64, size: f64, along_u: bool) -> Vec<f64> {
    let q = &plane.corners;
    let d = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    // Image pixels per metric unit at the nearest (longest-looking) edge.
    let k = [d(q[0], q[1]) / lu, d(q[3], q[2]) / lu, d(q[0], q[3]) / lv, d(q[1], q[2]) / lv].into_iter().fold(0.0, f64::max);
    let cell = size.max(1.0) / k;
    let len = if along_u { lu } else { lv };
    if !cell.is_finite() || cell <= 0.0 || !len.is_finite() {
        return Vec::new();
    }
    let step = cell / len;
    (1..MAX_GRID_LINES).map(|j| j as f64 * step).take_while(|t| *t < 1.0 - 1e-9).collect()
}

/// The clipboard image on plane `index` (the one [`paste_plane`] chose): a mesh through the plane's
/// frame tinted white at the paste alpha, cells outside the plane left out (the engine
/// clips the paste to the plane). Its extent is the one the editor drags ([`paste_extent`]).
#[expect(clippy::too_many_arguments, reason = "the paste reads the same state as the planes")]
fn paint_paste(
    painter: &Painter,
    screen: &dyn Fn([f64; 2]) -> Pos2,
    planes: &[VanishingPlane],
    index: usize,
    session: &VanishingPointSession,
    scene: &dyn VanishingScene,
    source: &dyn VanishingPointSource,
    style: &VanishingPointStyle,
    frame: &dyn Fn(usize) -> Option<(Homography, f64, f64)>,
) {
    let Some(texture) = source.paste() else { return };
    if is_unset(&planes[index]) || scene.check(planes, index) == PlaneCheck::Invalid {
        return;
    }
    let Some((h, lu, lv)) = frame(index) else { return };
    let extent = paste_extent(texture.size, session.paste.width, lu, lv);
    let at = session.paste.at;
    let tint = Color32::WHITE.gamma_multiply(style.paste_alpha);
    let n = PASTE_CELLS;
    let mut mesh = Mesh::with_texture(texture.id);
    for j in 0..=n {
        for i in 0..=n {
            let (s, t) = (i as f64 / n as f64, j as f64 / n as f64);
            let p = h.apply(Point::new(at[0] + s * extent[0], at[1] + t * extent[1]));
            mesh.vertices.push(Vertex { pos: screen([p.x, p.y]), uv: pos2(s as f32, t as f32), color: tint });
        }
    }
    let row = n as u32 + 1;
    for j in 0..n {
        for i in 0..n {
            let u = at[0] + (i as f64 + 0.5) / n as f64 * extent[0];
            let v = at[1] + (j as f64 + 0.5) / n as f64 * extent[1];
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                continue;
            }
            let a = (j * (n + 1) + i) as u32;
            mesh.add_triangle(a, a + 1, a + row + 1);
            mesh.add_triangle(a, a + row + 1, a + row);
        }
    }
    painter.add(mesh);
}

/// Stamp: the source as a cross and the brush under the pointer as the circle it paints on the
/// plane, foreshortened.
fn paint_stamp(
    painter: &Painter,
    screen: &dyn Fn([f64; 2]) -> Pos2,
    editor: &VanishingPointEditor,
    session: &VanishingPointSession,
    settings: &VanishingPointSettings,
    scene: &dyn VanishingScene,
    style: &VanishingPointStyle,
) {
    // PhotoCraft's ring and cross: a `halo_width` halo under a `ring_line` white line.
    let halo = Stroke::new(style.halo_width, style.halo);
    let ink = Stroke::new(style.ring_line, style.node_fill);
    if let Some(source) = session.source {
        let c = screen(source);
        for arm in [Vec2::new(style.source_arm, 0.0), Vec2::new(0.0, style.source_arm)] {
            painter.line_segment([c - arm, c + arm], halo);
            painter.line_segment([c - arm, c + arm], ink);
        }
    }
    let Some(at) = editor.pointer() else { return };
    let Some(ring) = scene.stamp_ring(&session.planes, at, settings.stamp.diameter) else { return };
    let ring: Vec<Pos2> = ring.iter().map(|p| screen([p.x, p.y])).collect();
    if ring.len() > 2 && ring.iter().all(|p| p.is_finite()) {
        painter.add(Shape::closed_line(ring.clone(), halo));
        painter.add(Shape::closed_line(ring, ink));
    }
}
