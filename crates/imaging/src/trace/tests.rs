//! Tests of the outline tracer (task SE-F5). Each uses a small `MaskSource` over a `Vec<f32>`
//! written in the test.

use super::*;

/// A mask over a row-major `Vec<f32>` at the origin.
struct Plane {
    size: [u32; 2],
    data: Vec<f32>,
}

impl Plane {
    fn blank(w: u32, h: u32) -> Self {
        Self { size: [w, h], data: vec![0.0; (w * h) as usize] }
    }

    fn fill(&mut self, x0: u32, y0: u32, x1: u32, y1: u32) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.data[(y * self.size[0] + x) as usize] = 1.0;
            }
        }
    }
}

impl MaskSource for Plane {
    fn size(&self) -> [u32; 2] {
        self.size
    }

    fn revision(&self) -> u64 {
        0
    }

    fn content_bounds(&self) -> MaskRect {
        MaskRect::of_size(self.size)
    }

    fn read_row(&self, y: i32, x0: i32, step: u32, out: &mut [f32]) {
        for (i, v) in out.iter_mut().enumerate() {
            let x = x0 + i32::try_from(i).unwrap() * i32::try_from(step).unwrap();
            let inside = x >= 0 && y >= 0 && (x as u32) < self.size[0] && (y as u32) < self.size[1];
            *v = if inside { self.data[y as usize * self.size[0] as usize + x as usize] } else { 0.0 };
        }
    }
}

// Segments of a known 4 × 3 block mask: two merged horizontal runs, two merged vertical runs, one
// padding cell found at a mask touching `bounds`.
#[test]
fn trace_merges_runs_and_pads_one_cell() {
    let mut mask = Plane::blank(4, 3);
    mask.fill(0, 0, 4, 3);
    let mut segments = outline(&mask, MaskRect::of_size([4, 3]), 1);
    segments.sort_unstable();
    let mut expected = vec![([0, 0], [4, 0]), ([0, 3], [4, 3]), ([0, 0], [0, 3]), ([4, 0], [4, 3])];
    expected.sort_unstable();
    assert_eq!(segments, expected);
    assert!(outline(&mask, MaskRect::default(), 1).is_empty());
}

// A one-pixel-wide line traced at step 8 still gives segments (the coarse-cell edge sample), and
// every segment end stays inside `bounds`.
#[test]
fn trace_coarse_cells_keep_one_pixel_selections() {
    let mut mask = Plane::blank(64, 64);
    mask.fill(1, 0, 2, 64);
    let bounds = MaskRect::new(1, 0, 2, 64);
    let segments = outline(&mask, bounds, 8);
    assert!(!segments.is_empty());
    for (a, b) in segments {
        for p in [a, b] {
            assert!(p[0] >= bounds.x0 && p[0] <= bounds.x1 && p[1] >= bounds.y0 && p[1] <= bounds.y1);
        }
    }
}

// `bounded_step`: 200 × 150 keeps step 1, 6000 × 6000 gives 8, and huge or negative bounds give a
// grid of at most `TRACE_BUDGET` cells (PhotoCraft's `whole_transform_trace_has_a_bounded_grid`).
#[test]
fn trace_bounded_step_caps_cells() {
    assert_eq!(bounded_step(MaskRect::new(0, 0, 200, 150), 1), 1);
    assert_eq!(bounded_step(MaskRect::new(0, 0, 6000, 6000), 1), 8);
    let huge = MaskRect::new(-(1 << 30), -(1 << 30), 1 << 30, 1 << 30);
    let step = i64::from(bounded_step(huge, 1));
    let axis = ((1_i64 << 30) + step - 1).div_euclid(step) - (-(1_i64 << 30)).div_euclid(step) + 2;
    assert!(axis * axis <= TRACE_BUDGET as i64);
}

// `edges_of_grid` gives PhotoCraft's Content-Aware Fill unit edges for an L-shaped grid, and
// `None` past the limit.
#[test]
fn trace_grid_edges_match_unit_edges() {
    // X.
    // XX
    let grid = CoverageGrid::new([10, 20], [2, 2], vec![true, false, true, true]);
    let edges = edges_of_grid(&grid, GRID_EDGE_LIMIT).unwrap();
    assert_eq!(edges.len(), 8);
    assert_eq!(edges[..3], [([10, 20], [10, 21]), ([11, 20], [11, 21]), ([10, 20], [11, 20])]);
    assert!(edges.contains(&([12, 21], [12, 22])));
    assert!(edges_of_grid(&grid, 7).is_none());
    assert_eq!(grid.size(), [12, 22]);
    assert_eq!(grid.at(10, 21), 1.0);
    assert_eq!(grid.at(11, 20), 0.0);
}

// `trace_step` at 100 %, 50 %, 12 % and 1 % (with a reduction of 2), and `quantised_region` keeps
// the region for a pan of less than `256 · step` and changes it past that.
#[test]
fn trace_step_and_region_follow_zoom_and_pan() {
    assert_eq!(trace_step(1.0, 1), 1);
    assert_eq!(trace_step(0.5, 1), 2);
    assert_eq!(trace_step(0.12, 1), 8);
    assert_eq!(trace_step(0.01, 2), 32);
    assert_eq!(trace_step(0.001, 1), 64);
    assert_eq!(trace_step(4.0, 1), 1);
    let region = |x: f64| quantised_region([x, 0.0, x + 800.0, 600.0], 2, 1);
    assert_eq!(region(10.0), region(100.0));
    assert_ne!(region(10.0), region(600.0));
    assert_eq!(region(10.0).x0 % 512, 0);
    let huge = quantised_region([f64::MAX, f64::MIN, f64::MAX, f64::MAX], 64, 1);
    assert!(huge.x1 > 0 && huge.y0 < 0);
}
