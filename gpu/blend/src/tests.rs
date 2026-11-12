//! Formula and compositing behaviour with real room for error: Photoshop's measured variants,
//! the 32-bit overflow replay, ClipColor out of range, the variants' differences, After
//! Effects' 32 bpc rules and the compositing alpha rules.

use crate::cmyk::{cmyk_planes, planes_cmyk};
use crate::formula::shared;
use crate::{Alpha, Blend, BlendMode, Layer, Linear, Range, Srgb, TEXT_GAMMA, TextGamma, Variant, lum};

const E: f32 = 1e-5;
const MAX: f32 = f32::MAX;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < E
}

const PS: Blend = Blend::new(Variant::Photoshop, Range::Unit);
const PS_FLOAT: Blend = Blend::new(Variant::Photoshop, Range::Float);
const AE: Blend = Blend::new(Variant::AfterEffects, Range::Unit);
const STANDARD: Blend = Blend::new(Variant::Standard, Range::Unit);
const SUBSTANCE: Blend = Blend::new(Variant::Substance, Range::Unit);

const NON_SEPARABLE: [BlendMode; 4] = [BlendMode::Hue, BlendMode::Saturation, BlendMode::Color, BlendMode::Luminosity];

/// PhotoCraft's editable modes: Photoshop's menu and Paint.NET's.
fn photoshop_modes() -> impl Iterator<Item = BlendMode> {
    BlendMode::ALL.iter().copied().filter(|m| m.id() <= BlendMode::PaintNetColorDodge.id())
}

fn straight(b: Blend, layer: Layer, backdrop: [f32; 4], source: [f32; 4]) -> [f32; 4] {
    b.composite(&layer, Alpha::Straight, backdrop, source, (&Linear, &Linear))
}

fn over(b: Blend, mode: BlendMode, backdrop: [f32; 4], source: [f32; 4], opacity: f32) -> [f32; 4] {
    straight(b, Layer { opacity, ..Layer::new(mode) }, backdrop, source)
}

fn premul(b: Blend, mode: BlendMode, dst: [f32; 4], src: [f32; 4]) -> [f32; 4] {
    b.composite(&Layer { noise: 0.5, ..Layer::new(mode) }, Alpha::Premultiplied, dst, src, (&Linear, &Linear))
}

// ---- Photoshop's 32-bit overflow replay ----

/// Overflow is recomputed in f64 and bounded, but ordinary finite results keep their f32 bits,
/// and in-range values give the integer-depth results except for the Add/Divide cap.
#[test]
fn separable_hdr_extremes_stay_finite_and_keep_ordinary_f32_bits() {
    let values = [-MAX, -MAX / 2., -2e19, -4., -1., -0., 0., f32::from_bits(1), 1e-20, 0.25, 0.5, 0.75, 1., 4., 2e19, MAX / 2., MAX];
    let replaced = [BlendMode::ColorBurn, BlendMode::ColorDodge, BlendMode::VividLight, BlendMode::HardMix];
    for mode in photoshop_modes().filter(|m| m.is_separable()) {
        for cb in values {
            for cs in values {
                let actual = PS_FLOAT.channel(mode, cb, cs);
                assert!(actual.is_finite(), "{mode:?}: {cb}, {cs} -> {actual}");
                let ordinary = shared(mode, cb, cs, MAX);
                if !replaced.contains(&mode) && ordinary.is_finite() {
                    assert_eq!(actual.to_bits(), ordinary.to_bits(), "{mode:?}: ordinary rounding changed for {cb}, {cs}");
                }
            }
        }
        let rgb = PS_FLOAT.rgb(mode, [MAX, -MAX, 0.5], [-MAX, MAX, MAX]);
        assert!(rgb.iter().all(|v| v.is_finite()), "{mode:?}: {rgb:?}");
        for cb in [0., 0.25, 0.5, 0.75, 1.] {
            for cs in [0., 0.25, 0.5, 0.75, 1.] {
                if !matches!(mode, BlendMode::LinearDodge | BlendMode::Divide) {
                    assert_eq!(PS_FLOAT.channel(mode, cb, cs).to_bits(), PS.channel(mode, cb, cs).to_bits(), "{mode:?}: {cb}, {cs}");
                }
            }
        }
    }
}

/// Signed saturation and the blend identities of the f64 replay.
#[test]
fn overflowing_hdr_retries_keep_signed_results_and_blend_identities() {
    use BlendMode::*;
    for (mode, b, s, expected) in [
        (Exclusion, MAX, MAX, -MAX),
        (Exclusion, MAX, 0.5, 0.5),
        (Exclusion, -MAX, 0.5, 0.5),
        (HardLight, MAX, 0., 0.),
        (HardLight, MAX, 0.5, MAX),
        (HardLight, 1., MAX, 1.),
        (Overlay, 0., MAX, 0.),
        (Overlay, 0.5, MAX, MAX),
        (Overlay, MAX, 1., 1.),
        (SoftLight, MAX, 0.5, MAX),
        (SoftLight, MAX, 1., MAX.sqrt()),
        (SoftLight, 1., MAX, 1.),
        (Difference, MAX, -MAX, MAX),
        (Subtract, MAX, -MAX, MAX),
        (LinearBurn, MAX, MAX, MAX),
        (LinearBurn, MAX, -MAX, 0.),
        (LinearDodge, -MAX, -MAX, -MAX),
        (Divide, -MAX, f32::from_bits(1), -MAX),
        (PinLight, 0., -MAX, -MAX),
        (PinLight, 0., MAX, MAX),
        (Negation, MAX, MAX, -MAX),
        (PaintNetColorBurn, MAX, f32::from_bits(1), MAX),
        (PaintNetColorDodge, -MAX, 0.5, -MAX),
    ] {
        assert_eq!(PS_FLOAT.channel(mode, b, s), expected, "{mode:?}: {b}, {s}");
    }
}

/// A shared f32 normalisation erased a tiny source's hue next to a huge backdrop.
#[test]
fn overflowing_hdr_hue_keeps_even_subnormal_source_channel_ordering() {
    let cb = [MAX, -MAX / 2., 0.];
    // Backdrop luminosity is .005 · MAX. Pure green at that luminosity is .005/.59 · MAX,
    // whatever the positive source's magnitude.
    for tiny in [1e-20, f32::from_bits(1)] {
        let out = PS_FLOAT.rgb(BlendMode::Hue, cb, [0., tiny, 0.]);
        assert!(out.iter().all(|v| v.is_finite()), "{tiny}: {out:?}");
        assert!(out[1] > out[0] && out[1] > out[2], "tiny green hue became grey: {out:?}");
        assert!((out[1] / MAX - 0.005 / 0.59).abs() < 1e-6, "{out:?}");
        assert!((out[0] / MAX).abs() < 1e-6 && (out[2] / MAX).abs() < 1e-6, "{out:?}");
    }
}

/// Only non-finite Screen results are replayed; finite ones keep their f32 arithmetic.
#[test]
fn hdr_screen_replays_only_nonfinite_results_in_wide_arithmetic() {
    assert_eq!(PS_FLOAT.channel(BlendMode::Screen, 2e19, 2e19), -MAX);
    assert_eq!(PS_FLOAT.channel(BlendMode::Screen, MAX, MAX), -MAX);
    let values = [-MAX, -2e19, -1., -0., 0., f32::from_bits(1), 0.25, 1., 2e19, MAX];
    for cb in values {
        for cs in values {
            let ordinary = cb + cs - cb * cs;
            let actual = PS_FLOAT.channel(BlendMode::Screen, cb, cs);
            assert!(actual.is_finite(), "{cb}, {cs}: {actual}");
            if ordinary.is_finite() {
                assert_eq!(actual.to_bits(), ordinary.to_bits(), "ordinary f32 rounding changed for {cb}, {cs}");
            } else {
                let (b, s) = (f64::from(cb), f64::from(cs));
                let expected = (b + s - b * s).clamp(-f64::from(MAX), f64::from(MAX)) as f32;
                assert_eq!(actual.to_bits(), expected.to_bits(), "{cb}, {cs}");
            }
            assert_eq!(PS.channel(BlendMode::Screen, cb, cs).to_bits(), ordinary.to_bits());
        }
    }
}

/// Non-separable blends stay finite at signed f32 extremes; ordinary results are unchanged.
#[test]
fn finite_signed_extreme_nonseparable_hdr_does_not_publish_infinity_or_nan() {
    let grey = [MAX; 3];
    let signed = [MAX, -MAX, 0.];
    // The SetLum subtraction overflowed before gamut clipping could produce white.
    for (mode, cb, cs) in [(BlendMode::Color, grey, signed), (BlendMode::Luminosity, signed, grey)] {
        let out = PS_FLOAT.rgb(mode, cb, cs);
        assert!(out.iter().all(|v| v.is_finite() && (v / MAX - 1.).abs() < E), "{mode:?}: {out:?}");
    }
    let values = [-MAX, 0., MAX];
    for mode in NON_SEPARABLE {
        for b in 0..27 {
            let cb = [values[b % 3], values[b / 3 % 3], values[b / 9]];
            for s in 0..27 {
                let cs = [values[s % 3], values[s / 3 % 3], values[s / 9]];
                let out = PS_FLOAT.rgb(mode, cb, cs);
                assert!(out.iter().all(|v| v.is_finite()), "{mode:?}: {cb:?}, {cs:?} -> {out:?}");
            }
        }
    }
}

// ---- Photoshop's measured variants ----

/// #8804: with sqrt throughout, dark backdrops came out up to 20 levels too light.
/// (backdrop, source, Photoshop).
#[test]
fn soft_light_follows_photoshop_over_dark_backdrops() {
    for (cb, cs, ps) in [(4u8, 247u8, 11.0f32), (6, 254, 19.0), (9, 252, 25.0), (10, 237, 32.0), (14, 253, 46.0)] {
        let got = PS.channel(BlendMode::SoftLight, f32::from(cb) / 255.0, f32::from(cs) / 255.0) * 255.0;
        assert!((got - ps).abs() <= 7.0, "cb {cb} cs {cs}: {got} vs Photoshop {ps}");
    }
}

/// Vivid Light and Hard Mix corners differ by variant and flip 255 levels when mixed up:
/// Photoshop lets the source's extremes win in Vivid Light, After Effects and Premiere the
/// backdrop's; Hard Mix is a threshold of the backdrop-wins Vivid Light (Photoshop within EDGE,
/// Premiere exact) or `cb + cs ≥ 1` (After Effects). Through the composite, as Photoshop shows.
#[test]
fn vivid_light_and_hard_mix_corners_per_variant() {
    let (white, black) = ([1.0; 4], [0.0, 0.0, 0.0, 1.0]);
    let v = |b: Blend, cb: [f32; 4], cs: [f32; 4]| over(b, BlendMode::VividLight, cb, cs, 1.0)[0];
    assert_eq!(v(PS, white, black), 0.0);
    assert_eq!(v(PS, black, white), 1.0);
    assert_eq!(v(AE, white, black), 1.0);
    assert_eq!(v(STANDARD, black, white), 0.0);
    for cb in [0.0, 0.25, 0.5, 0.75, 1.0] {
        assert_eq!(PS.channel(BlendMode::VividLight, cb, 0.0), 0.0);
        assert_eq!(PS.channel(BlendMode::VividLight, cb, 1.0), 1.0);
    }
    assert!(close(PS.channel(BlendMode::VividLight, 0.75, 0.25), 0.5));
    assert!(close(PS.channel(BlendMode::VividLight, 0.25, 0.75), 0.5));
    let h = |b: Blend, cb: f32, cs: f32| b.channel(BlendMode::HardMix, cb, cs);
    // Photoshop: black over white is white, white over black is black, also a hair from them.
    assert_eq!(h(PS, 1.0, 0.0), 1.0);
    assert_eq!(h(PS, 0.0, 1.0), 0.0);
    assert_eq!(h(PS, 1.0 - 1e-5, 0.0), 1.0);
    assert_eq!(h(PS, 1e-5, 1.0), 0.0);
    assert_eq!(h(PS, 0.6, 0.4), 1.0);
    assert_eq!(h(PS, 0.6, 0.3), 0.0);
    assert_eq!(over(PS, BlendMode::HardMix, white, black, 1.0), white);
    // After Effects: the sum rule, so white over black is white.
    assert_eq!(h(AE, 0.0, 1.0), 1.0);
    // Premiere: exact extremes, so a hair below white over black is black.
    assert_eq!(h(STANDARD, 1.0, 0.0), 1.0);
    assert_eq!(h(STANDARD, 1.0 - 1e-5, 0.0), 0.0);
}

/// Photoshop's Color Burn / Dodge count a backdrop within EDGE of 0 / 1 as exact: a composited
/// value a rounding step off the extreme flipped the corner by 255 levels. The other variants
/// are exact.
#[test]
fn burn_and_dodge_treat_near_extremes_as_exact() {
    assert_eq!(PS.channel(BlendMode::ColorBurn, 1.0 - 1e-6, 0.0), 1.0);
    assert_eq!(PS.channel(BlendMode::ColorDodge, 1e-6, 1.0), 0.0);
    assert_eq!(AE.channel(BlendMode::ColorBurn, 1.0 - 1e-6, 0.0), 0.0);
    assert!(close(PS.channel(BlendMode::ColorBurn, 0.5, 0.5), 0.0));
    assert!(close(PS.channel(BlendMode::ColorDodge, 0.25, 0.5), 0.5));
}

/// Integer depths clip Add and Divide at 1 (Photoshop, Premiere); 32-bit documents add light
/// past 1; After Effects and Substance never clip.
#[test]
fn add_and_divide_clip_by_range_and_variant() {
    assert_eq!(PS.channel(BlendMode::LinearDodge, 2.0, 0.5), 1.0);
    assert_eq!(PS.channel(BlendMode::LinearDodge, 0.75, 0.5), 1.0);
    assert_eq!(PS.channel(BlendMode::Divide, 2.0, 0.5), 1.0);
    assert_eq!(STANDARD.channel(BlendMode::LinearDodge, 0.75, 0.5), 1.0);
    assert!(close(PS_FLOAT.channel(BlendMode::LinearDodge, 2.0, 0.5), 2.5));
    assert!(close(PS_FLOAT.channel(BlendMode::Divide, 0.5, 0.25), 2.0));
    assert_eq!(PS_FLOAT.rgb(BlendMode::LinearDodge, [2.0, 0.5, 4.0], [0.5, 0.25, 0.5]), [2.5, 0.75, 4.5]);
    assert!(close(AE.channel(BlendMode::Add, 0.75, 0.5), 1.25));
    assert!(close(AE.channel(BlendMode::Divide, 0.5, 0.25), 2.0));
    assert!(close(SUBSTANCE.channel(BlendMode::LinearDodge, 0.75, 0.5), 1.25));
    // Divide by zero: black stays black (Photoshop, After Effects); Premiere gives white.
    assert_eq!(PS_FLOAT.channel(BlendMode::Divide, 0.0, 0.0), 0.0);
    assert_eq!(PS_FLOAT.channel(BlendMode::Divide, 3.0, -1.0), 1.0);
    assert_eq!(STANDARD.channel(BlendMode::Divide, 0.0, 0.0), 1.0);
    // Overflow stays finite.
    assert_eq!(PS_FLOAT.channel(BlendMode::Divide, MAX, 1e-30), MAX);
    assert_eq!(PS_FLOAT.channel(BlendMode::LinearDodge, MAX, MAX), MAX);
    assert_eq!(PS_FLOAT.channel(BlendMode::Multiply, MAX, -2.0), -MAX);
    assert!(PS_FLOAT.channel(BlendMode::LinearDodge, f32::INFINITY, 1.0).is_finite());
}

/// The plain ClipColor divided by `max − lum`, rounding noise for a grey above 1, and turned
/// colours inside out when the luminosity was out of range (a grey of 2 as the Luminosity of
/// (0.5, 0.25, 0.125) gave (1.0, 2.32, 2.99)). Such colours are their grey now; every variant
/// uses this form.
#[test]
fn non_separable_modes_stable_when_luminosity_is_out_of_range() {
    let colors: [[f32; 3]; 10] = [
        [1.0, 1.0, 1.0],
        [1.5, 1.5, 1.5],
        [2.0, 2.0, 2.0],
        [4.0, 4.0, 4.0],
        [1000.0, 1000.0, 1000.0],
        [2.0, 2.0, 1.999_999],
        [4.0, 3.999_9, 4.0],
        [3.0, 0.25, 0.125],
        [1.25, 0.875, 3.75],
        [-0.25, 0.5, 1.375],
    ];
    for b in [PS, PS_FLOAT] {
        for m in NON_SEPARABLE {
            for cb in colors {
                for cs in colors {
                    let r = b.rgb(m, cb, cs);
                    assert!(r.iter().all(|v| v.is_finite()), "{m:?} {cb:?} {cs:?} {b:?}: {r:?}");
                    // The target luminosity is kept, at any depth.
                    let want = if m == BlendMode::Luminosity { lum(cs) } else { lum(cb) };
                    if want >= 0.0 {
                        assert!((lum(r) - want).abs() <= 1e-4 * want.max(1.0), "{m:?} {cb:?} {cs:?} {b:?}: {r:?}");
                    }
                    let (lo, hi) = (r[0].min(r[1]).min(r[2]), r[0].max(r[1]).max(r[2]));
                    if b.range == Range::Unit {
                        // In gamut, or (a luminosity above 1 has no in-gamut colour) its grey.
                        assert!((lo >= 0.0 && hi <= 1.0 + E) || hi - lo <= 1e-6 * hi, "{m:?} {cb:?} {cs:?}: {r:?}");
                    } else {
                        assert!(lo >= -1e-6 * hi.max(1.0), "{m:?} {cb:?} {cs:?}: {r:?}");
                    }
                }
            }
        }
    }
    // 32-bit: bright colours keep their values above 1, in order.
    for g in [1.5f32, 2.0, 4.0, 1000.0] {
        let r = PS_FLOAT.rgb(BlendMode::Luminosity, [0.5, 0.25, 0.125], [g; 3]);
        assert!(r[0] > 1.0 && r[0] > r[1] && r[1] > r[2], "{g}: {r:?}");
        // Integer documents: no colour of that luminosity fits in 0..1, so it is its grey.
        let r = PS.rgb(BlendMode::Luminosity, [0.5, 0.25, 0.125], [g; 3]);
        assert!(r.iter().all(|&v| (v - lum([g; 3])).abs() <= 1e-5 * g), "{g}: {r:?}");
    }
    // Continuous across near-greys.
    for m in NON_SEPARABLE {
        let a = PS_FLOAT.rgb(m, [2.0, 2.0, 2.0], [4.0, 4.0, 4.0]);
        let b = PS_FLOAT.rgb(m, [2.0, 2.0, 1.999_999], [4.0, 3.999_999, 4.0]);
        assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 1e-4), "{m:?}: {a:?} {b:?}");
    }
}

/// Darker / Lighter Color choose a whole pixel, by the W3C luma except in After Effects
/// (Rec. 709): the two weightings disagree on red against green.
#[test]
fn darker_lighter_color_pick_whole_pixel_by_the_variants_luma() {
    let (red, green) = ([0.9, 0.1, 0.1], [0.2, 0.6, 0.2]);
    assert_eq!(PS.rgb(BlendMode::DarkerColor, red, green), if lum(green) < lum(red) { green } else { red });
    assert_eq!(PS.rgb(BlendMode::LighterColor, red, green), if lum(green) > lum(red) { green } else { red });
    let (a, b) = ([0.6, 0.2, 0.2], [0.15, 0.4, 0.15]);
    assert!(lum(b) < lum(a) && craft_wgsl::luminance(b) > craft_wgsl::luminance(a));
    assert_eq!(PS.rgb(BlendMode::DarkerColor, a, b), b);
    assert_eq!(AE.rgb(BlendMode::DarkerColor, a, b), a);
}

// ---- compositing ----

#[test]
fn composite_alpha_rules() {
    let bg = [0.2, 0.4, 0.6, 1.0];
    // A transparent source leaves the backdrop.
    assert_eq!(over(PS, BlendMode::Multiply, bg, [1.0, 0.0, 0.0, 0.0], 1.0), bg);
    // An opaque Normal source replaces it.
    let r = over(PS, BlendMode::Normal, bg, [1.0, 0.0, 0.0, 1.0], 1.0);
    assert!(close(r[0], 1.0) && close(r[3], 1.0));
    // Half opacity Normal is a lerp.
    let r = over(PS, BlendMode::Normal, bg, [1.0, 1.0, 1.0, 1.0], 0.5);
    assert!(close(r[0], 0.6) && close(r[1], 0.7) && close(r[2], 0.8));
    // Over a transparent backdrop any mode gives the source.
    let r = over(PS, BlendMode::Multiply, [0.0; 4], [0.3, 0.5, 0.7, 1.0], 1.0);
    assert!(close(r[0], 0.3) && close(r[1], 0.5) && close(r[2], 0.7) && close(r[3], 1.0));
    // Alpha accumulates.
    let r = over(PS, BlendMode::Screen, [0.0, 0.0, 0.0, 0.5], [0.0, 0.0, 0.0, 0.5], 1.0);
    assert!(close(r[3], 0.75));
}

#[test]
fn composite_atop_keeps_alpha_and_moves_colour_by_source_alpha() {
    let bg = [1.0, 1.0, 1.0, 0.5];
    // Half-transparent white, black at 50%: halfway, alpha kept (#5783).
    let r = PS.composite_atop(BlendMode::Normal, bg, [0.0, 0.0, 0.0, 1.0], 0.5);
    assert!(close(r[0], 0.5) && close(r[2], 0.5) && r[3] == 0.5, "{r:?}");
    // Other modes blend against the backdrop as it is: Multiply by black at 50% halves it.
    let r = PS.composite_atop(BlendMode::Multiply, bg, [0.0, 0.0, 0.0, 1.0], 0.5);
    assert!(close(r[0], 0.5) && r[3] == 0.5, "{r:?}");
    // Opaque backdrops agree with source-over.
    let opaque = [0.2, 0.4, 0.6, 1.0];
    assert_eq!(
        PS.composite_atop(BlendMode::Screen, opaque, [0.5, 0.5, 0.5, 0.7], 0.8),
        over(PS, BlendMode::Screen, opaque, [0.5, 0.5, 0.5, 0.7], 0.8)
    );
    assert_eq!(PS.composite_atop(BlendMode::Normal, bg, [0.0; 4], 1.0), bg);
}

/// Photoshop 27.11, measured: Background #c89664, a layer #64b4dc at Fill 50% (128/255),
/// Opacity 100%, in each of the special eight; then Background #3c78b4, layer #d25a1e at Fill
/// 25% and 75%. Values are the composite at 8 bits. Hard Mix at 75% is 3 levels off in red:
/// dividing by 1 − f = 0.25 magnifies Photoshop's 8-bit rounding.
#[test]
fn special_eight_fill_matches_photoshop() {
    let fill = |mode, b, t, opacity, fill| straight(PS, Layer { mode, opacity, fill, noise: 0.0 }, b, t);
    let px = |c: [u8; 3]| [f32::from(c[0]) / 255.0, f32::from(c[1]) / 255.0, f32::from(c[2]) / 255.0, 1.0];
    let measured: [(BlendMode, [u8; 3]); 8] = [
        (BlendMode::ColorBurn, [176, 132, 88]),
        (BlendMode::LinearBurn, [122, 112, 82]),
        (BlendMode::ColorDodge, [249, 232, 176]),
        (BlendMode::LinearDodge, [250, 240, 210]),
        (BlendMode::VividLight, [193, 188, 157]),
        (BlendMode::LinearLight, [171, 202, 192]),
        (BlendMode::HardMix, [245, 225, 165]),
        (BlendMode::Difference, [150, 60, 10]),
    ];
    let (b, t) = (px([200, 150, 100]), px([100, 180, 220]));
    for (mode, want) in measured {
        let got = fill(mode, b, t, 1.0, 128.0 / 255.0);
        for i in 0..3 {
            assert!((got[i] * 255.0 - f32::from(want[i])).abs() <= 1.5, "{mode:?} channel {i}: {} vs Photoshop {}", got[i] * 255.0, want[i]);
        }
        let none = fill(mode, b, t, 1.0, 0.0);
        assert!((0..3).all(|i| (none[i] - b[i]).abs() < 1e-5), "{mode:?} at Fill 0: {none:?}");
    }
    let confirm: [(BlendMode, [u8; 3], [u8; 3]); 8] = [
        (BlendMode::ColorBurn, [51, 94, 159], [30, 0, 33]),
        (BlendMode::LinearBurn, [49, 79, 124], [26, 0, 11]),
        (BlendMode::ColorDodge, [76, 132, 186], [156, 163, 197]),
        (BlendMode::LinearDodge, [113, 143, 188], [217, 187, 202]),
        (BlendMode::VividLight, [71, 109, 162], [117, 81, 80]),
        (BlendMode::LinearLight, [100, 100, 130], [183, 63, 33]),
        (BlendMode::HardMix, [65, 105, 166], [102, 0, 47]),
        (BlendMode::Difference, [7, 97, 172], [97, 53, 158]),
    ];
    let (b2, t2) = (px([60, 120, 180]), px([210, 90, 30]));
    for (mode, q, tq) in confirm {
        for (f, want) in [(64.0 / 255.0, q), (191.0 / 255.0, tq)] {
            let got = fill(mode, b2, t2, 1.0, f);
            let tol = if mode == BlendMode::HardMix { 3.5 } else { 1.5 };
            for i in 0..3 {
                assert!(
                    (got[i] * 255.0 - f32::from(want[i])).abs() <= tol,
                    "{mode:?} fill {f} channel {i}: {} vs Photoshop {}",
                    got[i] * 255.0,
                    want[i]
                );
            }
        }
    }
    // Other modes take Fill as coverage, like Opacity.
    assert_eq!(fill(BlendMode::Multiply, b, t, 0.8, 0.5), over(PS, BlendMode::Multiply, b, t, 0.4));
    // Over transparency Fill is still coverage for the special eight.
    let r = fill(BlendMode::LinearDodge, [0.0; 4], t, 1.0, 0.5);
    assert!((r[3] - 0.5).abs() < 1e-6 && (r[0] - t[0]).abs() < 1e-6, "{r:?}");
}

/// Type layers mix in the text gamma space: 69% blue (0, 51, 153) over grey 129 shows
/// (57, 80, 146) in Photoshop; 94% shows (13, 57, 152), where a `v^1.45` mix gives 19 red.
#[test]
fn text_gamma_lightens_dark_edges() {
    let grey = [129.0 / 255.0, 129.0 / 255.0, 129.0 / 255.0, 1.0];
    let text = |a: f32| PS.composite(&Layer::new(BlendMode::Normal), Alpha::Straight, grey, [0.0, 0.2, 0.6, a], (&Linear, &TextGamma(TEXT_GAMMA)));
    for (a, want) in [(0.686, [57.0, 80.0, 146.0]), (0.937, [13.0, 57.0, 152.0])] {
        let r = text(a);
        assert!((0..3).all(|i| (r[i] * 255.0 - want[i]).abs() < 1.0), "{a}: {r:?}");
    }
}

/// After Effects' 32 bpc rules (#494, measured by the reporter): Classic modes fade the source
/// toward their neutral colour by opacity, over-range top layers keep the plain formulas where
/// After Effects keeps them, negative Classic Color Burn results stay, and the other modes clamp
/// their inputs.
#[test]
fn after_effects_32bpc_formulas() {
    let opaque = |v: f32| [v, v, v, 1.0];
    let at = |m, b: f32, o: f32, k: f32| premul(AE, m, opaque(b), [o * k, o * k, o * k, k])[0];
    let near = |a: f32, e: f32| (a - e).abs() < 1e-5;
    for (b, o) in [(0.4, 0.5), (0.8, 0.9), (0.1, 0.3)] {
        let k = 0.15;
        assert!(near(at(BlendMode::ClassicColorDodge, b, o, k), (b / (1.0 - k * o)).min(1.0)));
        assert!(near(at(BlendMode::ClassicDifference, b, o, k), (b - k * o).abs()));
        assert!(near(at(BlendMode::ClassicColorBurn, b, o, k), 1.0 - (1.0 - b) / (1.0 - k * (1.0 - o))));
    }
    assert!(near(at(BlendMode::ClassicDifference, 0.6, 0.3, 1.0), 0.3));
    let (b, o) = (0.7, 3.0);
    assert!(near(at(BlendMode::Exclusion, b, o, 1.0), b + o - 2.0 * b * o));
    let s = 2.0 * b - 1.0;
    assert!(near(at(BlendMode::Overlay, b, o, 1.0), o + s - o * s));
    assert!(near(at(BlendMode::ColorBurn, 0.0, 1.35, 0.15), 0.15 * (1.0 - 1.0 / 1.35)));
    assert!(near(at(BlendMode::ClassicColorBurn, 0.0, 0.1, 1.0), -9.0), "negative results are kept");
    // Add and Screen keep over-range values; Soft Light clamps its inputs.
    let (d, src) = (opaque(2.0), opaque(0.5));
    assert!(near(premul(AE, BlendMode::Add, d, src)[0], 2.5));
    assert!(near(premul(AE, BlendMode::Screen, d, src)[0], 1.5));
    assert!(premul(AE, BlendMode::SoftLight, d, src)[0] <= 1.0);
}

#[test]
fn stencil() {
    let d = [0.5, 0.5, 0.5, 1.0];
    assert_eq!(premul(AE, BlendMode::StencilAlpha, d, [0.0; 4]), [0.0; 4]);
    assert_eq!(premul(AE, BlendMode::SilhouetteAlpha, d, [0.0; 4]), d);
}

/// Premiere: Normal mixes in linear light, the other modes blend the encoded values (sRGB blend
/// space) and stay in range.
#[test]
fn standard_srgb_blend_space() {
    let blend = |m, d: [f32; 4], s: [f32; 4]| STANDARD.composite(&Layer::new(m), Alpha::Premultiplied, d, s, (&Srgb, &Linear));
    let normal = blend(BlendMode::Normal, [0.5, 0.0, 0.0, 1.0], [0.0, 0.25, 0.0, 0.5]);
    assert_eq!(normal, [0.25, 0.25, 0.0, 1.0]);
    // Multiply by white leaves the backdrop: true only if the blend runs on encoded values
    // and comes back through the same curve.
    let base = [0.2, 0.4, 0.6, 1.0];
    let m = blend(BlendMode::Multiply, base, [1.0; 4]);
    assert!((0..3).all(|k| (m[k] - base[k]).abs() < 1e-4), "{m:?}");
    // Screen of mid-greys: encoded 0.5 screen 0.5 = 0.75 encoded, not the linear 0.75.
    let g = craft_wgsl::srgb_to_linear(0.5);
    let s = blend(BlendMode::Screen, [g, g, g, 1.0], [g, g, g, 1.0]);
    assert!((s[0] - craft_wgsl::srgb_to_linear(0.75)).abs() < 1e-5, "{s:?}");
    for mode in BlendMode::ALL.iter().copied().filter(|m| m.id() <= BlendMode::Luminosity.id()) {
        let r = blend(mode, [0.3, 0.1, 0.7, 1.0], [0.4 * 0.8, 0.4 * 0.8, 0.1 * 0.8, 0.8]);
        assert!(r.iter().all(|v| v.is_finite() && *v >= -1e-5 && *v <= 1.0001), "{mode:?} {r:?}");
    }
}

/// Substance's modes stay finite on their whole range, and Color Burn / Dodge give the edge
/// values at their extremes.
#[test]
fn substance_burns_and_dodges_handle_edges() {
    assert!(close(SUBSTANCE.channel(BlendMode::ColorBurn, 1.0, 0.0), 1.0));
    assert!(close(SUBSTANCE.channel(BlendMode::ColorBurn, 0.5, 0.0), 0.0));
    assert!(close(SUBSTANCE.channel(BlendMode::ColorDodge, 0.0, 1.0), 0.0));
    assert!(close(SUBSTANCE.channel(BlendMode::ColorDodge, 0.5, 1.0), 1.0));
    assert!(close(SUBSTANCE.channel(BlendMode::ColorDodge, 0.25, 0.5), 0.5));
    for &m in BlendMode::ALL {
        for a in [0.0, 0.25, 0.5, 1.0] {
            for b in [0.0, 0.3, 0.7, 1.0] {
                assert!(SUBSTANCE.rgb(m, [a; 3], [b; 3]).iter().all(|v| v.is_finite()), "{m:?} {a} {b}");
            }
        }
    }
}

/// A flat normal (0.5, 0.5, 1) as the detail leaves the base normal (reoriented normal mapping
/// and the combine mode).
#[test]
fn flat_normal_detail_is_identity() {
    let flat = [0.5, 0.5, 1.0];
    let n = [0.3f32, -0.2, 0.9];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    let n = n.map(|v| v / len * 0.5 + 0.5);
    for m in [BlendMode::NormalMapDetail, BlendMode::NormalMapCombine] {
        let out = SUBSTANCE.rgb(m, n, flat);
        assert!((0..3).all(|i| (out[i] - n[i]).abs() < 1e-4), "{m:?} {out:?} {n:?}");
    }
}

/// CMYK blends as complemented planes: Multiply adds inks, Screen keeps the lighter plate, and
/// K follows the backdrop for Hue, Saturation and Color but the source for Luminosity.
#[test]
fn cmyk_planes_blend_inks() {
    let blend = |m: BlendMode, b: [f32; 4], s: [f32; 4]| {
        let (pb, ps) = (cmyk_planes(b), cmyk_planes(s));
        planes_cmyk(STANDARD.rgb(m, pb[0], ps[0]), STANDARD.rgb(m, pb[1], ps[1])[0])
    };
    let (cyan, magenta) = ([1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]);
    assert_eq!(blend(BlendMode::Multiply, magenta, cyan), [1.0, 1.0, 0.0, 0.0]);
    assert_eq!(blend(BlendMode::Screen, magenta, cyan), [0.0; 4]);
    let (b, s) = ([0.2, 0.6, 0.1, 0.3], [0.7, 0.1, 0.5, 0.8]);
    for m in [BlendMode::Hue, BlendMode::Saturation, BlendMode::Color] {
        assert!((blend(m, b, s)[3] - b[3]).abs() < 1e-6, "{m:?}");
    }
    assert!((blend(BlendMode::Luminosity, b, s)[3] - s[3]).abs() < 1e-6);
}

/// A WGSL edit that doesn't parse or validate. CI has no GPU, so the GPU target skips there
/// and only this sees it. Every variant, with an entry point calling the public functions.
#[test]
fn blend_modules_validate() {
    const ENTRY: &str = "@group(0) @binding(0) var<storage, read_write> out: array<vec4<f32>>;
@compute @workgroup_size(1)
fn main() {
    let b = vec4<f32>(0.2, 0.4, 0.6, 1.0);
    let s = vec4<f32>(0.5, 0.1, 0.9, 0.7);
    out[0] = blend_straight(BLEND_MULTIPLY, b, s, 0.8, 0.5, 0.3, false, 1.45);
    out[1] = blend_premul(BLEND_CLASSIC_COLOR_BURN, b, s, 0.3, false, true);
    out[2] = blend_atop(BLEND_SCREEN, b, s, 0.5, true);
    out[3] = vec4<f32>(blend_rgb_fill(BLEND_HARD_MIX, b.rgb, s.rgb, 0.5, false), blend_weights(0.5, 0.25).w);
}
";
    for v in [Variant::Standard, Variant::Photoshop, Variant::AfterEffects, Variant::Substance] {
        let src = craft_wgsl::assemble(&[crate::wgsl::module(v)], &[ENTRY]);
        let module = naga::front::wgsl::parse_str(&src).unwrap_or_else(|e| panic!("{v:?}: {}", e.emit_to_string(&src)));
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{v:?}: {e:?}"));
    }
}
