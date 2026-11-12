//! Meshes from [`craft_mesh::Scene`]s, the neutral model craft-mesh's file readers produce:
//! every placed mesh in world space, its polygons fanned into triangles with their diagonals
//! marked, and one contiguous submesh range per material and part.

use glam::{Vec2, Vec3, Vec4};

use craft_mesh::{Limits, Scene, Unified, is_mirror, transform_normal, transform_point};

use crate::mesh::{MAX_TRIANGLES, Mesh, SubMesh};
use crate::{GeomError, Result};

/// How [`Mesh::from_scene`] fills in the normals of a mesh whose file gives none. The rule
/// belongs to the file format, so the caller picks it per format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MissingNormals {
    /// Area-weighted smooth normals over welded positions, for the whole result
    /// ([`Mesh::compute_normals_welded`]). OBJ and PLY importers do this.
    #[default]
    Smooth,
    /// Each triangle gets its own corners and its face normal, for the meshes without normals
    /// only. glTF requires this.
    Flat,
}

/// Options of [`Mesh::from_scene`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FromScene {
    /// How to fill in missing normals.
    pub missing_normals: MissingNormals,
}

/// Triangles of one (material, part) pair, in first-seen order.
struct Group {
    material: usize,
    part: String,
    indices: Vec<u32>,
    diagonals: Vec<u8>,
}

/// The vertex arrays being built. Attributes an instance lacks get placeholders, so the arrays
/// stay aligned; the `all_*` flags say whether the placeholders were needed.
#[derive(Default)]
struct Vertices {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    uvs: Vec<Vec2>,
    colors: Vec<Vec4>,
    missing_normals: bool,
    missing_uvs: bool,
    any_colors: bool,
}

impl Vertices {
    /// Append vertex `i` of `u`, placed by `world`, with `normal` when the source has none.
    fn push(&mut self, u: &Unified, i: usize, world: &craft_mesh::Mat4, normal: Option<Vec3>) -> Result<u32> {
        let index = u32::try_from(self.positions.len()).map_err(|_| GeomError::Invalid("the scene has too many vertices".into()))?;
        let p = u.positions.get(i).copied().unwrap_or_default();
        self.positions.push(vec3(transform_point(world, p)));
        self.normals.push(match (normal, u.normals.get(i)) {
            (Some(n), _) => n,
            (None, Some(n)) => vec3(transform_normal(world, *n)),
            (None, None) => Vec3::Y,
        });
        self.uvs.push(u.uvs.get(i).map_or(Vec2::ZERO, |uv| Vec2::from(*uv)));
        self.colors.push(u.colors.get(i).map_or(Vec4::ONE, |c| {
            let c = Vec4::from(*c);
            if c.is_finite() { c.clamp(Vec4::ZERO, Vec4::ONE) } else { Vec4::ONE }
        }));
        Ok(index)
    }
}

fn vec3(v: [f64; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

/// Diagonal bits of a triangle whose corners 1 and 2 swapped: edge 0 and edge 2 trade places.
fn mirrored(bits: u8) -> u8 {
    (bits & 0b010) | ((bits & 0b001) << 2) | ((bits >> 2) & 0b001)
}

/// The index in `materials` of scene material `material` by name, added on first use.
fn material_index(materials: &mut Vec<String>, scene: &Scene, material: Option<usize>) -> usize {
    let name = match material {
        Some(i) => {
            scene.materials.get(i).map_or_else(|| "Default".to_owned(), |m| if m.name.is_empty() { format!("Material_{i}") } else { m.name.clone() })
        }
        None => "Default".to_owned(),
    };
    materials.iter().position(|m| *m == name).unwrap_or_else(|| {
        materials.push(name);
        materials.len() - 1
    })
}

/// The index in `groups` of the (material, part) pair, added on first use.
fn group_index(groups: &mut Vec<Group>, material: usize, part: &str) -> usize {
    groups.iter().position(|g| g.material == material && g.part == part).unwrap_or_else(|| {
        groups.push(Group { material, part: part.to_owned(), indices: Vec::new(), diagonals: Vec::new() });
        groups.len() - 1
    })
}

impl Mesh {
    /// Every mesh `scene` places, in world space and in one mesh.
    ///
    /// - Polygons are fanned into triangles; [`Mesh::diagonals`] marks the edges the fan added.
    ///   A mirroring placement flips the winding, so faces keep pointing out.
    /// - Each material's triangles form one contiguous range of submeshes, one per part (the
    ///   part's name, else the mesh's). Materials are named after the scene's (`Material_{i}`
    ///   when unnamed, `Default` for faces without one), in order of first use.
    /// - Normals: the file's when every vertex has one, else per `options`.
    /// - UVs (the first set): the file's when every vertex has one, with tangents. Otherwise
    ///   `uvs` and `tangents` stay empty and the caller generates UVs its own way (for example
    ///   [`crate::unwrap::auto_unwrap`]), then calls [`Mesh::compute_tangents`].
    /// - Colours: kept when any mesh has them, white elsewhere, clamped to 0..1.
    ///
    /// Up axis and units stay the file's. Errors: a malformed scene or hierarchy, non-finite
    /// positions, no triangles, or more than [`MAX_TRIANGLES`] after placing every instance.
    pub fn from_scene(scene: &Scene, limits: &Limits, options: FromScene) -> Result<Mesh> {
        scene.validate()?;
        let instances = scene.instances(limits)?;
        let mut unified: Vec<Option<Unified>> = vec![None; scene.meshes.len()];
        let mut materials: Vec<String> = Vec::new();
        let mut groups: Vec<Group> = Vec::new();
        let mut v = Vertices::default();
        let mut triangles = 0usize;

        for (index, world) in &instances {
            let (Some(source), Some(slot)) = (scene.meshes.get(*index), unified.get_mut(*index)) else { continue };
            let u = slot.get_or_insert_with(|| source.unified());
            triangles = triangles.saturating_add(u.parts.iter().map(Vec::len).sum());
            if triangles > MAX_TRIANGLES {
                return Err(GeomError::Invalid(format!("the scene places more than {MAX_TRIANGLES} triangles")));
            }
            let has = |len: usize| len == u.positions.len();
            let flat = !has(u.normals.len()) && options.missing_normals == MissingNormals::Flat;
            v.missing_normals |= !has(u.normals.len()) && !flat;
            v.missing_uvs |= !has(u.uvs.len());
            v.any_colors |= !u.colors.is_empty() && has(u.colors.len());
            let mirror = is_mirror(world);

            // Shared vertices, unless every triangle gets its own corners
            let mut shared = Vec::new();
            if !flat {
                for i in 0..u.positions.len() {
                    shared.push(v.push(u, i, world, None)?);
                }
            }

            for (k, (part, tris)) in source.parts.iter().zip(&u.parts).enumerate() {
                let material = material_index(&mut materials, scene, part.material);
                let g = group_index(&mut groups, material, if part.name.is_empty() { &source.name } else { &part.name });
                let bits = u.diagonals.get(k);

                for (t, tri) in tris.iter().enumerate() {
                    let tri = if mirror { [tri[0], tri[2], tri[1]] } else { *tri };
                    let d = bits.and_then(|b| b.get(t)).copied().unwrap_or(0);
                    let d = if mirror { mirrored(d) } else { d };
                    let corners = if flat {
                        let p = tri.map(|i| vec3(transform_point(world, u.positions.get(i as usize).copied().unwrap_or_default())));
                        let face = (p[1] - p[0]).cross(p[2] - p[0]).try_normalize().unwrap_or(Vec3::Y);
                        [
                            v.push(u, tri[0] as usize, world, Some(face))?,
                            v.push(u, tri[1] as usize, world, Some(face))?,
                            v.push(u, tri[2] as usize, world, Some(face))?,
                        ]
                    } else {
                        tri.map(|i| shared.get(i as usize).copied().unwrap_or(0))
                    };
                    if let Some(group) = groups.get_mut(g) {
                        group.indices.extend_from_slice(&corners);
                        group.diagonals.push(d);
                    }
                }
            }
        }

        let mut mesh = Mesh { positions: v.positions, materials, ..Mesh::default() };
        // Each material in one range: stable, so parts keep their first-seen order
        groups.sort_by_key(|g| g.material);
        for g in groups.into_iter().filter(|g| !g.indices.is_empty()) {
            let start = u32::try_from(mesh.indices.len()).map_err(|_| GeomError::Invalid("the scene has too many triangles".into()))?;
            mesh.submeshes.push(SubMesh { name: g.part, material: g.material, start, count: g.indices.len() as u32 });
            mesh.indices.extend(g.indices);
            mesh.diagonals.extend(g.diagonals);
        }
        if mesh.indices.is_empty() {
            return Err(GeomError::Invalid("the scene contains no triangles".into()));
        }
        if mesh.positions.iter().any(|p| !p.is_finite()) {
            return Err(GeomError::Invalid("the scene has non-finite vertex positions".into()));
        }
        if !mesh.diagonals.iter().any(|d| *d != 0) {
            mesh.diagonals.clear();
        }
        if v.any_colors {
            mesh.colors = v.colors;
        }

        if v.missing_normals {
            mesh.compute_normals_welded();
        } else {
            mesh.normals = v.normals.into_iter().map(|n| n.try_normalize().unwrap_or(Vec3::Y)).collect();
        }
        if !v.missing_uvs {
            mesh.uvs = v.uvs.into_iter().map(|uv| if uv.is_finite() { uv } else { Vec2::ZERO }).collect();
            mesh.compute_tangents();
        }
        Ok(mesh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_mesh::{Corner, Material, Node, Part};

    /// A quad (one polygon) facing +z with material `red`, then a triangle with `blue`, then
    /// another triangle with `red` in a second part: placed once as is and once mirrored in x.
    fn scene() -> Scene {
        let quad =
            Part { name: "front".into(), material: Some(0), face_sizes: vec![4], corners: (0..4).map(Corner::all).collect(), ..Part::default() };
        let side = Part { name: "side".into(), material: Some(1), corners: [0, 1, 4].map(Corner::all).to_vec(), ..Part::default() };
        let back = Part { name: "back".into(), material: Some(0), corners: [3, 2, 4].map(Corner::all).to_vec(), ..Part::default() };
        let mesh = craft_mesh::Mesh {
            name: "box".into(),
            positions: vec![[1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.5, 0.5, -1.0]],
            uv_sets: vec![vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.5, 0.5]]],
            parts: vec![quad, side, back],
            ..craft_mesh::Mesh::default()
        };
        let mut scene =
            Scene::from_meshes(vec![mesh], vec![Material::plain("red", [1.0, 0.0, 0.0, 1.0]), Material::plain("blue", [0.0, 0.0, 1.0, 1.0])]);
        let mut mirror = craft_mesh::IDENTITY;
        mirror[0] = -1.0;
        scene.nodes.push(Node { mesh: Some(0), transform: mirror, ..Node::default() });
        scene.roots.push(1);
        scene
    }

    /// Polygon diagonals, material ranges and mirrored winding survive the flattening, in both
    /// normal modes.
    #[test]
    fn scenes_flatten_into_one_mesh_per_material_range() {
        for missing_normals in [MissingNormals::Smooth, MissingNormals::Flat] {
            let mesh = Mesh::from_scene(&scene(), &Limits::default(), FromScene { missing_normals }).unwrap();
            mesh.validate().unwrap();
            assert_eq!(mesh.materials, ["red", "blue"]);
            let parts: Vec<_> = mesh.submeshes.iter().map(|s| (s.name.as_str(), s.material, s.count / 3)).collect();
            // Two instances each: the quad's two triangles and both lone triangles per instance
            assert_eq!(parts, [("front", 0, 4), ("back", 0, 2), ("side", 1, 2)]);

            // Quad triangles: unmirrored, then mirrored with the diagonal bits following the swap
            assert_eq!(&mesh.diagonals[..4], [0b100, 0b001, 0b001, 0b100]);

            // Every front triangle faces +z, the mirrored ones too
            for t in 0..4 {
                let [a, b, c] = mesh.tri(t).unwrap().map(|i| mesh.positions[i]);
                assert!((b - a).cross(c - a).z > 0.0, "{missing_normals:?}: triangle {t} faces away");
            }
            let flat_corners = mesh.positions.len() == mesh.indices.len();
            assert_eq!(flat_corners, missing_normals == MissingNormals::Flat);
        }
    }
}
