//! # craft-geom3d
//!
//! 3D geometry on the CPU: the [`Mesh`] type with normals and MikkTSpace tangents, built-in
//! [`primitives`] (material ball, sphere, cube, …), flat [`subdivide`]sion, a [`Bvh`] for ray
//! casts and box queries, the UV-space [`UvGBuffer`] that maps texels to surface points, the
//! automatic [`unwrap`], UV padding ([`DilationMap`]), the [`OrbitCamera`], and meshes built
//! from a [`craft_mesh::Scene`] ([`Mesh::from_scene`]).
//!
//! Engine tier: no GPU and no UI dependency. Renderers, path tracers and bakers build on it.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod bvh;
pub mod camera;
pub mod dilate;
pub mod gbuffer;
pub mod mesh;
pub mod primitives;
pub mod scene;
pub mod subdivide;
pub mod unwrap;

pub use bvh::{Bvh, Hit};
pub use camera::{OrbitCamera, Ray};
pub use dilate::DilationMap;
pub use gbuffer::{NO_TRI, Surface, UvGBuffer, surface_at};
pub use glam;
pub use mesh::{BaseColor, MAX_TRIANGLES, Mesh, SubMesh};
pub use scene::{FromScene, MissingNormals};

/// Why a mesh could not be built or used.
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum GeomError {
    /// The mesh breaks one of [`Mesh::validate`]'s rules, or has nothing to work on.
    #[error("invalid mesh: {0}")]
    Invalid(String),
    /// The scene given to [`Mesh::from_scene`] is malformed or too large.
    #[error(transparent)]
    Scene(#[from] craft_mesh::Error),
}

/// The result of a geometry operation.
pub type Result<T> = std::result::Result<T, GeomError>;
