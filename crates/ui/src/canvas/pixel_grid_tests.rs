//! The pixel grid.

use egui::{Color32, ColorImage, Context, RawInput, pos2, vec2};
use kurbo::Point;

use super::{CanvasId, CanvasView, PixelGrid, PixelRect, RasterSource};

// shows_at(5.0) is false, shows_at(5.01) true.
#[test]
fn the_grid_shows_only_above_five_hundred_percent() {
    assert!(!PixelGrid::shows_at(5.0));
    assert!(!PixelGrid::shows_at(1.0));
    assert!(PixelGrid::shows_at(5.01));
    assert!(PixelGrid::shows_at(128.0));
}

// cell_color of alpha 0 or NaN is None; opaque dark gives a light line, opaque light a dark one.
#[test]
fn no_line_over_transparent_pixels() {
    assert_eq!(PixelGrid::cell_color([0.0; 4]), None);
    assert_eq!(PixelGrid::cell_color([1.0, 1.0, 1.0, 0.0]), None);
    assert_eq!(PixelGrid::cell_color([0.5, 0.5, 0.5, f32::NAN]), None);
    assert_eq!(PixelGrid::cell_color([0.0, 0.0, 0.0, 0.001]), None);
    assert_eq!(PixelGrid::cell_color([0.1, 0.1, 0.1, 1.0]), Some(Color32::from_white_alpha(64)));
    assert_eq!(PixelGrid::cell_color([0.9, 0.9, 0.9, 1.0]), Some(Color32::from_black_alpha(41)));
    assert_eq!(PixelGrid::light_share(0.2), 0.0);
    assert_eq!(PixelGrid::light_share(0.9), 1.0);
    assert_eq!(PixelGrid::light_share(f32::NAN), 0.0);
}

// A uniform 4 × 4 region gives one segment per grid line.
#[test]
fn runs_of_one_colour_merge_into_one_segment() {
    let px = [[0.1, 0.1, 0.1, 1.0]; 16];
    let segments = PixelGrid::segments(&px, PixelRect::new(10, 20, 14, 24));
    // Four vertical lines and four horizontal ones.
    assert_eq!(segments.len(), 8);
    assert!(segments.iter().any(|s| s.from == [10, 20] && s.to == [10, 24]));
    assert!(segments.iter().any(|s| s.from == [10, 23] && s.to == [14, 23]));
    // A transparent cell splits the run.
    let mut holey = px;
    holey[5] = [0.0; 4];
    assert!(PixelGrid::segments(&holey, PixelRect::new(0, 0, 4, 4)).len() > 8);
}

/// A 4 × 4 dark document that records its reads.
struct Dark(std::sync::atomic::AtomicUsize);

impl RasterSource for Dark {
    fn size(&self) -> [u32; 2] {
        [4, 4]
    }

    fn revision(&self) -> u64 {
        1
    }

    fn read(&self, _: PixelRect, factor: u32, out: &mut ColorImage) {
        assert_eq!(factor, 1);
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        out.pixels.fill(Color32::from_gray(25));
    }
}

fn grid_pass(ctx: &Context, zoom: f64, source: &Dark) -> usize {
    let view = CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 200.0)), Point::new(2.0, 2.0), zoom);
    let output = ctx.run_ui(RawInput::default(), |ui| {
        PixelGrid::show(ui.ctx(), CanvasId::new(ui, "grid"), ui.painter(), &view, source);
    });
    let lines = output.shapes.len();
    output.drop_without_applying_deltas();
    lines
}

// Above 500 % the grid draws lines over opaque pixels, reading the source once while its revision
// holds; at 500 % or less it draws and reads nothing.
#[test]
fn the_grid_draws_at_high_zoom_and_reads_the_source_once() {
    let (ctx, source) = (Context::default(), Dark(Default::default()));
    assert_eq!(grid_pass(&ctx, 5.0, &source), 0);
    assert_eq!(source.0.load(std::sync::atomic::Ordering::Relaxed), 0, "no read below the threshold");
    assert!(grid_pass(&ctx, 20.0, &source) > 0, "lines over the pixels");
    assert_eq!(source.0.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert!(grid_pass(&ctx, 20.0, &source) > 0);
    assert_eq!(source.0.load(std::sync::atomic::Ordering::Relaxed), 1, "the cached segments are reused");
}
