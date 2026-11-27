//! Clipping meshes to a rounded rectangle. A rounded rect is convex, so each triangle is cut
//! against its outline's edges (Sutherland-Hodgman) and the colours and texture coordinates are
//! interpolated along the cuts. The cut edge is not anti-aliased.

use craft_ui_rt::CornerRadii;
use emath::{Pos2, Rect, pos2};
use epaint::{Color32, Mesh, Vertex};

/// The outline of `rect` with rounded `radii`, as a polygon in screen orientation (y down)
/// running clockwise. Arcs get segments for `scale` pixels per point.
pub(crate) fn rounded_outline(rect: Rect, radii: CornerRadii, scale: f32) -> Vec<Pos2> {
    let max = (rect.width().min(rect.height()) / 2.0).max(0.0);
    let corner = |r: f32, centre: Pos2, from: f32, out: &mut Vec<Pos2>| {
        let r = r.clamp(0.0, max);
        if r <= 0.0 {
            // A square corner is the centre's own corner point.
            out.push(centre);
            return;
        }
        let segments = ((r * scale) / 2.0).ceil().clamp(0.0, 60.0) as u32 + 4;
        for i in 0..=segments {
            let a = from + std::f32::consts::FRAC_PI_2 * i as f32 / segments as f32;
            out.push(pos2(centre.x + r * a.cos(), centre.y + r * a.sin()));
        }
    };
    let (x0, y0, x1, y1) = (rect.min.x, rect.min.y, rect.max.x, rect.max.y);
    let pi = std::f32::consts::PI;
    let mut out = Vec::new();
    let clamp = |r: f32| r.clamp(0.0, max);
    // A zero radius puts the "centre" at the corner itself, which the closure pushes as is.
    corner(radii.top_left, pos2(x0 + clamp(radii.top_left), y0 + clamp(radii.top_left)), pi, &mut out);
    corner(radii.top_right, pos2(x1 - clamp(radii.top_right), y0 + clamp(radii.top_right)), 1.5 * pi, &mut out);
    corner(radii.bottom_right, pos2(x1 - clamp(radii.bottom_right), y1 - clamp(radii.bottom_right)), 0.0, &mut out);
    corner(radii.bottom_left, pos2(x0 + clamp(radii.bottom_left), y1 - clamp(radii.bottom_left)), 0.5 * pi, &mut out);
    out
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

fn lerp(a: Vertex, b: Vertex, t: f32) -> Vertex {
    Vertex {
        pos: pos2(a.pos.x + (b.pos.x - a.pos.x) * t, a.pos.y + (b.pos.y - a.pos.y) * t),
        uv: pos2(a.uv.x + (b.uv.x - a.uv.x) * t, a.uv.y + (b.uv.y - a.uv.y) * t),
        color: lerp_color(a.color, b.color, t),
    }
}

/// Keeps what of `mesh` lies inside the convex `outline`.
pub(crate) fn clip_to_outline(mesh: &mut Mesh, outline: &[Pos2]) {
    if outline.len() < 3 {
        mesh.clear();
        return;
    }
    let area: f32 = (0..outline.len())
        .map(|i| {
            let (p, q) = (outline[i], outline[(i + 1) % outline.len()]);
            p.x * q.y - q.x * p.y
        })
        .sum();
    // Positive area is clockwise on screen: the inside is on the right of each edge.
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let side = |a: Pos2, b: Pos2, p: Pos2| sign * ((b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x));

    let mut out = Mesh::with_texture(mesh.texture_id);
    let mut poly: Vec<Vertex> = Vec::new();
    let mut next: Vec<Vertex> = Vec::new();
    for tri in mesh.indices.chunks_exact(3) {
        poly.clear();
        poly.extend(tri.iter().map(|&i| mesh.vertices[i as usize]));
        for i in 0..outline.len() {
            let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
            if poly.is_empty() {
                break;
            }
            next.clear();
            for j in 0..poly.len() {
                let (cur, prev) = (poly[j], poly[(j + poly.len() - 1) % poly.len()]);
                let (sc, sp) = (side(a, b, cur.pos), side(a, b, prev.pos));
                if sc >= 0.0 {
                    if sp < 0.0 {
                        next.push(lerp(prev, cur, sp / (sp - sc)));
                    }
                    next.push(cur);
                } else if sp >= 0.0 {
                    next.push(lerp(prev, cur, sp / (sp - sc)));
                }
            }
            std::mem::swap(&mut poly, &mut next);
        }
        if poly.len() >= 3 {
            let base = out.vertices.len() as u32;
            out.vertices.extend_from_slice(&poly);
            for k in 1..poly.len() as u32 - 1 {
                out.indices.extend_from_slice(&[base, base + k, base + k + 1]);
            }
        }
    }
    *mesh = out;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(rect: Rect) -> Mesh {
        let mut mesh = Mesh::default();
        mesh.add_colored_rect(rect, Color32::WHITE);
        mesh
    }

    fn area(mesh: &Mesh) -> f32 {
        mesh.indices
            .chunks_exact(3)
            .map(|t| {
                let (a, b, c) = (mesh.vertices[t[0] as usize].pos, mesh.vertices[t[1] as usize].pos, mesh.vertices[t[2] as usize].pos);
                ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0
            })
            .sum()
    }

    #[test]
    fn a_rounded_clip_cuts_the_corners_and_keeps_the_middle() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(20.0, 20.0));
        let outline = rounded_outline(rect, CornerRadii::same(6.0), 1.0);
        let mut mesh = quad(rect);
        clip_to_outline(&mut mesh, &outline);
        // A square minus four corners of (r^2 - pi r^2 / 4) each: about 369.1.
        let expected = 400.0 - 4.0 * (36.0 - 9.0 * std::f32::consts::PI);
        assert!((area(&mesh) - expected).abs() < 1.0, "{} vs {expected}", area(&mesh));
        assert!(mesh.vertices.iter().all(|v| rect.expand(0.01).contains(v.pos)));
    }

    #[test]
    fn a_mesh_outside_the_outline_vanishes_and_one_inside_is_unchanged() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(20.0, 20.0));
        let outline = rounded_outline(rect, CornerRadii::same(6.0), 1.0);
        let mut outside = quad(Rect::from_min_max(pos2(30.0, 0.0), pos2(40.0, 10.0)));
        clip_to_outline(&mut outside, &outline);
        assert!(outside.is_empty());
        let mut inside = quad(Rect::from_min_max(pos2(6.0, 6.0), pos2(14.0, 14.0)));
        clip_to_outline(&mut inside, &outline);
        assert!((area(&inside) - 64.0).abs() < 1e-3);
    }
}
