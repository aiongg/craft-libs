//! The tip's falloff, drawn as the resize preview.
#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Color32, Painter, Pos2};

/// How a dab of the tip covers from its centre out: solid to `core`, then falling to nothing at
/// `reach` (document units), sampled at [`RINGS`](Self::RINGS) + 1 rings. The engine supplies the
/// numbers ([`sampled`](Self::sampled)); an app without an engine uses
/// [`from_hardness`](Self::from_hardness).
#[derive(Clone, Debug, PartialEq)]
pub struct TipFalloff {
    core: f64,
    reach: f64,
    /// The coverage at ring `k`, at `core + (reach − core) × k / RINGS`; the last is always 0.
    coverage: [f32; TipFalloff::RINGS + 1],
}

impl TipFalloff {
    /// Rings across the falloff: the shoulder is smooth, so straight steps between them are close
    /// (PhotoCraft's 16).
    pub const RINGS: usize = 16;

    /// The falloff the engine computes: solid to `core`, `coverage(d)` (0..=1) at each ring's
    /// distance `d` out to `reach`, and 0 at `reach` whatever `coverage` says there. `reach` below
    /// `core` counts as `core`; non-finite or negative values give an empty falloff (nothing
    /// drawn).
    pub fn sampled(core: f64, reach: f64, coverage: impl Fn(f64) -> f32) -> TipFalloff {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:175 `tip_preview` (`tip_shape` core, `tip_reach`
        // max core, `tip_falloff(d)` per ring, the last ring transparent).
        todo!()
    }

    /// A hard tip of `radius`: solid to the edge.
    pub fn hard(radius: f64) -> TipFalloff {
        // new: `sampled(radius, radius, |_| 1.0)`.
        todo!()
    }

    /// The library's own falloff for a round tip of `radius` and `hardness` (0..=1), for apps
    /// without a brush engine and for stories: solid to `radius × hardness`, then a smoothstep
    /// from 1 down to 0 at `radius`.
    pub fn from_hardness(radius: f64, hardness: f64) -> TipFalloff {
        // new: no engine in the library (step7-brush lead call 1); a smoothstep is the usual soft edge.
        todo!()
    }

    /// The core radius (document units).
    pub fn core(&self) -> f64 {
        self.core
    }

    /// The outer radius where the coverage reaches 0 (document units).
    pub fn reach(&self) -> f64 {
        self.reach
    }

    /// Paints the falloff centred at `centre` (screen) at `zoom` screen points per document unit
    /// in `tint`: one mesh, a centre vertex and the core ring in `tint`, each ring out to the reach
    /// in `tint` scaled by its coverage (`gamma_multiply`), the reach transparent; ring segments
    /// `(reach × zoom × 0.5)` clamped to 24..=160. Nothing when the disc lies outside the painter's
    /// clip rect, or for a non-finite or non-positive size or zoom.
    pub fn paint(&self, painter: &Painter, centre: Pos2, zoom: f32, tint: Color32) {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:175 `tip_preview` (mesh layout and triangle order).
        todo!()
    }
}
