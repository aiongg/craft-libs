//! Laying a source pixel over a backdrop pixel in a blend mode.
//!
//! One W3C/PDF general formula in two encodings of alpha:
//! `αo = αs + αb·(1 − αs)` and `αo·Co = (1 − αs)·αb·Cb + (1 − αb)·αs·Cs + αs·αb·B(Cb, Cs)`.
//! PhotoCraft composites straight colour, After Effects and Premiere premultiplied.

use crate::fill::fill_is_special;
use crate::formula::Blend;
use crate::mode::{BlendMode, Variant};
use crate::space::Space;

/// How a pixel's colour relates to its alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Alpha {
    /// Colour independent of alpha.
    #[default]
    Straight,
    /// Colour multiplied by alpha.
    Premultiplied,
}

/// One layer's blending settings for a composite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    /// The blend mode.
    pub mode: BlendMode,
    /// Opacity, 0..1.
    pub opacity: f32,
    /// Photoshop's Fill, 0..1: coverage, except for the special eight below 100% in the
    /// Photoshop variant ([`fill_is_special`]).
    pub fill: f32,
    /// The caller's per-pixel value in 0..1 for Dissolve: the pixel shows where it is below the
    /// coverage. Each compositor keeps its own hash, so its pattern follows its document pixels.
    pub noise: f32,
}

impl Layer {
    /// `mode` at full opacity and fill.
    pub const fn new(mode: BlendMode) -> Layer {
        Layer { mode, opacity: 1.0, fill: 1.0, noise: 0.0 }
    }
}

/// The three weights of a straight composite and the result alpha: `Co = b·Cb + s·Cs + bs·B`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights {
    /// The backdrop's weight.
    pub b: f32,
    /// The source's weight.
    pub s: f32,
    /// The blend's weight.
    pub bs: f32,
    /// The result alpha.
    pub alpha: f32,
}

impl Weights {
    /// The weights of a source of coverage `cover` over a backdrop of alpha `ab`, or `None`
    /// where the result is transparent.
    pub fn new(ab: f32, cover: f32) -> Option<Weights> {
        let ao = cover + ab * (1.0 - cover);
        (ao > 0.0).then(|| Weights { b: (1.0 - cover) * ab / ao, s: (1.0 - ab) * cover / ao, bs: cover * ab / ao, alpha: ao })
    }
}

impl Blend {
    /// `source` laid over `backdrop` with `layer`'s mode, opacity, fill and dissolve noise.
    /// `spaces` are the blend space (where `B(Cb, Cs)` is computed) and the mix space (where
    /// coverage mixes); modes that don't read the backdrop skip the blend space.
    pub fn composite<B: Space, M: Space>(&self, layer: &Layer, alpha: Alpha, backdrop: [f32; 4], source: [f32; 4], spaces: (&B, &M)) -> [f32; 4] {
        match alpha {
            Alpha::Straight => self.straight(layer, backdrop, source, spaces),
            Alpha::Premultiplied => self.premultiplied(layer, backdrop, source, spaces),
        }
    }

    /// The source's colour over the backdrop's with the source's coverage, keeping the
    /// backdrop's alpha ("source atop": Lock Transparent Pixels, Fill's Preserve Transparency):
    /// `Co = (1 − αs)·Cb + αs·B(Cb, Cs)`, `αo = αb`. Unlike source-over followed by restoring the
    /// alpha, a half-transparent backdrop takes the new colour by `αs` alone. Straight colour.
    pub fn composite_atop(&self, mode: BlendMode, backdrop: [f32; 4], source: [f32; 4], opacity: f32) -> [f32; 4] {
        let as_ = source[3] * opacity;
        if as_ <= 0.0 {
            return backdrop;
        }
        let b = self.rgb(mode, rgb(backdrop), rgb(source));
        let c = |i: usize| (1.0 - as_) * backdrop[i] + as_ * b[i];
        [c(0), c(1), c(2), backdrop[3]]
    }

    /// `B(Cb, Cs)` in the blend space, with Photoshop's Fill.
    fn blend_in<B: Space>(&self, mode: BlendMode, cb: [f32; 3], cs: [f32; 3], fill: f32, space: &B) -> [f32; 3] {
        if B::IDENTITY || !mode.reads_backdrop() {
            return self.rgb_fill(mode, cb, cs, fill);
        }
        space.decode(self.rgb_fill(mode, space.encode(cb), space.encode(cs), fill))
    }

    fn straight<B: Space, M: Space>(&self, layer: &Layer, backdrop: [f32; 4], source: [f32; 4], (blend, mix): (&B, &M)) -> [f32; 4] {
        let mode = layer.mode;
        let fill = layer.fill.clamp(0.0, 1.0);
        if source[3] <= 0.0 {
            return backdrop;
        }
        if matches!(mode, BlendMode::Dissolve | BlendMode::DancingDissolve) {
            let shown = layer.noise < source[3] * layer.opacity * fill;
            return if shown { [source[0], source[1], source[2], 1.0] } else { backdrop };
        }
        if self.variant == Variant::Photoshop && fill < 1.0 && fill_is_special(mode) {
            return self.straight_special_fill(layer, backdrop, source, (blend, mix));
        }
        // Opacity and Fill multiply first, as PhotoCraft's composite took them (same bits).
        let as_ = source[3] * (layer.opacity * fill);
        if as_ <= 0.0 {
            return backdrop;
        }
        let ab = backdrop[3];
        let (cb, cs) = (rgb(backdrop), rgb(source));
        if M::IDENTITY && mode == BlendMode::Normal {
            return normal_rgb(backdrop, source, as_, as_ + ab * (1.0 - as_));
        }
        let b = self.blend_in(mode, cb, cs, 1.0, blend);
        let Some(w) = Weights::new(ab, as_) else { return [0.0; 4] };
        if M::IDENTITY {
            let ao = w.alpha;
            let c = |i: usize| ((1.0 - as_) * ab * cb[i] + (1.0 - ab) * as_ * cs[i] + as_ * ab * b[i]) / ao;
            return [c(0), c(1), c(2), ao];
        }
        if ab <= 0.0 {
            // Nothing to mix with: the source's colour.
            return [cs[0], cs[1], cs[2], w.alpha];
        }
        let (eb, es, ebs) = (mix.encode(cb), mix.encode(cs), mix.encode(b));
        let o = mix.decode(std::array::from_fn(|i| w.b * eb[i] + w.s * es[i] + w.bs * ebs[i]));
        [o[0], o[1], o[2], w.alpha]
    }

    /// The special eight below 100% Fill in Photoshop: coverage over the backdrop is the
    /// opacity and the blend is [`Blend::rgb_fill`]; over transparency Fill stays coverage, so a
    /// layer over nothing looks as it did.
    fn straight_special_fill<B: Space, M: Space>(&self, layer: &Layer, backdrop: [f32; 4], source: [f32; 4], (blend, mix): (&B, &M)) -> [f32; 4] {
        let fill = layer.fill.clamp(0.0, 1.0);
        let ab = backdrop[3];
        let over = source[3] * layer.opacity;
        let alone = over * fill;
        if over <= 0.0 {
            return backdrop;
        }
        let ao = ab + (1.0 - ab) * alone;
        if ao <= 0.0 {
            return [0.0; 4];
        }
        let (cb, cs) = (rgb(backdrop), rgb(source));
        let b = self.blend_in(layer.mode, cb, cs, fill, blend);
        let (wb, wbs, ws) = (ab * (1.0 - over) / ao, ab * over / ao, (1.0 - ab) * alone / ao);
        let o = if M::IDENTITY {
            std::array::from_fn(|i| wb * cb[i] + wbs * b[i] + ws * cs[i])
        } else {
            let (eb, ebs, es) = (mix.encode(cb), mix.encode(b), mix.encode(cs));
            mix.decode(std::array::from_fn(|i| wb * eb[i] + wbs * ebs[i] + ws * es[i]))
        };
        [o[0], o[1], o[2], ao]
    }

    /// Premultiplied source over premultiplied backdrop. After Effects' Stencil, Silhouette,
    /// Alpha Add and Luminescent Premul act on the backdrop; its Classic modes fade the source
    /// toward their neutral colour by coverage; its other modes clamp inputs outside 0..1 unless
    /// After Effects keeps them over range.
    fn premultiplied<B: Space, M: Space>(&self, layer: &Layer, dst: [f32; 4], src: [f32; 4], (blend, mix): (&B, &M)) -> [f32; 4] {
        let k = layer.opacity * layer.fill.clamp(0.0, 1.0);
        let src = if k == 1.0 { src } else { src.map(|v| v * k) };
        let (sa, da) = (src[3], dst[3]);
        let mode = layer.mode;
        if let Some(out) = alpha_only(mode, dst, src, layer.noise) {
            return out;
        }
        if sa <= 0.0 {
            return dst;
        }
        if da <= 0.0 {
            return src;
        }
        let mut cs = [src[0] / sa, src[1] / sa, src[2] / sa];
        let mut cb = [dst[0] / da, dst[1] / da, dst[2] / da];
        if self.variant == Variant::AfterEffects && !mode.after_effects_keeps_over_range() {
            cs = cs.map(|v| v.clamp(0.0, 1.0));
            cb = cb.map(|v| v.clamp(0.0, 1.0));
        }
        let ao = sa + da - sa * da;
        let o: [f32; 3] = if let Some(neutral) = mode.classic_neutral() {
            // Fade the source toward the neutral colour by its coverage, then apply the formula.
            let b = self.blend_in(mode, cb, cs.map(|v| neutral + sa * (v - neutral)), 1.0, blend);
            std::array::from_fn(|i| (1.0 - da) * src[i] + da * b[i])
        } else if M::IDENTITY {
            let b = self.blend_in(mode, cb, cs, 1.0, blend);
            std::array::from_fn(|i| (1.0 - da) * src[i] + (1.0 - sa) * dst[i] + sa * da * b[i])
        } else {
            let b = self.blend_in(mode, cb, cs, 1.0, blend);
            let (eb, es, ebs) = (mix.encode(cb), mix.encode(cs), mix.encode(b));
            let w = [(1.0 - sa) * da / ao, (1.0 - da) * sa / ao, sa * da / ao];
            mix.decode(std::array::from_fn(|i| w[0] * eb[i] + w[1] * es[i] + w[2] * ebs[i])).map(|v| v * ao)
        };
        [o[0], o[1], o[2], ao]
    }
}

/// The premultiplied modes that don't need `B(Cb, Cs)`: Normal's "over", Dissolve and After
/// Effects' alpha modes. `None` for the rest.
fn alpha_only(mode: BlendMode, dst: [f32; 4], src: [f32; 4], noise: f32) -> Option<[f32; 4]> {
    let (sa, da) = (src[3], dst[3]);
    let over = |alpha: f32| [src[0] + dst[0] * (1.0 - sa), src[1] + dst[1] * (1.0 - sa), src[2] + dst[2] * (1.0 - sa), alpha];
    Some(match mode {
        BlendMode::Normal | BlendMode::PassThrough => over(sa + da * (1.0 - sa)),
        BlendMode::Dissolve | BlendMode::DancingDissolve => {
            if sa <= 0.0 || noise >= sa {
                dst
            } else {
                let inv = 1.0 / sa;
                [src[0] * inv, src[1] * inv, src[2] * inv, 1.0]
            }
        }
        BlendMode::StencilAlpha => dst.map(|v| v * sa),
        BlendMode::SilhouetteAlpha => dst.map(|v| v * (1.0 - sa)),
        BlendMode::StencilLuma => dst.map(|v| v * craft_wgsl::luminance(rgb(src))),
        BlendMode::SilhouetteLuma => dst.map(|v| v * (1.0 - craft_wgsl::luminance(rgb(src)))),
        BlendMode::AlphaAdd => over((sa + da).min(1.0)),
        BlendMode::LuminescentPremul => over(sa + da - sa * da),
        _ => return None,
    })
}

fn rgb(p: [f32; 4]) -> [f32; 3] {
    [p[0], p[1], p[2]]
}

/// Normal outside any mix space: `B(Cb, Cs) = Cs`, in the operation order the span kernel
/// vectorises.
#[inline]
fn normal_rgb(backdrop: [f32; 4], source: [f32; 4], as_: f32, ao: f32) -> [f32; 4] {
    if ao <= 0.0 {
        return [0.0; 4];
    }
    let kb = backdrop[3] * (1.0 - as_);
    let inv = 1.0 / ao;
    [(kb * backdrop[0] + as_ * source[0]) * inv, (kb * backdrop[1] + as_ * source[1]) * inv, (kb * backdrop[2] + as_ * source[2]) * inv, ao]
}

/// Straight Normal at `opacity` over a row, with the bits of the per-pixel composite, in blocks
/// the compiler vectorises; the tail that doesn't fill a block goes through the scalar kernel.
pub fn composite_normal_span(dst: &mut [[f32; 4]], src: &[[f32; 4]], opacity: f32) {
    let n = dst.len().min(src.len());
    let (dc, dt) = dst[..n].as_chunks_mut::<LANES>();
    let (sc, st) = src[..n].as_chunks::<LANES>();
    for (b, s) in dc.iter_mut().zip(sc) {
        normal_lanes(b, s, opacity);
    }
    for (b, s) in dt.iter_mut().zip(st) {
        // Not `s[3] > 0`: a NaN alpha is blended (into NaN), as the scalar loop does.
        if s[3] <= 0.0 {
            continue;
        }
        let as_ = s[3] * opacity;
        if as_ <= 0.0 {
            continue;
        }
        *b = normal_rgb(*b, *s, as_, as_ + b[3] * (1.0 - as_));
    }
}

/// Pixels per block of [`normal_lanes`]: two 128-bit vectors (or one 256-bit) per channel.
const LANES: usize = 8;

/// [`normal_rgb`] (after the caller's `source alpha > 0` test) on one block of pixels, without
/// branches: one array per channel, fixed-length loops the compiler vectorises, and a select per
/// pixel between the backdrop, zero and the mix. Same operations in the same order (Rust never
/// fuses them into multiply-adds) and the same tests: the same bits, except that a NaN's sign
/// and payload may differ (IEEE 754 doesn't fix which operand's NaN is passed on).
#[inline]
#[allow(clippy::needless_range_loop)] // lanes index several arrays in lockstep
fn normal_lanes(b: &mut [[f32; 4]; LANES], s: &[[f32; 4]; LANES], opacity: f32) {
    let lane = |p: &[[f32; 4]; LANES], c: usize| -> [f32; LANES] { std::array::from_fn(|i| p[i][c]) };
    let (b0, b1, b2, ab) = (lane(b, 0), lane(b, 1), lane(b, 2), lane(b, 3));
    let (s0, s1, s2, s3) = (lane(s, 0), lane(s, 1), lane(s, 2), lane(s, 3));
    let mut as_ = [0.0f32; LANES];
    let mut ao = [0.0f32; LANES];
    let mut kb = [0.0f32; LANES];
    let mut inv = [0.0f32; LANES];
    for i in 0..LANES {
        as_[i] = s3[i] * opacity;
        ao[i] = as_[i] + ab[i] * (1.0 - as_[i]);
        kb[i] = ab[i] * (1.0 - as_[i]);
        inv[i] = 1.0 / ao[i];
    }
    for i in 0..LANES {
        let mix =
            [(kb[i] * b0[i] + as_[i] * s0[i]) * inv[i], (kb[i] * b1[i] + as_[i] * s1[i]) * inv[i], (kb[i] * b2[i] + as_[i] * s2[i]) * inv[i], ao[i]];
        // The scalar path's tests: skipped where the source is empty (`s[3] <= 0`) or adds
        // nothing (`as_ <= 0`), zero where nothing is left (`ao <= 0`).
        let keep = (s3[i] <= 0.0) | (as_[i] <= 0.0);
        let gone = ao[i] <= 0.0;
        b[i] = if keep {
            b[i]
        } else if gone {
            [0.0; 4]
        } else {
            mix
        };
    }
}
