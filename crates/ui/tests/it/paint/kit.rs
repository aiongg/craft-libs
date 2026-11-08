//! The content-paint kit's dashed line and marching ants.

use craft_behavior::Axis;
use craft_theme::{Contrast, Mode};
use craft_ui::primitives::{dashed, marching_ants, ramp};
use craft_ui::style::DataColor;
use craft_ui::testing::style::tokens;
use craft_ui::testing::trial;
use egui::{Color32, Pos2, Rect, Shape, pos2};

const EPS: f32 = 1e-3;

/// How far along the polyline `path` the point `p` lies (it lies on the path).
fn along(path: &[Pos2], p: Pos2) -> f32 {
    let mut walked = 0.0;
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let on = (a.distance(p) + p.distance(b) - a.distance(b)).abs() < EPS;
        if on {
            return walked + a.distance(p);
        }
        walked += a.distance(b);
    }
    walked
}

fn dashes(shapes: &[Shape]) -> Vec<Vec<Pos2>> {
    shapes
        .iter()
        .map(|shape| match shape {
            Shape::LineSegment { points, .. } => points.to_vec(),
            Shape::Path(path) => path.points.clone(),
            other => panic!("a dash is a line, not {other:?}"),
        })
        .collect()
}

#[trial]
fn a_dashed_line_keeps_its_gap_and_continues_its_pattern_round_a_corner() {
    let path = [pos2(0.0, 0.0), pos2(22.0, 0.0), pos2(22.0, 40.0)];
    let (dash, gap, phase) = (6.0, 4.0, 1.0);
    let found = dashes(&dashed(&path, 1.0, dash, gap, phase, Color32::from_gray(200)));
    assert!(found.len() >= 4, "the line is cut into dashes: {}", found.len());
    // A dash that runs round a corner comes as one shape per straight piece.
    let mut spans: Vec<(f32, f32)> = Vec::new();
    for points in &found {
        let (from, to) = (along(&path, points[0]), along(&path, *points.last().unwrap()));
        match spans.last_mut() {
            Some(last) if (last.1 - from).abs() < 0.01 => last.1 = to,
            _ => spans.push((from, to)),
        }
    }
    assert!((spans[1].0 - (phase + dash + gap)).abs() < 0.01, "the phase shifts the pattern: {spans:?}");
    for pair in spans.windows(2) {
        assert!((pair[1].0 - pair[0].1 - gap).abs() < 0.01, "a gap of {gap} between {pair:?}");
    }
    for (from, to) in &spans[1..spans.len() - 1] {
        assert!((to - from - dash).abs() < 0.01, "a whole dash is {dash} long: {from} {to}");
    }
    assert!(spans.iter().any(|(from, to)| *from < 22.0 && *to > 22.0), "a dash runs round the corner: {spans:?}");
}

/// The mesh quads of the dark dashes and of the light band (the colour the mesh starts with), in device pixels:
/// `[x0, y0, x1, y1]`.
fn quads(shapes: &[Shape], ppp: f32) -> (Vec<[f32; 4]>, Vec<[f32; 4]>) {
    let (mut dark_quads, mut light_quads) = (Vec::new(), Vec::new());
    for shape in shapes {
        let Shape::Mesh(mesh) = shape else { continue };
        let light = mesh.vertices[0].color;
        for triangles in mesh.indices.chunks(6) {
            let vertices: Vec<_> = triangles.iter().map(|i| mesh.vertices[*i as usize]).collect();
            let xs = vertices.iter().map(|v| v.pos.x * ppp);
            let ys = vertices.iter().map(|v| v.pos.y * ppp);
            let quad =
                [xs.clone().fold(f32::MAX, f32::min), ys.clone().fold(f32::MAX, f32::min), xs.fold(f32::MIN, f32::max), ys.fold(f32::MIN, f32::max)];
            if vertices[0].color != light { dark_quads.push(quad) } else { light_quads.push(quad) }
        }
    }
    (dark_quads, light_quads)
}

#[trial]
fn marching_ants_sit_on_device_pixels_march_with_the_phase_and_keep_a_whole_pixel_width() {
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let clip = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 400.0));
    let edge = [pos2(10.3, 20.4), pos2(150.7, 20.4)];
    for (ppp, band) in [(1.0, 1.0), (1.5, 2.0), (2.0, 2.0)] {
        let at = |phase| quads(&marching_ants(&tokens, &edge, false, phase, clip, ppp), ppp);
        let (dark_quads, light_quads) = at(0.0);
        assert!(!dark_quads.is_empty(), "dashes at {ppp}");
        let on_grid = |quad: &[f32; 4]| quad.iter().all(|v| (v - v.round()).abs() < EPS);
        assert!(dark_quads.iter().chain(&light_quads).all(on_grid), "every quad sits on device pixels at {ppp}");
        assert!(light_quads.iter().all(|quad| (quad[3] - quad[1] - band).abs() < EPS), "a band is {band} device pixels thick at {ppp}");
        let starts = |quads: &[[f32; 4]]| quads.iter().map(|quad| quad[0] as i64).collect::<Vec<_>>();
        let (marched, _) = at(1.0);
        assert_ne!(starts(&dark_quads), starts(&marched), "the dashes move with the phase at {ppp}");
        let (again, _) = at(0.0);
        assert_eq!(dark_quads, again, "the same phase gives the same dashes at {ppp}");
    }
}

fn ramp_vertices(shapes: &[Shape]) -> Vec<(Pos2, Color32)> {
    match shapes {
        [Shape::Mesh(mesh)] => mesh.vertices.iter().map(|vertex| (vertex.pos, vertex.color)).collect(),
        other => panic!("a ramp is one mesh, not {other:?}"),
    }
}

#[trial]
fn a_ramp_holds_its_end_colours_out_to_the_edges_and_clamps_stops() {
    let (red, blue) = (Color32::RED, Color32::BLUE);
    let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 10.0));
    let stops = [(-1.0, DataColor::new(red)), (0.25, DataColor::new(red)), (0.75, DataColor::new(blue))];
    let found = ramp_vertices(&ramp(rect, &stops, Axis::X));
    let top: Vec<(f32, Color32)> = found.iter().step_by(2).map(|(pos, color)| (pos.x, *color)).collect();
    assert_eq!(top, [(0.0, red), (25.0, red), (75.0, blue), (100.0, blue)]);
    assert!(found.chunks(2).all(|pair| pair[0].0.y == 0.0 && pair[1].0.y == 10.0), "each stop spans the strip across the axis");
    let down = ramp_vertices(&ramp(rect, &stops[1..2], Axis::Y));
    assert!(down.iter().all(|(pos, _)| pos.y == 0.0 || pos.y == 10.0), "the vertical ramp runs top to bottom: {down:?}");
    assert!(ramp(rect, &[], Axis::X).is_empty());
}
