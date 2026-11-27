//! Trials for the Skia renderer: damage-only frames equal full redraws (CPU and, with `gpu`,
//! Vulkan), complex scripts lay out with fallback faces, app surfaces as CPU tiles.

mod damage;
mod scene;
mod surfaces;
mod text;
