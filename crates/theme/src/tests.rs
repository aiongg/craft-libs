#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn hex(s: &str) -> Rgba {
    Rgba::from_hex(s).unwrap()
}

fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

fn hue_gap(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

const SAMPLES: [&str; 20] = [
    "#ff0000", "#00ff00", "#0000ff", "#ffff00", "#00ffff", "#ff00ff", "#000000", "#ffffff", "#808080", "#777777", "#3e63dd", "#e5484d", "#12a594",
    "#f5d90a", "#8e4ec6", "#101010", "#fafafa", "#c0ffee", "#123456", "#fedcba",
];

#[test]
fn easing_curves_run_from_zero_to_one_as_their_css_cubic_beziers() {
    for easing in [Easing::Standard, Easing::Enter, Easing::Exit] {
        assert!(near(easing.at(0.0).into(), 0.0, 1e-4) && near(easing.at(1.0).into(), 1.0, 1e-4), "{easing:?}");
        let samples: Vec<f32> = (0..=20).map(|i| easing.at(i as f32 / 20.0)).collect();
        assert!(samples.windows(2).all(|w| w[0] <= w[1]), "{easing:?} rises: {samples:?}");
    }
    // Values from the CSS definition (x(s) = t solved for s, then y(s)).
    assert!(near(Easing::Standard.at(0.5).into(), 0.878, 0.005), "{}", Easing::Standard.at(0.5));
    assert!(Easing::Exit.at(0.5) < 0.5, "exit starts slowly");
}

#[test]
fn hex_parse_and_format() {
    assert_eq!(hex("#FF8000").0, [255, 128, 0, 255]);
    assert_eq!(hex("#ff800080").to_hex(), "#ff800080");
    assert_eq!(hex("#FF8000").to_hex(), "#ff8000");
    assert_eq!(Rgba::from_hex("ff8000"), None);
    assert_eq!(Rgba::from_hex("#ff80"), None);
    assert_eq!(Rgba::from_hex("#gg0000"), None);
}

#[test]
fn known_oklch_values() {
    let red = hex("#ff0000").to_oklch();
    assert!(near(red.l, 0.628, 1e-3) && near(red.c, 0.2577, 1e-3) && near(red.h, 29.23, 0.05));
    let white = Rgba::WHITE.to_oklch();
    assert!(near(white.l, 1.0, 1e-4) && white.c < 1e-4);
    let black = Rgba::BLACK.to_oklch();
    assert!(black.l.abs() < 1e-9 && black.c < 1e-9);
    let grey = hex("#808080").to_oklch();
    assert!(near(grey.l, 0.5999, 2e-3) && grey.c < 1e-4);
}

#[test]
fn srgb_greys_have_no_chroma_or_hue() {
    for v in 0..=255u8 {
        let grey = Rgba::rgb(v, v, v);
        let o = grey.to_oklch();
        assert_eq!((o.c, o.h), (0.0, 0.0), "{grey:?}");
        assert_eq!(grey.to_oklch().to_rgba(), grey);
    }
}

/// A lightness search stops where the contrast test flips, which for a colour with any residual
/// tint lies between the points where its channels round to their next 8-bit value.
#[test]
fn contrast_moves_keep_greys_neutral() {
    let is_grey = |c: Rgba| c.0[0] == c.0[1] && c.0[1] == c.0[2];
    for fg in (0..=255u8).step_by(3) {
        for bg in [0x00, 0x28, 0x42, 0x53, 0x80, 0xd6, 0xe2, 0xf0, 0xff] {
            let (fg, bg) = (Rgba::rgb(fg, fg, fg), Rgba::rgb(bg, bg, bg));
            for min in [3.0, 4.5, 7.0] {
                let out = ensure_contrast(fg, bg, min);
                assert!(is_grey(out), "{fg:?} on {bg:?} at {min}: {out:?}");
            }
        }
    }
}

#[test]
fn srgb_oklch_round_trip_is_exact() {
    for s in SAMPLES {
        let c = hex(s);
        assert_eq!(c.to_oklch().to_rgba(), c, "{s}");
    }
}

#[test]
fn gamut_mapping_lands_in_gamut() {
    for (l, c, h) in [(0.5, 0.4, 30.0), (0.9, 0.3, 140.0), (0.2, 0.35, 265.0), (0.7, 0.5, 90.0), (1.2, 0.1, 0.0)] {
        let m = Oklch { l, c, h }.gamut_map();
        assert!(m.in_gamut(), "{l} {c} {h}");
        assert!(m.c <= c);
        if l <= 1.0 {
            assert!(near(m.l, l, 1e-12) && near(m.h, h, 1e-12));
        }
    }
}

#[test]
fn wcag_contrast() {
    assert!(near(contrast_ratio(Rgba::WHITE, Rgba::BLACK), 21.0, 1e-9));
    assert!(near(contrast_ratio(hex("#777777"), Rgba::WHITE), 4.48, 0.01));
}

#[test]
fn ensure_contrast_reaches_target_and_keeps_hue() {
    let pairs = [("#3e63dd", "#ffffff"), ("#e5484d", "#fafafa"), ("#12a594", "#ffffff"), ("#8e4ec6", "#1c1c1c"), ("#f5d90a", "#ffffff")];
    for (fg, bg) in pairs {
        for min in [4.5, 7.0] {
            let (fg, bg) = (hex(fg), hex(bg));
            let out = ensure_contrast(fg, bg, min);
            assert!(contrast_ratio(out, bg) >= min);
            assert!(hue_gap(out.to_oklch().h, fg.to_oklch().h) < 2.0, "{fg:?} on {bg:?}");
        }
    }
    let ok = hex("#000000");
    assert_eq!(ensure_contrast(ok, Rgba::WHITE, 4.5), ok);
}

const SEEDS: [&str; 5] = ["#3e63dd", "#e5484d", "#12a594", "#f5d90a", "#8e4ec6"];

#[test]
fn ramps_are_monotonic_in_gamut_and_hit_the_seed() {
    for mode in [Mode::Light, Mode::Dark] {
        for kind in [RampKind::Chromatic, RampKind::Neutral] {
            for s in SEEDS {
                let seed = hex(s);
                let ramp = Ramp::from_seed(seed, mode, kind);
                let ls: Vec<f64> = ramp.0.iter().map(|c| c.to_oklch().l).collect();
                for w in ls.windows(2) {
                    match mode {
                        Mode::Light => assert!(w[0] > w[1], "{s} {mode:?} {ls:?}"),
                        Mode::Dark => assert!(w[0] < w[1], "{s} {mode:?} {ls:?}"),
                    }
                }
                if kind == RampKind::Chromatic {
                    assert_eq!(ramp.step(9), seed);
                }
            }
        }
    }
}

#[test]
fn neutral_ramp_is_nearly_grey() {
    for mode in [Mode::Light, Mode::Dark] {
        let ramp = Ramp::from_seed(hex("#3e63dd"), mode, RampKind::Neutral);
        for c in ramp.0 {
            assert!(c.to_oklch().c <= ramp::NEUTRAL_MAX_CHROMA + 2e-3);
        }
    }
}

#[test]
fn neutral_ramp_lightness_comes_from_the_curve_for_any_seed() {
    for seed in ["#000000", "#ffffff", "#808080", "#535353", "#d4d0c8", "#3e63dd"] {
        let seed = hex(seed);
        let s = seed.to_oklch();
        for (mode, curve) in [(Mode::Light, &ramp::LIGHT_L), (Mode::Dark, &ramp::DARK_L)] {
            let ramp = Ramp::from_seed(seed, mode, RampKind::Neutral);
            let ls: Vec<f64> = ramp.0.iter().map(|c| c.to_oklch().l).collect();
            for (i, l) in ls.iter().enumerate() {
                assert!((l - curve[i]).abs() < 5e-3, "{seed:?} {mode:?} step {}: {l}", i + 1);
            }
            for w in ls.windows(2) {
                assert!((w[0] < w[1]) == (mode == Mode::Dark), "{seed:?} {mode:?} {ls:?}");
            }
            let n9 = ramp.step(9).to_oklch();
            assert!(n9.c <= s.c.min(ramp::NEUTRAL_MAX_CHROMA) + 2e-3);
            if s.c > 0.01 && s.l > 0.1 && s.l < 0.95 {
                assert!(hue_gap(n9.h, s.h) < 15.0, "{seed:?} {mode:?}");
            }
        }
    }
}
