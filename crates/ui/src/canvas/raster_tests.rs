//! The CPU raster display.

use std::sync::Mutex;

use egui::{Color32, ColorImage, Context, RawInput, Shape, pos2, vec2};
use kurbo::Point;

use super::raster::texture_factor;
use super::{CanvasView, CheckerColors, CheckerSize, Checkerboard, DetailPlan, PixelRect, RasterDisplay, RasterResponse, RasterSource};

/// A solid document that records every read.
struct Solid {
    size: [u32; 2],
    revision: u64,
    reads: Mutex<Vec<(PixelRect, u32)>>,
}

impl Solid {
    fn new(size: [u32; 2]) -> Self {
        Self { size, revision: 1, reads: Mutex::new(Vec::new()) }
    }
}

impl RasterSource for Solid {
    fn size(&self) -> [u32; 2] {
        self.size
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        self.reads.lock().unwrap().push((region, factor));
        out.pixels.fill(Color32::RED);
    }
}

fn context() -> Context {
    Context::default()
}

fn input() -> RawInput {
    RawInput { max_texture_side: Some(4096), ..RawInput::default() }
}

fn view(zoom: f64) -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0)), Point::new(5000.0, 1500.0), zoom)
}

/// One pass; the texture uploads are dropped as no backend takes them.
fn run(ctx: &Context, ui: impl FnMut(&mut egui::Ui)) -> Vec<egui::epaint::ClippedShape> {
    let mut output = ctx.run_ui(input(), ui);
    output.textures_delta.clear();
    output.shapes
}

fn pass(ctx: &Context, view: &CanvasView, source: &Solid) -> RasterResponse {
    let mut response = None;
    run(ctx, |ui| response = Some(RasterDisplay::new("doc").show(ui, view, source)));
    response.unwrap()
}

// texture_factor of 10000 × 3000 at side 4096 is 3; a fitting document is 1.
#[test]
fn a_document_over_the_texture_limit_is_uploaded_reduced() {
    assert_eq!(texture_factor([10_000, 3000], 4096), 3);
    assert_eq!(texture_factor([4096, 4096], 4096), 1);
    assert_eq!(texture_factor([100, 100], 4096), 1);

    let (ctx, source) = (context(), Solid::new([10_000, 3000]));
    let response = pass(&ctx, &view(0.1), &source);
    assert_eq!(response.read, Some(PixelRect::of_size([10_000, 3000])));
    assert_eq!(*source.reads.lock().unwrap(), vec![(PixelRect::of_size([10_000, 3000]), 3)]);
}

// DetailPlan::of at 100 % over a reduced document is Some, snapped to 256; at 10 % it is None.
#[test]
fn zoomed_in_a_reduced_document_gets_a_detail_texture() {
    let size = [10_000, 3000];
    let visible = PixelRect::new(4100, 1000, 4900, 1600);
    let plan = DetailPlan::of(size, 4096, 1.0, visible).expect("100 % needs a detail texture");
    assert_eq!(plan, DetailPlan { factor: 1, region: PixelRect::new(4096, 768, 5120, 1792) });
    assert_eq!(DetailPlan::of(size, 4096, 0.1, visible), None);
    assert_eq!(DetailPlan::of([3000, 2000], 4096, 1.0, visible), None);

    let (ctx, source) = (context(), Solid::new(size));
    let canvas = view(1.0);
    assert_eq!(pass(&ctx, &canvas, &source).detail, DetailPlan::of(size, 4096, 1.0, PixelRect::visible(&canvas, size)));
}

// After damage(rect), the next show reads only that rect from the source.
#[test]
fn only_damaged_pixels_are_read_again() {
    let (ctx, mut source) = (context(), Solid::new([400, 300]));
    let view = view(1.0);
    let first = pass(&ctx, &view, &source);
    assert_eq!(pass(&ctx, &view, &source).read, None, "nothing changed");

    source.reads.lock().unwrap().clear();
    source.revision = 2;
    let rect = PixelRect::new(10, 20, 50, 60);
    RasterDisplay::damage(&ctx, first.id, rect);
    assert_eq!(pass(&ctx, &view, &source).read, Some(rect));
    assert_eq!(*source.reads.lock().unwrap(), vec![(rect, 1)]);

    // A new revision with no damage reads everything.
    source.revision = 3;
    assert_eq!(pass(&ctx, &view, &source).read, Some(PixelRect::of_size([400, 300])));
    RasterDisplay::forget(&ctx, first.id);
    assert_eq!(pass(&ctx, &view, &source).read, Some(PixelRect::of_size([400, 300])));
}

// Squares stay aligned to the canvas's top-left when the view centre moves.
#[test]
fn the_checkerboard_holds_still_while_the_document_scrolls() {
    let ctx = context();
    let square = 8.0;
    for center in [Point::new(100.0, 100.0), Point::new(137.0, 91.5)] {
        let view = CanvasView { center, ..view(2.0) };
        let shapes = run(&ctx, |ui| {
            let checker = Checkerboard { size: CheckerSize::Medium, colors: CheckerColors::Light };
            checker.paint(ui.painter(), &view, [200, 200]);
        });
        let meshes: Vec<_> = shapes.iter().filter_map(|s| if let Shape::Mesh(m) = &s.shape { Some(m.clone()) } else { None }).collect();
        assert_eq!(meshes.len(), 1);
        for v in &meshes[0].vertices {
            let expected = pos2((v.pos.x - view.rect.min.x) / (2.0 * square), (v.pos.y - view.rect.min.y) / (2.0 * square));
            assert!((v.uv - expected).length() < 1e-4, "{:?} has uv {:?}, expected {expected:?}", v.pos, v.uv);
        }
    }
}
