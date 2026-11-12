//! The WGSL formulas against the CPU's, per variant. References are computed, none stored.
//! Skips without an adapter.

use craft_blend::{Alpha, Blend, BlendMode, EDGE, Layer, Linear, Range, Srgb, TextGamma, Variant};
use craft_gpu_testkit::{Gpu, PROBE_BINDINGS, close, gpu};

const VARIANTS: [Variant; 4] = [Variant::Standard, Variant::Photoshop, Variant::AfterEffects, Variant::Substance];

/// Relative tolerance: the GPU's division, `pow` and `sqrt` are within a few ulps, not exact.
const TOL: f32 = 1e-5;

/// The modes whose result jumps at a threshold: a one-ulp difference before the comparison may
/// flip them. Up to 1% of their cases may differ.
fn discontinuous(m: BlendMode) -> bool {
    matches!(m, BlendMode::HardMix | BlendMode::DarkerColor | BlendMode::LighterColor | BlendMode::Xor)
}

/// Exact 0 and 1, values within an ulp and within EDGE of the jumps at 0, ½ and 1, the interior,
/// over-range light and a negative value.
fn values() -> [f32; 14] {
    [0.0, 1e-5, EDGE, EDGE * 2.0, 0.25, 0.5f32.next_down(), 0.5, 0.5f32.next_up(), 0.75, 1.0 - EDGE, 1.0f32.next_down(), 1.0, 2.0, -0.5]
}

const FORMULA_PROBE: &str = r#"
@compute @workgroup_size(64)
fn probe(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i * 8u >= arrayLength(&probe_in)) { return; }
    let o = i * 8u;
    let mode = u32(probe_in[o]);
    let cb = vec3<f32>(probe_in[o + 1u], probe_in[o + 2u], probe_in[o + 3u]);
    let cs = vec3<f32>(probe_in[o + 4u], probe_in[o + 5u], probe_in[o + 6u]);
    let r = blend_rgb(mode, cb, cs, probe_in[o + 7u] > 0.5);
    probe_out[i * 3u] = r.x;
    probe_out[i * 3u + 1u] = r.y;
    probe_out[i * 3u + 2u] = r.z;
}
"#;

/// Collects the cases that differ, and fails with them at the end.
struct Report {
    failures: Vec<String>,
    discontinuous: (usize, usize),
}

impl Report {
    fn new() -> Report {
        Report { failures: Vec::new(), discontinuous: (0, 0) }
    }

    fn check(&mut self, mode: BlendMode, what: impl Fn() -> String, cpu: &[f32], gpu: &[f32]) {
        let ok = cpu.iter().zip(gpu).all(|(c, g)| close(*c, *g, TOL));
        if discontinuous(mode) {
            self.discontinuous.0 += 1;
            if !ok {
                self.discontinuous.1 += 1;
            }
        } else if !ok {
            self.failures.push(format!("{mode:?} {}: CPU {cpu:?} GPU {gpu:?}", what()));
        }
    }

    fn finish(self, g: &Gpu) {
        let (n, bad) = self.discontinuous;
        assert!(bad * 100 <= n, "{}: {bad} of {n} threshold-mode cases differ", g.name);
        let shown: Vec<&String> = self.failures.iter().take(40).collect();
        assert!(self.failures.is_empty(), "{}: {} cases differ\n{shown:#?}", g.name, self.failures.len());
    }
}

/// Every mode of every variant at both ranges on the value grid: channel 0 runs over the grid,
/// channels 1 and 2 are fixed and different, so the non-separable modes see real colours.
#[test]
fn formulas_match_cpu() {
    let Some(g) = gpu() else { return };
    let mut report = Report::new();
    for variant in VARIANTS {
        let mut input = Vec::new();
        let mut cases = Vec::new();
        for &mode in BlendMode::ALL {
            for range in [Range::Unit, Range::Float] {
                for b in values() {
                    for s in values() {
                        let (cb, cs) = ([b, 0.3, 0.8], [s, 0.6, 0.1]);
                        input.extend([mode.id() as f32, cb[0], cb[1], cb[2], cs[0], cs[1], cs[2], if range == Range::Float { 1.0 } else { 0.0 }]);
                        cases.push((mode, range, cb, cs));
                    }
                }
            }
        }
        let src = craft_wgsl::assemble(&[craft_blend::wgsl::module(variant)], &[PROBE_BINDINGS, FORMULA_PROBE]);
        let out = g.run(&src, "probe", &input, cases.len() * 3, cases.len() as u32);
        for (i, (mode, range, cb, cs)) in cases.into_iter().enumerate() {
            let cpu = Blend::new(variant, range).rgb(mode, cb, cs);
            report.check(mode, || format!("{variant:?} {range:?} cb {cb:?} cs {cs:?}"), &cpu, &out[i * 3..i * 3 + 3]);
        }
    }
    report.finish(&g);
}

const COMPOSITE_PROBE: &str = r#"
@compute @workgroup_size(64)
fn probe(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i * 16u >= arrayLength(&probe_in)) { return; }
    let o = i * 16u;
    let kind = u32(probe_in[o]);
    let mode = u32(probe_in[o + 1u]);
    let b = vec4<f32>(probe_in[o + 2u], probe_in[o + 3u], probe_in[o + 4u], probe_in[o + 5u]);
    let s = vec4<f32>(probe_in[o + 6u], probe_in[o + 7u], probe_in[o + 8u], probe_in[o + 9u]);
    let opacity = probe_in[o + 10u];
    let fill = probe_in[o + 11u];
    let noise = probe_in[o + 12u];
    let float_range = probe_in[o + 13u] > 0.5;
    let extra = probe_in[o + 14u];
    var r: vec4<f32>;
    if (kind == 0u) {
        r = blend_straight(mode, b, s, opacity, fill, noise, float_range, extra);
    } else if (kind == 1u) {
        r = blend_premul(mode, b, s, noise, float_range, extra > 0.5);
    } else {
        r = blend_atop(mode, b, s, opacity, float_range);
    }
    probe_out[i * 4u] = r.x;
    probe_out[i * 4u + 1u] = r.y;
    probe_out[i * 4u + 2u] = r.z;
    probe_out[i * 4u + 3u] = r.w;
}
"#;

/// One composite case: kind (0 straight, 1 premultiplied, 2 atop), the layer, the pixels, the
/// range and the kind's extra argument (text gamma, or the sRGB blend space as 1).
struct Case {
    kind: u32,
    layer: Layer,
    b: [f32; 4],
    s: [f32; 4],
    range: Range,
    extra: f32,
}

impl Case {
    fn input(&self) -> [f32; 16] {
        let l = &self.layer;
        let r = if self.range == Range::Float { 1.0 } else { 0.0 };
        let [b0, b1, b2, b3] = self.b;
        let [s0, s1, s2, s3] = self.s;
        [self.kind as f32, l.mode.id() as f32, b0, b1, b2, b3, s0, s1, s2, s3, l.opacity, l.fill, l.noise, r, self.extra, 0.0]
    }

    fn cpu(&self, variant: Variant) -> [f32; 4] {
        let blend = Blend::new(variant, self.range);
        match self.kind {
            0 if self.extra > 0.0 => blend.composite(&self.layer, Alpha::Straight, self.b, self.s, (&Linear, &TextGamma(self.extra))),
            0 => blend.composite(&self.layer, Alpha::Straight, self.b, self.s, (&Linear, &Linear)),
            1 if self.extra > 0.5 => blend.composite(&self.layer, Alpha::Premultiplied, self.b, self.s, (&Srgb, &Linear)),
            1 => blend.composite(&self.layer, Alpha::Premultiplied, self.b, self.s, (&Linear, &Linear)),
            _ => blend.composite_atop(self.layer.mode, self.b, self.s, self.layer.opacity),
        }
    }
}

fn premultiplied(p: [f32; 4]) -> [f32; 4] {
    [p[0] * p[3], p[1] * p[3], p[2] * p[3], p[3]]
}

fn composite_cases() -> Vec<Case> {
    let backdrops = [[0.2, 0.4, 0.6, 1.0], [0.7, 0.1, 0.9, 0.5], [0.5, 0.5, 0.5, 0.0]];
    let sources = [[0.8, 0.3, 0.1, 1.0], [0.1, 0.6, 0.4, 0.6], [0.3, 0.3, 0.9, 0.25]];
    let mut cases = Vec::new();
    for &mode in BlendMode::ALL {
        for range in [Range::Unit, Range::Float] {
            for b in backdrops {
                for s in sources {
                    for (opacity, fill) in [(1.0, 1.0), (0.7, 1.0), (1.0, 0.5), (0.8, 0.25)] {
                        // Noise 0.3 shows Dissolve where coverage is above it and hides it elsewhere.
                        let layer = Layer { mode, opacity, fill, noise: 0.3 };
                        for gamma in [0.0, 1.45] {
                            cases.push(Case { kind: 0, layer, b, s, range, extra: gamma });
                        }
                        cases.push(Case { kind: 2, layer, b, s, range, extra: 0.0 });
                    }
                    // Premultiplied sources carry their opacity already.
                    let layer = Layer { mode, opacity: 1.0, fill: 1.0, noise: 0.3 };
                    for srgb in [0.0, 1.0] {
                        cases.push(Case { kind: 1, layer, b: premultiplied(b), s: premultiplied(s), range, extra: srgb });
                    }
                }
            }
        }
    }
    cases
}

/// Straight composites with Opacity, Fill (Photoshop's special eight), text gamma and Dissolve;
/// premultiplied composites in linear and sRGB blend spaces with After Effects' alpha and
/// Classic modes; atop.
#[test]
fn composites_match_cpu() {
    let Some(g) = gpu() else { return };
    let cases = composite_cases();
    let input: Vec<f32> = cases.iter().flat_map(|c| c.input()).collect();
    let mut report = Report::new();
    for variant in VARIANTS {
        let src = craft_wgsl::assemble(&[craft_blend::wgsl::module(variant)], &[PROBE_BINDINGS, COMPOSITE_PROBE]);
        let out = g.run(&src, "probe", &input, cases.len() * 4, cases.len() as u32);
        for (i, c) in cases.iter().enumerate() {
            let what = || format!("{variant:?} kind {} {:?} {:?} b {:?} s {:?} extra {}", c.kind, c.range, c.layer, c.b, c.s, c.extra);
            report.check(c.layer.mode, what, &c.cpu(variant), &out[i * 4..i * 4 + 4]);
        }
    }
    report.finish(&g);
}
