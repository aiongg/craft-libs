//! Flat subdivision: every triangle split into a regular grid of `4^level` triangles, the
//! attributes interpolated linearly, so the surface stays where it was until something moves it
//! (viewport displacement, a displaced preview). A point on an edge two triangles share has the
//! same bits in both, whichever way round each gives the edge, and corners split at a UV seam
//! agree too (the edge is keyed by position): moving points by position leaves no cracks.

use glam::{Vec2, Vec3, Vec4};

use crate::mesh::{Mesh, SubMesh};

/// Finest split: 64 × 64 segments per edge (for very coarse meshes such as a cube).
pub const MAX_LEVEL: u32 = 6;

/// The highest subdivision level (at most [`MAX_LEVEL`]) at which a mesh of `triangles` stays
/// within `budget` triangles; 0 when it is dense enough as it is.
pub fn level_for(triangles: usize, budget: usize) -> u32 {
    (0..=MAX_LEVEL).rev().find(|l| triangles.saturating_mul(1 << (2 * l)) <= budget).unwrap_or(0)
}

/// One vertex with every attribute (absent ones at their defaults).
#[derive(Clone, Copy)]
struct Point {
    pos: Vec3,
    nrm: Vec3,
    tan: Vec4,
    uv: Vec2,
    color: Vec4,
}

impl Point {
    fn of(mesh: &Mesh, i: u32) -> Option<Point> {
        let i = i as usize;
        Some(Point {
            pos: *mesh.positions.get(i)?,
            nrm: mesh.normals.get(i).copied().unwrap_or(Vec3::Y),
            tan: mesh.tangents.get(i).copied().unwrap_or(Vec4::new(1.0, 0.0, 0.0, 1.0)),
            uv: mesh.uvs.get(i).copied().unwrap_or_default(),
            color: mesh.colors.get(i).copied().unwrap_or(Vec4::ONE),
        })
    }

    /// `w` of `a`, `b`, `c`.
    fn mix(a: &Point, b: &Point, c: &Point, w: [f32; 3]) -> Point {
        Point {
            pos: a.pos * w[0] + b.pos * w[1] + c.pos * w[2],
            nrm: a.nrm * w[0] + b.nrm * w[1] + c.nrm * w[2],
            tan: a.tan * w[0] + b.tan * w[1] + c.tan * w[2],
            uv: a.uv * w[0] + b.uv * w[1] + c.uv * w[2],
            color: a.color * w[0] + b.color * w[1] + c.color * w[2],
        }
    }

    fn lerp(a: &Point, b: &Point, t: f32) -> Point {
        Point {
            pos: a.pos + (b.pos - a.pos) * t,
            nrm: a.nrm + (b.nrm - a.nrm) * t,
            tan: a.tan + (b.tan - a.tan) * t,
            uv: a.uv + (b.uv - a.uv) * t,
            color: a.color + (b.color - a.color) * t,
        }
    }
}

/// The point `k / n` of the way from vertex `p` to vertex `q`, with the same bits whichever way
/// round the edge is given (by position: vertices split at UV seams agree too).
fn on_edge(mesh: &Mesh, (p, q): (u32, u32), k: u32, n: u32) -> Option<Point> {
    let (p, q) = (Point::of(mesh, p)?, Point::of(mesh, q)?);
    let key = |v: &Point| v.pos.to_array().map(f32::to_bits);
    let (a, b, k) = if key(&p) <= key(&q) { (p, q, k) } else { (q, p, n - k) };
    if k == 0 || k == n {
        return Some(if k == 0 { a } else { b });
    }
    Some(Point::lerp(&a, &b, k as f32 / n as f32))
}

impl Mesh {
    /// Every triangle split `level` times (at most [`MAX_LEVEL`]) into a grid of `4^level`
    /// triangles of its own vertices; submeshes keep their triangles and materials. Normals and
    /// tangents are renormalised; each grid keeps its first corner's bitangent sign. The
    /// attributes the mesh has (one per vertex) are interpolated, the others stay empty.
    /// Level 0 gives the mesh unchanged.
    pub fn subdivided(&self, level: u32) -> Mesh {
        if level == 0 {
            return self.clone();
        }
        let segments = 1u32 << level.min(MAX_LEVEL);
        let whole = [SubMesh { name: String::new(), material: 0, start: 0, count: self.indices.len() as u32 }];
        let ranges = if self.submeshes.is_empty() { &whole[..] } else { &self.submeshes[..] };
        let mut points = Vec::new();
        let mut indices = Vec::new();
        let mut submeshes = Vec::with_capacity(self.submeshes.len());

        for range in ranges {
            let first = indices.len() as u32;
            let triangles = self.indices.get(range.start as usize..(range.start as usize).saturating_add(range.count as usize)).unwrap_or_default();
            for t in triangles.as_chunks::<3>().0 {
                let (Some(a), Some(b), Some(c)) = (Point::of(self, t[0]), Point::of(self, t[1]), Point::of(self, t[2])) else { continue };
                let base = points.len() as u32;

                // Row j holds segments + 1 - j points inside the triangle
                // Shared edges use identical interpolation in both triangles to prevent cracks
                for j in 0..=segments {
                    for i in 0..=segments - j {
                        let edge = match (i, j) {
                            (_, 0) => Some(((t[0], t[1]), i)),
                            (0, _) => Some(((t[0], t[2]), j)),
                            _ if i + j == segments => Some(((t[1], t[2]), j)),
                            _ => None,
                        };
                        let p = match edge.and_then(|(ends, k)| on_edge(self, ends, k, segments)) {
                            Some(p) => p,
                            None => {
                                let w = [1.0 - (i + j) as f32 / segments as f32, i as f32 / segments as f32, j as f32 / segments as f32];
                                Point::mix(&a, &b, &c, w)
                            }
                        };
                        points.push(Point { tan: p.tan.truncate().extend(a.tan.w), ..p });
                    }
                }

                let at = |i: u32, j: u32| base + j * (segments + 1) - j * j.saturating_sub(1) / 2 + i;
                for j in 0..segments {
                    for i in 0..segments - j {
                        indices.extend_from_slice(&[at(i, j), at(i + 1, j), at(i, j + 1)]);
                        if i + j + 1 < segments {
                            indices.extend_from_slice(&[at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)]);
                        }
                    }
                }
            }
            if !self.submeshes.is_empty() {
                submeshes.push(SubMesh { start: first, count: indices.len() as u32 - first, ..range.clone() });
            }
        }

        let has = |len: usize| len == self.positions.len() && len > 0;
        Mesh {
            positions: points.iter().map(|p| p.pos).collect(),
            normals: if has(self.normals.len()) { points.iter().map(|p| p.nrm.try_normalize().unwrap_or(p.nrm)).collect() } else { Vec::new() },
            tangents: if has(self.tangents.len()) {
                points.iter().map(|p| p.tan.truncate().try_normalize().unwrap_or(p.tan.truncate()).extend(p.tan.w)).collect()
            } else {
                Vec::new()
            },
            uvs: if has(self.uvs.len()) { points.iter().map(|p| p.uv).collect() } else { Vec::new() },
            colors: if has(self.colors.len()) { points.iter().map(|p| p.color).collect() } else { Vec::new() },
            indices,
            submeshes,
            materials: self.materials.clone(),
            diagonals: Vec::new(),
        }
    }
}

/// World length per unit of UV over the whole mesh (the square root of their area ratio): how
/// far a height change moves the surface for the slope a height-to-normal conversion shows.
/// None without UVs or area.
pub fn world_per_uv(mesh: &Mesh) -> Option<f32> {
    let (mut world_area, mut uv_area) = (0.0f64, 0.0f64);
    for t in mesh.indices.as_chunks::<3>().0 {
        let (Some(a), Some(b), Some(c)) = (Point::of(mesh, t[0]), Point::of(mesh, t[1]), Point::of(mesh, t[2])) else { continue };
        world_area += f64::from((b.pos - a.pos).cross(c.pos - a.pos).length()) * 0.5;
        let (u1, u2) = (b.uv - a.uv, c.uv - a.uv);
        uv_area += f64::from(u1.x * u2.y - u1.y * u2.x).abs() * 0.5;
    }

    let scale = (world_area / uv_area).sqrt() as f32;
    (uv_area > 1e-12 && scale.is_finite() && scale > 0.0).then_some(scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shared edges get the same points to the bit from both sides, whichever way round each
    /// triangle gives the edge; area, winding and attributes survive every level.
    #[test]
    fn grids_cover_each_triangle_with_four_per_level() {
        let quad = |indices: Vec<u32>| Mesh {
            positions: vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0), Vec3::new(2.0, 2.0, 0.0)],
            normals: vec![Vec3::Z; 4],
            tangents: vec![Vec4::new(1.0, 0.0, 0.0, -1.0); 4],
            uvs: vec![Vec2::ZERO, Vec2::X, Vec2::Y, Vec2::ONE],
            indices,
            submeshes: vec![
                SubMesh { name: "a".into(), material: 0, start: 0, count: 3 },
                SubMesh { name: "b".into(), material: 1, start: 3, count: 3 },
            ],
            materials: vec!["a".into(), "b".into()],
            ..Mesh::default()
        };
        let bits = |p: &Vec3| p.to_array().map(f32::to_bits);
        let shared = |m: &Mesh| {
            let (first, second) = m.positions.split_at(m.positions.len() / 2);
            first.iter().filter(|p| second.iter().any(|q| bits(q) == bits(p))).count()
        };

        for level in 1..=3 {
            let m = quad(vec![0, 1, 2, 1, 3, 2]).subdivided(level);
            let per = 1u32 << (2 * level);
            let ranges: Vec<_> = m.submeshes.iter().map(|s| (s.start, s.count, s.material)).collect();
            assert_eq!(ranges, vec![(0, 3 * per, 0), (3 * per, 3 * per, 1)]);
            m.validate().unwrap();
            // Same area, same winding (all +z), attributes interpolated.
            let area: f32 = m
                .indices
                .chunks(3)
                .map(|t| {
                    let [a, b, c] = [t[0], t[1], t[2]].map(|i| m.positions[i as usize]);
                    (b - a).cross(c - a).z * 0.5
                })
                .inspect(|a| assert!(*a > 0.0))
                .sum();
            assert!((area - 4.0).abs() < 1e-4, "level {level}: area {area}");
            assert!(m.positions.iter().zip(&m.uvs).all(|(p, uv)| (uv.x * 2.0 - p.x).abs() < 1e-5));
            assert!(m.tangents.iter().all(|t| t.w == -1.0));
            assert_eq!(shared(&m), (1 << level) + 1, "level {level}");
        }
        assert_eq!(shared(&quad(vec![0, 1, 2, 2, 1, 3]).subdivided(3)), 9, "the second triangle gives the edge the other way round");
    }
}
