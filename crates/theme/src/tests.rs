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
