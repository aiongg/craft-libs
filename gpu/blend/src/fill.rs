//! Photoshop's Fill on its "special eight" modes.

use crate::formula::Blend;
use crate::mode::BlendMode;

/// Photoshop's "special eight": the modes for which a layer's Fill isn't just less coverage.
/// Below 100% Fill, Photoshop blends at full strength a source pulled toward the mode's neutral
/// colour, the one that leaves the backdrop unchanged (Hard Mix has its own formula); Opacity
/// still mixes the result in. Measured on Photoshop 27.11 (Fill 50%, eight modes, ±1 level).
pub fn fill_is_special(mode: BlendMode) -> bool {
    matches!(
        mode,
        BlendMode::ColorBurn
            | BlendMode::LinearBurn
            | BlendMode::ColorDodge
            | BlendMode::LinearDodge
            | BlendMode::VividLight
            | BlendMode::LinearLight
            | BlendMode::HardMix
            | BlendMode::Difference
    )
}

impl Blend {
    /// [`Blend::rgb`] for a layer at `fill` (0..1): the special eight ([`fill_is_special`])
    /// change their blend below 100%; every other mode, or `fill >= 1`, is plain [`Blend::rgb`].
    pub fn rgb_fill(&self, mode: BlendMode, cb: [f32; 3], cs: [f32; 3], fill: f32) -> [f32; 3] {
        let f = fill.clamp(0.0, 1.0);
        if f >= 1.0 || !fill_is_special(mode) {
            return self.rgb(mode, cb, cs);
        }
        let toward = |neutral: f32| -> [f32; 3] { std::array::from_fn(|i| neutral + (cs[i] - neutral) * f) };
        match mode {
            // Hard Mix at Fill f: (cb − f·(1 − cs)) / (1 − f), clamped. At f = 1 it is the hard
            // threshold (cb + cs ≥ 1), at f = 0 the backdrop.
            BlendMode::HardMix => std::array::from_fn(|i| ((cb[i] - f * (1.0 - cs[i])) / (1.0 - f)).clamp(0.0, 1.0)),
            BlendMode::ColorBurn | BlendMode::LinearBurn => self.rgb(mode, cb, toward(1.0)),
            BlendMode::VividLight | BlendMode::LinearLight => self.rgb(mode, cb, toward(0.5)),
            _ => self.rgb(mode, cb, toward(0.0)),
        }
    }
}
