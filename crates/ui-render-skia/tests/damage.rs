//! Damage-only frames give the bytes a full redraw gives: two renderers run side by side over the
//! same frames, one drawing only the damage and one drawing every pixel.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use craft_theme::Rgba;
use craft_ui_render_skia::{SkiaOptions, SkiaRenderer, SurfaceProducer, SurfaceSpec};
use craft_ui_rt::{ImageOptions, ReadBack, RectU, Renderer, SurfaceHandle};
use emath::{Rect, pos2};

use super::scene::{IMAGE, SURFACE, SURFACE_SIZE, WINDOW, engine, image_pixels, scene, target_size};

/// Pixels that depend on a phase the test moves.
pub struct Synth(pub Rc<Cell<u32>>);

impl SurfaceProducer for Synth {
    fn fill(&mut self, rect: RectU, out: &mut [u8]) {
        let phase = self.0.get();
        let w = rect.width() as usize;
        for (row, line) in out.chunks_exact_mut(w * 4).enumerate() {
            let y = rect.y0 + row as u32;
            for (i, px) in line.chunks_exact_mut(4).enumerate() {
                let x = rect.x0 + i as u32;
                px.copy_from_slice(&[(x + phase) as u8, (y * 3) as u8, phase as u8, 255]);
            }
        }
    }
}

pub fn pixels(renderer: &mut SkiaRenderer) -> Vec<u8> {
    let id = renderer.request_read_back(WINDOW, None);
    match renderer.poll_read_back(id) {
        ReadBack::Ready(img) => img.pixels,
        other => panic!("read back: {other:?}"),
    }
}

fn prepared(make: &dyn Fn(SkiaOptions) -> SkiaRenderer, full_redraw: bool, scale: f32, phase: &Rc<Cell<u32>>) -> SkiaRenderer {
    let mut r = make(SkiaOptions { clear: Rgba([0, 0, 0, 255]), full_redraw, gpu_timing: false });
    r.set_target(WINDOW, SurfaceHandle::Offscreen, target_size(scale), scale);
    r.register_surface(SURFACE, SurfaceSpec { size: SURFACE_SIZE, options: ImageOptions::default() }, Box::new(Synth(phase.clone())));
    r
}

struct Compared {
    drawn_damage_only: u64,
    drawn_full: u64,
    differing_bytes: usize,
    compared_bytes: usize,
    max_delta: u8,
}

/// Runs [`FRAMES`] frames of the scene and compares the two renderers after each. Returns the pixels the
/// damage-only renderer drew and the pixels the full one drew.
/// Enough frames for every cell, the image and the surface to change at least once.
const FRAMES: usize = 30;

fn compare(make: &dyn Fn(SkiaOptions) -> SkiaRenderer, scale: f32, plain: &'static [usize]) -> Compared {
    let phase = Rc::new(Cell::new(0));
    let mut a = prepared(make, false, scale, &phase);
    let mut b = prepared(make, true, scale, &phase);
    let mut s = scene(scale, plain);
    let mut text = engine();
    let (mut worst, mut bytes_off, mut bytes) = (0u8, 0usize, 0usize);
    let (mut drawn_a, mut drawn_b, mut frames) = (0, 0, 0);
    for i in 0..FRAMES {
        if i > 0 {
            let cell = (i * 7 + i / 3) % s.cells.len();
            s.cells[cell].update(&mut s.runtime, |v| *v += 1);
            if i % 5 == 0 {
                let part = RectU::new(0, 0, 8, 8);
                s.runtime.images_mut().update(IMAGE, part, &image_pixels(8, i as u8).pixels);
            }
            if i % 7 == 0 {
                phase.set(phase.get() + 1);
                let (item, px) = (Rect::from_min_max(pos2(0.0, 0.0), pos2(50.0, 30.0)), RectU::new(0, 0, 54, 33));
                s.runtime.invalidate_surface(SURFACE, Some(item));
                a.invalidate_surface(SURFACE, Some(px));
                b.invalidate_surface(SURFACE, Some(px));
            }
        }
        s.runtime.frame(&mut text);
        if !s.runtime.has_damage(WINDOW) {
            s.runtime.end_frame();
            continue;
        }
        let damage = s.runtime.damage(WINDOW).to_vec();
        a.sync_images(s.runtime.images());
        b.sync_images(s.runtime.images());
        a.render(WINDOW, &s.runtime.frame_tree(WINDOW), &damage);
        b.render(WINDOW, &s.runtime.frame_tree(WINDOW), &damage);
        s.runtime.end_frame();
        let (pa, pb) = (pixels(&mut a), pixels(&mut b));
        for (x, y) in pa.iter().zip(&pb) {
            let d = x.abs_diff(*y);
            bytes_off += usize::from(d != 0);
            worst = worst.max(d);
        }
        bytes += pa.len();
        drawn_a += a.last_frame().pixels;
        drawn_b += b.last_frame().pixels;
        frames += 1;
    }
    assert!(frames > FRAMES * 2 / 3, "frames compared: {frames}");
    Compared { drawn_damage_only: drawn_a, drawn_full: drawn_b, differing_bytes: bytes_off, compared_bytes: bytes, max_delta: worst }
}

/// `without` kinds are left out of the scene so every remaining pixel must match; `tolerated`
/// is what the full scene may differ by: (bytes in a million, largest channel delta).
fn check(make: &dyn Fn(SkiaOptions) -> SkiaRenderer, scales: &[f32], without: Option<&'static [usize]>, tolerated: (usize, u8)) {
    for &scale in scales {
        if let Some(without) = without {
            let c = compare(make, scale, without);
            assert_eq!(
                c.differing_bytes, 0,
                "scale {scale}, without the kinds {without:?}: damage-only differs from the full redraw (max delta {})",
                c.max_delta
            );
            // The first frame is the whole window for both; the later ones are a few cells.
            assert!(c.drawn_damage_only * 3 < c.drawn_full, "scale {scale}: damage-only drew {} pixels, full {}", c.drawn_damage_only, c.drawn_full);
        }
        let c = compare(make, scale, &[]);
        eprintln!("scale {scale}, full scene: {} of {} bytes differ, max delta {}", c.differing_bytes, c.compared_bytes, c.max_delta);
        assert!(c.differing_bytes * 1_000_000 <= tolerated.0 * c.compared_bytes && c.max_delta <= tolerated.1, "full scene differs too much");
    }
}

#[test]
fn cpu_damage_only_frames_equal_a_full_redraw() {
    // Each item kind alone is exact (the next test). Together, cells overlap through shadows and
    // layers, and a handful of edge pixels move by one.
    check(&|options| SkiaRenderer::new(options), &[1.0, 1.5], None, (50, 1));
}

/// The same on the Vulkan backend (lavapipe on a box without a GPU). Skipped where Vulkan cannot
/// start.
#[cfg(feature = "gpu")]
#[test]
fn gpu_damage_only_frames_equal_a_full_redraw() {
    if SkiaRenderer::new_gpu(SkiaOptions::default(), None).is_err() {
        eprintln!("no Vulkan device: skipped");
        return;
    }
    // Ganesh draws glyphs (kind 5) through an atlas whose choice depends on the clip: a few edge pixels move.
    // One fractional scale: the CPU trial covers the others, and each lavapipe frame read back costs.
    check(&|options| SkiaRenderer::new_gpu(options, None).expect("Vulkan device"), &[1.5], Some(&[1, 5]), (100, 64));
}

#[test]
fn the_scene_draws_something_in_every_cell() {
    let phase = Rc::new(Cell::new(0));
    let mut r = prepared(&|o| SkiaRenderer::new(o), false, 1.0, &phase);
    let mut s = scene(1.0, &[]);
    let mut text = engine();
    s.runtime.frame(&mut text);
    let damage = s.runtime.damage(WINDOW).to_vec();
    r.sync_images(s.runtime.images());
    let stats = r.render(WINDOW, &s.runtime.frame_tree(WINDOW), &damage);
    assert!(stats.prepared >= 24, "{stats:?}");
    let px = pixels(&mut r);
    let size = target_size(1.0);
    for cell in 0..24usize {
        let (x0, y0) = ((cell % 6) * 100, (cell / 6) * 60);
        let mut lit = false;
        for y in y0..y0 + 60 {
            for x in x0..x0 + 100 {
                let at = (y * size.width as usize + x) * 4;
                lit |= px[at..at + 3] != [0, 0, 0];
            }
        }
        assert!(lit, "cell {cell} drew nothing");
    }
    let _ = Arc::new(0);
}

/// Every item kind on its own: no overlap between cells, so every pixel matches.
#[test]
fn cpu_damage_only_frames_equal_a_full_redraw_for_each_item_kind() {
    for kind in 0..8usize {
        let others: &'static [usize] = Box::leak((0..8).filter(|k| *k != kind).collect::<Vec<_>>().into_boxed_slice());
        let c = compare(&|o| SkiaRenderer::new(o), 1.5, others);
        assert_eq!(c.differing_bytes, 0, "kind {kind} alone differs from the full redraw (max delta {})", c.max_delta);
    }
}
