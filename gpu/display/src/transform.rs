//! The display transform: how a canvas shows a composite's colours on a monitor. The composite's
//! values go through one cached transform to the monitor's profile; when that transform changes
//! nothing, the canvas skips it.
//!
//! Linear composites (EXR, linear profiles) are stored in the canvas texture sRGB-encoded
//! ([`DisplayTransform::encode_texture`]) so shadows keep their precision in 8 bits; the
//! transform's source is then the same primaries with the sRGB curve.

use std::sync::Arc;

use craft_cms::transform::{TransformOptions, cached};
use craft_cms::{Builtin, CmsError, ColorSpace, Lut3d, Profile, Transform};

use crate::monitor::DISPLAY_INTENT;
use crate::pass::{DisplayLut, PassColor};

/// Edge length of the display LUT for transforms the GPU can't take as a shaper.
pub const LUT_SIZE: usize = 33;

/// Pixels per piece [`DisplayTransform::encode_texture`] hands to one thread.
const PAR_PIXELS: usize = 16 * 1024;

pub(crate) fn hash_of(v: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

/// The [`DisplayTransform::key`] of a composite and monitor pair. The source profile, the
/// encoding and whether the transform is the identity all follow from the pair.
pub(crate) fn signature(composite: &Profile, monitor: &Profile) -> u64 {
    hash_of((composite.content_hash(), monitor.content_hash()))
}

/// Does `t` map every lattice point (the neutral axis only for a grey source) to itself within
/// half an 8-bit step?
fn is_identity(t: &Transform, neutral_only: bool) -> bool {
    const N: usize = 9;
    let s = (N - 1) as f32;
    let mut out = [0.0f32; 16];
    let mut check = |v: [f32; 3]| match t.eval(&v, &mut out) {
        Ok(()) => (0..3).all(|i| (out[i] - v[i]).abs() <= 0.5 / 255.0),
        Err(_) => false,
    };
    if neutral_only {
        return (0..=32).all(|i| check([i as f32 / 32.0; 3]));
    }
    (0..N).all(|b| (0..N).all(|g| (0..N).all(|r| check([r as f32 / s, g as f32 / s, b as f32 / s]))))
}

/// Is `p` an RGB matrix/TRC profile with linear curves?
fn is_linear_rgb(p: &Profile) -> bool {
    p.color_space == ColorSpace::Rgb && p.is_matrix_shaper() && p.trc.as_ref().is_some_and(|t| t.iter().all(craft_cms::Curve::is_identity))
}

/// The same primaries with the sRGB curve: what an sRGB-encoded texture of a linear composite
/// is in.
fn srgb_curve_twin(p: &Profile) -> Profile {
    let c = craft_cms::curve::srgb_trc();
    let mut q = p.clone();
    q.trc = Some([c.clone(), c.clone(), c]);
    q.description = format!("{} (sRGB-encoded)", p.description);
    q.with_encoded_bytes()
}

/// The profile a canvas shows `profile`'s values in: a gray profile's RGB view (sGray's when it
/// has none, sRGB as the last resort); any other profile itself.
pub fn rgb_view(profile: &Profile) -> Arc<Profile> {
    if profile.color_space != ColorSpace::Gray {
        return Arc::new(profile.clone());
    }
    profile
        .gray_as_rgb()
        .or_else(|| Builtin::SGray.profile().gray_as_rgb())
        .map(Arc::new)
        .unwrap_or_else(|| Arc::new(Builtin::Srgb.profile().clone()))
}

/// How one composite profile shows on one monitor.
#[derive(Debug)]
pub struct DisplayTransform {
    /// The profile of the values the canvas texture holds: the composite's, or its sRGB-curve
    /// twin when [`encode_srgb`](Self::encode_srgb).
    pub source: Arc<Profile>,
    /// The monitor profile.
    pub monitor: Arc<Profile>,
    /// Texture values to monitor values; `None` when that is the identity within half an 8-bit
    /// step on every lattice point (the neutral axis only for a gray composite).
    pub transform: Option<Arc<Transform>>,
    /// The texture stores the composite sRGB-encoded (a linear composite).
    pub encode_srgb: bool,
    /// Changes whenever any of the above changes, for the canvas's display caches.
    pub key: u64,
    /// Changes whenever what the texture stores changes (the composite profile), not with the
    /// monitor: one texture serves every display, and only the display colour is per display.
    pub texture_key: u64,
}

impl DisplayTransform {
    /// The transform from `composite` (the profile of the compositor's RGB output; see
    /// [`rgb_view`] for gray) to `monitor` ([`DISPLAY_INTENT`], BPC), built through
    /// `craft_cms::cached`.
    pub fn new(composite: Arc<Profile>, monitor: Arc<Profile>) -> Result<DisplayTransform, CmsError> {
        let key = signature(&composite, &monitor);
        let texture_key = hash_of(composite.content_hash());
        let encode_srgb = is_linear_rgb(&composite);
        let source = if encode_srgb { Arc::new(srgb_curve_twin(&composite)) } else { composite };
        let transform = if source.content_hash() == monitor.content_hash() {
            None
        } else {
            let opts = TransformOptions { intent: DISPLAY_INTENT, bpc: true, ..TransformOptions::default() };
            let t = cached(&source, &monitor, opts)?;
            (!is_identity(&t, source.color_space == ColorSpace::Gray)).then_some(t)
        };
        Ok(DisplayTransform { source, monitor, transform, encode_srgb, key, texture_key })
    }

    /// Nothing to do: the composite's values go to the screen unchanged.
    pub fn is_identity(&self) -> bool {
        !self.encode_srgb && self.transform.is_none()
    }

    /// `rgb` (0–1 each, as the texture holds it) as the monitor shows it, exactly.
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let Some(t) = &self.transform else { return rgb };
        let mut out = [0.0f32; 16];
        match t.eval(&rgb, &mut out) {
            Ok(()) => [out[0], out[1], out[2]],
            Err(_) => rgb,
        }
    }

    /// Straight RGBA composite pixels as the texture stores them, in place: sRGB-encoded when
    /// [`encode_srgb`](Self::encode_srgb), else untouched. Values above 1.0 are encoded too (the
    /// curve extended), up to the largest half float, for float textures.
    pub fn encode_texture(&self, px: &mut [[f32; 4]]) {
        if !self.encode_srgb {
            return;
        }
        let encode = |piece: &mut [[f32; 4]]| {
            for p in piece {
                for v in &mut p[..3] {
                    *v = craft_wgsl::linear_to_srgb(v.clamp(0.0, 65504.0));
                }
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rayon::prelude::*;
            px.par_chunks_mut(PAR_PIXELS).for_each(encode);
        }
        #[cfg(target_arch = "wasm32")]
        px.chunks_mut(PAR_PIXELS).for_each(encode);
    }

    /// The display colour as the display pass runs it: the transform's shaper tables when it is
    /// a matrix/TRC conversion, else a [`LUT_SIZE`]³ LUT. `None` for the identity.
    pub fn pass_color(&self) -> Result<Option<PassColor>, CmsError> {
        let Some(t) = &self.transform else { return Ok(None) };
        if let Some(shaper) = t.gpu_matrix_shaper() {
            return Ok(Some(PassColor::Shapers { first: Arc::new(shaper), quantize: false, second: None }));
        }
        let lut = Lut3d::from_transform(t, LUT_SIZE)?;
        Ok(Some(PassColor::Lut { lut: Arc::new(DisplayLut::new(&lut, None)?), gamut: None }))
    }
}
