//! App surfaces as CPU tiles, and the R5 measurement: a 2560x1440 region updated every frame.

use std::cell::Cell;
use std::rc::Rc;

use craft_theme::Rgba;
use craft_ui_render_skia::{SkiaOptions, SkiaRenderer, SurfaceProducer, SurfaceSpec, to_xrgb};
use craft_ui_rt::{
    BuildCx, Constraints, Element, ImageOptions, LayoutCx, PaintCx, ReadBack, RectU, Renderer, Runtime, SizeU, SurfaceHandle, SurfaceKey, Vec2,
    WindowId, vec2,
};
use emath::Rect;

use super::scene::{engine, tokens};

const KEY: SurfaceKey = SurfaceKey(3);
const WINDOW: WindowId = WindowId(5);

struct Canvas;

impl Element for Canvas {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        c.max
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        cx.surface(KEY, bounds);
    }
}

/// A moving gradient: each row copies a slice of one precomputed line, so the producer costs a
/// memcpy per row and the measurement is the renderer's.
struct Moving {
    phase: Rc<Cell<u32>>,
    line: Vec<u8>,
}

impl Moving {
    fn new(phase: Rc<Cell<u32>>) -> Moving {
        let line = (0..8192u32).flat_map(|x| [(x * 255 / 8192) as u8, (x % 256) as u8, 90, 255]).collect();
        Moving { phase, line }
    }
}

impl SurfaceProducer for Moving {
    fn fill(&mut self, rect: RectU, out: &mut [u8]) {
        let w = rect.width() as usize;
        for (row, dst) in out.chunks_exact_mut(w * 4).enumerate() {
            let at = ((rect.x0 + self.phase.get() * 8 + row as u32 * 3) as usize % 4096) * 4;
            dst.copy_from_slice(&self.line[at..at + w * 4]);
        }
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

#[test]
fn a_surface_region_updates_every_frame_and_shows_the_new_pixels() {
    surface_frames(SizeU::new(320, 180), 3);
}

/// R5: the same at 2560 x 1440 for 24 frames, with the medians printed. A measurement, so it runs
/// only when asked, in a release build: `cargo test --release -p craft-ui-render-skia -- --ignored r5`.
#[test]
#[ignore]
fn r5_a_full_screen_surface_region_per_frame() {
    surface_frames(SizeU::new(2560, 1440), 24);
}

fn surface_frames(size: SizeU, frames: usize) {
    let phase = Rc::new(Cell::new(0));
    let mut renderer = SkiaRenderer::new(SkiaOptions { clear: Rgba([0, 0, 0, 255]), ..SkiaOptions::default() });
    renderer.set_target(WINDOW, SurfaceHandle::Offscreen, size, 1.0);
    renderer.register_surface(KEY, SurfaceSpec { size, options: ImageOptions::default() }, Box::new(Moving::new(phase.clone())));
    let mut rt = Runtime::new(tokens());
    let (w, h) = (size.width, size.height);
    rt.add_window(WINDOW, vec2(w as f32, h as f32), 1.0);
    rt.set_root(WINDOW, Canvas);
    rt.invalidate_window(WINDOW);
    let mut text = engine();

    let region = RectU::of_size(size);
    let mut samples: Vec<[f64; 6]> = Vec::new();
    let mut dst = vec![0u32; (w * h) as usize];
    for frame in 0..frames {
        if frame > 0 {
            phase.set(phase.get() + 1);
            rt.invalidate_surface(KEY, Some(Rect::from_min_size(emath::pos2(0.0, 0.0), vec2(w as f32, h as f32))));
            renderer.invalidate_surface(KEY, Some(region));
        }
        rt.frame(&mut text);
        let damage = rt.damage(WINDOW).to_vec();
        let t = web_time::Instant::now();
        renderer.sync_images(rt.images());
        let sync = t.elapsed().as_secs_f64() * 1000.0;
        let stats = renderer.render(WINDOW, &rt.frame_tree(WINDOW), &damage);
        rt.end_frame();
        let (ingest, drawn) = (renderer.last_ingest(), renderer.last_frame());
        assert_eq!(drawn.pixels, u64::from(w * h), "the whole canvas is damaged");
        // What a window frame adds after the raster: the canvas words converted for softbuffer.
        let id = renderer.request_read_back(WINDOW, None);
        let ReadBack::Ready(img) = renderer.poll_read_back(id) else { panic!("read back") };
        let t = web_time::Instant::now();
        to_xrgb(&img.pixels, w as usize * 4, true, &mut dst, w as usize, region);
        let convert = t.elapsed().as_secs_f64() * 1000.0;
        if frame >= frames / 6 {
            samples.push([sync, ingest.fill_ms, ingest.write_ms, stats.prepare_ms, drawn.raster_ms, convert]);
        }
        // The first pixel of row 0 is the producer's pixel for this phase.
        let expect = phase.get() * 8 % 4096;
        let want_r = (expect * 255 / 8192) as u8;
        assert_eq!(img.pixels[0], want_r, "frame {frame}: the frame shows the producer's current pixels");
    }
    let col = |i: usize| median(samples.iter().map(|s| s[i]).collect());
    let total = col(0) + col(3) + col(4) + col(5);
    eprintln!(
        "{w}x{h} region per frame, CPU raster (medians, ms): tile ingest (fill {:.2} + write {:.2}) = sync {:.2}, record {:.2}, raster {:.2}, window words {:.2}; sync + record + raster + words = {:.2}",
        col(1),
        col(2),
        col(0),
        col(3),
        col(4),
        col(5),
        total
    );
}

#[test]
fn only_the_changed_tiles_are_refilled() {
    struct Counting(Rc<Cell<u64>>);
    impl SurfaceProducer for Counting {
        fn fill(&mut self, rect: RectU, out: &mut [u8]) {
            self.0.set(self.0.get() + u64::from(rect.width()) * u64::from(rect.height()));
            out.fill(255);
        }
    }
    let size = SizeU::new(2048, 1024);
    let filled = Rc::new(Cell::new(0));
    let mut renderer = SkiaRenderer::new(SkiaOptions::default());
    renderer.set_target(WINDOW, SurfaceHandle::Offscreen, SizeU::new(256, 128), 1.0);
    renderer.register_surface(KEY, SurfaceSpec { size, options: ImageOptions::default() }, Box::new(Counting(filled.clone())));
    let store = craft_ui_rt::ImageStore::default();
    renderer.sync_images(&store);
    assert_eq!(filled.get(), 2048 * 1024, "the first ingest fills every tile");
    filled.set(0);
    renderer.invalidate_surface(KEY, Some(RectU::new(600, 100, 700, 140)));
    renderer.sync_images(&store);
    assert_eq!(filled.get(), 100 * 40, "only the changed part is asked for");
    filled.set(0);
    renderer.sync_images(&store);
    assert_eq!(filled.get(), 0, "nothing changed, nothing filled");
    let _ = SizeU::new(1, 1);
}
