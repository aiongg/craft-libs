use egui::Color32;

use super::*;
use crate::theme::{Base, install, tokens_in_mode};
use craft_theme::Mode;

fn all_points(g: &[Polyline]) -> impl Iterator<Item = [f32; 2]> + '_ {
    g.iter().flatten().copied()
}

#[test]
fn every_guide_stays_in_the_unit_square() {
    for kind in CropGuide::ALL {
        for o in 0..8 {
            for (w, h) in [(1.0, 1.0), (300.0, 200.0), (200.0, 300.0), (5000.0, 3.0), (3.0, 5000.0), (1e7, 1e7), (1e30, 2.0)] {
                let g = kind.guides(o, w, h);
                assert!(!g.is_empty(), "{kind:?} {w}x{h}");
                for p in all_points(&g) {
                    assert!(p.iter().all(|v| v.is_finite() && (-1e-5..=1.0 + 1e-5).contains(v)), "{kind:?} o{o} {w}x{h}: {p:?}");
                }
                assert!(g.iter().all(|l| l.len() >= 2));
            }
        }
    }
}

#[test]
fn degenerate_frames_have_no_guides() {
    for kind in CropGuide::ALL {
        for (w, h) in [(0.0, 10.0), (10.0, 0.0), (f32::NAN, 10.0), (f32::INFINITY, 10.0), (-5.0, -5.0), (0.5, 100.0)] {
            assert!(kind.guides(0, w, h).is_empty(), "{kind:?} {w}x{h}");
        }
    }
}

#[test]
fn thirds_and_golden_ratio_positions() {
    let xs = |g: Vec<Polyline>| -> Vec<f32> { g.iter().filter(|l| l[0][1] == 0.0 && l[1][1] == 1.0).map(|l| l[0][0]).collect() };
    let t = xs(CropGuide::Thirds.guides(0, 300.0, 200.0));
    assert_eq!(t.len(), 2);
    assert!((t[0] - 1.0 / 3.0).abs() < 1e-6 && (t[1] - 2.0 / 3.0).abs() < 1e-6);
    let g = xs(CropGuide::GoldenRatio.guides(0, 300.0, 200.0));
    assert!((g[0] - 0.381_966).abs() < 1e-5 && (g[1] - 0.618_034).abs() < 1e-5, "{g:?}");
    assert_eq!(CropGuide::GoldenRatio.guides(0, 300.0, 200.0).len(), 4);
}

#[test]
fn grid_spacing_is_bounded() {
    for (w, h) in [(10.0, 10.0), (64.0, 64.0), (640.0, 320.0), (1e6, 1e6)] {
        let g = CropGuide::Grid.guides(0, w, h);
        let v = g.iter().filter(|l| l[0][0] == l[1][0]).count();
        let hz = g.len() - v;
        assert!((1..GRID_MAX).contains(&v) && (1..GRID_MAX).contains(&hz), "{w}x{h}: {v} {hz}");
    }
    // About one line per grid step on screen.
    let g = CropGuide::Grid.guides(0, 640.0, 320.0);
    assert_eq!(g.iter().filter(|l| l[0][0] == l[1][0]).count(), 19);
    assert_eq!(g.iter().filter(|l| l[0][1] == l[1][1]).count(), 9);
    // The cell size and the cap are the caller's to give.
    let g = CropGuide::Grid.guides_with(0, 640.0, 320.0, 64.0, 4);
    assert_eq!(g.iter().filter(|l| l[0][0] == l[1][0]).count(), 3);
}

#[test]
fn diagonals_run_at_45_degrees_on_screen() {
    let (w, h) = (300.0f32, 200.0f32);
    for l in CropGuide::Diagonal.guides(0, w, h) {
        let (dx, dy) = ((l[1][0] - l[0][0]) * w, (l[1][1] - l[0][1]) * h);
        assert!((dx.abs() - dy.abs()).abs() < 1e-3 && (dx.abs() - 200.0).abs() < 1e-3, "{l:?}");
    }
    // A square frame: corner to corner.
    let g = CropGuide::Diagonal.guides(0, 100.0, 100.0);
    assert_eq!(g[0], vec![[0.0, 0.0], [1.0, 1.0]]);
}

#[test]
fn triangle_perpendiculars_meet_the_diagonal_at_right_angles() {
    let (w, h) = (300.0f32, 200.0f32);
    let g = CropGuide::Triangle.guides(0, w, h);
    let diag = [w, h];
    for l in &g[1..] {
        let d = [(l[1][0] - l[0][0]) * w, (l[1][1] - l[0][1]) * h];
        assert!((d[0] * diag[0] + d[1] * diag[1]).abs() < 1e-2, "{l:?}");
        assert!((l[1][0] - l[1][1]).abs() < 1e-6, "foot on the diagonal: {l:?}");
    }
    // The other orientation mirrors it: the diagonal runs top-right to bottom-left.
    let m = CropGuide::Triangle.guides(1, w, h);
    assert_eq!(m[0], vec![[1.0, 0.0], [0.0, 1.0]]);
    for (a, b) in g.iter().zip(&m) {
        for (p, q) in a.iter().zip(b) {
            assert!((p[0] - (1.0 - q[0])).abs() < 1e-6 && (p[1] - q[1]).abs() < 1e-6);
        }
    }
    // Two orientations: 2 is 0 again.
    assert_eq!(CropGuide::Triangle.guides(2, w, h), g);
}

#[test]
fn golden_spiral_orientations_mirror_it_into_each_corner() {
    let base = CropGuide::GoldenSpiral.guides(0, 300.0, 200.0);
    let pts = &base[0];
    assert!(pts.len() > 40);
    // It starts at the bottom-left corner and winds in towards the golden point (lower right).
    assert_eq!(pts[0], [0.0, 1.0]);
    let end = pts[pts.len() - 1];
    assert!((end[0] - 0.7236).abs() < 0.02 && (end[1] - 0.7236).abs() < 0.02, "{end:?}");
    for o in 1..4u8 {
        let g = CropGuide::GoldenSpiral.guides(o, 300.0, 200.0);
        for (p, q) in pts.iter().zip(&g[0]) {
            let x = if o & 1 != 0 { 1.0 - p[0] } else { p[0] };
            let y = if o & 2 != 0 { 1.0 - p[1] } else { p[1] };
            assert!((x - q[0]).abs() < 1e-6 && (y - q[1]).abs() < 1e-6);
        }
    }
    assert_eq!(CropGuide::GoldenSpiral.guides(4, 300.0, 200.0), base);
    // Symmetric guides ignore the orientation.
    assert_eq!(CropGuide::Thirds.guides(3, 300.0, 200.0), CropGuide::Thirds.guides(0, 300.0, 200.0));
}

#[test]
fn cycling_guides_and_orientations() {
    let mut kind = CropGuide::default();
    assert_eq!((kind, GuideShow::default()), (CropGuide::Thirds, GuideShow::Auto));
    let mut seen = vec![kind];
    for _ in 0..6 {
        kind = kind.next();
        seen.push(kind);
    }
    assert_eq!(&seen[..6], &CropGuide::ALL);
    assert_eq!(seen[6], CropGuide::Thirds, "wraps around");
    assert_eq!(CropGuide::Thirds.next_orientation(0), 0, "thirds has one orientation");
    let turns: Vec<u8> = (0..5)
        .scan(0u8, |o, _| {
            *o = CropGuide::GoldenSpiral.next_orientation(*o);
            Some(*o)
        })
        .collect();
    assert_eq!(turns, [1, 2, 3, 0, 1]);
    // A stray orientation still cycles in range.
    assert!(CropGuide::Triangle.next_orientation(200) < 2);
}

#[test]
fn visibility_modes() {
    assert!(GuideShow::Auto.shows(true) && !GuideShow::Auto.shows(false));
    assert!(GuideShow::Always.shows(true) && GuideShow::Always.shows(false));
    assert!(!GuideShow::Never.shows(true) && !GuideShow::Never.shows(false));
}

const CANVAS: Color32 = Color32::from_rgb(40, 40, 40);

#[test]
fn shield_defaults_match_photoshop() {
    let s = CropShield::default();
    assert!(s.show_cropped_area && s.enabled && s.auto_adjust);
    assert_eq!((s.color, s.opacity), (ShieldColor::MatchCanvas, 0.75));
    assert_eq!(s.fill(CANVAS, false), Some(Color32::from_rgba_unmultiplied(40, 40, 40, 191)));
}

#[test]
fn auto_adjust_lightens_the_shield_while_dragging() {
    let mut s = CropShield::default();
    let still = s.fill(CANVAS, false).unwrap();
    let dragging = s.fill(CANVAS, true).unwrap();
    assert!(dragging.a() < still.a(), "{dragging:?} {still:?}");
    assert_eq!(dragging.to_srgba_unmultiplied()[3], 96);
    s.auto_adjust = false;
    assert_eq!(s.fill(CANVAS, true), Some(still), "without Auto Adjust the opacity holds");
}

#[test]
fn a_disabled_shield_leaves_the_outside_undimmed() {
    let s = CropShield { enabled: false, ..Default::default() };
    assert_eq!(s.fill(CANVAS, false), None);
    assert_eq!(s.fill(CANVAS, true), None);
    let s = CropShield { opacity: 0.0, ..Default::default() };
    assert_eq!(s.fill(CANVAS, false), None, "0 % draws nothing");
}

#[test]
fn custom_colour_and_match_canvas() {
    let s = CropShield { color: ShieldColor::Custom, custom_color: [200, 10, 30], opacity: 1.0, ..Default::default() };
    assert_eq!(s.fill(CANVAS, false), Some(Color32::from_rgb(200, 10, 30)));
    let s = CropShield { opacity: 1.0, ..Default::default() };
    assert_eq!(s.fill(Color32::from_rgb(184, 184, 184), false), Some(Color32::from_rgb(184, 184, 184)));
}

#[test]
fn hidden_cropped_area_is_opaque_canvas_whatever_the_shield() {
    for enabled in [true, false] {
        let s = CropShield {
            show_cropped_area: false,
            enabled,
            color: ShieldColor::Custom,
            custom_color: [255, 0, 0],
            opacity: 0.1,
            ..Default::default()
        };
        assert_eq!(s.fill(CANVAS, false), Some(CANVAS));
        assert_eq!(s.fill(CANVAS, true), Some(CANVAS));
    }
    // Classic mode keeps the cropped area shown whatever the stored choice.
    let s = CropShield { show_cropped_area: false, classic_mode: true, opacity: 1.0, ..Default::default() };
    assert_eq!(s.fill(CANVAS, false), Some(CANVAS));
    let s = CropShield {
        show_cropped_area: false,
        classic_mode: true,
        color: ShieldColor::Custom,
        custom_color: [9, 9, 9],
        opacity: 1.0,
        ..Default::default()
    };
    assert_eq!(s.fill(CANVAS, false), Some(Color32::from_rgb(9, 9, 9)));
}

#[test]
fn stray_opacities_are_clamped() {
    for (o, want) in [(f32::NAN, 0.75), (f32::INFINITY, 0.75), (f32::NEG_INFINITY, 0.75), (-0.2, 0.0), (2.5, 1.0), (0.4, 0.4)] {
        let s = CropShield { opacity: o, ..Default::default() };
        assert!((s.opacity() - want).abs() < 1e-6, "{o}");
        assert_eq!(s.fill(CANVAS, false).map_or(0, |c| c.to_srgba_unmultiplied()[3]), (want * 255.0f32).round() as u8, "{o}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn older_settings_get_the_defaults() {
    let s: CropShield = serde_json::from_str(r#"{"enabled": false}"#).unwrap();
    assert_eq!(s, CropShield { enabled: false, ..Default::default() });
    let back: CropShield = serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
    assert_eq!(back, s);
    assert_eq!(serde_json::to_value(ShieldColor::MatchCanvas).unwrap(), "matchCanvas");
    assert_eq!(serde_json::to_value(CropGuide::GoldenSpiral).unwrap(), "goldenSpiral");
    assert!(serde_json::from_str::<CropShield>(r#"{"opacity": "high"}"#).is_err());
}

/// The geometry's defaults are the `crop` family's values.
#[test]
fn defaults_follow_the_crop_tokens() {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    let crop = tokens_in_mode(&ctx, Mode::Dark).component.crop;
    assert_eq!(
        (crop.grid_step, crop.grid_max, crop.shield_opacity, crop.shield_editing),
        (GRID_STEP, GRID_MAX as f32, SHIELD_OPACITY, SHIELD_EDITING)
    );
}
