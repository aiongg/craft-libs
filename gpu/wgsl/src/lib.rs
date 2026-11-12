#![forbid(unsafe_code)]

//! The shared WGSL prelude and the one way shader sources are put together.
//!
//! Shaders across the framework need the same small functions: the sRGB curve, luma, HSL, the
//! PQ and HLG transfers, and numeric guards that make every backend give the CPU's f32 result
//! (`round_away`, `unit_div`, `asin_p`, `sincos_p`). They live here once, as [`Module`]s, each
//! WGSL function beside a Rust twin of the same name that runs the same operations in the same
//! order, so CPU and GPU paths agree to the bit or to float rounding. The GPU test target checks
//! each pair.
//!
//! [`assemble`] joins modules into one shader source: every module once, its dependencies first.
//! Module-scope WGSL declarations may be used before they are declared, so the order only has
//! to keep the text readable.
//!
//! Nothing here reads a binding. Functions that need a crate's bind group (sampling the fx
//! kernels' textures, the fx `data` buffer) live with that crate.

/// A piece of WGSL and the pieces it calls.
#[derive(Debug)]
pub struct Module {
    /// Unique name, for diagnostics and to include each module once.
    pub name: &'static str,
    /// The WGSL text.
    pub source: &'static str,
    /// Modules whose functions or constants `source` uses.
    pub deps: &'static [&'static Module],
}

/// `modules` and everything they depend on, each once and dependencies first, followed by
/// `extra` (the caller's own entry points and bindings).
pub fn assemble(modules: &[&'static Module], extra: &[&str]) -> String {
    fn visit(m: &'static Module, seen: &mut Vec<&'static str>, out: &mut String) {
        if seen.contains(&m.name) {
            return;
        }
        seen.push(m.name);
        for d in m.deps {
            visit(d, seen, out);
        }
        out.push_str("// ---- ");
        out.push_str(m.name);
        out.push('\n');
        push_line(out, m.source);
    }
    let mut seen = Vec::new();
    let mut out = String::new();
    for m in modules {
        visit(m, &mut seen, &mut out);
    }
    for e in extra {
        push_line(&mut out, e);
    }
    out
}

fn push_line(out: &mut String, text: &str) {
    out.push_str(text);
    if !text.ends_with('\n') {
        out.push('\n');
    }
}

/// Numeric guards: `imod`, `asin_p`, `acos_p`, `sincos_p`, `sin_p`, `cos_p`, `unit_div`,
/// `hash_noise`, `powz`, `round_away`, `fract_euclid`.
pub static MATH: Module = Module { name: "craft-wgsl/math", source: include_str!("shaders/math.wgsl"), deps: &[] };

/// `luminance`, `srgb_to_linear(3)`, `linear_to_srgb(3)`, `rgb_to_hsl`, `hsl_channel`, `hsl_to_rgb`.
pub static COLOR: Module = Module { name: "craft-wgsl/color", source: include_str!("shaders/color.wgsl"), deps: &[&MATH] };

/// PQ and HLG constants, `pq_decode`, `pq_encode`, `hlg_decode`, `hlg_encode`, `curve_decode`,
/// `curve_encode`.
pub static TRANSFER: Module = Module { name: "craft-wgsl/transfer", source: include_str!("shaders/transfer.wgsl"), deps: &[&COLOR, &MATH] };

/// sRGB decode: encoded → linear. Twin of the WGSL `srgb_to_linear`.
pub fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

/// sRGB encode: linear → encoded. Twin of the WGSL `linear_to_srgb`.
pub fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.max(0.0).powf(1.0 / 2.4) - 0.055 }
}

/// Rec. 709 luma. Twin of the WGSL `luminance`.
pub fn luminance(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// `x − floor(x)`, the WGSL `fract_euclid` (`f32::rem_euclid(1.0)` for ordinary values).
pub fn fract_euclid(x: f32) -> f32 {
    x - x.floor()
}

/// RGB → (h, s, l), h in 0..1. Twin of the WGSL `rgb_to_hsl`.
pub fn rgb_to_hsl(c: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = c;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    if (max - min).abs() < 1e-7 {
        return [0.0, 0.0, l];
    }
    let d = max - min;
    let denom = if l > 0.5 { 2.0 - max - min } else { max + min };
    // Over-range (HDR) or negative inputs can make the denominator zero or negative.
    let s = if denom > 0.0 { d / denom } else { 1.0 };
    let h = if max == r {
        fract_euclid(((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0)
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    [h, s, l]
}

fn hsl_channel(t0: f32, p: f32, q: f32) -> f32 {
    let t = fract_euclid(t0);
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

/// (h, s, l) → RGB. Twin of the WGSL `hsl_to_rgb`.
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    if s <= 0.0 {
        return [l; 3];
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    [hsl_channel(h + 1.0 / 3.0, p, q), hsl_channel(h, p, q), hsl_channel(h - 1.0 / 3.0, p, q)]
}

/// A per-pixel hash in 0..1 (24 bits). Twin of the WGSL `hash_noise`.
pub fn hash_noise(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 16_777_216.0
}

/// `x^y` for `x >= 0` with `0^y = 0` for `y > 0` and `0^0 = 1`. Twin of the WGSL `powz`.
pub fn powz(x: f32, y: f32) -> f32 {
    if x <= 0.0 {
        return if y == 0.0 { 1.0 } else { 0.0 };
    }
    x.powf(y)
}

const PQ_M1: f32 = 2610.0 / 16384.0;
const PQ_M2: f32 = 2523.0 / 4096.0 * 128.0;
const PQ_C1: f32 = 3424.0 / 4096.0;
const PQ_C2: f32 = 2413.0 / 4096.0 * 32.0;
const PQ_C3: f32 = 2392.0 / 4096.0 * 32.0;
/// Linear 1.0 as a fraction of PQ's 10 000 cd/m²: the BT.2408 reference white of 203 cd/m².
const PQ_WHITE: f32 = 0.0203;
const HLG_A: f32 = 0.178_832_77;
const HLG_B: f32 = 0.284_668_92;
const HLG_C: f32 = 0.559_910_7;
/// Scene light of linear 1.0: the BT.2408 reference white at a 75% HLG signal.
const HLG_REF: f32 = 0.264_962_55;

/// SMPTE ST 2084 signal → linear (1.0 = reference white). Twin of the WGSL `pq_decode`.
pub fn pq_decode(e: f32) -> f32 {
    let p = e.clamp(0.0, 1.0).powf(1.0 / PQ_M2);
    ((p - PQ_C1).max(0.0) / (PQ_C2 - PQ_C3 * p)).powf(1.0 / PQ_M1) / PQ_WHITE
}

/// Linear (1.0 = reference white) → SMPTE ST 2084 signal. Twin of the WGSL `pq_encode`.
pub fn pq_encode(l: f32) -> f32 {
    let y = (l * PQ_WHITE).max(0.0).powf(PQ_M1);
    ((PQ_C1 + PQ_C2 * y) / (1.0 + PQ_C3 * y)).powf(PQ_M2)
}

/// BT.2100 HLG signal → linear (1.0 = reference white). Twin of the WGSL `hlg_decode`.
pub fn hlg_decode(v: f32) -> f32 {
    if v <= 0.5 { v * v / 3.0 / HLG_REF } else { (((v - HLG_C) / HLG_A).exp() + HLG_B) / 12.0 / HLG_REF }
}

/// Linear (1.0 = reference white) → BT.2100 HLG signal. Twin of the WGSL `hlg_encode`.
pub fn hlg_encode(l: f32) -> f32 {
    let e = (l * HLG_REF).max(0.0);
    if e <= 1.0 / 12.0 { (3.0 * e).sqrt() } else { HLG_A * (12.0 * e - HLG_B).ln() + HLG_C }
}

/// A transfer curve; the discriminant is the WGSL `curve` argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Curve {
    /// No curve.
    Linear = 0,
    /// IEC 61966-2-1 (sRGB, Display P3).
    Srgb = 1,
    /// A 2.4 power (Rec. 709 and Rec. 2020 displays).
    Gamma24 = 2,
    /// SMPTE ST 2084.
    Pq = 3,
    /// BT.2100 HLG.
    Hlg = 4,
}

/// Encoded → linear through `curve`, odd around 0. Twin of the WGSL `curve_decode`.
pub fn curve_decode(curve: Curve, v: f32) -> f32 {
    let a = v.abs();
    let l = match curve {
        Curve::Linear => a,
        Curve::Srgb => srgb_to_linear(a),
        Curve::Gamma24 => powz(a, 2.4),
        Curve::Pq => pq_decode(a),
        Curve::Hlg => hlg_decode(a),
    };
    if v < 0.0 { -l } else { l }
}

/// Linear → encoded through `curve`, odd around 0. Twin of the WGSL `curve_encode`.
pub fn curve_encode(curve: Curve, v: f32) -> f32 {
    let a = v.abs();
    let e = match curve {
        Curve::Linear => a,
        Curve::Srgb => linear_to_srgb(a),
        Curve::Gamma24 => powz(a, 1.0 / 2.4),
        Curve::Pq => pq_encode(a),
        Curve::Hlg => hlg_encode(a),
    };
    if v < 0.0 { -e } else { e }
}

#[cfg(test)]
mod tests;
