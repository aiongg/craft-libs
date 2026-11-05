//! [`PreviewPixels`] and [`PreviewTextures`]: the moving pixels of a transform, read once and
//! uploaded as a mip chain built on demand (task TR-T4).

use std::sync::{Arc, Mutex, PoisonError};

use egui::{Color32, ColorImage, Context, TextureHandle, TextureId, TextureOptions};

use super::PreviewQuality;

/// The app's pixels of the moving layer: premultiplied texels of which `base_uv` covers the
/// source frame (the rest is padding up to the GPU's texture limit). The app reads them from its
/// own surface once per session (PhotoCraft `read_surface`).
#[derive(Clone, Debug)]
pub struct PreviewPixels {
    /// The texels, premultiplied.
    pub image: ColorImage,
    /// The uv extent of `image` that covers the source frame, each 0 to 1.
    pub base_uv: [f32; 2],
}

impl PreviewPixels {
    /// `image` covering the source frame with `base_uv` of its extent (clamped to 0 to 1).
    pub fn new(image: ColorImage, base_uv: [f32; 2]) -> Self {
        let base_uv = [base_uv[0].clamp(0.0, 1.0), base_uv[1].clamp(0.0, 1.0)];
        Self { image, base_uv }
    }

    /// The pixels with `mask` applied: each texel's alpha (and its premultiplied colour) scaled
    /// by `1 − density × (1 − value)`, as the compositor applies a pixel mask.
    pub fn masked(mut self, mask: &PreviewMask) -> Self {
        let density = mask.density.clamp(0.0, 1.0);
        for (texel, &value) in self.image.pixels.iter_mut().zip(&mask.values) {
            let keep = (1.0 - density * (1.0 - f32::from(value) / 255.0)).clamp(0.0, 1.0);
            let [r, g, b, a] = texel.to_array();
            let scale = |c: u8| (f32::from(c) * keep).round() as u8;
            *texel = Color32::from_rgba_premultiplied(scale(r), scale(g), scale(b), scale(a));
        }
        self
    }
}

/// A layer mask over the preview's pixels: one value per texel of [`PreviewPixels::image`], row by
/// row, 0 hiding and 255 showing, and the mask's density (0 to 1).
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewMask {
    /// One value per texel of the image, row by row.
    pub values: Vec<u8>,
    /// The mask's density, 0 to 1.
    pub density: f32,
}

/// The monitor conversion of the displayed preview (LC27): `key` changes whenever `convert` does;
/// `convert` takes premultiplied document texels to the monitor's. The app builds it from
/// `craft-color`'s display transform (PhotoCraft `CanvasDisplay`). Effects keep reading the raw
/// levels.
#[derive(Clone)]
pub struct PreviewDisplay {
    /// Changes whenever `convert` does, so caches keyed on it refresh.
    pub key: u64,
    /// Converts premultiplied document texels to the monitor's.
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
    /// Whether to draw with nearest filtering instead of linear.
    pub nearest: bool,
}

/// One uploaded texture: its level, filter, display key and the uv extent covering the source.
pub(super) struct Level {
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
    pub(super) images: Mutex<Vec<Arc<ColorImage>>>,
    base_uv: [f32; 2],
    pub(super) uploaded: Mutex<Vec<Level>>,
}

impl PreviewTextures {
    /// Uploads `pixels` as level 0 (linear), named `name` in egui's texture manager.
    pub fn new(ctx: &Context, name: impl Into<String>, pixels: PreviewPixels) -> Self {
        let name = name.into();
        let base_uv = pixels.base_uv;
        let image = Arc::new(pixels.image);
        let texture = ctx.load_texture(format!("{name}-0-linear"), Arc::clone(&image), TextureOptions::LINEAR);
        let choice = TextureChoice::default();
        Self { name, images: Mutex::new(vec![image]), base_uv, uploaded: Mutex::new(vec![Level { choice, display_key: 0, texture, uv: base_uv }]) }
    }

    /// The size of level 0 in texels.
    pub fn size(&self) -> [usize; 2] {
        self.images.lock().unwrap_or_else(PoisonError::into_inner).first().map_or([0, 0], |image| image.size)
    }

    /// The level and filter for a source pixel shown at `zoom` screen pixels per document pixel,
    /// where the transform scales it by `scale` (PhotoCraft's rule, LC24): with `nearest`
    /// interpolation always level 0 nearest (point sampling; a mip level would blur pixel art);
    /// below `MIP_BELOW` (½) screen pixels per source pixel level `⌊log2(1 / (zoom × scale))⌋`, at
    /// most `MIP_LEVELS` (12); otherwise level 0, nearest when `zoom ≥ CRISP_ZOOM` (2) and
    /// `scale ≤ CRISP_SCALE` (1.05). A product that is not finite or not positive gives level 0
    /// linear. Constants in [`PreviewQuality`](super::PreviewQuality).
    pub fn choose(zoom: f32, scale: f32, nearest: bool) -> TextureChoice {
        let screen = zoom * scale;
        if !screen.is_finite() || screen <= 0.0 {
            return TextureChoice::default();
        }
        if nearest {
            return TextureChoice { level: 0, nearest: true };
        }
        if screen < PreviewQuality::MIP_BELOW {
            let level = (1.0 / screen).log2().floor().clamp(0.0, PreviewQuality::MIP_LEVELS as f32) as u32;
            return TextureChoice { level, nearest: false };
        }
        TextureChoice { level: 0, nearest: zoom >= PreviewQuality::CRISP_ZOOM && scale <= PreviewQuality::CRISP_SCALE }
    }

    /// The texture and its uv extent for [`choose`](Self::choose)'s answer, uploading that level
    /// and filter on first use; with `display`, the converted upload (the CPU level stays raw).
    pub fn pick(&self, ctx: &Context, choice: TextureChoice, display: Option<&PreviewDisplay>) -> (TextureId, [f32; 2]) {
        let display_key = display.map_or(0, |d| d.key);
        let mut uploaded = self.uploaded.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(level) = uploaded.iter().find(|l| l.choice == choice && l.display_key == display_key) {
            return (level.texture.id(), level.uv);
        }
        let Some((image, uv)) = self.level_image(choice.level) else {
            return uploaded.first().map_or((TextureId::default(), [1.0, 1.0]), |l| (l.texture.id(), l.uv));
        };
        let image = display.map_or_else(|| Arc::clone(&image), |d| Arc::new((d.convert)(&image)));
        let (options, filter) = if choice.nearest { (TextureOptions::NEAREST, "nearest") } else { (TextureOptions::LINEAR, "linear") };
        let texture = ctx.load_texture(format!("{}-{}-{filter}", self.name, choice.level), image, options);
        let id = texture.id();
        // A new display key drops the old uploads, so a profile change does not pile up textures.
        uploaded.retain(|l| l.display_key == display_key);
        uploaded.push(Level { choice, display_key, texture, uv });
        (id, uv)
    }

    /// Level `n` of the mip chain (built on demand, at most down to one texel) and its uv extent
    /// covering the source. The styled preview resamples from it.
    pub fn level_image(&self, n: u32) -> Option<(Arc<ColorImage>, [f32; 2])> {
        let mut images = self.images.lock().unwrap_or_else(PoisonError::into_inner);
        while images.len() <= n as usize {
            let previous = images.last()?;
            if previous.size[0] <= 1 && previous.size[1] <= 1 {
                break;
            }
            let next = Arc::new(halve(previous));
            images.push(next);
        }
        let level = (images.len() - 1).min(n as usize);
        let image = Arc::clone(images.get(level)?);
        let base = images.first()?.size;
        // Level k covers size × 2^k base texels; the last row or column may be padding.
        let factor = (1u64 << level) as f32;
        let uv = |axis: usize| (self.base_uv[axis] * base[axis] as f32 / (image.size[axis] as f32 * factor).max(1.0)).min(1.0);
        let uv = [uv(0), uv(1)];
        Some((image, uv))
    }
}

/// 2 × 2 box filter of a premultiplied image; an odd edge repeats its last row or column.
fn halve(image: &ColorImage) -> ColorImage {
    let [w, h] = image.size;
    let (nw, nh) = (w.div_ceil(2).max(1), h.div_ceil(2).max(1));
    let mut out = vec![Color32::TRANSPARENT; nw * nh];
    let texel = |x: usize, y: usize| image.pixels.get(y.min(h.saturating_sub(1)) * w + x.min(w.saturating_sub(1))).copied().unwrap_or_default();
    for (y, row) in out.chunks_mut(nw).enumerate() {
        for (x, o) in row.iter_mut().enumerate() {
            let quad = [texel(2 * x, 2 * y), texel(2 * x + 1, 2 * y), texel(2 * x, 2 * y + 1), texel(2 * x + 1, 2 * y + 1)];
            let mean = |channel: fn(&Color32) -> u8| ((quad.iter().map(|p| u32::from(channel(p))).sum::<u32>() + 2) / 4) as u8;
            *o = Color32::from_rgba_premultiplied(mean(Color32::r), mean(Color32::g), mean(Color32::b), mean(Color32::a));
        }
    }
    ColorImage::new([nw, nh], out)
}
