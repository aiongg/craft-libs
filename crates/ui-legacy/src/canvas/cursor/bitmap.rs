//! The bitmap pointers' shapes and their rasteriser: signed-distance coverage per pixel, so the
//! outline stays one hardware pixel crisp at every scale.

use std::sync::Arc;

use egui::{Color32, CustomCursorImage, Vec2};

use super::{hand, zoom};

/// The largest cursor bitmap (physical pixels; points on macOS); a larger tip paints its outline
/// instead. Windows takes winit's limit, and so does macOS: AppKit sets no size limit on an
/// `NSCursor` image. On Linux, X servers and hardware cursor planes commonly cap cursors at 256 px
/// and fall back to slow software cursors (or reject them) past that. Either way it keeps
/// allocations bounded for hostile brush sizes or zooms.
pub(super) const MAX_SIDE: u16 = if cfg!(any(target_os = "windows", target_os = "macos")) { 2048 } else { 255 };

/// What a bitmap cursor shows. `Outline` is a brush tip outline: closed rings of logical-point
/// offsets from the hotspot.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Shape {
    Circle { radius: f32, centre: bool },
    Crosshair { length: f32, gap: f32 },
    Hand { closed: bool },
    Zoom { out: bool },
    Outline { rings: Arc<[Vec<Vec2>]>, centre: bool },
}

/// How the line shapes are inked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Look {
    pub line: Color32,
    pub halo: Color32,
    /// Line width in points.
    pub stroke: f32,
}

/// Distance to the nearest arm (round caps), in physical pixels from the hotspot.
pub(super) fn cross_distance(x: f32, y: f32, length: f32, gap: f32) -> f32 {
    let horizontal = (x - x.clamp(gap, length)).hypot(y);
    let vertical = x.hypot(y - y.clamp(gap, length));
    horizontal.min(vertical)
}

pub(super) fn extent(shape: &Shape) -> Option<f32> {
    Some(match shape {
        Shape::Circle { radius, centre } => {
            if !radius.is_finite() || *radius <= 0.0 {
                return None;
            }
            if *centre { radius.max(3.0) } else { *radius }
        }
        Shape::Crosshair { length, gap } => {
            if !length.is_finite() || !gap.is_finite() || *gap < 0.0 || length < gap {
                return None;
            }
            *length
        }
        Shape::Outline { rings, centre } => {
            let mut e = 0.0f32;
            for ring in rings.iter() {
                for p in ring {
                    if !p.x.is_finite() || !p.y.is_finite() {
                        return None;
                    }
                    e = e.max(p.x.hypot(p.y));
                }
            }
            if e <= 0.0 {
                return None;
            }
            if *centre { e.max(3.0) } else { e }
        }
        Shape::Hand { .. } => hand::HALF,
        Shape::Zoom { .. } => zoom::HALF,
    })
}

/// Distance from (`x`, `y`) to the segment `a`-`b`, all in the same units.
fn seg_distance(x: f32, y: f32, a: Vec2, b: Vec2) -> f32 {
    let (abx, aby) = (b.x - a.x, b.y - a.y);
    let t = (((x - a.x) * abx + (y - a.y) * aby) / (abx * abx + aby * aby).max(1e-12)).clamp(0.0, 1.0);
    ((x - a.x - abx * t).powi(2) + (y - a.y - aby * t).powi(2)).sqrt()
}

/// How much of the pixel at `(dx, dy)` physical pixels from the hotspot the light fill covers and
/// how much the dark outline around it does, each 0 to 1. A dark crease inside the hand takes
/// from the fill, so the fill never covers it.
fn coverage(shape: &Shape, dx: f32, dy: f32, scale: f32, width: f32) -> (f32, f32) {
    let (x, y) = (dx.abs(), dy.abs());
    match shape {
        Shape::Circle { radius, centre } => {
            let ring = (x.hypot(y) - radius * scale).abs();
            let distance = if *centre { ring.min(cross_distance(x, y, 3.0 * scale, 0.0)) } else { ring };
            stroke_coverage(distance, scale, width)
        }
        Shape::Crosshair { length, gap } => stroke_coverage(cross_distance(x, y, length * scale, gap * scale), scale, width),
        Shape::Outline { rings, centre } => {
            let (lx, ly) = (dx / scale, dy / scale);
            let mut distance = f32::INFINITY;
            for ring in rings.iter() {
                let n = ring.len();
                if n < 2 {
                    continue;
                }
                for (i, &a) in ring.iter().enumerate() {
                    distance = distance.min(seg_distance(lx, ly, a, ring[(i + 1) % n]));
                }
            }
            if *centre {
                distance = distance.min(cross_distance(dx, dy, 3.0 * scale, 0.0));
            }
            stroke_coverage(distance * scale, scale, width)
        }
        Shape::Hand { closed } => {
            let (lx, ly) = (dx / scale, dy / scale);
            silhouette(hand::shape(*closed, lx, ly), hand::crease(*closed, lx, ly), 0.45, scale)
        }
        Shape::Zoom { out } => {
            let (lx, ly) = (dx / scale, dy / scale);
            silhouette(zoom::shape(lx, ly), zoom::glyph(*out, lx, ly), 0.75, scale)
        }
    }
}

/// A light filled shape with a 1 px dark outline and dark lines `half_width` logical pixels either
/// side of its creases, from their signed distances in logical pixels.
fn silhouette(edge: f32, crease: f32, half_width: f32, scale: f32) -> (f32, f32) {
    let edge = edge * scale;
    let inside = (0.5 - edge).clamp(0.0, 1.0);
    let outline = (scale + 0.5 - edge).clamp(0.0, 1.0);
    let crease = (half_width * scale + 0.5 - crease * scale).clamp(0.0, 1.0);
    (inside * (1.0 - crease), outline.max(inside * crease))
}

/// A light line `width` points wide with a 1 px dark one on each side, `distance` physical pixels from its centre.
fn stroke_coverage(distance: f32, scale: f32, width: f32) -> (f32, f32) {
    let half = 0.5 * width * scale;
    ((half + 0.5 - distance).clamp(0.0, 1.0), (half + scale + 0.5 - distance).clamp(0.0, 1.0))
}

/// Splat the distance to one ring segment into an outline's side × side distance buffer.
fn splat_segment(dist: &mut [f32], side: usize, hot: f32, a: Vec2, b: Vec2, reach: f32) {
    let n = side as f32;
    let (x0, x1) = (a.x.min(b.x) + hot - reach, a.x.max(b.x) + hot + reach);
    let (y0, y1) = (a.y.min(b.y) + hot - reach, a.y.max(b.y) + hot + reach);
    if x1 < 0.0 || y1 < 0.0 || x0 > n - 1.0 || y0 > n - 1.0 {
        return;
    }
    let (c0, c1) = (x0.max(0.0).floor() as i32, x1.min(n - 1.0).ceil() as i32);
    let (r0, r1) = (y0.max(0.0).floor() as i32, y1.min(n - 1.0).ceil() as i32);
    for row in r0..=r1 {
        let dy = row as f32 - hot;
        for col in c0..=c1 {
            let d = seg_distance(col as f32 - hot, dy, a, b);
            let i = row as usize * side + col as usize;
            if d < dist[i] {
                dist[i] = d;
            }
        }
    }
}

/// Fold the centre crosshair (the circle cursor's) into the outline distances.
fn splat_cross(dist: &mut [f32], side: usize, hot: f32, arm: f32) {
    let reach = arm + 2.0;
    let lo = (hot - reach).max(0.0).floor() as i32;
    let hi = ((hot + reach).min(side as f32 - 1.0)).ceil() as i32;
    for row in lo..=hi {
        let dy = row as f32 - hot;
        for col in lo..=hi {
            let dx = col as f32 - hot;
            let d = cross_distance(dx, dy, arm, 0.0);
            let i = row as usize * side + col as usize;
            if d < dist[i] {
                dist[i] = d;
            }
        }
    }
}

/// One bitmap pixel from its distance to the stroke centre, in straight RGBA (winit form).
/// A pixel with light `fill` over a dark `outline` (coverages 0 to 1), in straight RGBA as winit
/// requires (not egui's premultiplied). No coverage is `[0; 4]`.
fn stroke_pixel((fill, outline): (f32, f32), dark: egui::Color32, light: egui::Color32) -> [u8; 4] {
    let a_dark = outline * f32::from(dark.a()) / 255.0;
    let a_light = fill * f32::from(light.a()) / 255.0;
    let alpha = a_light + a_dark * (1.0 - a_light);
    let white = if alpha > 0.0 { (255.0 * a_light / alpha).round() as u8 } else { 0 };
    [white, white, white, (255.0 * alpha).round() as u8]
}

/// The columns of row offset `dy` (from the hotspot, in pixels) that can be painted by a ring of
/// radius `r` drawn `reach` wide either side: the left and right arcs of the annulus, padded by
/// a pixel. Every other pixel of the row is transparent.
fn ring_spans(dy: f32, r: f32, reach: f32) -> Option<[(f32, f32); 2]> {
    let (ro, ri) = (r + reach + 1.0, (r - reach - 1.0).max(0.0));
    if dy.abs() > ro {
        return None;
    }
    let xo = (ro * ro - dy * dy).max(0.0).sqrt();
    let xi = if dy.abs() < ri { (ri * ri - dy * dy).sqrt() } else { 0.0 };
    Some([(-xo, -xi), (xi, xo)])
}

pub(super) fn rasterize(shape: &Shape, scale: f32, look: &Look) -> Option<CustomCursorImage> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let extent = extent(shape)?;
    // Odd dimensions put the hotspot on the centre pixel at every DPI. Include the outline
    // and its antialiasing fringe, then check before converting or allocating.
    let half = ((extent + 0.5 * look.stroke + 1.0) * scale + 1.0).ceil();
    if !half.is_finite() || half > f32::from((MAX_SIDE - 1) / 2) {
        return None;
    }
    let hot = half as u16;
    let side = hot * 2 + 1;
    let bytes = usize::from(side).checked_mul(usize::from(side))?.checked_mul(4)?;
    // Transparent to start; only pixels a shape can reach are evaluated below (a big tip's
    // outline is a thin ring in a large square,).
    let mut rgba = vec![0u8; bytes];
    let (dark, light) = (look.halo, look.line);
    let n = usize::from(side);
    let hotf = f32::from(hot);
    let mut put = |col: usize, row: usize, cov: (f32, f32)| {
        if let Some(px) = row.checked_mul(n).and_then(|i| i.checked_add(col)).and_then(|i| rgba.get_mut(i * 4..i * 4 + 4)) {
            px.copy_from_slice(&stroke_pixel(cov, dark, light));
        }
    };
    if let Shape::Outline { rings, centre } = shape {
        // Splat each segment into its bounding box instead of measuring every ring per pixel:
        // tip outlines may carry hundreds of points.
        let reach = (0.5 * look.stroke + 1.0) * scale + 2.0;
        let mut dist = vec![f32::INFINITY; n * n];
        for ring in rings.iter() {
            let len = ring.len();
            if len < 2 {
                continue;
            }
            for (i, &a) in ring.iter().enumerate() {
                splat_segment(&mut dist, n, hotf, a * scale, ring[(i + 1) % len] * scale, reach);
            }
        }
        if *centre {
            splat_cross(&mut dist, n, hotf, 3.0 * scale);
        }
        for (i, &d) in dist.iter().enumerate() {
            if d.is_finite() {
                put(i % n, i / n, stroke_coverage(d, scale, look.stroke));
            }
        }
        return Some(CustomCursorImage { rgba: Arc::from(rgba), size: [side, side], hotspot: [hot, hot] });
    }
    // Columns to evaluate in each row: the ring's two arcs (and the centre mark) for a circle,
    // the whole row otherwise.
    let last = f32::from(side - 1);
    for row in 0..n {
        let dy = row as f32 - hotf;
        let mut spans: [Option<(f32, f32)>; 3] = [None; 3];
        if let Shape::Circle { radius, centre } = shape {
            let reach = (0.5 * look.stroke + 1.0) * scale + 0.5;
            if let Some([l, r]) = ring_spans(dy, radius * scale, reach) {
                spans = [Some(l), Some(r), None];
            }
            let cross = 3.0 * scale + reach + 1.0;
            if *centre && dy.abs() <= cross {
                spans[2] = Some((-cross, cross));
            }
        } else {
            spans[0] = Some((-hotf, hotf));
        }
        for (a, b) in spans.into_iter().flatten() {
            let (c0, c1) = ((hotf + a).floor().clamp(0.0, last) as usize, (hotf + b).ceil().clamp(0.0, last) as usize);
            for col in c0..=c1 {
                put(col, row, coverage(shape, col as f32 - hotf, dy, scale, look.stroke));
            }
        }
    }
    Some(CustomCursorImage { rgba: Arc::from(rgba), size: [side, side], hotspot: [hot, hot] })
}
