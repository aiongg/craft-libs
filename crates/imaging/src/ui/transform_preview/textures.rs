//! [`PreviewPixels`] and [`PreviewTextures`]: the moving pixels of a transform, read once and
//! uploaded as a mip chain built on demand (task TR-T4).

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T4)")]

use std::sync::{Arc, Mutex};

use egui::{ColorImage, Context, TextureHandle, TextureId};

/// The app's pixels of the moving layer: premultiplied texels of which `base_uv` covers the
/// source frame (the rest is padding up to the GPU's texture limit). The app reads them from its
/// own surface once per session (PhotoCraft `read_surface`).
#[derive(Clone, Debug)]
pub struct PreviewPixels {
    pub image: ColorImage,
    /// The uv extent of `image` that covers the source frame, each 0 to 1.
    pub base_uv: [f32; 2],
}

impl PreviewPixels {
    /// `image` covering the source frame with `base_uv` of its extent (clamped to 0 to 1).
    pub fn new(image: ColorImage, base_uv: [f32; 2]) -> Self {
        // pc: crates/ui-egui/src/tools/transform_tex.rs:47 (`base_uv` clamped).
        todo!()
    }

    /// The pixels with `mask` applied: each texel's alpha (and its premultiplied colour) scaled
    /// by `1 − density × (1 − value)`, as the compositor applies a pixel mask.
    pub fn masked(self, mask: &PreviewMask) -> Self {
        // pc: crates/ui-egui/src/tools/transform_tex.rs:181 (`PreviewMask`).
        todo!()
    }
}

/// A layer mask over the preview's pixels: one value per texel of [`PreviewPixels::image`], row by
/// row, 0 hiding and 255 showing, and the mask's density (0 to 1).
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewMask {
    pub values: Vec<u8>,
    pub density: f32,
}

/// The monitor conversion of the displayed preview (LC27): `key` changes whenever `convert` does;
/// `convert` takes premultiplied document texels to the monitor's. The app builds it from
/// `craft-color`'s display transform (PhotoCraft `CanvasDisplay`). Effects keep reading the raw
/// levels.
#[derive(Clone)]
pub struct PreviewDisplay {
    pub key: u64,
    pub convert: Arc<dyn Fn(&ColorImage) -> ColorImage + Send + Sync>,
}

impl std::fmt::Debug for PreviewDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewDisplay").field("key", &self.key).finish_non_exhaustive()
    }
}

/// Which level of the mip chain to draw with, and whether with nearest filtering
/// ([`PreviewTextures::choose`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextureChoice {
    /// 0 is full resolution; level `n` is `2ⁿ` times smaller.
    pub level: u32,
    pub nearest: bool,
}

/// One uploaded texture: its level, filter, display key and the uv extent covering the source.
struct Level {
    choice: TextureChoice,
    display_key: u64,
    texture: TextureHandle,
    uv: [f32; 2],
}

/// The moving pixels uploaded for the preview: level 0 at once, the other mip levels (2 × 2 box
/// filtered) and the nearest-filtered variants on first use, cached for the session. A new
/// display key drops the uploads of the old one, so a profile change does not pile up textures.
pub struct PreviewTextures {
    name: String,
    /// The CPU levels built so far, premultiplied; level 0 first.
    images: Mutex<Vec<Arc<ColorImage>>>,
    base_uv: [f32; 2],
    uploaded: Mutex<Vec<Level>>,
}

impl PreviewTextures {
    /// Uploads `pixels` as level 0 (linear), named `name` in egui's texture manager.
    pub fn new(ctx: &Context, name: impl Into<String>, pixels: PreviewPixels) -> Self {
        // pc: crates/ui-egui/src/tools/transform_tex.rs:44.
        todo!()
    }

    /// The size of level 0 in texels.
    pub fn size(&self) -> [usize; 2] {
        // pc: crates/ui-egui/src/tools/transform_tex.rs:57.
        todo!()
    }

    /// The level and filter for a source pixel shown at `zoom` screen pixels per document pixel,
    /// where the transform scales it by `scale` (PhotoCraft's rule, LC24): with `nearest`
    /// interpolation always level 0 nearest (point sampling; a mip level would blur pixel art);
    /// below `MIP_BELOW` (½) screen pixels per source pixel level `⌊log2(1 / (zoom × scale))⌋`, at
    /// most `MIP_LEVELS` (12); otherwise level 0, nearest when `zoom ≥ CRISP_ZOOM` (2) and
    /// `scale ≤ CRISP_SCALE` (1.05). A product that is not finite or not positive gives level 0
    /// linear. Constants in [`PreviewQuality`](super::PreviewQuality).
    pub fn choose(zoom: f32, scale: f32, nearest: bool) -> TextureChoice {
        // pc: crates/ui-egui/src/tools/transform_tex.rs:63 (`choose`).
        todo!()
    }

    /// The texture and its uv extent for [`choose`](Self::choose)'s answer, uploading that level
    /// and filter on first use; with `display`, the converted upload (the CPU level stays raw).
    pub fn pick(&self, ctx: &Context, choice: TextureChoice, display: Option<&PreviewDisplay>) -> (TextureId, [f32; 2]) {
        // pc: crates/ui-egui/src/tools/transform_tex.rs:88 (`pick_display`).
        todo!()
    }

    /// Level `n` of the mip chain (built on demand, at most down to one texel) and its uv extent
    /// covering the source. The styled preview resamples from it.
    pub fn level_image(&self, n: u32) -> Option<(Arc<ColorImage>, [f32; 2])> {
        // pc: crates/ui-egui/src/tools/transform_tex.rs:118 (`level_image`), :161 (`halve`).
        todo!()
    }
}
