//! Convenience conversions for UI code (F04.15): sRGB to and from CMYK through cached
//! transforms, and the nearest in-gamut colour for a picker's "out of gamut" chip. The picker
//! and the Color panel call these per frame, so every transform is built once and shared.
//!
//! PhotoCraft kept these in `photocraft_color::convert` with a thread-local "active CMYK space";
//! here the space is explicit: a [`CmykConverter`] holds one CMYK profile (the document's, or the
//! built-in coated CMYK) and its two transforms.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::sync::Arc;

use crate::{CmsError, Profile, Transform};

/// sRGB to and from one CMYK profile, relative colorimetric with black point compensation (what
/// PhotoCraft's picker and Color panel show as C, M, Y, K). Clone is cheap: the transforms are
/// shared.
#[derive(Clone)]
pub struct CmykConverter {
    profile: Arc<Profile>,
    to_srgb: Arc<Transform>,
    from_srgb: Arc<Transform>,
}

impl CmykConverter {
    /// A converter for `cmyk` (a CMYK output profile). Errors when the profile can't link to sRGB
    /// in either direction.
    // pc: crates/color/src/convert.rs:94 `cmyk_to_srgb_transform` / `srgb_to_cmyk_transform`, on
    // the given profile instead of the built-in; `Transform::new(.., Intent::RelativeColorimetric, true)`
    // both ways, via `transform::cached`.
    pub fn new(cmyk: Arc<Profile>) -> Result<CmykConverter, CmsError> {
        todo!()
    }

    /// The built-in coated CMYK converter ([`Builtin::CoatedCmyk`](crate::Builtin::CoatedCmyk)),
    /// built once per process; what a document without a CMYK profile uses.
    // pc: crates/color/src/convert.rs:94 (OnceLock); `Builtin::CoatedCmyk.profile()`.
    pub fn coated() -> &'static CmykConverter {
        todo!()
    }

    /// The CMYK profile this converter uses.
    pub fn profile(&self) -> &Arc<Profile> {
        &self.profile
    }

    /// Encoded sRGB (0–1 each) for `cmyk` inks (0–1 each, 0 = no ink).
    // pc: crates/color/src/convert.rs:107 `cmyk_to_rgb` (eval_fast on the to-sRGB transform).
    pub fn to_srgb(&self, cmyk: [f32; 4]) -> [f32; 3] {
        todo!()
    }

    /// Inks (0–1 each) for encoded sRGB `rgb` (clamped to 0–1 first).
    // pc: crates/color/src/convert.rs:129 `rgb_to_cmyk`.
    pub fn from_srgb(&self, rgb: [f32; 3]) -> [f32; 4] {
        todo!()
    }

    /// Whether sRGB `rgb` is outside this CMYK profile's gamut: the round trip through the inks
    /// moves it by more than `threshold` ΔE (CIE76, Lab D50); PhotoCraft's picker uses 3.0,
    /// [`gamut::DEFAULT_THRESHOLD`](crate::gamut::DEFAULT_THRESHOLD) is 4.0.
    // pc: crates/ui-egui/src/dialogs/color_picker_ui.rs:45 `out_of_gamut` (GAMUT_DE = 3.0).
    pub fn out_of_gamut(&self, rgb: [f32; 3], threshold: f32) -> bool {
        todo!()
    }

    /// The nearest colour inside the gamut: the round trip sRGB → inks → sRGB, which the picker's
    /// gamut chip sets on a click. `None` when `rgb` is already in gamut (within `threshold`).
    // pc: crates/ui-egui/src/dialogs/color_picker_ui.rs:45 `out_of_gamut` returns the round trip.
    pub fn nearest_in_gamut(&self, rgb: [f32; 3], threshold: f32) -> Option<[f32; 3]> {
        todo!()
    }
}

/// Naive conversions without a profile, for tests and for a CMS failure: `k = 1 - max(r, g, b)`,
/// inks proportional to the remainder.
// pc: crates/color/src/convert.rs:161 `cmyk_to_rgb_naive`, :167 `rgb_to_cmyk_naive`.
pub fn cmyk_to_srgb_naive(cmyk: [f32; 4]) -> [f32; 3] {
    todo!()
}

/// See [`cmyk_to_srgb_naive`].
// pc: crates/color/src/convert.rs:167 `rgb_to_cmyk_naive`.
pub fn srgb_to_cmyk_naive(rgb: [f32; 3]) -> [f32; 4] {
    todo!()
}

#[cfg(test)]
mod tests {
    /// Pure cyan ink round-trips to a blue-green sRGB and back to about (1, 0, 0, 0).
    #[test]
    #[ignore = "skeleton"]
    fn coated_cmyk_round_trips_the_primaries() {
        todo!()
    }

    /// sRGB pure green is out of the coated gamut; its nearest in-gamut colour is in gamut.
    #[test]
    #[ignore = "skeleton"]
    fn nearest_in_gamut_is_in_gamut() {
        todo!()
    }
}
