//! The device texture limit, in one place: the largest side a texture may have, how much a large
//! image is reduced to fit it, the raster display's detail texture, and [`fit_image`] for any
//! image past the limit.

use egui::{Color32, ColorImage, Context};

use crate::canvas::PixelRect;

/// The largest texture side the display uploads.
pub const MAX_TEXTURE: u32 = 4096;

/// The texture side on this device: [`MAX_TEXTURE`], or the device's smaller limit (2048 on some
/// older GPUs and WebGL); past it egui asserts and the backend fails.
// pc: crates/ui-egui/src/canvas/textures.rs:15 `texture_side`, :20 `clamp_texture_side`.
pub fn texture_side(ctx: &Context) -> u32 {
    u32::try_from(ctx.input(|i| i.max_texture_side)).unwrap_or(MAX_TEXTURE).clamp(1, MAX_TEXTURE)
}

/// How many document pixels per side one texel of the base texture holds for a document of
/// `size` with textures at most `side` per side: 1 when it fits.
// pc: crates/ui-egui/src/canvas/textures.rs:26 `texture_factor`.
pub fn texture_factor(size: [u32; 2], side: u32) -> u32 {
    size[0].max(size[1]).div_ceil(side.max(1)).max(1)
}

/// The detail texture a zoomed-in view of a reduced document needs: the part in view (`region`,
/// snapped outward to 256-texel steps so a small pan reuses it) at one texel per `factor`
/// pixels, a power of two no coarser than the screen and capped by the texture side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailPlan {
    pub factor: u32,
    pub region: PixelRect,
}

/// The detail region snaps outward to multiples of this many texels.
const DETAIL_QUANTUM: i32 = 256;

/// The largest power of two, up to 64, whose texels are still no coarser than a device pixel at
/// `device_zoom`.
// pc: crates/ui-egui/src/dialogs/adjust_preview.rs:363 `proxy_factor`.
fn proxy_factor(device_zoom: f64) -> u32 {
    let mut k = 1u32;
    while k < 64 && f64::from(k * 2) * device_zoom <= 1.0 {
        k *= 2;
    }
    k
}

impl DetailPlan {
    /// The plan for a view at `device_zoom` (device pixels per document pixel) over `visible`,
    /// or `None` when the base texture is already as fine as the screen or nothing is in view.
    // pc: crates/ui-egui/src/canvas/textures.rs:50 `detail_plan`.
    pub fn of(size: [u32; 2], side: u32, device_zoom: f64, visible: PixelRect) -> Option<DetailPlan> {
        if !device_zoom.is_finite() || device_zoom <= 0.0 {
            return None;
        }
        let base = texture_factor(size, side);
        let mut factor = proxy_factor(device_zoom);
        let v = visible.intersect(&PixelRect::of_size(size));
        if v.is_empty() {
            return None;
        }
        let bounds = PixelRect::of_size(size);
        while factor < base {
            let q = DETAIL_QUANTUM.saturating_mul(i32::try_from(factor).unwrap_or(i32::MAX));
            let down = |x: i32| x.div_euclid(q).saturating_mul(q);
            let up = |x: i32| x.div_euclid(q).saturating_add(i32::from(x.rem_euclid(q) != 0)).saturating_mul(q);
            let region = PixelRect::new(down(v.x0), down(v.y0), up(v.x1), up(v.y1)).intersect(&bounds);
            // A reduced region's texel count can round one up.
            let fits = |n: i32| n.unsigned_abs().div_ceil(factor).saturating_add(1) <= side;
            // Snapped when that fits, else just what is in view (a window near the device's limit).
            for region in [region, v] {
                if !region.is_empty() && fits(region.width()) && fits(region.height()) {
                    return Some(DetailPlan { factor, region });
                }
            }
            factor = factor.saturating_mul(2);
        }
        None
    }
}

/// `image` reduced to at most `max_side` texels per side, or unchanged when it fits. The image is
/// reduced by the whole factor [`texture_factor`]`(size, max_side)`: each texel is the box average
/// of `factor × factor` pixels (of fewer at the right and bottom edges), averaged in egui's
/// premultiplied values, so a transparent edge stays clean. The result is
/// `size.div_ceil(factor)`, at least 1 × 1. A whole factor keeps every reduced texel on whole
/// source pixels, so a later region update maps onto it exactly
/// ([`TextureCache::put_region`](super::TextureCache::put_region)).
// deckcraft: crates/ui-egui/src/textures.rs:174 `shrink_to` (the same whole-factor box average);
// designcraft: crates/ui-egui/src/widgets.rs:916 `fit_texture`.
pub fn fit_image(image: ColorImage, max_side: u32) -> ColorImage {
    let size = image.size.map(|n| u32::try_from(n).unwrap_or(u32::MAX));
    let factor = texture_factor(size, max_side);
    if factor == 1 { image } else { reduce_image(&image, factor) }
}

/// `image` box-averaged by the whole `factor`, in premultiplied values.
pub(super) fn reduce_image(image: &ColorImage, factor: u32) -> ColorImage {
    let f = factor.max(1) as usize;
    let [w, h] = image.size;
    let (rw, rh) = (w.div_ceil(f).max(1), h.div_ceil(f).max(1));
    let mut pixels = Vec::with_capacity(rw * rh);
    for ry in 0..rh {
        for rx in 0..rw {
            let (x0, y0) = (rx * f, ry * f);
            let (x1, y1) = ((x0 + f).min(w), (y0 + f).min(h));
            let mut sum = [0u64; 4];
            for y in y0..y1 {
                for pixel in &image.pixels[y * w + x0..y * w + x1] {
                    for (total, channel) in sum.iter_mut().zip(pixel.to_array()) {
                        *total += u64::from(channel);
                    }
                }
            }
            let count = ((x1 - x0) * (y1 - y0)).max(1) as u64;
            let mean = sum.map(|total| u8::try_from((total + count / 2) / count).unwrap_or(u8::MAX));
            pixels.push(Color32::from_rgba_premultiplied(mean[0], mean[1], mean[2], mean[3]));
        }
    }
    ColorImage::new([rw, rh], pixels)
}
