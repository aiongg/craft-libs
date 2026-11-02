//! Tests of the preview maps and painter (task TR-T10).

use egui::{Color32, ColorImage, Context, Pos2, RawInput, Shape};
use kurbo::{Point, Rect};

use craft_ui::canvas::CanvasView;

use super::maps::{PreviewMesh, clip_to_canvas, samples, triangle_order};
use super::{PreviewMap, PreviewPixels, PreviewTextures, TransformPreview};
use crate::ui::warp::WarpMesh;
use crate::ui::warp::pin::PinMeshView;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// The meshes `TransformPreview::show` draws for `build`, in a 200 × 200 canvas view at zoom 1.
fn shown(
    build: impl Fn(&Context, &PreviewTextures) -> PreviewMap<'static>,
    opacity: f32,
    clip: Option<[u32; 2]>,
    view: CanvasView,
) -> Vec<egui::Mesh> {
    let ctx = Context::default();
    let textures = PreviewTextures::new(&ctx, "t", PreviewPixels::new(ColorImage::new([2, 2], vec![Color32::WHITE; 4]), [1.0, 1.0]));
    let map = build(&ctx, &textures);
    let mut output = ctx.run_ui(RawInput::default(), |ui| {
        let mut preview = TransformPreview::new("t", &textures).map(map).opacity(opacity);
        if let Some(clip) = clip {
            preview = preview.clip(clip);
        }
        preview.show(ui, &view);
    });
    output.textures_delta.clear();
    output
        .shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            Shape::Mesh(mesh) => Some((*mesh).clone()),
            _ => None,
        })
        .collect()
}

fn view() -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(200.0, 200.0)), pt(100.0, 100.0), 1.0)
}

fn triangles(mesh: &PreviewMesh) -> impl Iterator<Item = [Point; 3]> + '_ {
    mesh.indices.as_chunks::<3>().0.iter().map(|t| [mesh.points[t[0] as usize], mesh.points[t[1] as usize], mesh.points[t[2] as usize]])
}

// A Distort dragged past the horizon leaves the cells with a corner on the far side out of the
// mesh; the cells in front stay.
#[test]
fn cells_past_the_horizon_are_left_out_of_a_distorted_quad() {
    let flat = PreviewMap::quad([pt(0.0, 0.0), pt(100.0, 0.0), pt(100.0, 100.0), pt(0.0, 100.0)]).build(1.0, [1.0, 1.0]);
    // 24 × 24 cells, two triangles each.
    assert_eq!(flat.indices.len(), 24 * 24 * 6);
    // A dart (the bottom right corner dragged inside the box): the horizon crosses the square.
    let dart = PreviewMap::quad([pt(0.0, 0.0), pt(100.0, 0.0), pt(30.0, 30.0), pt(0.0, 100.0)]).build(1.0, [1.0, 1.0]);
    assert!(!dart.indices.is_empty() && dart.indices.len() < flat.indices.len(), "{}", dart.indices.len());
    assert!(dart.indices.iter().all(|&k| dart.points[k as usize].x.is_finite()));
}

// A quad hanging past the canvas edge draws nothing past it, also under a turned view.
#[test]
fn pixels_past_the_canvas_edge_are_clipped_in_document_space() {
    let quad = |_: &Context, _: &PreviewTextures| PreviewMap::quad([pt(-50.0, -50.0), pt(150.0, -50.0), pt(150.0, 150.0), pt(-50.0, 150.0)]);
    let mut turned = view();
    turned.rotation = 30.0;
    for v in [view(), turned] {
        let meshes = shown(quad, 1.0, Some([100, 100]), v);
        assert_eq!(meshes.len(), 1);
        for vertex in &meshes[0].vertices {
            let at = v.to_doc(vertex.pos);
            assert!(at.x > -1e-3 && at.y > -1e-3 && at.x < 100.001 && at.y < 100.001, "{at:?}");
        }
        // The covered area is the whole canvas (100 × 100 document units).
        let area: f32 = meshes[0]
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|k| meshes[0].vertices[k as usize].pos);
                ((b - a).x * (c - a).y - (b - a).y * (c - a).x).abs() / 2.0
            })
            .sum();
        assert!((area - 10_000.0).abs() < 5.0, "{area}");
    }
    // Without a clip the whole quad is drawn.
    let open = shown(quad, 1.0, None, view());
    assert!(open[0].vertices.iter().any(|v| v.pos.x < 0.0));
}

// A pulled warp's cells come out from the most displaced to the least.
#[test]
fn the_most_displaced_triangles_are_drawn_first() {
    // A 4 x 3 vertex grid; the middle columns were pulled 30 right, so they moved most.
    let rest: Vec<Point> = (0..3).flat_map(|j| (0..4).map(move |i| pt(f64::from(i) * 10.0, f64::from(j) * 10.0))).collect();
    let points: Vec<Point> = rest.iter().enumerate().map(|(k, p)| if k % 4 == 1 || k % 4 == 2 { pt(p.x + 30.0, p.y) } else { *p }).collect();
    let indices = triangle_order(&points, &rest, 4);
    assert_eq!(indices.len(), 2 * 2 * 3 * 3);
    let moved = |k: u32| (points[k as usize] - rest[k as usize]).hypot();
    let per_cell: Vec<f64> = indices.chunks(6).map(|c| c.iter().map(|&k| moved(k)).sum()).collect();
    assert!(per_cell.windows(2).all(|w| w[0] >= w[1]), "{per_cell:?}");
    assert!(per_cell[0] > per_cell[per_cell.len() - 1]);
    assert!(triangle_order(&points, &rest[..5], 4).is_empty());
}

// A narrow, hard-pulled patch gets as many samples as its stretch needs (up to 96 per patch, 240
// per axis); an unwarped mesh keeps 4 per patch.
#[test]
fn a_narrow_pulled_cell_gets_as_many_samples_as_its_stretch_needs() {
    let mut mesh = WarpMesh::identity_with_knots(Rect::new(0.0, 0.0, 900.0, 1100.0), vec![0.0, 0.45, 0.5, 1.0], vec![0.0, 1.0]).expect("valid knots");
    let width = mesh.width();
    for j in 0..mesh.height() {
        for i in 3..=6 {
            mesh.points[j * width + i].x -= 300.0;
        }
    }
    let (us, vs) = samples(&mesh, 1.0);
    let narrow = us.iter().filter(|&&u| (0.45..0.5).contains(&u)).count();
    assert!(narrow >= 30, "{narrow} samples in the narrow patch");
    for knot in [0.0, 0.45, 0.5, 1.0] {
        assert!(us.iter().any(|u| (u - knot).abs() < 1e-12), "{knot}");
    }
    assert!(us.windows(2).all(|w| w[1] > w[0]) && vs.windows(2).all(|w| w[1] > w[0]));
    assert!(us.len() <= 240 + 8 && vs.len() <= 240 + 8, "{} x {}", us.len(), vs.len());
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let (us, _) = samples(&mesh, bad);
        assert!(us.len() > 3);
    }
}

// An identity mesh at zoom 1 samples each patch at the minimum.
#[test]
fn an_unwarped_mesh_stays_cheap() {
    let mesh = WarpMesh::identity(Rect::new(0.0, 0.0, 20.0, 20.0), 2, 2);
    let (us, vs) = samples(&mesh, 1.0);
    assert_eq!((us.len(), vs.len()), (9, 9));
    let drawn = PreviewMap::mesh(&mesh, Rect::new(0.0, 0.0, 20.0, 20.0)).build(1.0, [1.0, 1.0]);
    assert_eq!(drawn.points.len(), 81);
    assert!(drawn.points.iter().zip(&drawn.rest).all(|(p, r)| (*p - *r).hypot() < 1e-9));
}

// A warp mesh's rest is the unwarped frame, also when only the corner controls moved: pulling the
// top-left corner out leaves its rest on the frame's corner, so the pull ranks as a displacement.
#[test]
fn a_warp_meshs_rest_is_the_frame() {
    let frame = Rect::new(10.0, 20.0, 50.0, 60.0);
    let mut mesh = WarpMesh::identity(frame, 2, 2);
    mesh.points[0] = pt(0.0, 0.0);
    let drawn = PreviewMap::mesh(&mesh, frame).build(1.0, [1.0, 1.0]);
    assert!((drawn.rest[0] - pt(10.0, 20.0)).hypot() < 1e-9);
    assert!((drawn.points[0] - pt(0.0, 0.0)).hypot() < 1e-9);
    let last = drawn.rest.len() - 1;
    assert!((drawn.rest[last] - pt(50.0, 60.0)).hypot() < 1e-9);
}

// Opacity 0.5 tints every vertex white at alpha 128.
#[test]
fn opacity_tints_the_mesh() {
    let meshes = shown(|_, _| PreviewMap::quad([pt(10.0, 10.0), pt(90.0, 10.0), pt(90.0, 90.0), pt(10.0, 90.0)]), 0.5, None, view());
    assert_eq!(meshes.len(), 1);
    assert!(meshes[0].vertices.iter().all(|v| v.color == Color32::from_white_alpha(128)));
}

// A pin map draws the solver's triangles in its order with texels at the rest positions; a
// function map draws a 40 × 40 grid through the function.
#[test]
fn pin_and_function_maps_draw_their_meshes() {
    let rest = [pt(10.0, 20.0), pt(30.0, 20.0), pt(30.0, 60.0), pt(10.0, 60.0)];
    let deformed = [pt(15.0, 25.0), pt(35.0, 25.0), pt(35.0, 65.0), pt(15.0, 65.0)];
    let triangles = [[0, 1, 2], [0, 2, 3]];
    let view = PinMeshView { rest: &rest, deformed: &deformed, triangles: &triangles, order: &[1, 0] };
    let pins = PreviewMap::pins(view, Rect::new(10.0, 20.0, 30.0, 60.0)).build(1.0, [0.5, 0.5]);
    assert_eq!(pins.points, deformed);
    assert_eq!(pins.indices, vec![0, 2, 3, 0, 1, 2]);
    assert_eq!(pins.uv[2], [0.5, 0.5]);
    assert_eq!(pins.uv[0], [0.0, 0.0]);
    assert_eq!(triangles_of(&pins), 2);

    let shift = |p: Point| pt(p.x + 5.0, p.y * 2.0);
    let map: &dyn Fn(Point) -> Point = &shift;
    let function = PreviewMap::function(Rect::new(0.0, 0.0, 80.0, 40.0), 40, map).build(1.0, [1.0, 1.0]);
    assert_eq!(function.points.len(), 41 * 41);
    assert_eq!(function.indices.len(), 40 * 40 * 6);
    assert_eq!(function.points[40], pt(85.0, 0.0));
    assert_eq!(function.points[41 * 41 - 1], pt(85.0, 80.0));
    assert_eq!(function.uv[40], [1.0, 0.0]);
}

fn triangles_of(mesh: &PreviewMesh) -> usize {
    triangles(mesh).count()
}

// The quad's scale for the texture level is its longest side over the source frame's.
#[test]
fn a_box_reports_how_much_it_enlarges_the_source() {
    let quad = PreviewMap::quad([pt(0.0, 0.0), pt(200.0, 0.0), pt(200.0, 50.0), pt(0.0, 50.0)]);
    assert!((quad.source_scale([100, 100]) - 2.0).abs() < 1e-6);
    let mesh = WarpMesh::identity(Rect::new(0.0, 0.0, 10.0, 10.0), 1, 1);
    assert!((PreviewMap::mesh(&mesh, Rect::new(0.0, 0.0, 100.0, 100.0)).source_scale([100, 100]) - 1.0).abs() < 1e-6);
}

#[test]
fn clipping_keeps_a_mesh_inside_the_canvas_whole() {
    let mesh = PreviewMap::quad([pt(10.0, 10.0), pt(20.0, 10.0), pt(20.0, 20.0), pt(10.0, 20.0)]).build(1.0, [1.0, 1.0]);
    assert_eq!(clip_to_canvas(mesh.clone(), [100, 100]), mesh);
}
