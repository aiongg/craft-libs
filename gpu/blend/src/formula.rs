//! The blend function B(Cb, Cs) of every mode, per [`Variant`] and [`Range`].
//!
//! Formulas follow the public PDF / ISO 32000-2 and W3C Compositing definitions where they
//! define a mode, with each app's variant where it differs. A mode shared by several apps has one
//! arithmetic; a variant branch appears only where the results differ beyond float rounding. The
//! WGSL in `shaders/blend.wgsl` mirrors this file function for function.

use crate::mode::{BlendMode, Range, Variant};

/// Backdrop values within `EDGE` of 0 / 1 count as exact in Photoshop's modes whose result jumps
/// there (Color Burn / Dodge, Hard Mix): a composited value that should be 1 can come out a
/// rounding step below it (and differently on the GPU), flipping the corner case by up to 255
/// levels.
pub const EDGE: f32 = 1e-4;

/// The formulas of one app at one value range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Blend {
    /// Whose formulas.
    pub variant: Variant,
    /// The values the document holds.
    pub range: Range,
}

impl Blend {
    /// The formulas of `variant` at `range`.
    pub const fn new(variant: Variant, range: Range) -> Blend {
        Blend { variant, range }
    }

    /// The separable blend `B(cb, cs)` of one channel (straight values). Non-separable modes
    /// give `cs`; use [`Blend::rgb`] for them.
    pub fn channel(&self, mode: BlendMode, cb: f32, cs: f32) -> f32 {
        let hi = self.range.top();
        match self.variant {
            Variant::Photoshop => photoshop(mode, cb, cs, self.range),
            Variant::AfterEffects => after_effects(mode, cb, cs),
            Variant::Standard => standard(mode, cb, cs, hi),
            Variant::Substance => substance(mode, cb, cs),
        }
    }

    /// `B(Cb, Cs)` of a whole straight RGB colour, non-separable modes included.
    pub fn rgb(&self, mode: BlendMode, cb: [f32; 3], cs: [f32; 3]) -> [f32; 3] {
        let top = self.range.top();
        let out = match mode {
            BlendMode::Hue => set_lum_within(set_sat(cs, sat(cb)), lum(cb), top),
            BlendMode::Saturation => set_lum_within(set_sat(cb, sat(cs)), lum(cb), top),
            BlendMode::Color => set_lum_within(cs, lum(cb), top),
            BlendMode::Luminosity => set_lum_within(cb, lum(cs), top),
            BlendMode::DarkerColor => {
                if self.luma(cs) < self.luma(cb) {
                    cs
                } else {
                    cb
                }
            }
            BlendMode::LighterColor => {
                if self.luma(cs) > self.luma(cb) {
                    cs
                } else {
                    cb
                }
            }
            BlendMode::Tint => {
                let l = lum(cb);
                cs.map(|v| v * l)
            }
            BlendMode::Value => value(cb, cs),
            BlendMode::NormalMapCombine => {
                let (nb, ns) = (unpack(cb), unpack(cs));
                pack([nb[0] + ns[0], nb[1] + ns[1], nb[2] * ns[2]])
            }
            BlendMode::NormalMapDetail => rnm(cb, cs),
            BlendMode::NormalMapInverseDetail => rnm(cs, cb),
            m => return std::array::from_fn(|i| self.channel(m, cb[i], cs[i])),
        };
        let wide = self.variant == Variant::Photoshop && self.range == Range::Float;
        if wide && !out.iter().all(|v| v.is_finite()) && cb.iter().chain(&cs).all(|v| v.is_finite()) {
            return nonseparable_wide(mode, cb, cs);
        }
        out
    }

    /// The luma Darker and Lighter Color compare: After Effects' Rec. 709, the W3C weights
    /// elsewhere.
    fn luma(&self, c: [f32; 3]) -> f32 {
        if self.variant == Variant::AfterEffects { craft_wgsl::luminance(c) } else { lum(c) }
    }
}

/// Photoshop: Color Burn / Dodge and Hard Mix treat near-extreme backdrops as exact, Vivid Light
/// lets the source's extremes win; Add and Divide clip at the range's top. 32-bit documents
/// recompute overflowing results in f64.
fn photoshop(mode: BlendMode, cb: f32, cs: f32, range: Range) -> f32 {
    match mode {
        BlendMode::ColorBurn => color_burn_edge(cb, cs),
        BlendMode::ColorDodge => color_dodge_edge(cb, cs),
        BlendMode::VividLight => vivid_light_source_wins(cb, cs),
        BlendMode::HardMix => hard_mix_edge(cb, cs),
        _ if range == Range::Float => {
            let out = shared(mode, cb, cs, f32::MAX);
            if !out.is_finite() && cb.is_finite() && cs.is_finite() { channel_wide(mode, cb, cs) } else { out }
        }
        _ => shared(mode, cb, cs, 1.0),
    }
}

/// After Effects: the backdrop's extremes win in Vivid Light, Hard Mix is `cb + cs ≥ 1`, and
/// Add, Linear Dodge and Divide never clip.
fn after_effects(mode: BlendMode, cb: f32, cs: f32) -> f32 {
    match mode {
        BlendMode::VividLight => vivid_light(cb, cs),
        BlendMode::HardMix => step(cb + cs >= 1.0),
        BlendMode::Add | BlendMode::LinearDodge => cb + cs,
        BlendMode::Divide => divide_unclipped(cb, cs),
        _ => shared(mode, cb, cs, 1.0),
    }
}

/// Premiere and the W3C: the backdrop's extremes win in Vivid Light and Hard Mix thresholds it;
/// Divide by zero is white.
fn standard(mode: BlendMode, cb: f32, cs: f32, hi: f32) -> f32 {
    match mode {
        BlendMode::VividLight => vivid_light(cb, cs),
        BlendMode::HardMix => step(vivid_light(cb, cs) >= 0.5),
        BlendMode::Divide if cs <= 0.0 => 1.0,
        _ => shared(mode, cb, cs, hi),
    }
}

/// Substance: signed results without clipping; Soft Light on clamped inputs.
fn substance(mode: BlendMode, cb: f32, cs: f32) -> f32 {
    match mode {
        BlendMode::Divide => cb / cs.max(1e-6),
        BlendMode::LinearDodge => cb + cs,
        BlendMode::Subtract => cb - cs,
        BlendMode::LinearBurn => cb + cs - 1.0,
        BlendMode::LinearLight => cb + 2.0 * cs - 1.0,
        BlendMode::SoftLight => soft_light(cb.clamp(0.0, 1.0), cs.clamp(0.0, 1.0)),
        BlendMode::VividLight => vivid_light(cb, cs),
        _ => shared(mode, cb, cs, 1.0),
    }
}

/// The formulas every variant shares, and the modes only one app has. Linear Dodge and Divide
/// clip at `hi`.
pub(crate) fn shared(mode: BlendMode, cb: f32, cs: f32, hi: f32) -> f32 {
    match mode {
        BlendMode::Darken => cb.min(cs),
        // Saturated so coverage-compositing callers never see an infinity from finite inputs.
        BlendMode::Multiply => (cb * cs).clamp(-f32::MAX, f32::MAX),
        BlendMode::ColorBurn => color_burn(cb, cs),
        BlendMode::LinearBurn => (cb + cs - 1.0).max(0.0),
        BlendMode::Lighten => cb.max(cs),
        BlendMode::Screen => cb + cs - cb * cs,
        BlendMode::ColorDodge => color_dodge(cb, cs),
        BlendMode::LinearDodge | BlendMode::Add => (cb + cs).min(hi),
        BlendMode::Overlay => hard_light(cs, cb),
        BlendMode::SoftLight => soft_light(cb, cs),
        BlendMode::HardLight => hard_light(cb, cs),
        BlendMode::VividLight => vivid_light_source_wins(cb, cs),
        BlendMode::LinearLight => (cb + 2.0 * cs - 1.0).clamp(0.0, 1.0),
        BlendMode::PinLight => pin_light(cb, cs),
        BlendMode::HardMix => hard_mix_edge(cb, cs),
        BlendMode::Difference | BlendMode::ClassicDifference => (cb - cs).abs(),
        BlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
        BlendMode::Subtract => (cb - cs).max(0.0),
        BlendMode::Divide => divide(cb, cs, hi),
        BlendMode::Reflect => reflect(cb, cs),
        BlendMode::Glow => reflect(cs, cb),
        BlendMode::Negation => 1.0 - (1.0 - cb - cs).abs(),
        BlendMode::Xor => xor(cb, cs),
        BlendMode::PaintNetColorBurn => paint_net_burn(cb, cs),
        BlendMode::PaintNetColorDodge => paint_net_dodge(cb, cs),
        BlendMode::ClassicColorBurn => classic_burn(cb, cs),
        BlendMode::ClassicColorDodge => paint_net_dodge(cb, cs),
        BlendMode::Disable => cb,
        BlendMode::InverseDivide => cs / cb.max(1e-6),
        BlendMode::InverseSubtract => cs - cb,
        BlendMode::SignedAddition => cb + 2.0 * cs - 1.0,
        // Normal-like, alpha-only and non-separable modes: the source (compositing and
        // `Blend::rgb` handle them).
        _ => cs,
    }
}

fn step(on: bool) -> f32 {
    if on { 1.0 } else { 0.0 }
}

fn color_burn(cb: f32, cs: f32) -> f32 {
    if cb >= 1.0 {
        1.0
    } else if cs <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - cb) / cs).min(1.0)
    }
}

fn color_dodge(cb: f32, cs: f32) -> f32 {
    if cb <= 0.0 {
        0.0
    } else if cs >= 1.0 {
        1.0
    } else {
        (cb / (1.0 - cs)).min(1.0)
    }
}

fn color_burn_edge(cb: f32, cs: f32) -> f32 {
    if cb >= 1.0 - EDGE {
        1.0
    } else if cs <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - cb) / cs).min(1.0)
    }
}

fn color_dodge_edge(cb: f32, cs: f32) -> f32 {
    if cb <= EDGE {
        0.0
    } else if cs >= 1.0 {
        1.0
    } else {
        (cb / (1.0 - cs)).min(1.0)
    }
}

fn hard_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 {
        cb * 2.0 * cs
    } else {
        let s = 2.0 * cs - 1.0;
        cb + s - cb * s
    }
}

/// The W3C/PDF Soft Light. In the upper half the backdrop moves toward `D(cb)`: `sqrt(cb)` above
/// a quarter and the cubic `((16·cb − 12)·cb + 4)·cb` below it. With `sqrt` throughout, dark
/// backdrops came out up to 20 levels too light in Photoshop documents (#8804, psd-tools
/// `blend-modes/rgb-blend-modes.psd`).
fn soft_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 {
        2.0 * cb * cs + cb * cb * (1.0 - 2.0 * cs)
    } else {
        let b = cb.max(0.0);
        let d = if b <= 0.25 { ((16.0 * b - 12.0) * b + 4.0) * b } else { b.sqrt() };
        2.0 * cb * (1.0 - cs) + d * (2.0 * cs - 1.0)
    }
}

/// Vivid Light where the backdrop's extremes win (Color Burn / Dodge of the doubled source).
fn vivid_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 { color_burn(cb, 2.0 * cs) } else { color_dodge(cb, 2.0 * (cs - 0.5)) }
}

/// Photoshop's Vivid Light: the source's extremes win, `cs = 0` gives 0 even over white and
/// `cs = 1` gives 1 even over black.
fn vivid_light_source_wins(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 {
        if cs <= 0.0 { 0.0 } else { 1.0 - ((1.0 - cb) / (2.0 * cs)).min(1.0) }
    } else if cs >= 1.0 {
        1.0
    } else {
        (cb / (2.0 * (1.0 - cs))).min(1.0)
    }
}

/// Photoshop's Hard Mix: the thresholded Vivid Light where the backdrop's extremes win (within
/// [`EDGE`]). For interior values this is `cb + cs ≥ 1`; black over white gives white and white
/// over black gives black, as Photoshop shows.
fn hard_mix_edge(cb: f32, cs: f32) -> f32 {
    let v = if cs <= 0.5 {
        if cb >= 1.0 - EDGE {
            1.0
        } else if cs <= 0.0 {
            0.0
        } else {
            1.0 - ((1.0 - cb) / (2.0 * cs)).min(1.0)
        }
    } else if cb <= EDGE {
        0.0
    } else if cs >= 1.0 {
        1.0
    } else {
        (cb / (2.0 * (1.0 - cs))).min(1.0)
    };
    step(v >= 0.5 - 1e-6)
}

fn pin_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 { cb.min(2.0 * cs) } else { cb.max(2.0 * cs - 1.0) }
}

/// Divide clipped at `hi`; by zero, black stays black and everything else is white.
fn divide(cb: f32, cs: f32, hi: f32) -> f32 {
    if cs <= 0.0 { step(cb > 0.0) } else { (cb / cs).min(hi) }
}

fn divide_unclipped(cb: f32, cs: f32) -> f32 {
    if cs <= 0.0 { step(cb > 0.0) } else { cb / cs }
}

fn reflect(cb: f32, cs: f32) -> f32 {
    if cs >= 1.0 { 1.0 } else { (cb * cb / (1.0 - cs)).min(1.0) }
}

/// Paint.NET's XOR is defined on 8-bit channels, even in a floating-point compositor.
fn xor(cb: f32, cs: f32) -> f32 {
    let b = (cb.clamp(0.0, 1.0) * 255.0).round() as u8;
    let s = (cs.clamp(0.0, 1.0) * 255.0).round() as u8;
    f32::from(b ^ s) / 255.0
}

fn paint_net_burn(cb: f32, cs: f32) -> f32 {
    if cs <= 0.0 { 0.0 } else { (1.0 - (1.0 - cb) / cs).max(0.0) }
}

/// Paint.NET's Color Dodge, which is also After Effects' Classic Color Dodge.
fn paint_net_dodge(cb: f32, cs: f32) -> f32 {
    if cs >= 1.0 { 1.0 } else { (cb / (1.0 - cs)).min(1.0) }
}

/// After Effects' Classic Color Burn keeps negative results.
fn classic_burn(cb: f32, cs: f32) -> f32 {
    if cs <= 0.0 { 0.0 } else { 1.0 - (1.0 - cb) / cs }
}

/// Re-evaluates an overflowing separable blend of a 32-bit document in f64, then bounds the
/// signed f32 result. Ordinary finite results never come here, so they keep their f32 bits.
#[cold]
fn channel_wide(mode: BlendMode, cb: f32, cs: f32) -> f32 {
    let (b, s) = (f64::from(cb), f64::from(cs));
    let hard_light = |b: f64, s: f64| {
        if s <= 0.5 {
            b * 2.0 * s
        } else {
            let s = 2.0 * s - 1.0;
            b + (1.0 - b) * s
        }
    };
    let out = match mode {
        BlendMode::Screen => b + s - b * s,
        BlendMode::LinearBurn => (b + s - 1.0).max(0.0),
        BlendMode::LinearDodge | BlendMode::Add => b + s,
        BlendMode::Overlay => hard_light(s, b),
        BlendMode::HardLight => hard_light(b, s),
        BlendMode::SoftLight => soft_light_wide(b, s),
        BlendMode::PinLight => {
            if s <= 0.5 {
                b.min(2.0 * s)
            } else {
                b.max(2.0 * s - 1.0)
            }
        }
        BlendMode::Difference => (b - s).abs(),
        BlendMode::Exclusion => b * (1.0 - 2.0 * s) + s,
        BlendMode::Subtract => (b - s).max(0.0),
        BlendMode::Divide => {
            if s <= 0.0 {
                if b <= 0.0 { 0.0 } else { 1.0 }
            } else {
                b / s
            }
        }
        BlendMode::Negation => 1.0 - (1.0 - b - s).abs(),
        BlendMode::PaintNetColorBurn => {
            if s <= 0.0 {
                0.0
            } else {
                (1.0 - (1.0 - b) / s).max(0.0)
            }
        }
        BlendMode::PaintNetColorDodge => {
            if s >= 1.0 {
                1.0
            } else {
                (b / (1.0 - s)).min(1.0)
            }
        }
        // The other formulas already give finite output for finite inputs.
        _ => f64::from(shared(mode, cb, cs, f32::MAX)),
    };
    out.clamp(-f64::from(f32::MAX), f64::from(f32::MAX)) as f32
}

fn soft_light_wide(b: f64, s: f64) -> f64 {
    if s <= 0.5 {
        b + (1.0 - 2.0 * s) * (b * b - b)
    } else {
        let backdrop = b.max(0.0);
        let d = if backdrop <= 0.25 { ((16.0 * backdrop - 12.0) * backdrop + 4.0) * backdrop } else { backdrop.sqrt() };
        if d == b { b } else { 2.0 * b * (1.0 - s) + d * (2.0 * s - 1.0) }
    }
}

// ---- non-separable helpers (W3C Compositing Level 1) ----

/// The W3C luminosity of the non-separable modes: `0.3·R + 0.59·G + 0.11·B`.
pub fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

/// ClipColor: moves `c` toward its grey ([`lum`]) until every channel is within `0..=top`,
/// keeping the luminosity. `top` is 1, or `f32::MAX` where values above 1 are light (only
/// negatives are pulled in).
///
/// The scale factors `l / (l − min)` and `(top − l) / (max − l)` lie in 0..1 only while the
/// luminosity itself is in range. Outside it no colour of that luminosity is in gamut: the plain
/// formula turned colours inside out (a grey of 2 as the Luminosity of (0.5, 0.25, 0.125) gave
/// (1.0, 2.32, 2.99)) and, for greys, divided by the rounding noise of `max − l`, so results were
/// arbitrary and differed between the CPU and the GPU. Such colours become their grey instead
/// (the formula's value as the luminosity reaches the bound).
fn clip_color(c: [f32; 3], top: f32) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 {
        let d = l - n;
        for v in &mut out {
            *v = if l > 0.0 && d > 0.0 { l + (*v - l) * l / d } else { l };
        }
    }
    if x > top {
        let d = x - l;
        for v in &mut out {
            *v = if l < top && d > 0.0 { l + (*v - l) * (top - l) / d } else { l };
        }
    }
    out
}

/// `c` with its luminosity ([`lum`]) moved to `l`, clipped back into 0..1 toward grey (the
/// Luminosity mode's SetLum + ClipColor).
pub fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    set_lum_within(c, l, 1.0)
}

pub(crate) fn set_lum_within(c: [f32; 3], l: f32, top: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d], top)
}

fn sat(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

/// SetSat as `(c − min)·s / (max − min)`: the same operations as the shader.
fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let mx = c[0].max(c[1]).max(c[2]);
    let mn = c[0].min(c[1]).min(c[2]);
    if mx > mn { c.map(|v| (v - mn) * s / (mx - mn)) } else { [0.0; 3] }
}

/// Substance's Value: the backdrop scaled so its HSV value is the source's.
fn value(cb: [f32; 3], cs: [f32; 3]) -> [f32; 3] {
    let vb = cb[0].max(cb[1]).max(cb[2]);
    let vs = cs[0].max(cs[1]).max(cs[2]);
    if vb <= 1e-6 { [vs; 3] } else { cb.map(|v| v * vs / vb) }
}

fn unpack(n: [f32; 3]) -> [f32; 3] {
    n.map(|v| v * 2.0 - 1.0)
}

fn pack(n: [f32; 3]) -> [f32; 3] {
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
    n.map(|v| v / len * 0.5 + 0.5)
}

/// Reoriented normal mapping (Barré-Brisebois & Hill): `detail` reoriented onto `base`, both and
/// the result packed into 0..1.
fn rnm(base: [f32; 3], detail: [f32; 3]) -> [f32; 3] {
    let t = unpack(base);
    let t = [t[0], t[1], t[2] + 1.0];
    let u = unpack(detail);
    let u = [-u[0], -u[1], u[2]];
    let d = t[0] * u[0] + t[1] * u[1] + t[2] * u[2];
    pack([t[0] * d - u[0] * t[2], t[1] * d - u[1] * t[2], t[2] * d - u[2] * t[2]])
}

/// Replays an overflowing non-separable blend of finite inputs in f64. A shared f32
/// normalisation can erase a tiny source's hue; raw f32 inputs and these products fit in f64.
#[cold]
fn nonseparable_wide(mode: BlendMode, cb: [f32; 3], cs: [f32; 3]) -> [f32; 3] {
    fn lum(c: [f64; 3]) -> f64 {
        0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
    }
    fn sat(c: [f64; 3]) -> f64 {
        c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
    }
    /// Keeps the channel order and puts the smallest channel at zero.
    fn set_sat(c: [f64; 3], s: f64) -> [f64; 3] {
        let mut idx = [0usize, 1, 2];
        idx.sort_by(|&a, &b| c[a].total_cmp(&c[b]));
        let (imin, imid, imax) = (idx[0], idx[1], idx[2]);
        let mut out = [0.; 3];
        if c[imax] > c[imin] {
            out[imid] = (c[imid] - c[imin]) * s / (c[imax] - c[imin]);
            out[imax] = s;
        }
        out
    }
    /// SetLum and ClipColor against the f32 maximum.
    fn set_lum(c: [f64; 3], target: f64) -> [f64; 3] {
        let d = target - lum(c);
        let c = c.map(|v| v + d);
        let l = lum(c);
        let n = c[0].min(c[1]).min(c[2]);
        let x = c[0].max(c[1]).max(c[2]);
        let top = f64::from(f32::MAX);
        let mut out = c;
        if n < 0. {
            let d = l - n;
            for v in &mut out {
                *v = if l > 0. && d > 0. { l + (*v - l) * l / d } else { l };
            }
        }
        if x > top {
            let d = x - l;
            for v in &mut out {
                *v = if l < top && d > 0. { l + (*v - l) * (top - l) / d } else { l };
            }
        }
        out
    }
    let (cb, cs) = (cb.map(f64::from), cs.map(f64::from));
    let out = match mode {
        BlendMode::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
        BlendMode::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
        BlendMode::Color => set_lum(cs, lum(cb)),
        BlendMode::Luminosity => set_lum(cb, lum(cs)),
        _ => cs,
    };
    out.map(|v| v.clamp(-f64::from(f32::MAX), f64::from(f32::MAX)) as f32)
}
