//! The GPU display shaper (`Transform::gpu_matrix_shaper`) against an independent ICC
//! matrix/TRC reference.
//!
//! Fixtures are generated here from the published sRGB equation and a synthetic monitor, with
//! no photograph or device profile. The expected values implement ICC.1:2010 Annex F.3 directly
//! in f64, without Transform, Curve evaluation or craft_cms::math:
//! https://www.color.org/specification/ICC1v43_2010-12.pdf

use craft_cms::pipeline::{Pipeline, Stage};
use craft_cms::{Builtin, Clut, Curve, GpuMatrixShaper, Intent, Profile, Transform, TransformOptions};

const SOURCE_MATRIX: [[f64; 3]; 3] = [[0.4360747, 0.3850649, 0.1430804], [0.2225045, 0.7168786, 0.0606169], [0.0139322, 0.0971045, 0.7141733]];
// Deliberately synthetic, D50-referenced wider-gamut monitor; not a user's measured profile.
const MONITOR_MATRIX: [[f64; 3]; 3] = [[0.55, 0.27, 0.1442], [0.25, 0.69, 0.06], [0.01, 0.06, 0.7549]];
const GAMMA: f64 = 2.2;
const SOURCE_ENTRIES: usize = 1024;
const DISPLAY_TOLERANCE: f64 = 1.0 / 255.0;
const IDENTITY: [[f32; 4]; 3] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]];

fn fixed16(x: f64) -> f64 {
    (x * 65536.0).round() / 65536.0
}

fn srgb_sample(i: usize) -> f64 {
    let x = i as f64 / (SOURCE_ENTRIES - 1) as f64;
    let linear = if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) };
    (linear * 65535.0).round() / 65535.0
}

fn synthetic_transform(bpc: bool) -> Transform {
    let mut source = Builtin::Srgb.profile().clone();
    source.description = "Synthetic sampled sRGB for display regression".into();
    source.matrix = Some(SOURCE_MATRIX);
    source.trc = Some(std::array::from_fn(|_| Curve::Table((0..SOURCE_ENTRIES).map(|i| srgb_sample(i) as f32).collect())));
    let source = source.with_encoded_bytes();
    let mut monitor = Builtin::Srgb.profile().clone();
    monitor.description = "Synthetic gamma 2.2 display for regression".into();
    monitor.matrix = Some(MONITOR_MATRIX);
    monitor.trc = Some(std::array::from_fn(|_| Curve::Parametric { kind: 0, p: [GAMMA, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] }));
    let monitor = monitor.with_encoded_bytes();
    // Through ICC serialisation and parsing, with fixed-point matrix and TRC values.
    let (source, monitor) = (Profile::parse(&source.to_bytes()).unwrap(), Profile::parse(&monitor.to_bytes()).unwrap());
    Transform::new(&source, &monitor, Intent::RelativeColorimetric, bpc).unwrap()
}

fn reference(rgb: [f32; 3]) -> [f64; 3] {
    let linear = rgb.map(|x| {
        let p = f64::from(x.clamp(0.0, 1.0)) * (SOURCE_ENTRIES - 1) as f64;
        let i = (p as usize).min(SOURCE_ENTRIES - 2);
        srgb_sample(i) + (srgb_sample(i + 1) - srgb_sample(i)) * (p - i as f64)
    });
    let xyz = SOURCE_MATRIX.map(|row| row.iter().zip(linear).map(|(m, x)| fixed16(*m) * x).sum::<f64>());
    // Solve M_monitor · linear_monitor = XYZ by elimination, not through the production
    // matrix inverse.
    let mut a: [[f64; 4]; 3] =
        std::array::from_fn(|r| [fixed16(MONITOR_MATRIX[r][0]), fixed16(MONITOR_MATRIX[r][1]), fixed16(MONITOR_MATRIX[r][2]), xyz[r]]);
    for pivot in 0..3 {
        let divisor = a[pivot][pivot];
        for value in &mut a[pivot][pivot..] {
            *value /= divisor;
        }
        let normalized = a[pivot];
        for (r, row) in a.iter_mut().enumerate() {
            if r != pivot {
                let factor = row[pivot];
                for (value, coefficient) in row.iter_mut().zip(normalized).skip(pivot) {
                    *value -= factor * coefficient;
                }
            }
        }
    }
    std::array::from_fn(|c| a[c][3].clamp(0.0, 1.0).powf(1.0 / fixed16(GAMMA)))
}

/// The public table layout read on the CPU; expected colours come only from `reference`, so
/// this tests the sampled representation, not Transform against itself.
fn sample_gpu(gpu: &GpuMatrixShaper, rgb: [f32; 3]) -> [f32; 3] {
    fn interpolate(table: &[[f32; 4]], channel: usize, coordinate: f32) -> f32 {
        let i = (coordinate as usize).min(table.len() - 2);
        let f = coordinate - i as f32;
        table[i][channel] + (table[i + 1][channel] - table[i][channel]) * f
    }
    let linear: [f32; 3] =
        std::array::from_fn(|c| interpolate(gpu.input_table(), c, rgb[c].clamp(0.0, 1.0) * GpuMatrixShaper::INPUT_INTERVALS as f32));
    let rows = gpu.matrix_rows();
    std::array::from_fn(|c| {
        let row = rows[c];
        let value = row[0] * linear[0] + row[1] * linear[1] + row[2] * linear[2] + row[3];
        let coordinate = value.clamp(0.0, 1.0).sqrt().sqrt() * (GpuMatrixShaper::OUTPUT_ENTRIES - 1) as f32;
        interpolate(gpu.output_table(), c, coordinate).clamp(0.0, 1.0)
    })
}

fn check_reference(gpu: &GpuMatrixShaper, rgb: [f32; 3]) {
    let actual = sample_gpu(gpu, rgb);
    let expected = reference(rgb);
    for c in 0..3 {
        assert!((f64::from(actual[c]) - expected[c]).abs() <= DISPLAY_TOLERANCE, "input {rgb:?}, channel {c}: GPU {actual:?}, ICC {expected:?}");
    }
}

/// Guards the fourth-root output spacing and the input table range: near-black and coloured
/// shadows (inside a 33³ LUT's first cell) would otherwise fall back to LUT error, and 16-bit,
/// tiny and out-of-range inputs must clamp the same way.
#[test]
fn shaper_keeps_shadow_detail() {
    for bpc in [false, true] {
        let gpu = synthetic_transform(bpc).gpu_matrix_shaper().unwrap();
        assert_eq!(gpu.input_table().len(), GpuMatrixShaper::INPUT_INTERVALS + 1);
        assert_eq!(gpu.output_table().len(), GpuMatrixShaper::OUTPUT_ENTRIES);
        for v in 0..=255 {
            check_reference(&gpu, [v as f32 / 255.0; 3]);
        }
        for r in 0..=16 {
            for g in 0..=16 {
                for b in 0..=16 {
                    check_reference(&gpu, [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);
                }
            }
        }
        for v in [1_u16, 2, 15, 63, 257, 514, 1028, 4096, 32768, 65535] {
            check_reference(&gpu, [v as f32 / 65535.0; 3]);
        }
        for rgb in [[0.000003, 0.00001, 0.00002], [0.00123, 0.00456, 0.00789], [0.9, 0.6, 0.3], [-0.5, 0.0, 2.0], [1.0, 0.0, 0.0]] {
            check_reference(&gpu, rgb);
        }
        assert_eq!(sample_gpu(&gpu, [0.0; 3]), [0.0; 3]);
    }
}

/// Guards which transforms the GPU may take as a shaper: a CLUT, Lab, gray or proof transform
/// exposed as one would show wrong colours; an offset-only matrix must keep its offsets.
#[test]
fn shaper_needs_a_matrix_shaper_transform() {
    let identity = Transform::from_pipeline(Pipeline::new(3, Vec::new()).unwrap(), TransformOptions::default()).unwrap().gpu_matrix_shaper().unwrap();
    for rgb in [[0.0; 3], [1.0; 3], [0.001, 0.2, 0.8], [-2.0, 0.5, 2.0]] {
        for (actual, input) in sample_gpu(&identity, rgb).into_iter().zip(rgb) {
            assert!((actual - input.clamp(0.0, 1.0)).abs() < 1e-6);
        }
    }
    let matrix = Stage::Matrix { rows: 3, cols: 3, m: vec![0.8, 0.0, 0.0, 0.0, 0.7, 0.0, 0.0, 0.0, 0.6], offset: vec![0.01, 0.02, 0.03] };
    let offsets =
        Transform::from_pipeline(Pipeline::new(3, vec![matrix]).unwrap(), TransformOptions::default()).unwrap().gpu_matrix_shaper().unwrap();
    let output = sample_gpu(&offsets, [0.1, 0.2, 0.3]);
    for (actual, expected) in output.into_iter().zip([0.09, 0.16, 0.21]) {
        assert!((actual - expected).abs() < 1e-6, "{output:?}");
    }

    let identity_clut = Clut::sample(vec![2; 3], 3, |input, output| output.copy_from_slice(input)).unwrap();
    let clut = Transform::from_pipeline(Pipeline::new(3, vec![Stage::Clut(identity_clut)]).unwrap(), TransformOptions::default()).unwrap();
    assert!(clut.gpu_matrix_shaper().is_none());
    let srgb = Builtin::Srgb.profile();
    assert!(Transform::new(Builtin::LabD50.profile(), srgb, Intent::RelativeColorimetric, false).unwrap().gpu_matrix_shaper().is_none());
    assert!(Transform::new(Builtin::GrayGamma22.profile(), srgb, Intent::RelativeColorimetric, false).unwrap().gpu_matrix_shaper().is_none());
    let proof = Transform::proof(srgb, Builtin::DisplayP3.profile(), srgb, Intent::RelativeColorimetric, false, false).unwrap();
    assert!(proof.gpu_matrix_shaper().is_none());
}

/// Guards the validation before upload: wrong table sizes, non-finite values, malformed matrix
/// storage and finite values that overflow the shader's arithmetic never reach the GPU.
#[test]
fn shaper_rejects_what_overflows_the_shader() {
    let ramp: Vec<f32> = (0..=GpuMatrixShaper::INPUT_INTERVALS).map(|i| i as f32 / GpuMatrixShaper::INPUT_INTERVALS as f32).collect();
    let inputs: [Vec<f32>; 3] = std::array::from_fn(|_| ramp.clone());
    let outputs: [Vec<f32>; 3] = std::array::from_fn(|_| vec![0.0; GpuMatrixShaper::OUTPUT_ENTRIES]);
    assert!(GpuMatrixShaper::from_tables(IDENTITY, &inputs, &outputs).is_some());
    let mut short = inputs.clone();
    short[0].pop();
    assert!(GpuMatrixShaper::from_tables(IDENTITY, &short, &outputs).is_none());
    for v in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut bad = inputs.clone();
        bad[1][123] = v;
        assert!(GpuMatrixShaper::from_tables(IDENTITY, &bad, &outputs).is_none());
        let mut bad = outputs.clone();
        bad[2][321] = v;
        assert!(GpuMatrixShaper::from_tables(IDENTITY, &inputs, &bad).is_none());
        let mut bad = IDENTITY;
        bad[1][3] = v;
        assert!(GpuMatrixShaper::from_tables(bad, &inputs, &outputs).is_none());
    }
    let mut huge = IDENTITY;
    huge[0] = [f32::MAX; 4];
    assert!(GpuMatrixShaper::from_tables(huge, &inputs, &outputs).is_none());
    let mut steep = inputs.clone();
    steep[0][100] = -f32::MAX;
    steep[0][101] = f32::MAX;
    assert!(GpuMatrixShaper::from_tables(IDENTITY, &steep, &outputs).is_none());
    for (m, offset) in [(vec![2.0; 8], vec![0.0; 3]), (vec![2.0; 9], vec![0.0; 2]), (vec![f64::INFINITY; 9], vec![0.0; 3])] {
        let Ok(pipeline) = Pipeline::new(3, vec![Stage::Matrix { rows: 3, cols: 3, m, offset }]) else { continue };
        if let Ok(t) = Transform::from_pipeline(pipeline, TransformOptions::default()) {
            assert!(t.gpu_matrix_shaper().is_none());
        }
    }
}
