// craft-display: the display pass. `display_pixel` turns one premultiplied texel, encoded with the
// frame's curve, into the premultiplied output the view shows: unpremultiply → decode, × gain,
// HDR step, ^(1 / gamma), encode → clamp → display colour → Show Channel, premultiply →
// floor(v · 255 + 0.5) / 255. Line by line the operations of `craft_display::DisplayPass::pixel`,
// its CPU reference and fallback.
//
// The including shader defines the two lookups (craft_display::gpu::texture_access writes them
// over textures; a test probe over a buffer):
//   fn display_table(i: u32) -> vec3<f32>                  entry i of the shaper tables
//   fn display_lut(r: u32, g: u32, b: u32) -> vec4<f32>    a LUT node; alpha 1 = out of gamut
// Uses TRANSFER (curve_decode, curve_encode), COLOR (luminance) and MATH (powz).

struct DisplayParams {
    // channel (0 RGB, 1 R, 2 G, 3 B, 4 A, 5 RGB Straight), colorized, frame curve id, HDR step
    // (0 none, 1 compand, 2 tone map)
    ids: vec4<u32>,
    // display colour (0 none, 1 shapers, 2 LUT), 8-bit device numbers between the shapers,
    // a second shaper, LUT size
    color: vec4<u32>,
    // gain (2^stops), the frame curve's decode exponent, its encode exponent (1 / gamma, divided
    // on the CPU so both sides use the same bits), 1 / the preview gamma
    light: vec4<f32>,
    // Gamut Warning colour, opacity (0: off)
    gamut: vec4<f32>,
    // the shapers' matrix rows (three coefficients and an offset)
    m1: array<vec4<f32>, 3>,
    m2: array<vec4<f32>, 3>,
};

// GpuMatrixShaper: INPUT_INTERVALS + 1 input entries, then OUTPUT_ENTRIES output entries.
const SHAPER_INPUT: u32 = 4097u;
const SHAPER_ENTRIES: u32 = 8193u;

// The frame curve: id 2 is a power curve with its exponent; the others are craft-wgsl's.
fn pass_decode(curve: u32, g: f32, v: f32) -> f32 {
    if (curve == 2u) {
        let l = powz(abs(v), g);
        return select(l, -l, v < 0.0);
    }
    return curve_decode(curve, v);
}

fn pass_encode(curve: u32, e: f32, v: f32) -> f32 {
    if (curve == 2u) {
        let o = powz(abs(v), e);
        return select(o, -o, v < 0.0);
    }
    return curve_encode(curve, v);
}

fn pass_decode3(curve: u32, g: f32, c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(pass_decode(curve, g, c.x), pass_decode(curve, g, c.y), pass_decode(curve, g, c.z));
}

fn pass_encode3(curve: u32, e: f32, c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(pass_encode(curve, e, c.x), pass_encode(curve, e, c.y), pass_encode(curve, e, c.z));
}

fn pass_compand(v: f32) -> f32 {
    let k = 0.8;
    if (v <= k) {
        return v;
    }
    return k + (1.0 - k) * (1.0 - exp(-(v - k) / (1.0 - k)));
}

fn pass_tone_map(c: vec3<f32>) -> vec3<f32> {
    let w = 4.0;
    let l = luminance(c);
    if (l <= 0.0) {
        return c;
    }
    let lm = l * (1.0 + l / (w * w)) / (1.0 + l);
    let k = lm / l;
    return c * k;
}

// One table curve: `c` clamped to 0..1 over `intervals` steps, each channel between its two
// neighbouring entries (loads and explicit interpolation: no float filtering needed).
fn shaper_curve(c: vec3<f32>, offset: u32, intervals: f32) -> vec3<f32> {
    let p = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)) * intervals;
    let lo = vec3<u32>(floor(p));
    let hi = min(lo + vec3<u32>(1u), vec3<u32>(u32(intervals)));
    let f = p - floor(p);
    return vec3<f32>(
        mix(display_table(offset + lo.x).x, display_table(offset + hi.x).x, f.x),
        mix(display_table(offset + lo.y).y, display_table(offset + hi.y).y, f.y),
        mix(display_table(offset + lo.z).z, display_table(offset + hi.z).z, f.z),
    );
}

// A shaper conversion: input curves, matrix rows with offsets, output curves indexed by the
// fourth root (which keeps shadow detail where a gamma curve's inverse is steepest).
fn shaper_apply(c: vec3<f32>, base: u32, m: array<vec4<f32>, 3>) -> vec3<f32> {
    let l = shaper_curve(c, base, 4096.0);
    let linear = vec3<f32>(
        m[0].x * l.x + m[0].y * l.y + m[0].z * l.z + m[0].w,
        m[1].x * l.x + m[1].y * l.y + m[1].z * l.z + m[1].w,
        m[2].x * l.x + m[2].y * l.y + m[2].z * l.z + m[2].w,
    );
    let u = sqrt(sqrt(clamp(linear, vec3<f32>(0.0), vec3<f32>(1.0))));
    return shaper_curve(u, base + SHAPER_INPUT, 4095.0);
}

// Trilinear in the CPU's corner order (craft_display::DisplayLut::eval).
fn pass_lut(n: u32, c: vec3<f32>) -> vec4<f32> {
    let x = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)) * f32(n - 1u);
    let k = min(vec3<u32>(floor(x)), vec3<u32>(n - 2u));
    let f = x - vec3<f32>(k);
    var out = vec4<f32>(0.0);
    for (var corner = 0u; corner < 8u; corner++) {
        let d = vec3<u32>((corner >> 2u) & 1u, (corner >> 1u) & 1u, corner & 1u);
        var wr = 1.0 - f.x;
        if (d.x == 1u) {
            wr = f.x;
        }
        var wg = 1.0 - f.y;
        if (d.y == 1u) {
            wg = f.y;
        }
        var wb = 1.0 - f.z;
        if (d.z == 1u) {
            wb = f.z;
        }
        let w = wr * wg * wb;
        out = out + w * display_lut(k.x + d.x, k.y + d.y, k.z + d.z);
    }
    return out;
}

fn pass_quantize(v: f32) -> f32 {
    return floor(clamp(v, 0.0, 1.0) * 255.0 + 0.5) / 255.0;
}

fn display_colour(p: DisplayParams, c: vec3<f32>) -> vec3<f32> {
    var o = c;
    if (p.color.x == 2u) {
        let l = pass_lut(p.color.w, c);
        o = l.xyz;
        if (p.gamut.w > 0.0 && l.w > 0.5) {
            o = mix(o, p.gamut.xyz, vec3<f32>(p.gamut.w));
        }
    } else if (p.color.x == 1u) {
        o = shaper_apply(c, 0u, p.m1);
        if (p.color.y == 1u) {
            o = vec3<f32>(pass_quantize(o.x), pass_quantize(o.y), pass_quantize(o.z));
        }
        if (p.color.z == 1u) {
            o = shaper_apply(o, SHAPER_ENTRIES, p.m2);
        }
    }
    return clamp(o, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn display_pixel(p: DisplayParams, s: vec4<f32>) -> vec4<f32> {
    // NaN alpha counts as 0 (the test is the reference's; a compiler may drop it).
    var a = s.w;
    if (!(a == a)) {
        a = 0.0;
    }
    a = clamp(a, 0.0, 1.0);
    var c = vec3<f32>(0.0);
    if (a > 0.0) {
        c = s.xyz / a;
    }
    let channel = p.ids.x;
    let curve = p.ids.z;
    let gain = p.light.x;
    let light = gain != 1.0 || p.ids.w != 0u || p.light.w != 1.0;
    // Premultiplied channels without step 2: straight from the frame.
    var pm = clamp(s.xyz, vec3<f32>(0.0), vec3<f32>(a));
    if (light) {
        var l = pass_decode3(curve, p.light.y, c) * gain;
        if (p.ids.w == 1u) {
            l = vec3<f32>(pass_compand(l.x), pass_compand(l.y), pass_compand(l.z));
        } else if (p.ids.w == 2u) {
            l = pass_tone_map(l);
        }
        if (p.light.w != 1.0) {
            l = vec3<f32>(powz(l.x, p.light.w), powz(l.y, p.light.w), powz(l.z, p.light.w));
        }
        c = pass_encode3(curve, p.light.z, l);
    }
    c = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    if (light) {
        pm = c * a;
    }
    if (channel == 0u || channel == 5u) {
        if (p.color.x != 0u) {
            c = display_colour(p, c);
            pm = c * a;
        }
        if (channel == 5u) {
            return vec4<f32>(pass_quantize(c.x), pass_quantize(c.y), pass_quantize(c.z), 1.0);
        }
        return vec4<f32>(pass_quantize(pm.x), pass_quantize(pm.y), pass_quantize(pm.z), pass_quantize(a));
    }
    if (channel == 4u) {
        var v = a;
        if (gain != 1.0) {
            v = pass_encode(curve, p.light.z, pass_decode(curve, p.light.y, a) * gain);
        }
        let e = pass_quantize(v);
        return vec4<f32>(e, e, e, 1.0);
    }
    let v = pass_quantize(pm[channel - 1u]);
    if (p.ids.y == 1u) {
        var o = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        o[channel - 1u] = v;
        return o;
    }
    return vec4<f32>(v, v, v, 1.0);
}
