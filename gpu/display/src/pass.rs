//! The display pass as data: what turns a rendered frame or a canvas texture into the pixels a
//! view shows. The GPU runs it in WGSL ([`crate::gpu`]). The same steps on the CPU
//! ([`DisplayPass::pixel`]) are its fallback without a GPU and the reference the shader is tested
//! against, so both use the same `f32` operations in the same order.
//!
//! Steps, per pixel of a premultiplied frame encoded with [`FrameEncoding::curve`]:
//! 1. Unpremultiply (NaN alpha counts as 0, NaN colour as 0).
//! 2. When exposure, an HDR step or a preview gamma applies: decode, multiply by 2^stops in
//!    linear light, run the pending HDR step, raise to 1 / gamma, encode again.
//! 3. Clamp to 0..1.
//! 4. The display colour ([`PassColor`]), on RGB and RGB Straight only.
//! 5. Show Channel, premultiply (without step 2 or a display colour: the frame's own
//!    premultiplied values, clamped to alpha), and round to 8 bits with `floor(v · 255 + 0.5)`.
//!
//! Exposure therefore acts on the full float range of float frames, before tone mapping and
//! before the display conversion. With none of step 2 and no display colour, 8-bit frames pass
//! through unchanged.

use std::sync::Arc;

use craft_cms::{CmsError, GamutCheck, GpuMatrixShaper, Lut3d};
use craft_wgsl::powz;

/// A frame's transfer curve. Decoding and encoding are odd around zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Curve {
    /// Linear light.
    Linear,
    /// IEC 61966-2-1.
    Srgb,
    /// A pure power curve (BT.1886 2.4, Mac 1.8, a working gamma).
    Gamma(f32),
    /// SMPTE ST 2084, linear 1.0 = the reference white.
    Pq,
    /// BT.2100 HLG, linear 1.0 = the reference white.
    Hlg,
}

impl Curve {
    /// The shader's curve id (craft-wgsl's `curve` argument; 2 takes the gamma).
    pub fn id(self) -> u32 {
        match self {
            Curve::Linear => 0,
            Curve::Srgb => 1,
            Curve::Gamma(_) => 2,
            Curve::Pq => 3,
            Curve::Hlg => 4,
        }
    }

    /// The exponent to decode with: the gamma (1 for other curves).
    pub fn decode_param(self) -> f32 {
        if let Curve::Gamma(g) = self { g } else { 1.0 }
    }

    /// The exponent to encode with: 1 / the gamma, divided here so the shader gets the same bits.
    pub fn encode_param(self) -> f32 {
        if let Curve::Gamma(g) = self { 1.0 / g } else { 1.0 }
    }

    fn wgsl(self) -> craft_wgsl::Curve {
        match self {
            Curve::Linear | Curve::Gamma(_) => craft_wgsl::Curve::Linear,
            Curve::Srgb => craft_wgsl::Curve::Srgb,
            Curve::Pq => craft_wgsl::Curve::Pq,
            Curve::Hlg => craft_wgsl::Curve::Hlg,
        }
    }

    /// Encoded → linear light. Twin of the WGSL `pass_decode`.
    pub fn decode(self, v: f32) -> f32 {
        match self {
            Curve::Gamma(g) => {
                let l = powz(v.abs(), g);
                if v < 0.0 { -l } else { l }
            }
            _ => craft_wgsl::curve_decode(self.wgsl(), v),
        }
    }

    /// Linear light → encoded. Twin of the WGSL `pass_encode`.
    pub fn encode(self, v: f32) -> f32 {
        match self {
            Curve::Gamma(_) => {
                let e = powz(v.abs(), self.encode_param());
                if v < 0.0 { -e } else { e }
            }
            _ => craft_wgsl::curve_encode(self.wgsl(), v),
        }
    }
}

/// An output HDR step a frame was rendered without and the pass still runs (After Effects'
/// Project Settings ▸ HDR for SDR output).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HdrStep {
    /// None: values above 1.0 clip.
    #[default]
    None,
    /// Each channel companded above a knee ([`compand`]).
    Compand,
    /// Luminance tone mapped ([`tone_map`]).
    ToneMap,
}

impl HdrStep {
    fn id(self) -> u32 {
        match self {
            HdrStep::None => 0,
            HdrStep::Compand => 1,
            HdrStep::ToneMap => 2,
        }
    }

    /// The step on linear light.
    pub fn apply(self, l: [f32; 3]) -> [f32; 3] {
        match self {
            HdrStep::None => l,
            HdrStep::Compand => l.map(compand),
            HdrStep::ToneMap => tone_map(l),
        }
    }
}

/// Values above the 0.8 knee approach 1.0 exponentially. Twin of the WGSL `pass_compand`.
pub fn compand(v: f32) -> f32 {
    const K: f32 = 0.8;
    if v <= K { v } else { K + (1.0 - K) * (1.0 - (-(v - K) / (1.0 - K)).exp()) }
}

/// Extended Reinhard tone mapping of Rec. 709 luminance with white at 4.0 (linear light). Twin
/// of the WGSL `pass_tone_map`.
pub fn tone_map(c: [f32; 3]) -> [f32; 3] {
    const W: f32 = 4.0;
    let l = craft_wgsl::luminance(c);
    if l <= 0.0 {
        return c;
    }
    let lm = l * (1.0 + l / (W * W)) / (1.0 + l);
    let k = lm / l;
    c.map(|v| v * k)
}

/// How a frame's pixels are encoded: its curve, and the HDR step it was rendered without.
/// Kept with the frame, so a later settings change doesn't reinterpret older frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameEncoding {
    /// The transfer curve of the frame's values.
    pub curve: Curve,
    /// The pending HDR step.
    pub hdr: HdrStep,
}

impl FrameEncoding {
    /// The frame as it would have been rendered with its HDR step: one straight colour.
    pub fn finish(&self, c: [f32; 3]) -> [f32; 3] {
        if self.hdr == HdrStep::None {
            return c;
        }
        self.hdr.apply(c.map(|v| self.curve.decode(v))).map(|v| self.curve.encode(v))
    }
}

/// Show Channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Channel {
    /// The colour, premultiplied over black.
    #[default]
    Rgb,
    /// The red channel as grey (or red, colorized).
    Red,
    /// The green channel as grey (or green, colorized).
    Green,
    /// The blue channel as grey (or blue, colorized).
    Blue,
    /// Alpha as grey.
    Alpha,
    /// The colour without its alpha.
    RgbStraight,
}

impl Channel {
    fn id(self) -> u32 {
        match self {
            Channel::Rgb => 0,
            Channel::Red => 1,
            Channel::Green => 2,
            Channel::Blue => 3,
            Channel::Alpha => 4,
            Channel::RgbStraight => 5,
        }
    }
}

/// A display LUT for the pass: `size`³ RGBA nodes, red fastest (craft-cms [`Lut3d`] order), whose
/// alpha is 1 where the colour is out of the proof gamut and 0 elsewhere.
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayLut {
    size: usize,
    data: Vec<[f32; 4]>,
}

impl DisplayLut {
    /// `lut`'s nodes, flagged out of gamut where `gamut` says so.
    pub fn new(lut: &Lut3d, gamut: Option<&GamutCheck>) -> Result<DisplayLut, CmsError> {
        let n = lut.size();
        let s = (n - 1) as f32;
        let mut data = Vec::with_capacity(lut.data().len());
        for (i, v) in lut.data().iter().enumerate() {
            let rgb = [(i % n) as f32 / s, ((i / n) % n) as f32 / s, (i / (n * n)) as f32 / s];
            let out = match gamut {
                Some(g) => g.out_of_gamut(&rgb)?,
                None => false,
            };
            data.push([v[0], v[1], v[2], if out { 1.0 } else { 0.0 }]);
        }
        Ok(DisplayLut { size: n, data })
    }

    /// Edge length.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The nodes, red fastest.
    pub fn data(&self) -> &[[f32; 4]] {
        &self.data
    }

    /// Trilinear lookup, as the shader samples (the same products and sums, corner by corner).
    pub fn eval(&self, c: [f32; 3]) -> [f32; 4] {
        let n = self.size.max(2);
        let mut k = [0usize; 3];
        let mut f = [0f32; 3];
        for i in 0..3 {
            let x = c[i].clamp(0.0, 1.0) * (n - 1) as f32;
            k[i] = (x.floor() as usize).min(n - 2);
            f[i] = x - k[i] as f32;
        }
        let at = |r: usize, g: usize, b: usize| self.data.get(r + g * n + b * n * n).copied().unwrap_or_default();
        let mut out = [0f32; 4];
        for corner in 0..8 {
            let (dr, dg, db) = (corner >> 2 & 1, corner >> 1 & 1, corner & 1);
            let w = (if dr == 1 { f[0] } else { 1.0 - f[0] }) * (if dg == 1 { f[1] } else { 1.0 - f[1] }) * (if db == 1 { f[2] } else { 1.0 - f[2] });
            let v = at(k[0] + dr, k[1] + dg, k[2] + db);
            for i in 0..4 {
                out[i] += w * v[i];
            }
        }
        out
    }
}

/// The Gamut Warning: out-of-gamut colours mixed toward `color` by `opacity`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GamutOverlay {
    /// The warning colour, as the display shows it (0–1).
    pub color: [f32; 3],
    /// How much of it covers an out-of-gamut colour (0–1).
    pub opacity: f32,
}

/// The display colour conversion of the pass.
#[derive(Clone, Debug)]
pub enum PassColor {
    /// A matrix/shaper conversion; with a simulated output device, the conversion to the device,
    /// its 8-bit numbers (`quantize`) and the conversion from the device to the display.
    Shapers {
        /// The (first) conversion.
        first: Arc<GpuMatrixShaper>,
        /// Round to 8 bits between the two conversions.
        quantize: bool,
        /// The conversion from the simulated device to the display.
        second: Option<Arc<GpuMatrixShaper>>,
    },
    /// Anything else (CLUT profiles, proofing), baked into a LUT; with the Gamut Warning.
    Lut {
        /// The LUT.
        lut: Arc<DisplayLut>,
        /// The warning, when on.
        gamut: Option<GamutOverlay>,
    },
}

/// `mix(a, b, t)` as WGSL defines it.
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

/// One table curve, as the shader's `shaper_curve` samples it: `c` clamped to 0..1 over
/// `intervals` equal steps, each channel between its two neighbouring entries.
fn shaper_curve(table: &[[f32; 4]], c: [f32; 3], intervals: f32) -> [f32; 3] {
    std::array::from_fn(|i| {
        let p = c[i].clamp(0.0, 1.0) * intervals;
        let lo = p.floor() as usize;
        let hi = (lo + 1).min(intervals as usize);
        let f = p - p.floor();
        let at = |j: usize| table.get(j).map_or(0.0, |e| e[i]);
        mix(at(lo), at(hi), f)
    })
}

/// A shaper conversion as the shader's `shaper_apply` runs it: input curves, matrix rows with
/// offsets, output curves sampled at the fourth root.
pub(crate) fn shaper_apply(s: &GpuMatrixShaper, c: [f32; 3]) -> [f32; 3] {
    let l = shaper_curve(s.input_table(), c, GpuMatrixShaper::INPUT_INTERVALS as f32);
    let m = s.matrix_rows();
    let linear: [f32; 3] = std::array::from_fn(|r| m[r][0] * l[0] + m[r][1] * l[1] + m[r][2] * l[2] + m[r][3]);
    let u = linear.map(|v| v.clamp(0.0, 1.0).sqrt().sqrt());
    shaper_curve(s.output_table(), u, (GpuMatrixShaper::OUTPUT_ENTRIES - 1) as f32)
}

/// A conversion given as formulas, as shaper tables: `from` decodes, `matrix` (rows; `None`: the
/// same primaries) converts in linear light, `to` encodes. For colour spaces known by their
/// curves and primaries (a video app's working and output spaces).
pub fn shaper_from_curves(from: Curve, matrix: Option<[[f32; 3]; 3]>, to: Curve) -> Option<GpuMatrixShaper> {
    let n = GpuMatrixShaper::INPUT_INTERVALS;
    let input: Vec<f32> = (0..=n).map(|i| from.decode(i as f32 / n as f32)).collect();
    let m = GpuMatrixShaper::OUTPUT_ENTRIES - 1;
    let output: Vec<f32> = (0..=m)
        .map(|i| {
            let u = i as f32 / m as f32;
            to.encode(u * u * u * u)
        })
        .collect();
    let rows = matrix.unwrap_or([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]).map(|r| [r[0], r[1], r[2], 0.0]);
    GpuMatrixShaper::from_tables(rows, &[input.clone(), input.clone(), input], &[output.clone(), output.clone(), output])
}

/// 8-bit rounding of a 0..1 value.
fn quantize(v: f32) -> f32 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5).floor() / 255.0
}

impl PassColor {
    /// One straight colour (0..1) to the display's values (0..1).
    pub fn apply(&self, c: [f32; 3]) -> [f32; 3] {
        let o = match self {
            PassColor::Lut { lut, gamut } => {
                let l = lut.eval(c);
                let o = [l[0], l[1], l[2]];
                match gamut {
                    Some(g) if g.opacity > 0.0 && l[3] > 0.5 => std::array::from_fn(|i| mix(o[i], g.color[i], g.opacity)),
                    _ => o,
                }
            }
            PassColor::Shapers { first, quantize: q, second } => {
                let mut o = shaper_apply(first, c);
                if *q {
                    o = o.map(quantize);
                }
                match second {
                    Some(s) => shaper_apply(s, o),
                    None => o,
                }
            }
        };
        o.map(|v| v.clamp(0.0, 1.0))
    }
}

/// The whole display pass of one view: Show Channel, exposure, preview gamma, the frame's
/// encoding and the display colour.
#[derive(Clone, Debug)]
pub struct DisplayPass {
    /// Show Channel.
    pub channel: Channel,
    /// A single channel shows in its own colour rather than grey.
    pub colorized: bool,
    /// 2^stops of the view's exposure.
    pub gain: f32,
    /// The preview gamma applied in linear light after exposure and the HDR step (1: none).
    pub gamma: f32,
    /// How the frame is encoded.
    pub encoding: FrameEncoding,
    /// The display colour (colour views only).
    pub color: Option<PassColor>,
}

impl DisplayPass {
    /// `stops` of exposure on frames encoded as `encoding`, with no preview gamma. The display
    /// colour applies to the colour views only.
    pub fn new(channel: Channel, colorized: bool, stops: f32, encoding: FrameEncoding, color: Option<PassColor>) -> DisplayPass {
        let gain = if stops == 0.0 || !stops.is_finite() { 1.0 } else { 2f32.powf(stops) };
        let color = color.filter(|_| matches!(channel, Channel::Rgb | Channel::RgbStraight));
        DisplayPass { channel, colorized, gain, gamma: 1.0, encoding, color }
    }

    /// Step 2 of the module docs runs.
    pub fn changes_light(&self) -> bool {
        self.gain != 1.0 || self.encoding.hdr != HdrStep::None || self.inverse_gamma() != 1.0
    }

    /// The exponent of the preview gamma, as the shader gets it.
    pub(crate) fn inverse_gamma(&self) -> f32 {
        1.0 / self.gamma
    }

    /// Nothing changes the frame's pixels (beyond clamping and rounding).
    pub fn is_identity(&self) -> bool {
        self.channel == Channel::Rgb && !self.changes_light() && self.color.is_none()
    }

    pub(crate) fn channel_id(&self) -> u32 {
        self.channel.id()
    }

    pub(crate) fn hdr_id(&self) -> u32 {
        self.encoding.hdr.id()
    }

    /// Step 2 on straight colour.
    fn light(&self, c: [f32; 3]) -> [f32; 3] {
        if !self.changes_light() {
            return c;
        }
        let curve = self.encoding.curve;
        let mut l = self.encoding.hdr.apply(c.map(|v| curve.decode(v) * self.gain));
        let e = self.inverse_gamma();
        if e != 1.0 {
            l = l.map(|v| powz(v, e));
        }
        l.map(|v| curve.encode(v))
    }

    /// One premultiplied pixel (encoded with the frame's curve) → premultiplied RGBA8 on screen.
    pub fn pixel(&self, p: [f32; 4]) -> [u8; 4] {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8;
        let a = if p[3].is_nan() { 0.0 } else { p[3].clamp(0.0, 1.0) };
        let un = |v: f32| if a > 0.0 && !v.is_nan() { v / a } else { 0.0 };
        let c = self.light([un(p[0]), un(p[1]), un(p[2])]).map(|v| v.clamp(0.0, 1.0));
        // Without step 2 a premultiplied channel comes straight from the frame (clamped to
        // alpha), so no division's rounding changes it.
        let premul = |i: usize| if self.changes_light() || p[i].is_nan() { c[i] * a } else { p[i].clamp(0.0, a) };
        match self.channel {
            Channel::Rgb | Channel::RgbStraight => {
                let c = match &self.color {
                    Some(dc) => dc.apply(c),
                    None => c,
                };
                if self.channel == Channel::RgbStraight {
                    [q(c[0]), q(c[1]), q(c[2]), 255]
                } else if self.color.is_none() {
                    [q(premul(0)), q(premul(1)), q(premul(2)), q(a)]
                } else {
                    [q(c[0] * a), q(c[1] * a), q(c[2] * a), q(a)]
                }
            }
            Channel::Red | Channel::Green | Channel::Blue => {
                let i = match self.channel {
                    Channel::Red => 0,
                    Channel::Green => 1,
                    _ => 2,
                };
                let v = q(premul(i));
                if self.colorized {
                    let mut o = [0, 0, 0, 255];
                    o[i] = v;
                    o
                } else {
                    [v, v, v, 255]
                }
            }
            Channel::Alpha => {
                // Exposure lightens or darkens the alpha view as an encoded grey.
                let curve = self.encoding.curve;
                let v = if self.gain != 1.0 { curve.encode(curve.decode(a) * self.gain) } else { a };
                let v = q(v);
                [v, v, v, 255]
            }
        }
    }

    /// The pass on premultiplied 8-bit pixels in place: the CPU display path. Without a display
    /// colour or HDR step each channel of an opaque pixel goes through the same curve on its own,
    /// so those pixels (and the Alpha view, which reads alpha only) look the result up in a
    /// table of [`DisplayPass::pixel`].
    pub fn apply_rgba8(&self, px: &mut [[u8; 4]]) {
        if self.is_identity() {
            return;
        }
        let f = |v: u8| v as f32 / 255.0;
        if self.channel == Channel::Alpha {
            let t: [[u8; 4]; 256] = std::array::from_fn(|a| self.pixel([0.0, 0.0, 0.0, f(a as u8)]));
            for_each(px, |p| *p = t[p[3] as usize]);
            return;
        }
        let table = (self.color.is_none() && self.encoding.hdr == HdrStep::None)
            .then(|| -> [[u8; 4]; 256] { std::array::from_fn(|v| self.pixel([f(v as u8), f(v as u8), f(v as u8), 1.0])) });
        for_each(px, |p| {
            *p = match &table {
                Some(t) if p[3] == 255 => match self.channel {
                    Channel::Rgb | Channel::RgbStraight => [t[p[0] as usize][0], t[p[1] as usize][1], t[p[2] as usize][2], 255],
                    Channel::Red => t[p[0] as usize],
                    Channel::Green => t[p[1] as usize],
                    _ => t[p[2] as usize],
                },
                _ => self.pixel(p.map(f)),
            }
        });
    }
}

/// `f` on every pixel, in parallel off wasm.
fn for_each(px: &mut [[u8; 4]], f: impl Fn(&mut [u8; 4]) + Sync + Send) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        px.par_iter_mut().for_each(f);
    }
    #[cfg(target_arch = "wasm32")]
    px.iter_mut().for_each(f);
}
