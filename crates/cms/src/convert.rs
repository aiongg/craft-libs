//! Convenience conversions for UI code (F04.15): sRGB to and from CMYK through cached
//! transforms, and the nearest in-gamut colour for a picker's "out of gamut" chip. The picker
//! and the Color panel call these per frame, so every transform is built once and shared.
//!
//! PhotoCraft kept these in `photocraft_color::convert` with a thread-local "active CMYK space";
//! here the space is explicit: a [`CmykConverter`] holds one CMYK profile (the document's, or the
//! built-in coated CMYK) and its two transforms.

use std::sync::{Arc, OnceLock};

use crate::{Builtin, CmsError, Intent, Profile, Transform, TransformOptions, cached};

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
    /// The ΔE (CIE76) past which a colour is out of the CMYK gamut in a colour picker: 3.0, as
    /// PhotoCraft's picker (`GAMUT_DE`).
    pub const PICKER_GAMUT_DE: f32 = 3.0;

    /// A converter for `cmyk` (a CMYK output profile). Errors when the profile can't link to sRGB
    /// in either direction.
    // pc: crates/color/src/convert.rs:94 `cmyk_to_srgb_transform` / `srgb_to_cmyk_transform`, on
    // the given profile instead of the built-in; `Transform::new(.., Intent::RelativeColorimetric, true)`
    // both ways, via `transform::cached`.
    pub fn new(cmyk: Arc<Profile>) -> Result<CmykConverter, CmsError> {
        let opts = TransformOptions { intent: Intent::RelativeColorimetric, bpc: true, ..TransformOptions::default() };
        let srgb = Builtin::Srgb.profile();
        let to_srgb = cached(&cmyk, srgb, opts)?;
        let from_srgb = cached(srgb, &cmyk, opts)?;
        Ok(CmykConverter { profile: cmyk, to_srgb, from_srgb })
    }

    /// The built-in coated CMYK converter ([`Builtin::CoatedCmyk`]),
    /// built once per process; what a document without a CMYK profile uses.
    // pc: crates/color/src/convert.rs:94 (OnceLock); `Builtin::CoatedCmyk.profile()`.
    #[expect(clippy::expect_used, reason = "the built-in profiles always link; a unit test builds it")]
    pub fn coated() -> &'static CmykConverter {
        static COATED: OnceLock<CmykConverter> = OnceLock::new();
        COATED.get_or_init(|| CmykConverter::new(Arc::new(Builtin::CoatedCmyk.profile().clone())).expect("the built-in profiles link"))
    }

    /// The CMYK profile this converter uses.
    pub fn profile(&self) -> &Arc<Profile> {
        &self.profile
    }

    /// Encoded sRGB (0–1 each) for `cmyk` inks (0–1 each, 0 = no ink).
    // pc: crates/color/src/convert.rs:107 `cmyk_to_rgb` (eval_fast on the to-sRGB transform).
    pub fn to_srgb(&self, cmyk: [f32; 4]) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        // The transform is 4 -> 3, so `eval_fast` can only fail on a profile that links to
        // other channel counts; the naive formula then stands in.
        match self.to_srgb.eval_fast(&cmyk, &mut out) {
            Ok(()) => out,
            Err(_) => cmyk_to_srgb_naive(cmyk),
        }
    }

    /// Inks (0–1 each) for encoded sRGB `rgb` (clamped to 0–1 first).
    // pc: crates/color/src/convert.rs:129 `rgb_to_cmyk`.
    pub fn from_srgb(&self, rgb: [f32; 3]) -> [f32; 4] {
        let v = rgb.map(|c| c.clamp(0.0, 1.0));
        let mut out = [0.0f32; 4];
        match self.from_srgb.eval_fast(&v, &mut out) {
            Ok(()) => out,
            Err(_) => srgb_to_cmyk_naive(v),
        }
    }

    /// Whether sRGB `rgb` is outside this CMYK profile's gamut: the round trip through the inks
    /// moves it by more than `threshold` ΔE (CIE76, Lab D50); a picker uses [`Self::PICKER_GAMUT_DE`],
    /// [`gamut::DEFAULT_THRESHOLD`](crate::gamut::DEFAULT_THRESHOLD) is 4.0.
    // pc: crates/ui-egui/src/dialogs/color_picker_ui.rs:45 `out_of_gamut` (GAMUT_DE = 3.0).
    pub fn out_of_gamut(&self, rgb: [f32; 3], threshold: f32) -> bool {
        self.nearest_in_gamut(rgb, threshold).is_some()
    }

    /// The nearest colour inside the gamut: the round trip sRGB → inks → sRGB, which the picker's
    /// gamut chip sets on a click. `None` when `rgb` is already in gamut (within `threshold`).
    // pc: crates/ui-egui/src/dialogs/color_picker_ui.rs:45 `out_of_gamut` returns the round trip.
    pub fn nearest_in_gamut(&self, rgb: [f32; 3], threshold: f32) -> Option<[f32; 3]> {
        let back = self.to_srgb(self.from_srgb(rgb)).map(|c| c.clamp(0.0, 1.0));
        let (a, b) = (srgb_to_lab(rgb), srgb_to_lab(back));
        let de = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        (de > threshold).then_some(back)
    }
}

/// Encoded sRGB to CIE L*a*b* (D50), the space of the gamut distance.
fn srgb_to_lab(rgb: [f32; 3]) -> [f32; 3] {
    const D50: [f32; 3] = [0.964_22, 1.0, 0.825_21];
    const SRGB_TO_XYZ_D50: [[f32; 3]; 3] = [[0.436_074, 0.385_064, 0.143_080], [0.222_504, 0.716_878, 0.060_618], [0.013_932, 0.097_104, 0.714_173]];
    let lin = rgb.map(|v| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) });
    let xyz = SRGB_TO_XYZ_D50.map(|r| r[0] * lin[0] + r[1] * lin[1] + r[2] * lin[2]);
    let f = |t: f32| if t > (6.0f32 / 29.0).powi(3) { t.cbrt() } else { t / (3.0 * (6.0f32 / 29.0).powi(2)) + 4.0 / 29.0 };
    let (fx, fy, fz) = (f(xyz[0] / D50[0]), f(xyz[1] / D50[1]), f(xyz[2] / D50[2]));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// Naive conversions without a profile, for tests and for a CMS failure: `k = 1 - max(r, g, b)`,
/// inks proportional to the remainder.
// pc: crates/color/src/convert.rs:161 `cmyk_to_rgb_naive`, :167 `rgb_to_cmyk_naive`.
pub fn cmyk_to_srgb_naive(cmyk: [f32; 4]) -> [f32; 3] {
    let k = 1.0 - cmyk[3];
    [(1.0 - cmyk[0]) * k, (1.0 - cmyk[1]) * k, (1.0 - cmyk[2]) * k]
}

/// See [`cmyk_to_srgb_naive`].
// pc: crates/color/src/convert.rs:167 `rgb_to_cmyk_naive`.
pub fn srgb_to_cmyk_naive(rgb: [f32; 3]) -> [f32; 4] {
    let k = 1.0 - rgb[0].max(rgb[1]).max(rgb[2]);
    if k >= 1.0 - 1e-6 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let d = 1.0 - k;
    [(1.0 - rgb[0] - k) / d, (1.0 - rgb[1] - k) / d, (1.0 - rgb[2] - k) / d, k]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure cyan ink is a blue-green outside sRGB, so it reads back as mostly cyan with no yellow
    /// or black; paper is white, solid black ink a dark neutral, and in-gamut greys round-trip.
    #[test]
    fn coated_cmyk_round_trips_the_primaries() {
        let conv = CmykConverter::coated();
        let rgb = conv.to_srgb([1.0, 0.0, 0.0, 0.0]);
        assert!(rgb[0] < 0.2 && rgb[1] > 0.4 && rgb[2] > 0.5, "cyan ink gave {rgb:?}");
        let inks = conv.from_srgb(rgb);
        assert!(inks[0] > 0.6 && inks[1] < 0.25 && inks[2] < 0.05 && inks[3] < 0.02, "inks {inks:?}");
        assert!(conv.to_srgb([0.0; 4]).iter().all(|v| *v > 0.99));
        let k = conv.to_srgb([0.0, 0.0, 0.0, 1.0]);
        assert!(k[0] < 0.3 && (k[0] - k[2]).abs() < 0.05, "black ink gave {k:?}");
        let grey = conv.to_srgb(conv.from_srgb([0.5; 3]));
        assert!(grey.iter().all(|v| (v - 0.5).abs() < 0.03), "grey gave {grey:?}");
    }

    /// sRGB pure green is out of the coated gamut; its nearest in-gamut colour is in gamut.
    #[test]
    fn nearest_in_gamut_is_in_gamut() {
        let conv = CmykConverter::coated();
        let green = [0.0, 1.0, 0.0];
        assert!(conv.out_of_gamut(green, 3.0));
        let near = conv.nearest_in_gamut(green, 3.0).expect("green is out of gamut");
        assert!(!conv.out_of_gamut(near, 3.0));
        assert!(conv.nearest_in_gamut(near, 3.0).is_none());
    }
}
