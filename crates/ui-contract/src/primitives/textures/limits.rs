//! The texture limit in one place: the largest side a texture may have, how much a large image is reduced to fit
//! it, and `fit_image` for any image past the limit.

use craft_behavior::PixelSize;
use craft_ui_rt::{RgbaImage, SizeU};

/// The largest texture side the library uploads: every renderer's devices take it.
pub const MAX_TEXTURE: u32 = 4096;

/// How many document pixels per side one texel of the base texture holds for a document of `size` with textures at
/// most `side` per side: 1 when it fits.
pub fn texture_factor(size: PixelSize, side: u32) -> u32 {
    size.width.max(size.height).div_ceil(side.max(1)).max(1)
}

/// `image` reduced to at most `max_side` texels per side, or unchanged when it fits. The image is reduced by the
/// whole factor `texture_factor(size, max_side)`: each texel is the box average of `factor x factor` pixels (of
/// fewer at the right and bottom edges), averaged in premultiplied values, so a transparent edge stays clean. The
/// result is `size.div_ceil(factor)`, at least 1 x 1.
pub fn fit_image(image: RgbaImage, max_side: u32) -> RgbaImage {
    let factor = texture_factor(PixelSize::new(image.size.width, image.size.height), max_side);
    if factor == 1 { image } else { reduce_image(&image, factor) }
}

/// `image` box-averaged by the whole `factor`, in premultiplied values.
pub fn reduce_image(image: &RgbaImage, factor: u32) -> RgbaImage {
    let f = factor.max(1) as usize;
    let (w, h) = (image.size.width as usize, image.size.height as usize);
    let (rw, rh) = (w.div_ceil(f).max(1), h.div_ceil(f).max(1));
    let mut pixels = Vec::with_capacity(rw * rh * 4);
    for ry in 0..rh {
        for rx in 0..rw {
            let (x0, y0) = (rx * f, ry * f);
            let (x1, y1) = ((x0 + f).min(w), (y0 + f).min(h));
            // Colour weighted by alpha (premultiplied), alpha plain.
            let mut sum = [0u64; 4];
            for y in y0..y1 {
                for pixel in image.pixels[(y * w + x0) * 4..(y * w + x1) * 4].chunks_exact(4) {
                    let alpha = u64::from(pixel[3]);
                    for (total, channel) in sum.iter_mut().zip(&pixel[..3]) {
                        *total += u64::from(*channel) * alpha;
                    }
                    sum[3] += alpha;
                }
            }
            let count = ((x1 - x0) * (y1 - y0)).max(1) as u64;
            let alpha = sum[3];
            let colour = |total: u64| if alpha == 0 { 0 } else { u8::try_from((total + alpha / 2) / alpha).unwrap_or(u8::MAX) };
            pixels.extend_from_slice(&[colour(sum[0]), colour(sum[1]), colour(sum[2]), u8::try_from((alpha + count / 2) / count).unwrap_or(u8::MAX)]);
        }
    }
    RgbaImage { size: SizeU::new(rw as u32, rh as u32), pixels }
}
