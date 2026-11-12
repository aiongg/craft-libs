//! The WGSL display pass on a real device, held to its CPU reference `DisplayPass::pixel`.
//! Skips without an adapter (CI without a GPU; naga validation runs in the unit tests).
//!
//! `display_pixel` runs in a compute probe: the case's `DisplayParams` (from `gpu::params`), its
//! shaper tables and LUT, and the pixels go in one input buffer, and the probe defines
//! `display_table` and `display_lut` over it.

use std::sync::Arc;

use craft_cms::{Builtin, GamutCheck, Intent, Lut3d, Transform};
use craft_display::gpu::{self, DISPLAY};
use craft_display::{Channel, Curve, DisplayLut, DisplayPass, DisplayTransform, FrameEncoding, GamutOverlay, HdrStep, PassColor, shaper_from_curves};
use craft_gpu_testkit::{Gpu, PROBE_BINDINGS, WORKGROUP, gpu};

/// Floats of `DisplayParams` in the probe's input: 8 ids, then 32 values.
const PARAM_FLOATS: usize = 40;
/// Floats of one case's header: its params, then where its shaper tables and LUT start and the
/// LUT's size.
const CASE_FLOATS: usize = PARAM_FLOATS + 4;

/// One probe for every case over the same pixels: invocation `i` runs case `i / pixels` on pixel
/// `i % pixels`. Returns the WGSL and the input: the case headers, each display colour's tables
/// and LUT once, then the pixels.
fn probe(cases: &[(DisplayPass, usize)], colors: &[Option<PassColor>], pixels: &[[f32; 4]]) -> (String, Vec<f32>) {
    let mut data: Vec<f32> = Vec::new();
    let base = cases.len() * CASE_FLOATS;
    let mut placed = Vec::new();
    for color in colors {
        let table = base + data.len();
        data.extend(gpu::table_entries(color.as_ref()).iter().flatten());
        let lut = base + data.len();
        let size = match color {
            Some(PassColor::Lut { lut, .. }) => {
                data.extend(lut.data().iter().flatten());
                lut.size()
            }
            _ => 1,
        };
        placed.push([table as f32, lut as f32, size as f32]);
    }
    let mut input = Vec::with_capacity(base + data.len() + pixels.len() * 4);
    for (pass, color) in cases {
        let bytes = gpu::params(pass);
        let word = |i: usize| [bytes[4 * i], bytes[4 * i + 1], bytes[4 * i + 2], bytes[4 * i + 3]];
        // The ids as exact small floats (a bitcast of their bits could meet denormal flushing).
        input.extend((0..8).map(|i| u32::from_le_bytes(word(i)) as f32));
        input.extend((8..PARAM_FLOATS).map(|i| f32::from_le_bytes(word(i))));
        input.extend(placed[*color]);
        input.push(0.0);
    }
    input.extend(data);
    let pixel_base = input.len();
    input.extend(pixels.iter().flatten());
    let wgsl = format!(
        "{PROBE_BINDINGS}\n\
         var<private> case_base: u32;\n\
         fn probe_u(i: u32) -> u32 {{ return u32(probe_in[i]); }}\n\
         fn probe_v(i: u32) -> vec4<f32> {{ return vec4<f32>(probe_in[i], probe_in[i + 1u], probe_in[i + 2u], probe_in[i + 3u]); }}\n\
         fn probe_params(b: u32) -> DisplayParams {{\n\
         \x20   return DisplayParams(\n\
         \x20       vec4<u32>(probe_u(b), probe_u(b + 1u), probe_u(b + 2u), probe_u(b + 3u)),\n\
         \x20       vec4<u32>(probe_u(b + 4u), probe_u(b + 5u), probe_u(b + 6u), probe_u(b + 7u)),\n\
         \x20       probe_v(b + 8u), probe_v(b + 12u),\n\
         \x20       array<vec4<f32>, 3>(probe_v(b + 16u), probe_v(b + 20u), probe_v(b + 24u)),\n\
         \x20       array<vec4<f32>, 3>(probe_v(b + 28u), probe_v(b + 32u), probe_v(b + 36u)),\n\
         \x20   );\n\
         }}\n\
         fn display_table(i: u32) -> vec3<f32> {{ return probe_v(probe_u(case_base + {PARAM_FLOATS}u) + i * 4u).xyz; }}\n\
         fn display_lut(r: u32, g: u32, b: u32) -> vec4<f32> {{\n\
         \x20   let n = probe_u(case_base + {PARAM_FLOATS}u + 2u);\n\
         \x20   return probe_v(probe_u(case_base + {PARAM_FLOATS}u + 1u) + (r + g * n + b * n * n) * 4u);\n\
         }}\n\
         @compute @workgroup_size({WORKGROUP})\n\
         fn probe(@builtin(global_invocation_id) id: vec3<u32>) {{\n\
         \x20   let i = id.x;\n\
         \x20   if (i >= {total}u) {{ return; }}\n\
         \x20   case_base = (i / {count}u) * {CASE_FLOATS}u;\n\
         \x20   let o = display_pixel(probe_params(case_base), probe_v({pixel_base}u + (i % {count}u) * 4u));\n\
         \x20   probe_out[i * 4u] = o.x;\n\
         \x20   probe_out[i * 4u + 1u] = o.y;\n\
         \x20   probe_out[i * 4u + 2u] = o.z;\n\
         \x20   probe_out[i * 4u + 3u] = o.w;\n\
         }}\n",
        count = pixels.len(),
        total = pixels.len() * cases.len(),
    );
    (craft_wgsl::assemble(&[&DISPLAY], &[&wgsl]), input)
}

/// Premultiplied test pixels: greys and colours over 0..1, over-range values for float frames,
/// and partial alpha.
fn pixels() -> Vec<[f32; 4]> {
    let mut px = Vec::new();
    for i in 0..=32 {
        let v = i as f32 / 32.0;
        px.push([v, v, v, 1.0]);
        px.push([v, 1.0 - v, (v * 7.0).fract(), 1.0]);
        px.push([v * 0.5, 0.25, 0.4 * v, 0.5]);
        px.push([v * 4.0, v * 2.0, v * 9.0, 1.0]);
        px.push([0.0, v * 0.01, v * 0.03, v]);
    }
    px
}

/// A software rasterizer (lavapipe, llvmpipe, WARP, SwiftShader, Apple's paravirtual device):
/// its pow, exp and division round differently, so exact comparisons allow 1 LSB on up to 1% of
/// bytes there.
fn software(g: &Gpu) -> bool {
    let n = g.name.to_lowercase();
    ["llvmpipe", "lavapipe", "swiftshader", "paravirtual", "warp", "basic render"].iter().any(|s| n.contains(s))
}

/// Guards the shader against its CPU reference: a WGSL edit that leaves `DisplayPass::pixel`
/// behind (or the reverse) in any channel view, frame curve, HDR step, preview gamma or display
/// colour form.
#[test]
fn the_shader_is_the_reference() {
    let Some(g) = gpu() else { return };
    let srgb = Arc::new(Builtin::Srgb.profile().clone());
    let p3 = Arc::new(Builtin::DisplayP3.profile().clone());
    let shaper = DisplayTransform::new(srgb.clone(), p3).unwrap().pass_color().unwrap().unwrap();
    let to_device = shaper_from_curves(Curve::Srgb, Some([[0.8, 0.15, 0.05], [0.1, 0.85, 0.05], [0.02, 0.08, 0.9]]), Curve::Gamma(2.2)).unwrap();
    let from_device = shaper_from_curves(Curve::Gamma(2.2), None, Curve::Srgb).unwrap();
    let simulated = PassColor::Shapers { first: Arc::new(to_device), quantize: true, second: Some(Arc::new(from_device)) };
    let cmyk = Builtin::CoatedCmyk.profile();
    let proof = Transform::proof(&srgb, cmyk, &srgb, Intent::RelativeColorimetric, true, false).unwrap();
    let check = GamutCheck::new(&srgb, cmyk, craft_cms::gamut::DEFAULT_THRESHOLD).unwrap();
    let lut = DisplayLut::new(&Lut3d::from_transform(&proof, 17).unwrap(), Some(&check)).unwrap();
    let proofed = PassColor::Lut { lut: Arc::new(lut), gamut: Some(GamutOverlay { color: [0.5, 0.5, 0.5], opacity: 1.0 }) };

    let colors = [None, Some(shaper), Some(simulated), Some(proofed)];
    let encodings = [
        FrameEncoding { curve: Curve::Srgb, hdr: HdrStep::None },
        FrameEncoding { curve: Curve::Srgb, hdr: HdrStep::Compand },
        FrameEncoding { curve: Curve::Gamma(2.4), hdr: HdrStep::ToneMap },
        FrameEncoding { curve: Curve::Pq, hdr: HdrStep::None },
        FrameEncoding { curve: Curve::Hlg, hdr: HdrStep::None },
        FrameEncoding { curve: Curve::Linear, hdr: HdrStep::None },
    ];
    let channels = [(Channel::Rgb, false), (Channel::RgbStraight, false), (Channel::Red, true), (Channel::Blue, false), (Channel::Alpha, false)];
    let mut cases = Vec::new();
    for (c, color) in colors.iter().enumerate() {
        for encoding in encodings {
            for (channel, colorized) in channels {
                for (stops, gamma) in [(0.0, 1.0), (1.5, 1.0), (-2.0, 2.2)] {
                    let mut pass = DisplayPass::new(channel, colorized, stops, encoding, color.clone());
                    pass.gamma = gamma;
                    // The pass drops the display colour on single-channel views.
                    let c = if pass.color.is_some() { c } else { 0 };
                    cases.push((pass, c));
                }
            }
        }
    }
    let px = pixels();
    let (src, input) = probe(&cases, &colors, &px);
    let total = px.len() * cases.len();
    let out = g.run(&src, "probe", &input, total * 4, total as u32);
    let lenient = software(&g);
    for (n, (pass, _)) in cases.iter().enumerate() {
        let (mut off, mut worst) = (0usize, 0i32);
        for (i, p) in px.iter().enumerate() {
            let want = pass.pixel(*p);
            for k in 0..4 {
                let got = (out[(n * px.len() + i) * 4 + k] * 255.0).round() as i32;
                let d = (got - want[k] as i32).abs();
                if d > 0 {
                    off += 1;
                    worst = worst.max(d);
                }
            }
        }
        let bytes = px.len() * 4;
        let ok = off == 0 || lenient && worst <= 1 && off * 100 <= bytes;
        assert!(
            ok,
            "{}: {off} of {bytes} bytes differ (at most {worst}) for {:?} {:?} gain {} gamma {} {}",
            g.name,
            pass.channel,
            pass.encoding,
            pass.gain,
            pass.gamma,
            describe(&pass.color)
        );
    }
}

fn describe(color: &Option<PassColor>) -> &'static str {
    match color {
        None => "no display colour",
        Some(PassColor::Shapers { second: None, .. }) => "one shaper",
        Some(PassColor::Shapers { .. }) => "simulated device",
        Some(PassColor::Lut { .. }) => "proof LUT with gamut warning",
    }
}
