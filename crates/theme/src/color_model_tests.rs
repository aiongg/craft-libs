use crate::color::{Lab, Rgba, Srgba, srgb_decode};
use crate::color_model::{ModelValues, builtin_models, kept_or_converted};

fn samples() -> Vec<Srgba> {
    let mut out = Vec::new();
    for r in (0..=255).step_by(15) {
        for g in (0..=255).step_by(15) {
            for b in (0..=255).step_by(15) {
                out.push(Srgba::from(Rgba::rgb(r as u8, g as u8, b as u8)));
            }
        }
    }
    out
}

#[test]
fn every_model_round_trips_srgb_to_the_same_8_bit_colour() {
    for model in builtin_models() {
        for c in samples() {
            let back = model.to_srgb(&model.from_srgb(c).values);
            assert!(back.same_rgb8(c), "{}: {c:?} -> {back:?}", model.id());
        }
    }
}

#[test]
fn hsb_is_computed_on_encoded_values() {
    let grey = builtin_models()[0].from_srgb(Srgba::rgb(0.5, 0.5, 0.5));
    assert_eq!(grey.values[..3], [0.0, 0.0, 50.0]);
    let orange = builtin_models()[0].from_srgb(Srgba::rgb(1.0, 0.5, 0.0));
    assert!((orange.values[0] - 30.0).abs() < 1e-4);
}

/// PhotoCraft's `srgb_to_lab` and `lab_to_srgb` (`photocraft-color` `convert.rs`), copied.
mod photocraft {
    const D50: [f32; 3] = [0.964_22, 1.0, 0.825_21];
    const SRGB_TO_XYZ_D50: [[f32; 3]; 3] = [[0.436_074, 0.385_064, 0.143_080], [0.222_504, 0.716_878, 0.060_618], [0.013_932, 0.097_104, 0.714_173]];
    const XYZ_D50_TO_SRGB: [[f32; 3]; 3] =
        [[3.133_856, -1.616_867, -0.490_615], [-0.978_768, 1.916_142, 0.033_454], [0.071_945, -0.228_991, 1.405_243]];

    fn mul(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
        m.map(|r| r[0] * v[0] + r[1] * v[1] + r[2] * v[2])
    }

    pub fn decode(v: f32) -> f32 {
        super::srgb_decode(f64::from(v)) as f32
    }

    pub fn encode(v: f32) -> f32 {
        crate::color::linear_to_srgb(f64::from(v)) as f32
    }

    pub fn srgb_to_lab(rgb: [f32; 3]) -> [f32; 3] {
        let xyz = mul(&SRGB_TO_XYZ_D50, rgb.map(decode));
        let f = |t: f32| if t > (6.0f32 / 29.0).powi(3) { t.cbrt() } else { t / (3.0 * (6.0f32 / 29.0).powi(2)) + 4.0 / 29.0 };
        let (fx, fy, fz) = (f(xyz[0] / D50[0]), f(xyz[1] / D50[1]), f(xyz[2] / D50[2]));
        [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
    }

    pub fn lab_to_srgb([l, a, b]: [f32; 3]) -> [f32; 3] {
        let fy = (l + 16.0) / 116.0;
        let fx = fy + a / 500.0;
        let fz = fy - b / 200.0;
        let finv = |t: f32| if t > 6.0 / 29.0 { t * t * t } else { 3.0 * (6.0f32 / 29.0).powi(2) * (t - 4.0 / 29.0) };
        let xyz = [finv(fx) * D50[0], finv(fy) * D50[1], finv(fz) * D50[2]];
        mul(&XYZ_D50_TO_SRGB, xyz).map(|lin| encode(lin.clamp(0.0, 1.0)))
    }
}

#[test]
fn lab_matches_photocraft_and_craft_cms() {
    let mut worst = 0.0f32;
    for c in samples() {
        let ours = c.to_lab();
        let pc = photocraft::srgb_to_lab([c.r, c.g, c.b]);
        for (o, p) in [ours.l, ours.a, ours.b].into_iter().zip(pc) {
            worst = worst.max((o - p).abs());
        }
        // craft-cms on the same linear XYZ.
        let lin = [c.r, c.g, c.b].map(|v| srgb_decode(f64::from(v)));
        let m = [[0.436_074_7, 0.385_064_9, 0.143_080_4], [0.222_504_5, 0.716_878_6, 0.060_616_9], [0.013_932_2, 0.097_104_5, 0.714_173_3]];
        let xyz = m.map(|r: [f64; 3]| r[0] * lin[0] + r[1] * lin[1] + r[2] * lin[2]);
        let cms = craft_cms::math::xyz_to_lab(xyz, [0.964_22, 1.0, 0.825_21]);
        for (o, p) in [ours.l, ours.a, ours.b].into_iter().zip(cms) {
            assert!((f64::from(o) - p).abs() < 1e-3, "{c:?}: {ours:?} vs {cms:?}");
        }
        // Lab to sRGB, including colours outside sRGB, which clamp.
        for lab in [ours, Lab { l: ours.l, a: ours.a * 2.0, b: ours.b * 2.0 }] {
            let back = Srgba::from_lab(lab);
            let pc = photocraft::lab_to_srgb([lab.l, lab.a, lab.b]);
            for (o, p) in [back.r, back.g, back.b].into_iter().zip(pc) {
                assert!((o - p).abs() < 2e-3, "{lab:?}: {back:?} vs {pc:?}");
            }
        }
    }
    assert!(worst < 2e-2, "worst Lab difference to PhotoCraft: {worst}");
}

#[test]
fn kept_values_survive_while_they_give_the_same_colour() {
    let [hsb, _, lab, _] = builtin_models();
    // A grey keeps the hue typed before.
    let grey = Srgba::rgb(0.5, 0.5, 0.5);
    let typed = ModelValues::new("hsb", &[200.0, 0.0, 50.0]);
    assert_eq!(kept_or_converted(hsb, grey, Some(&typed)), typed);
    assert_eq!(kept_or_converted(hsb, grey, None).values[0], 0.0);
    // A typed Lab colour outside sRGB stays as typed while it clamps to the same colour.
    let out = ModelValues::new("lab", &[60.0, 120.0, 120.0]);
    let clamped = lab.to_srgb(&out.values);
    assert_eq!(kept_or_converted(lab, clamped, Some(&out)), out);
    // Another colour, or another model, recomputes.
    assert_ne!(kept_or_converted(lab, Srgba::rgb(0.1, 0.2, 0.3), Some(&out)), out);
    assert_eq!(kept_or_converted(hsb, clamped, Some(&out)), hsb.from_srgb(clamped));
}
