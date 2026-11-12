//! The prelude against its Rust twins on the GPU. Skips without an adapter.

use craft_gpu_testkit::{PROBE_BINDINGS, close, gpu};
use craft_wgsl::{COLOR, MATH, TRANSFER, assemble};

const PROBE: &str = r#"
@compute @workgroup_size(64)
fn probe(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i * 4u >= arrayLength(&probe_in)) { return; }
    let op = u32(probe_in[i * 4u]);
    let a = probe_in[i * 4u + 1u];
    let b = probe_in[i * 4u + 2u];
    let c = probe_in[i * 4u + 3u];
    var r = vec3<f32>(0.0);
    switch op {
        case 0u: { r = vec3<f32>(sincos_p(a), 0.0); }
        case 1u: { r.x = asin_p(a); r.y = acos_p(a); }
        case 2u: { r.x = round_away(a); }
        case 3u: { r.x = unit_div(a, b); }
        case 4u: { r.x = srgb_to_linear(a); r.y = linear_to_srgb(a); }
        case 5u: { r.x = pq_decode(a); r.y = pq_encode(b); }
        case 6u: { r.x = hlg_decode(a); r.y = hlg_encode(b); }
        case 7u: { r = rgb_to_hsl(vec3<f32>(a, b, c)); }
        case 8u: { r = hsl_to_rgb(a, b, c); }
        case 9u: { r.x = hash_noise(u32(a), u32(b), u32(c)); }
        default: {}
    }
    probe_out[i * 3u] = r.x;
    probe_out[i * 3u + 1u] = r.y;
    probe_out[i * 3u + 2u] = r.z;
}
"#;

/// One case: the op, its inputs, the Rust twin's three outputs and the tolerance (0 = equal).
struct Case {
    op: u32,
    args: [f32; 3],
    want: [f32; 3],
    tol: f32,
}

fn cases() -> Vec<Case> {
    let mut v = Vec::new();
    let mut push = |op, args: [f32; 3], want, tol| v.push(Case { op, args, want, tol });
    // Mesa's GL trig is coarse and some hardware saturates cos near a full turn: particle and VR
    // directions drift. f64 is the reference.
    for extent in [std::f32::consts::TAU, 100.0, 8192.0] {
        for i in (0..=4096).step_by(7) {
            let x = (i as f32 - 2048.0) * extent / 2048.0;
            push(0, [x, 0.0, 0.0], [(x as f64).sin() as f32, (x as f64).cos() as f32, 0.0], 2e-7);
        }
    }
    for i in -100..=100 {
        let x = i as f32 / 100.0;
        push(1, [x, 0.0, 0.0], [(x as f64).asin() as f32, (x as f64).acos() as f32, 0.0], 1e-6);
    }
    // f32::round from 2^23 up, where every value is already an integer (8388608 and 8388609 must
    // not merge), and a half that a fused product moved below .5 (5/6 · 3, #1605).
    for m in [1.0f32, -1.0] {
        for x in [
            0.0f32,
            0.5,
            1.5,
            2.5,
            0.499_999_97,
            4_194_303.5,
            8_388_607.5,
            8_388_608.0,
            8_388_609.0,
            8_388_610.0,
            16_777_216.0,
            3.0e9,
            5.0 / 6.0 * 3.0,
        ] {
            push(2, [m * x, 0.0, 0.0], [(m * x).round(), 0.0, 0.0], 0.0);
        }
    }
    // Exact alpha division: posterize and threshold effects need the CPU's f32 quotient.
    for a in [1.0f32, 0.1, 0.3, 1.0 / 255.0, 6.0 / 255.0] {
        for levels in [3u32, 8, 254] {
            for i in 0..levels {
                let half = (i as f32 + 0.5) / levels as f32;
                for x in [0.0, 1e-6, half.next_down(), half, half.next_up(), 1.0] {
                    push(3, [x * a, a, 0.0], [((x * a) / a).clamp(0.0, 1.0), 0.0, 0.0], 0.0);
                }
            }
        }
    }
    for i in -10..=60 {
        let x = i as f32 / 40.0;
        push(4, [x, 0.0, 0.0], [craft_wgsl::srgb_to_linear(x), craft_wgsl::linear_to_srgb(x), 0.0], 2e-6);
        let (e, l) = (x.clamp(0.0, 1.0), i as f32 / 10.0);
        push(5, [e, l, 0.0], [craft_wgsl::pq_decode(e), craft_wgsl::pq_encode(l), 0.0], 2e-5);
        push(6, [e, l, 0.0], [craft_wgsl::hlg_decode(e), craft_wgsl::hlg_encode(l), 0.0], 2e-6);
    }
    // HSL on in-range, grey, over-range and negative colours (the guarded denominator).
    for c in [[0.2f32, 0.5, 0.9], [0.9, 0.2, 0.4], [0.5, 0.5, 0.5], [2.0, 0.5, 1.0], [-0.2, 0.3, 0.1], [0.1, 0.8, 0.8], [1.0, 1.0, 0.0]] {
        push(7, c, craft_wgsl::rgb_to_hsl(c), 2e-6);
        let u = c.map(|v| v.clamp(0.0, 1.0));
        push(8, u, craft_wgsl::hsl_to_rgb(u[0], u[1], u[2]), 2e-6);
    }
    for (x, y, s) in [(0u32, 0u32, 0u32), (17, 3, 9), (1023, 767, 1), (4096, 1, 77)] {
        push(9, [x as f32, y as f32, s as f32], [craft_wgsl::hash_noise(x, y, s), 0.0, 0.0], 0.0);
    }
    v
}

#[test]
fn prelude_matches_cpu() {
    let Some(g) = gpu() else { return };
    let cases = cases();
    let input: Vec<f32> = cases.iter().flat_map(|c| [c.op as f32, c.args[0], c.args[1], c.args[2]]).collect();
    let src = assemble(&[&MATH, &COLOR, &TRANSFER], &[PROBE_BINDINGS, PROBE]);
    let out = g.run(&src, "probe", &input, cases.len() * 3, cases.len() as u32);
    let mut failures = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        for (k, want) in c.want.iter().enumerate() {
            let got = out[i * 3 + k];
            let ok = if c.tol == 0.0 { got == *want } else { close(*want, got, c.tol) };
            if !ok {
                failures.push(format!("op {} {:?}[{k}]: CPU {want} GPU {got}", c.op, c.args));
            }
        }
    }
    assert!(failures.is_empty(), "{}: {} mismatches\n{}", g.name, failures.len(), failures.join("\n"));
}
