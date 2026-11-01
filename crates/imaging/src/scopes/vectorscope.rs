//! The vectorscope's count grid ([`Vectorscope`], [`HUES`] × [`SATURATIONS`]) and
//! [`hue_saturation`].

/// Hue cells of the vectorscope grid (PhotoCraft's; lead call L29).
pub const HUES: usize = 128;
/// Saturation cells of the vectorscope grid.
pub const SATURATIONS: usize = 64;

/// A bounded hue and saturation distribution: `counts[hue * SATURATIONS + saturation]` pixels in
/// each cell, and the pixels counted. The colours must already be in the app's chosen RGB space;
/// orientation and colour management are the caller's.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Vectorscope {
    /// `HUES × SATURATIONS` counts, hue-major.
    pub counts: Vec<u32>,
    /// Pixels counted (transparent and non-finite ones are skipped).
    pub samples: u64,
}

impl Default for Vectorscope {
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    fn default() -> Self {
        // pc: crates/algo/src/vectorscope.rs:12-16 (zeroed `HUES * SATURATIONS`).
        todo!()
    }
}

/// What a screen reader hears about a vectorscope: the most saturated cell that holds pixels, and
/// the count-weighted circular mean hue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorscopeSummary {
    /// The highest saturation with a count, 0..=1.
    pub peak_saturation: f32,
    /// The mean hue in degrees, 0..360 (circular mean weighted by count × saturation), red at 0.
    pub mean_hue: f32,
}

impl Vectorscope {
    /// Counts `pixels` (straight RGBA): transparent (alpha `<= 0`) and non-finite pixels are
    /// skipped; RGB is clamped to 0..=1; the hue cell is `floor(h × HUES)` (at most `HUES − 1`),
    /// the saturation cell `round(s × (SATURATIONS − 1))`; counts saturate. PhotoCraft samples at
    /// most 8192 proxy pixels; the caller picks them.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn from_rgba(pixels: &[[f32; 4]]) -> Self {
        // pc: crates/algo/src/vectorscope.rs:19 `from_rgba`.
        todo!()
    }

    /// Adds `other`'s counts and samples (saturating), for a scope counted in tiles or jobs.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn merge(&mut self, other: &Vectorscope) {
        // new: cell-wise saturating add.
        todo!()
    }

    /// The largest count, at least 1: the cloud's alpha is `0.2 + 0.8 × sqrt(count / peak)`.
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn peak(&self) -> u32 {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:465 (`max().unwrap_or(1).max(1)`).
        todo!()
    }

    /// The hue (turns, 0..1) and saturation (0..=1) a cell index stands for: `(index /
    /// SATURATIONS) / HUES` and `(index % SATURATIONS) / (SATURATIONS − 1)`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn cell(index: usize) -> (f32, f32) {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:467-468.
        todo!()
    }

    /// The summary for AccessKit; `None` when nothing was counted.
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn summary(&self) -> Option<VectorscopeSummary> {
        // new: spec AccessKit "Vectorscope" (Peak saturation, mean hue).
        todo!()
    }
}

/// Hue in turns (0..1, red 0, yellow 1/6, green 1/3, blue 2/3) and saturation (0..=1) of an RGB
/// colour; achromatic colours (max − min `<= f32::EPSILON`, or max `<= 0`) are `(0, 0)`.
// pc: crates/algo/src/vectorscope.rs:39 `hue_saturation` (copied; the skeleton writes it, spec "Skeleton parts").
pub fn hue_saturation([r, g, b]: [f32; 3]) -> (f32, f32) {
    let high = r.max(g).max(b);
    let low = r.min(g).min(b);
    let delta = high - low;
    if delta <= f32::EPSILON || high <= 0.0 {
        return (0.0, 0.0);
    }
    let h = if high == r {
        (g - b) / delta
    } else if high == g {
        2.0 + (b - r) / delta
    } else {
        4.0 + (r - g) / delta
    };
    ((h / 6.0).rem_euclid(1.0), delta / high)
}
