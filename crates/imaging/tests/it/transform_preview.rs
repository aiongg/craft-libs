//! The transform preview: its maps, its texture levels and its styled composite.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect};
use craft_imaging::ui::transform_preview::{
    PreviewDisplay, PreviewMap, PreviewMask, PreviewMesh, PreviewPixels, PreviewTextures, StylePlan, StyledBudget, StyledContent, StyledFrame,
    StyledPreview, StyledSource, TextureChoice, TransformPreview, resample,
};
use craft_imaging::ui::warp::WarpMesh;
use craft_imaging::ui::warp::pin::PinMeshView;
use craft_ui::canvas::{Canvas, PixelRect, ToolPath};
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Color32, ColorImage, Shape};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn solid(size: [usize; 2], color: Color32) -> ColorImage {
    ColorImage::new(size, vec![color; size[0] * size[1]])
}

fn choice(level: u32, nearest: bool) -> TextureChoice {
    TextureChoice::new(level, nearest)
}

/// The meshes `TransformPreview` paints for `map` in the fixture's canvas under `rotation` degrees, and the view.
fn shown(map: fn() -> PreviewMap<'static>, opacity: f32, clip: Option<[u32; 2]>, rotation: f64) -> (Vec<egui::Mesh>, CanvasView) {
    let meshes = Rc::new(RefCell::new(Vec::new()));
    let seen_view = Rc::new(Cell::new(None));
    let (sink, view_sink) = (Rc::clone(&meshes), Rc::clone(&seen_view));
    let textures = PreviewTextures::new("t", PreviewPixels::new(solid([2, 2], Color32::from_gray(u8::MAX)), [1.0, 1.0]));
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, pt(100.0, 100.0), 1.0).rotation(rotation);
        view_sink.set(Some(view));
        Canvas::new("Document", ToolPath::Freehand).view(view).show(ui, |ui, body| {
            let mut preview = TransformPreview::new("Moving layer", &textures).map(map()).opacity(opacity);
            if let Some(clip) = clip {
                preview = preview.clip(clip);
            }
            preview.show_at(ui, body.rect)
        });
        let layer = ui.layer_id();
        *sink.borrow_mut() = ui.ctx().graphics(|g| {
            g.get(layer)
                .map(|list| {
                    list.all_entries()
                        .filter_map(|clipped| match &clipped.shape {
                            Shape::Mesh(mesh) => Some((**mesh).clone()),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default()
        });
    });
    fixture.settle();
    let view = seen_view.get().expect("the canvas was shown");
    (meshes.take(), view)
}

fn triangles(mesh: &PreviewMesh) -> impl Iterator<Item = [Point; 3]> + '_ {
    mesh.indices.as_chunks::<3>().0.iter().map(|t| [mesh.points[t[0] as usize], mesh.points[t[1] as usize], mesh.points[t[2] as usize]])
}

fn unit_quad() -> [Point; 4] {
    [pt(0.0, 0.0), pt(100.0, 0.0), pt(100.0, 100.0), pt(0.0, 100.0)]
}

// A Distort dragged past the horizon leaves the cells with a corner on the far side out of the mesh; the cells in
// front stay.
#[trial]
fn cells_past_the_horizon_are_left_out_of_a_distorted_quad() {
    let flat = PreviewMap::quad(unit_quad()).build(1.0, [1.0, 1.0]);
    // 24 × 24 cells, two triangles each.
    assert_eq!(flat.indices.len(), 24 * 24 * 6);
    // A dart (the bottom right corner dragged inside the box): the horizon crosses the square.
    let dart = PreviewMap::quad([pt(0.0, 0.0), pt(100.0, 0.0), pt(30.0, 30.0), pt(0.0, 100.0)]).build(1.0, [1.0, 1.0]);
    assert!(!dart.indices.is_empty() && dart.indices.len() < flat.indices.len(), "{}", dart.indices.len());
    assert!(dart.indices.iter().all(|&k| dart.points[k as usize].x.is_finite()));
}

// A quad hanging past the canvas edge draws nothing past it, also under a turned view.
#[trial]
fn pixels_past_the_canvas_edge_are_clipped_in_document_space() {
    fn quad() -> PreviewMap<'static> {
        PreviewMap::quad([pt(-50.0, -50.0), pt(150.0, -50.0), pt(150.0, 150.0), pt(-50.0, 150.0)])
    }
    for rotation in [0.0, 30.0] {
        let (meshes, view) = shown(quad, 1.0, Some([100, 100]), rotation);
        assert_eq!(meshes.len(), 1);
        for vertex in &meshes[0].vertices {
            let at = view.to_doc(vertex.pos);
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
    let (open, view) = shown(quad, 1.0, None, 0.0);
    assert!(open[0].vertices.iter().any(|v| view.to_doc(v.pos).x < 0.0));
}

// A pulled warp's cells come out from the most displaced to the least.
#[trial]
fn the_most_displaced_triangles_are_drawn_first() {
    let frame = Rect::new(0.0, 0.0, 300.0, 200.0);
    let mut mesh = WarpMesh::identity(frame, 3, 2);
    let width = mesh.width();
    // The middle columns are pulled 30 right, so they move most.
    for j in 0..mesh.height() {
        for i in 1..=2 {
            mesh.points[j * width + i].x += 30.0;
        }
    }
    let drawn = PreviewMap::mesh(&mesh, frame).build(1.0, [1.0, 1.0]);
    let moved = |k: u32| (drawn.points[k as usize] - drawn.rest[k as usize]).hypot();
    let per_cell: Vec<f64> = drawn.indices.chunks(6).map(|cell| cell.iter().map(|&k| moved(k)).sum()).collect();
    assert!(per_cell.len() > 4);
    assert!(per_cell.windows(2).all(|w| w[0] >= w[1]), "{per_cell:?}");
    assert!(per_cell[0] > per_cell[per_cell.len() - 1]);
}

// A narrow, hard-pulled patch gets as many samples as its stretch needs (up to 96 per patch, 240 per axis); an
// unwarped mesh keeps 4 per patch.
#[trial]
fn a_narrow_pulled_cell_gets_as_many_samples_as_its_stretch_needs() {
    let frame = Rect::new(0.0, 0.0, 900.0, 1100.0);
    let mut mesh = WarpMesh::identity_with_knots(frame, vec![0.0, 0.45, 0.5, 1.0], vec![0.0, 1.0]).expect("valid knots");
    let width = mesh.width();
    for j in 0..mesh.height() {
        for i in 3..=6 {
            mesh.points[j * width + i].x -= 300.0;
        }
    }
    // With uv extent 1 a vertex's uv is its sample's place across the frame.
    let along = |zoom: f64| {
        let drawn = PreviewMap::mesh(&mesh, frame).build(zoom, [1.0, 1.0]);
        let mut us: Vec<f32> = drawn.uv.iter().map(|uv| uv[0]).collect();
        us.dedup();
        us.sort_by(f32::total_cmp);
        us.dedup();
        us
    };
    let us = along(1.0);
    let narrow = us.iter().filter(|&&u| (0.45..0.5).contains(&u)).count();
    assert!(narrow >= 30, "{narrow} samples in the narrow patch");
    for knot in [0.0f32, 0.45, 0.5, 1.0] {
        assert!(us.iter().any(|u| (u - knot).abs() < 1e-6), "{knot}");
    }
    assert!(us.len() <= 240 + 8, "{}", us.len());
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(along(bad).len() > 3, "{bad}");
    }
}

// An identity mesh at zoom 1 samples each patch at the minimum.
#[trial]
fn an_unwarped_mesh_stays_cheap() {
    let frame = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mesh = WarpMesh::identity(frame, 2, 2);
    let drawn = PreviewMap::mesh(&mesh, frame).build(1.0, [1.0, 1.0]);
    assert_eq!(drawn.points.len(), 9 * 9);
    assert!(drawn.points.iter().zip(&drawn.rest).all(|(p, r)| (*p - *r).hypot() < 1e-9));
}

// A warp mesh's rest is the unwarped frame, also when only the corner controls moved: pulling the top-left corner out
// leaves its rest on the frame's corner, so the pull ranks as a displacement.
#[trial]
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
#[trial]
fn opacity_tints_the_mesh() {
    fn quad() -> PreviewMap<'static> {
        PreviewMap::quad([pt(10.0, 10.0), pt(90.0, 10.0), pt(90.0, 90.0), pt(10.0, 90.0)])
    }
    let (meshes, _) = shown(quad, 0.5, None, 0.0);
    assert_eq!(meshes.len(), 1);
    assert!(meshes[0].vertices.iter().all(|v| v.color == Color32::from_white_alpha(128)));
}

// A pin map draws the solver's triangles in its order with texels at the rest positions; a function map draws a
// 40 × 40 grid through the function.
#[trial]
fn pin_and_function_maps_draw_their_meshes() {
    let rest = [pt(10.0, 20.0), pt(30.0, 20.0), pt(30.0, 60.0), pt(10.0, 60.0)];
    let deformed = [pt(15.0, 25.0), pt(35.0, 25.0), pt(35.0, 65.0), pt(15.0, 65.0)];
    let triangle_list = [[0, 1, 2], [0, 2, 3]];
    let view = PinMeshView { rest: &rest, deformed: &deformed, triangles: &triangle_list, order: &[1, 0] };
    let pins = PreviewMap::pins(view, Rect::new(10.0, 20.0, 30.0, 60.0)).build(1.0, [0.5, 0.5]);
    assert_eq!(pins.points, deformed);
    assert_eq!(pins.indices, vec![0, 2, 3, 0, 1, 2]);
    assert_eq!(pins.uv[2], [0.5, 0.5]);
    assert_eq!(pins.uv[0], [0.0, 0.0]);
    assert_eq!(triangles(&pins).count(), 2);

    let shift = |p: Point| pt(p.x + 5.0, p.y * 2.0);
    let map: &dyn Fn(Point) -> Point = &shift;
    let function = PreviewMap::function(Rect::new(0.0, 0.0, 80.0, 40.0), 40, map).build(1.0, [1.0, 1.0]);
    assert_eq!(function.points.len(), 41 * 41);
    assert_eq!(function.indices.len(), 40 * 40 * 6);
    assert_eq!(function.points[40], pt(85.0, 0.0));
    assert_eq!(function.points[41 * 41 - 1], pt(85.0, 80.0));
    assert_eq!(function.uv[40], [1.0, 0.0]);
}

// The quad's scale for the texture level is its longest side over the source frame's.
#[trial]
fn a_box_reports_how_much_it_enlarges_the_source() {
    let quad = PreviewMap::quad([pt(0.0, 0.0), pt(200.0, 0.0), pt(200.0, 50.0), pt(0.0, 50.0)]);
    assert!((quad.source_scale([100, 100]) - 2.0).abs() < 1e-6);
    let mesh = WarpMesh::identity(Rect::new(0.0, 0.0, 10.0, 10.0), 1, 1);
    assert!((PreviewMap::mesh(&mesh, Rect::new(0.0, 0.0, 100.0, 100.0)).source_scale([100, 100]) - 1.0).abs() < 1e-6);
}

#[trial]
fn clipping_keeps_a_mesh_inside_the_canvas_whole() {
    let mesh = PreviewMap::quad([pt(10.0, 10.0), pt(20.0, 10.0), pt(20.0, 20.0), pt(10.0, 20.0)]).build(1.0, [1.0, 1.0]);
    assert_eq!(mesh.clone().clipped([100, 100]), mesh);
}

/// A source with a fixed reach (in work pixels, whatever the scale) that counts its compositions.
struct Fake {
    revision: Cell<u64>,
    reach: i32,
    composed: Cell<u32>,
}

impl Fake {
    fn new(reach: i32) -> Self {
        Self { revision: Cell::new(1), reach, composed: Cell::new(0) }
    }
}

impl StyledSource for Fake {
    fn revision(&self) -> u64 {
        self.revision.get()
    }
    fn pixel_cost_ns(&self) -> f64 {
        20.0
    }
    fn reach(&self, _scale: f64) -> i32 {
        self.reach
    }
    fn compose(&self, plan: &StylePlan, _content: &StyledContent) -> Option<ColorImage> {
        self.composed.set(self.composed.get() + 1);
        let size = [plan.shown.width() as usize, plan.shown.height() as usize];
        Some(solid(size, Color32::from_gray(u8::MAX)))
    }
}

fn square(x: f64, y: f64, size: f64) -> [Point; 4] {
    [pt(x, y), pt(x + size, y), pt(x + size, y + size), pt(x, y + size)]
}

fn frame(textures: &PreviewTextures, quad: [Point; 4], zoom: f32) -> StyledFrame<'_> {
    StyledFrame::new(textures, quad, [100, 100], zoom)
}

fn styled_textures() -> PreviewTextures {
    PreviewTextures::new("styled", PreviewPixels::new(solid([4, 4], Color32::from_rgb(200, 100, 50)), [1.0, 1.0]))
}

fn context() -> (Fixture, egui::Context) {
    let fixture = Fixture::new(|_| {});
    let ctx = fixture.ctx().clone();
    (fixture, ctx)
}

// Two passes with the same box, zoom, revision, display key, level and filter build once; a change of any one builds
// again.
#[trial]
fn the_styled_frame_is_rebuilt_only_when_its_key_changes() {
    let (_fixture, ctx) = context();
    let textures = styled_textures();
    let source = Fake::new(2);
    let preview = StyledPreview::new("styled-frame");
    let box_a = square(10.0, 10.0, 40.0);
    assert!(preview.composite(&ctx, &frame(&textures, box_a, 1.0), &source, None).is_some());
    assert!(preview.composite(&ctx, &frame(&textures, box_a, 1.0), &source, None).is_some());
    assert_eq!(preview.builds(), 1);
    assert_eq!(source.composed.get(), 1);

    let mut builds = 1;
    let mut expect_build = |frame: &StyledFrame<'_>, source: &Fake, display: Option<&PreviewDisplay>, what: &str| {
        preview.composite(&ctx, frame, source, display);
        builds += 1;
        assert_eq!(preview.builds(), builds, "{what}");
    };
    expect_build(&frame(&textures, square(11.0, 10.0, 40.0), 1.0), &source, None, "moved box");
    expect_build(&frame(&textures, square(11.0, 10.0, 40.0), 0.5), &source, None, "zoom");
    source.revision.set(2);
    expect_build(&frame(&textures, square(11.0, 10.0, 40.0), 0.5), &source, None, "revision");
    let display = PreviewDisplay::new(7, Clone::clone);
    expect_build(&frame(&textures, square(11.0, 10.0, 40.0), 0.5), &source, Some(&display), "display key");
    let mut nearest = frame(&textures, square(11.0, 10.0, 40.0), 0.5);
    nearest.nearest = true;
    expect_build(&nearest, &source, Some(&display), "filter");
}

// A composite of the wrong size, or none, is asked for once per key; a new key asks again.
#[trial]
fn a_failed_composite_is_not_asked_for_again_until_the_key_changes() {
    struct Wrong(Cell<u32>);
    impl StyledSource for Wrong {
        fn revision(&self) -> u64 {
            1
        }
        fn pixel_cost_ns(&self) -> f64 {
            20.0
        }
        fn reach(&self, _scale: f64) -> i32 {
            2
        }
        fn compose(&self, _plan: &StylePlan, _content: &StyledContent) -> Option<ColorImage> {
            self.0.set(self.0.get() + 1);
            Some(solid([1, 1], Color32::from_gray(u8::MAX)))
        }
    }
    let (_fixture, ctx) = context();
    let textures = styled_textures();
    let source = Wrong(Cell::new(0));
    let preview = StyledPreview::new("styled-wrong");
    for _ in 0..3 {
        assert!(preview.composite(&ctx, &frame(&textures, square(10.0, 10.0, 40.0), 1.0), &source, None).is_none());
    }
    assert_eq!(source.0.get(), 1);
    assert!(preview.composite(&ctx, &frame(&textures, square(11.0, 10.0, 40.0), 1.0), &source, None).is_none());
    assert_eq!(source.0.get(), 2);
    assert_eq!(preview.builds(), 0);
}

// A costly style shrinks the work scale until the content's area fits the budget, in up to 6 rounds; a box off the
// canvas gives None.
#[trial]
fn the_budget_shrinks_the_work_scale_until_the_composite_fits() {
    let canvas = [4000, 4000];
    let big = square(0.0, 0.0, 4000.0);
    let plan = StylePlan::new(big, canvas, 1.0, 1.0e6, |_| 10).unwrap_or_else(|| panic!("a plan"));
    assert!(plan.scale < 1.0);
    let area = f64::from(plan.content.width()) * f64::from(plan.content.height());
    assert!(area <= 1.0e6, "{area}");
    assert!(plan.shown.x0 >= 0 && plan.shown.x1 <= (4000.0 * plan.scale).ceil() as i32);
    assert!(plan.content.x0 <= plan.shown.x0 - 20 || plan.shown.x0 == 0 && plan.content.x0 == -20);

    let small = StylePlan::new(square(10.0, 10.0, 20.0), [100, 100], 4.0, 1.0e6, |_| 3).unwrap_or_else(|| panic!("a plan"));
    assert_eq!(small.scale, 1.0, "the scale never exceeds the document's");
    assert_eq!(small.bounds, PixelRect::new(10, 10, 30, 30));
    assert_eq!(small.shown, PixelRect::new(7, 7, 33, 33));
    assert_eq!(small.content, PixelRect::new(1, 1, 39, 39));
    assert_eq!(small.document_rect(), Rect::new(7.0, 7.0, 33.0, 33.0));

    assert_eq!(StylePlan::new(square(500.0, 500.0, 20.0), [100, 100], 1.0, 1.0e6, |_| 3), None);
    assert_eq!(StylePlan::new(square(f64::NAN, 0.0, 20.0), [100, 100], 1.0, 1.0e6, |_| 3), None);
    // A reach that grows with the scale and never fits gives up after 6 rounds.
    assert_eq!(StylePlan::new(big, canvas, 1.0, 1.0e6, |_| 100_000), None);
}

// The budget is 30 ms dragging and 120 ms at rest over the cost per pixel plus the resample's, clamped to 0.25 to 8
// million pixels.
#[trial]
fn the_budget_follows_what_the_style_costs() {
    let dear = StyledBudget::default();
    assert_eq!(dear.pixels(45.0, true), 0.5e6, "the resample's cost is in the sum");
    assert!(dear.pixels(60.0, true) < 0.5e6);
    let mut budget = StyledBudget::default();
    budget.resample_ns = 0.0;
    assert_eq!(budget.pixels(60.0, true), 0.5e6);
    assert_eq!(budget.pixels(60.0, false), 2.0e6);
    assert_eq!(budget.pixels(1000.0, true), StyledBudget::MIN_PIXELS);
    assert_eq!(budget.pixels(1.0, false), StyledBudget::MAX_PIXELS);
}

// Resampling a texel image through a box keeps texels whole with nearest, fades the source's edges over one texel,
// and leaves empty texels transparent.
#[trial]
fn resampling_handles_empty_texels_and_nearest_neighbor() {
    use craft_behavior::geometry::Homography;
    // A 2 x 1 image, red then blue, shown at 4 times its size on a 10 x 10 work rectangle.
    let image = ColorImage::new([2, 1], vec![Color32::from_rgb(255, 0, 0), Color32::from_rgb(0, 0, 255)]);
    let quad = [pt(1.0, 1.0), pt(9.0, 1.0), pt(9.0, 5.0), pt(1.0, 5.0)];
    let Some(to_unit) = Homography::unit_to_quad(quad).and_then(|h| h.inverse()) else { panic!("invertible") };
    let inside = pt(5.0, 3.0);
    let rect = PixelRect::new(0, 0, 10, 6);
    let at = |c: &StyledContent, x: usize, y: usize| c.texels[y * 10 + x];

    let crisp = resample(&image, [1.0, 1.0], &to_unit, inside, rect, true);
    assert_eq!(at(&crisp, 2, 2), [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(at(&crisp, 7, 2), [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(at(&crisp, 0, 0), [0.0; 4], "outside the box");

    let smooth = resample(&image, [1.0, 1.0], &to_unit, inside, rect, false);
    let edge = at(&smooth, 1, 3);
    assert!(edge[3] > 0.0 && edge[3] < 1.0, "the source's left edge fades over a texel: {edge:?}");
    assert!(at(&smooth, 0, 5)[3] < 0.5, "a texel outside the box fades out");

    let empty = resample(&ColorImage::new([0, 0], vec![]), [1.0, 1.0], &to_unit, inside, rect, false);
    assert!(empty.texels.iter().all(|t| *t == [0.0; 4]));
    assert_eq!(empty.texels.len(), 60);
}

// Below ½ screen pixel per source pixel a mip level (zoom 0.25 → level 2, at most 12); around 1:1 level 0 linear; at
// zoom ≥ 2 with scale ≤ 1.05 level 0 nearest; a scale over 1.05 stays linear.
#[trial]
fn the_texture_level_follows_the_on_screen_size() {
    assert_eq!(PreviewTextures::choose(1.0, 1.0, false), choice(0, false));
    assert_eq!(PreviewTextures::choose(3.0, 1.0, false), choice(0, true));
    assert_eq!(PreviewTextures::choose(3.0, 2.0, false), choice(0, false));
    assert_eq!(PreviewTextures::choose(0.25, 1.0, false), choice(2, false));
    assert_eq!(PreviewTextures::choose(0.3, 1.0, false), choice(1, false));
    assert_eq!(PreviewTextures::choose(1e-9, 1.0, false).level, 12);
    assert_eq!(PreviewTextures::choose(f32::NAN, 1.0, false), choice(0, false));
    assert_eq!(PreviewTextures::choose(1.0, 0.0, false), choice(0, false));
}

// With nearest interpolation every zoom and scale gives level 0 with nearest filtering.
#[trial]
fn nearest_interpolation_keeps_level_zero_and_nearest_filtering() {
    for zoom in [1.0 / 4096.0, 0.25, 1.0, 8.0] {
        for scale in [0.1, 1.0, 4.0] {
            assert_eq!(PreviewTextures::choose(zoom, scale, true), choice(0, true));
        }
    }
}

// A display conversion changes only the displayed upload: `level_image` stays raw, the conversion runs once per key,
// and a new key replaces the old key's upload.
#[trial]
fn the_display_conversion_changes_only_the_displayed_upload() {
    let (_fixture, ctx) = context();
    let raw = solid([2, 2], Color32::from_rgb(100, 50, 20));
    let textures = PreviewTextures::new("display", PreviewPixels::new(raw.clone(), [1.0, 1.0]));
    let converted = Arc::new(AtomicUsize::new(0));
    let display = |key: u64| {
        let converted = Arc::clone(&converted);
        PreviewDisplay::new(key, move |image: &ColorImage| {
            converted.fetch_add(1, Ordering::SeqCst);
            solid(image.size, Color32::from_rgb(1, 2, 3))
        })
    };
    let level0 = choice(0, false);
    let plain = textures.pick(&ctx, level0, None);
    assert!(plain.is_some());
    assert_eq!(converted.load(Ordering::SeqCst), 0, "no display, no conversion");
    textures.pick(&ctx, level0, Some(&display(1)));
    textures.pick(&ctx, level0, Some(&display(1)));
    assert_eq!(converted.load(Ordering::SeqCst), 1, "the same key converts once");
    textures.pick(&ctx, level0, Some(&display(2)));
    assert_eq!(converted.load(Ordering::SeqCst), 2, "a new key converts again");
    assert_eq!(textures.level_image(0).map(|(image, _)| image.pixels.clone()), Some(raw.pixels));
}

// A mask value of 0 at density 1 clears a texel; 128 at density 0.5 keeps three quarters of its alpha and colour.
#[trial]
fn the_mask_scales_the_texel_alpha() {
    let texel = Color32::from_rgba_premultiplied(200, 100, 40, 200);
    let pixels = |density: f32, value: u8| {
        PreviewPixels::new(solid([1, 1], texel), [1.0, 1.0]).masked(&PreviewMask::new(vec![value], density)).image.pixels[0]
    };
    assert_eq!(pixels(1.0, 0), Color32::TRANSPARENT);
    assert_eq!(pixels(1.0, 255), texel);
    let [r, g, b, a] = pixels(0.5, 128).to_array();
    assert_eq!([r, g, b, a], [150, 75, 30, 150]);
}

// Mip levels are built only when asked for, halve down to one texel, and keep the uv extent of the source.
#[trial]
fn mip_levels_are_built_on_demand_and_keep_the_source_extent() {
    let textures = PreviewTextures::new("mips", PreviewPixels::new(solid([5, 3], Color32::from_gray(u8::MAX)), [0.8, 0.6]));
    assert_eq!(textures.size(), [5, 3]);
    assert_eq!(textures.levels_built(), 1);
    let (level1, uv1) = textures.level_image(1).unwrap();
    assert_eq!(level1.size, [3, 2]);
    assert_eq!(textures.levels_built(), 2);
    assert!((uv1[0] - 0.8 * 5.0 / 6.0).abs() < 1e-6 && (uv1[1] - 0.6 * 3.0 / 4.0).abs() < 1e-6);
    let (last, _) = textures.level_image(12).unwrap();
    assert_eq!(last.size, [1, 1]);
    assert_eq!(textures.levels_built(), 4);
}
