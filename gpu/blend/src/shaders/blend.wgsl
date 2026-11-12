// craft-blend: every blend formula, mirroring src/formula.rs, fill.rs and composite.rs function
// for function. Mode numbers are the BLEND_<MODE> constants generated from BlendMode; the
// variant is the module constant BLEND_VARIANT (one variant per shader module). Uses craft-wgsl
// COLOR (luminance, sRGB).
//
// FXC (the DX12 compiler between naga's HLSL and the driver) stops following a big switch once
// every case returns ("error X3507: Not all control paths return a value"), and then no pipeline
// builds. Each switch case assigns instead; the one return at the end is what FXC can follow.

const BLEND_EDGE: f32 = 1e-4;
// f32::MAX exactly. The decimal 3.40282347e38 is just above it: naga rounds it down, but
// browsers' WebGPU compiler rejects it.
const BLEND_F32_MAX: f32 = 0x1.fffffep+127f;

fn blend_top(float_range: bool) -> f32 { return select(1.0, BLEND_F32_MAX, float_range); }
fn blend_step(on: bool) -> f32 { return select(0.0, 1.0, on); }

fn blend_color_burn(cb: f32, cs: f32) -> f32 {
    if (cb >= 1.0) { return 1.0; }
    if (cs <= 0.0) { return 0.0; }
    return 1.0 - min((1.0 - cb) / cs, 1.0);
}
fn blend_color_dodge(cb: f32, cs: f32) -> f32 {
    if (cb <= 0.0) { return 0.0; }
    if (cs >= 1.0) { return 1.0; }
    return min(cb / (1.0 - cs), 1.0);
}
fn blend_color_burn_edge(cb: f32, cs: f32) -> f32 {
    if (cb >= 1.0 - BLEND_EDGE) { return 1.0; }
    if (cs <= 0.0) { return 0.0; }
    return 1.0 - min((1.0 - cb) / cs, 1.0);
}
fn blend_color_dodge_edge(cb: f32, cs: f32) -> f32 {
    if (cb <= BLEND_EDGE) { return 0.0; }
    if (cs >= 1.0) { return 1.0; }
    return min(cb / (1.0 - cs), 1.0);
}
fn blend_hard_light(cb: f32, cs: f32) -> f32 {
    if (cs <= 0.5) { return cb * 2.0 * cs; }
    let s = 2.0 * cs - 1.0;
    return cb + s - cb * s;
}
// The W3C curve: D(cb) is the cubic below a quarter, sqrt above it (#8804).
fn blend_soft_light(cb: f32, cs: f32) -> f32 {
    if (cs <= 0.5) { return 2.0 * cb * cs + cb * cb * (1.0 - 2.0 * cs); }
    let b = max(cb, 0.0);
    var d = sqrt(b);
    if (b <= 0.25) { d = ((16.0 * b - 12.0) * b + 4.0) * b; }
    return 2.0 * cb * (1.0 - cs) + d * (2.0 * cs - 1.0);
}
fn blend_vivid_light(cb: f32, cs: f32) -> f32 {
    if (cs <= 0.5) { return blend_color_burn(cb, 2.0 * cs); }
    return blend_color_dodge(cb, 2.0 * (cs - 0.5));
}
fn blend_vivid_light_source_wins(cb: f32, cs: f32) -> f32 {
    if (cs <= 0.5) {
        if (cs <= 0.0) { return 0.0; }
        return 1.0 - min((1.0 - cb) / (2.0 * cs), 1.0);
    }
    if (cs >= 1.0) { return 1.0; }
    return min(cb / (2.0 * (1.0 - cs)), 1.0);
}
fn blend_hard_mix_edge(cb: f32, cs: f32) -> f32 {
    var v: f32;
    if (cs <= 0.5) {
        if (cb >= 1.0 - BLEND_EDGE) { v = 1.0; } else if (cs <= 0.0) { v = 0.0; } else { v = 1.0 - min((1.0 - cb) / (2.0 * cs), 1.0); }
    } else if (cb <= BLEND_EDGE) {
        v = 0.0;
    } else if (cs >= 1.0) {
        v = 1.0;
    } else {
        v = min(cb / (2.0 * (1.0 - cs)), 1.0);
    }
    return blend_step(v >= 0.5 - 1e-6);
}
fn blend_pin_light(cb: f32, cs: f32) -> f32 {
    if (cs <= 0.5) { return min(cb, 2.0 * cs); }
    return max(cb, 2.0 * cs - 1.0);
}
fn blend_divide(cb: f32, cs: f32, hi: f32) -> f32 {
    if (cs <= 0.0) { return blend_step(cb > 0.0); }
    return min(cb / cs, hi);
}
fn blend_divide_unclipped(cb: f32, cs: f32) -> f32 {
    if (cs <= 0.0) { return blend_step(cb > 0.0); }
    return cb / cs;
}
fn blend_reflect(cb: f32, cs: f32) -> f32 {
    if (cs >= 1.0) { return 1.0; }
    return min(cb * cb / (1.0 - cs), 1.0);
}
// Paint.NET's XOR works on 8-bit channels.
fn blend_xor(cb: f32, cs: f32) -> f32 {
    let b = u32(round_away(clamp(cb, 0.0, 1.0) * 255.0));
    let s = u32(round_away(clamp(cs, 0.0, 1.0) * 255.0));
    return f32(b ^ s) / 255.0;
}
fn blend_paint_net_burn(cb: f32, cs: f32) -> f32 {
    if (cs <= 0.0) { return 0.0; }
    return max(1.0 - (1.0 - cb) / cs, 0.0);
}
fn blend_paint_net_dodge(cb: f32, cs: f32) -> f32 {
    if (cs >= 1.0) { return 1.0; }
    return min(cb / (1.0 - cs), 1.0);
}
fn blend_classic_burn(cb: f32, cs: f32) -> f32 {
    if (cs <= 0.0) { return 0.0; }
    return 1.0 - (1.0 - cb) / cs;
}

// formula::shared: the formulas every variant shares and the modes only one app has.
fn blend_shared(mode: u32, cb: f32, cs: f32, hi: f32) -> f32 {
    var r = cs;
    switch mode {
        case BLEND_DARKEN: { r = min(cb, cs); }
        case BLEND_MULTIPLY: { r = clamp(cb * cs, -BLEND_F32_MAX, BLEND_F32_MAX); }
        case BLEND_COLOR_BURN: { r = blend_color_burn(cb, cs); }
        case BLEND_LINEAR_BURN: { r = max(cb + cs - 1.0, 0.0); }
        case BLEND_LIGHTEN: { r = max(cb, cs); }
        case BLEND_SCREEN: { r = cb + cs - cb * cs; }
        case BLEND_COLOR_DODGE: { r = blend_color_dodge(cb, cs); }
        case BLEND_LINEAR_DODGE, BLEND_ADD: { r = min(cb + cs, hi); }
        case BLEND_OVERLAY: { r = blend_hard_light(cs, cb); }
        case BLEND_SOFT_LIGHT: { r = blend_soft_light(cb, cs); }
        case BLEND_HARD_LIGHT: { r = blend_hard_light(cb, cs); }
        case BLEND_VIVID_LIGHT: { r = blend_vivid_light_source_wins(cb, cs); }
        case BLEND_LINEAR_LIGHT: { r = clamp(cb + 2.0 * cs - 1.0, 0.0, 1.0); }
        case BLEND_PIN_LIGHT: { r = blend_pin_light(cb, cs); }
        case BLEND_HARD_MIX: { r = blend_hard_mix_edge(cb, cs); }
        case BLEND_DIFFERENCE, BLEND_CLASSIC_DIFFERENCE: { r = abs(cb - cs); }
        case BLEND_EXCLUSION: { r = cb + cs - 2.0 * cb * cs; }
        case BLEND_SUBTRACT: { r = max(cb - cs, 0.0); }
        case BLEND_DIVIDE: { r = blend_divide(cb, cs, hi); }
        case BLEND_REFLECT: { r = blend_reflect(cb, cs); }
        case BLEND_GLOW: { r = blend_reflect(cs, cb); }
        case BLEND_NEGATION: { r = 1.0 - abs(1.0 - cb - cs); }
        case BLEND_XOR: { r = blend_xor(cb, cs); }
        case BLEND_PAINT_NET_COLOR_BURN: { r = blend_paint_net_burn(cb, cs); }
        case BLEND_PAINT_NET_COLOR_DODGE, BLEND_CLASSIC_COLOR_DODGE: { r = blend_paint_net_dodge(cb, cs); }
        case BLEND_CLASSIC_COLOR_BURN: { r = blend_classic_burn(cb, cs); }
        case BLEND_DISABLE: { r = cb; }
        case BLEND_INVERSE_DIVIDE: { r = cs / max(cb, 1e-6); }
        case BLEND_INVERSE_SUBTRACT: { r = cs - cb; }
        case BLEND_SIGNED_ADDITION: { r = cb + 2.0 * cs - 1.0; }
        default: {}
    }
    return r;
}

fn blend_photoshop(mode: u32, cb: f32, cs: f32, hi: f32) -> f32 {
    var r: f32;
    switch mode {
        case BLEND_COLOR_BURN: { r = blend_color_burn_edge(cb, cs); }
        case BLEND_COLOR_DODGE: { r = blend_color_dodge_edge(cb, cs); }
        case BLEND_VIVID_LIGHT: { r = blend_vivid_light_source_wins(cb, cs); }
        case BLEND_HARD_MIX: { r = blend_hard_mix_edge(cb, cs); }
        default: { r = blend_shared(mode, cb, cs, hi); }
    }
    return r;
}

fn blend_after_effects(mode: u32, cb: f32, cs: f32) -> f32 {
    var r: f32;
    switch mode {
        case BLEND_VIVID_LIGHT: { r = blend_vivid_light(cb, cs); }
        case BLEND_HARD_MIX: { r = blend_step(cb + cs >= 1.0); }
        case BLEND_ADD, BLEND_LINEAR_DODGE: { r = cb + cs; }
        case BLEND_DIVIDE: { r = blend_divide_unclipped(cb, cs); }
        default: { r = blend_shared(mode, cb, cs, 1.0); }
    }
    return r;
}

fn blend_standard(mode: u32, cb: f32, cs: f32, hi: f32) -> f32 {
    var r: f32;
    switch mode {
        case BLEND_VIVID_LIGHT: { r = blend_vivid_light(cb, cs); }
        case BLEND_HARD_MIX: { r = blend_step(blend_vivid_light(cb, cs) >= 0.5); }
        default: {
            if (mode == BLEND_DIVIDE && cs <= 0.0) { r = 1.0; } else { r = blend_shared(mode, cb, cs, hi); }
        }
    }
    return r;
}

fn blend_substance(mode: u32, cb: f32, cs: f32) -> f32 {
    var r: f32;
    switch mode {
        case BLEND_DIVIDE: { r = cb / max(cs, 1e-6); }
        case BLEND_LINEAR_DODGE: { r = cb + cs; }
        case BLEND_SUBTRACT: { r = cb - cs; }
        case BLEND_LINEAR_BURN: { r = cb + cs - 1.0; }
        case BLEND_LINEAR_LIGHT: { r = cb + 2.0 * cs - 1.0; }
        case BLEND_SOFT_LIGHT: { r = blend_soft_light(clamp(cb, 0.0, 1.0), clamp(cs, 0.0, 1.0)); }
        case BLEND_VIVID_LIGHT: { r = blend_vivid_light(cb, cs); }
        default: { r = blend_shared(mode, cb, cs, 1.0); }
    }
    return r;
}

// Blend::channel. Photoshop's f64 replay of overflowing 32-bit results has no GPU twin.
fn blend_channel(mode: u32, cb: f32, cs: f32, float_range: bool) -> f32 {
    let hi = blend_top(float_range);
    var r: f32;
    if (BLEND_VARIANT == BLEND_PHOTOSHOP) {
        r = blend_photoshop(mode, cb, cs, hi);
    } else if (BLEND_VARIANT == BLEND_AFTER_EFFECTS) {
        r = blend_after_effects(mode, cb, cs);
    } else if (BLEND_VARIANT == BLEND_SUBSTANCE) {
        r = blend_substance(mode, cb, cs);
    } else {
        r = blend_standard(mode, cb, cs, hi);
    }
    return r;
}

// ---- non-separable modes (W3C Compositing Level 1) ----

fn blend_lum(c: vec3<f32>) -> f32 { return 0.3 * c.r + 0.59 * c.g + 0.11 * c.b; }

// formula::clip_color: toward grey until within 0..top; a luminosity outside that range has no
// in-gamut colour (and max − lum is rounding noise), so it gives its grey.
fn blend_clip_color(c: vec3<f32>, top: f32) -> vec3<f32> {
    let l = blend_lum(c);
    let n = min(min(c.r, c.g), c.b);
    let x = max(max(c.r, c.g), c.b);
    var out = c;
    if (n < 0.0) {
        let d = l - n;
        if (l > 0.0 && d > 0.0) { out = l + (out - l) * l / d; } else { out = vec3<f32>(l); }
    }
    if (x > top) {
        let d = x - l;
        if (l < top && d > 0.0) { out = l + (out - l) * (top - l) / d; } else { out = vec3<f32>(l); }
    }
    return out;
}
fn blend_set_lum(c: vec3<f32>, l: f32, top: f32) -> vec3<f32> { return blend_clip_color(c + (l - blend_lum(c)), top); }
fn blend_sat(c: vec3<f32>) -> f32 { return max(max(c.r, c.g), c.b) - min(min(c.r, c.g), c.b); }
fn blend_set_sat(c: vec3<f32>, s: f32) -> vec3<f32> {
    let mx = max(max(c.r, c.g), c.b);
    let mn = min(min(c.r, c.g), c.b);
    if (mx > mn) { return (c - mn) * s / (mx - mn); }
    return vec3<f32>(0.0);
}
fn blend_value(cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    let vb = max(max(cb.r, cb.g), cb.b);
    let vs = max(max(cs.r, cs.g), cs.b);
    if (vb <= 1e-6) { return vec3<f32>(vs); }
    return cb * vs / vb;
}
fn blend_unpack(n: vec3<f32>) -> vec3<f32> { return n * 2.0 - 1.0; }
fn blend_pack(n: vec3<f32>) -> vec3<f32> {
    let len = max(sqrt(n.x * n.x + n.y * n.y + n.z * n.z), 1e-6);
    return n / len * 0.5 + 0.5;
}
// Reoriented normal mapping: `detail` onto `base`, packed into 0..1.
fn blend_rnm(base: vec3<f32>, detail: vec3<f32>) -> vec3<f32> {
    let t = blend_unpack(base) + vec3<f32>(0.0, 0.0, 1.0);
    let u = blend_unpack(detail) * vec3<f32>(-1.0, -1.0, 1.0);
    let d = t.x * u.x + t.y * u.y + t.z * u.z;
    return blend_pack(t * d - u * t.z);
}
fn blend_luma(c: vec3<f32>) -> f32 {
    if (BLEND_VARIANT == BLEND_AFTER_EFFECTS) { return luminance(c); }
    return blend_lum(c);
}

// Blend::rgb.
fn blend_rgb(mode: u32, cb: vec3<f32>, cs: vec3<f32>, float_range: bool) -> vec3<f32> {
    let top = blend_top(float_range);
    var r: vec3<f32>;
    switch mode {
        case BLEND_HUE: { r = blend_set_lum(blend_set_sat(cs, blend_sat(cb)), blend_lum(cb), top); }
        case BLEND_SATURATION: { r = blend_set_lum(blend_set_sat(cb, blend_sat(cs)), blend_lum(cb), top); }
        case BLEND_COLOR: { r = blend_set_lum(cs, blend_lum(cb), top); }
        case BLEND_LUMINOSITY: { r = blend_set_lum(cb, blend_lum(cs), top); }
        case BLEND_DARKER_COLOR: { r = select(cb, cs, blend_luma(cs) < blend_luma(cb)); }
        case BLEND_LIGHTER_COLOR: { r = select(cb, cs, blend_luma(cs) > blend_luma(cb)); }
        case BLEND_TINT: { r = cs * blend_lum(cb); }
        case BLEND_VALUE: { r = blend_value(cb, cs); }
        case BLEND_NORMAL_MAP_COMBINE: {
            let nb = blend_unpack(cb);
            let ns = blend_unpack(cs);
            r = blend_pack(vec3<f32>(nb.x + ns.x, nb.y + ns.y, nb.z * ns.z));
        }
        case BLEND_NORMAL_MAP_DETAIL: { r = blend_rnm(cb, cs); }
        case BLEND_NORMAL_MAP_INVERSE_DETAIL: { r = blend_rnm(cs, cb); }
        default: {
            r = vec3<f32>(blend_channel(mode, cb.r, cs.r, float_range), blend_channel(mode, cb.g, cs.g, float_range), blend_channel(mode, cb.b, cs.b, float_range));
        }
    }
    return r;
}

// fill::fill_is_special: Photoshop's special eight.
fn blend_fill_is_special(mode: u32) -> bool {
    return mode == BLEND_COLOR_BURN || mode == BLEND_LINEAR_BURN || mode == BLEND_COLOR_DODGE || mode == BLEND_LINEAR_DODGE
        || mode == BLEND_VIVID_LIGHT || mode == BLEND_LINEAR_LIGHT || mode == BLEND_HARD_MIX || mode == BLEND_DIFFERENCE;
}

// The source the special eight blend at Fill `f` below 100%: pulled toward the mode's neutral.
fn blend_fill_source(mode: u32, cs: vec3<f32>, f: f32) -> vec3<f32> {
    var neutral = 0.0;
    if (mode == BLEND_COLOR_BURN || mode == BLEND_LINEAR_BURN) { neutral = 1.0; }
    if (mode == BLEND_VIVID_LIGHT || mode == BLEND_LINEAR_LIGHT) { neutral = 0.5; }
    return vec3<f32>(neutral) + (cs - vec3<f32>(neutral)) * f;
}

// Blend::rgb_fill.
fn blend_rgb_fill(mode: u32, cb: vec3<f32>, cs: vec3<f32>, fill: f32, float_range: bool) -> vec3<f32> {
    let f = clamp(fill, 0.0, 1.0);
    if (f >= 1.0 || !blend_fill_is_special(mode)) { return blend_rgb(mode, cb, cs, float_range); }
    if (mode == BLEND_HARD_MIX) { return clamp((cb - f * (vec3<f32>(1.0) - cs)) / (1.0 - f), vec3<f32>(0.0), vec3<f32>(1.0)); }
    return blend_rgb(mode, cb, blend_fill_source(mode, cs, f), float_range);
}

// ---- compositing ----

// TextGamma: linear light raised to 1 / g.
fn blend_text_encode(c: vec3<f32>, g: f32) -> vec3<f32> { return pow(srgb_to_linear3(max(c, vec3<f32>(0.0))), vec3<f32>(1.0 / g)); }
fn blend_text_decode(c: vec3<f32>, g: f32) -> vec3<f32> { return linear_to_srgb3(pow(max(c, vec3<f32>(0.0)), vec3<f32>(g))); }

// Weights::new: (backdrop, source, blend) weights and the result alpha of a straight composite
// of coverage `cover`; alpha 0 where nothing is left. A caller mixing in a space of its own
// (PhotoCraft's Lab) mixes with these.
fn blend_weights(ab: f32, cover: f32) -> vec4<f32> {
    let ao = cover + ab * (1.0 - cover);
    if (ao <= 0.0) { return vec4<f32>(0.0); }
    return vec4<f32>((1.0 - cover) * ab / ao, (1.0 - ab) * cover / ao, cover * ab / ao, ao);
}

fn blend_normal_rgb(b: vec4<f32>, s: vec4<f32>, as_: f32) -> vec4<f32> {
    let ao = as_ + b.a * (1.0 - as_);
    if (ao <= 0.0) { return vec4<f32>(0.0); }
    let kb = b.a * (1.0 - as_);
    let inv = 1.0 / ao;
    return vec4<f32>((kb * b.rgb + as_ * s.rgb) * inv, ao);
}

// Blend::composite with Alpha::Straight and a Linear blend space. `text_gamma` > 0 mixes in
// TextGamma(text_gamma), 0 mixes the values as given (Linear). One blend_rgb call: every inlined
// copy grows the shader, and WARP compiles it slowly.
fn blend_straight(mode: u32, b: vec4<f32>, s: vec4<f32>, opacity: f32, fill: f32, noise: f32, float_range: bool, text_gamma: f32) -> vec4<f32> {
    let f = clamp(fill, 0.0, 1.0);
    if (s.a <= 0.0) { return b; }
    if (mode == BLEND_DISSOLVE || mode == BLEND_DANCING_DISSOLVE) {
        if (noise < s.a * opacity * f) { return vec4<f32>(s.rgb, 1.0); }
        return b;
    }
    let special = BLEND_VARIANT == BLEND_PHOTOSHOP && f < 1.0 && blend_fill_is_special(mode);
    var over = s.a * opacity;
    var alone = over * f;
    if (!special) {
        over = s.a * (opacity * f);
        alone = over;
    }
    if (over <= 0.0) { return b; }
    let ab = b.a;
    let gamma = text_gamma > 0.0;
    if (!special && !gamma && mode == BLEND_NORMAL) { return blend_normal_rgb(b, s, over); }
    var ao = over + ab * (1.0 - over);
    if (special) { ao = ab + (1.0 - ab) * alone; }
    if (ao <= 0.0) { return vec4<f32>(0.0); }
    var cs = s.rgb;
    if (special) { cs = blend_fill_source(mode, s.rgb, f); }
    var bl = blend_rgb(mode, b.rgb, cs, float_range);
    if (special && mode == BLEND_HARD_MIX) { bl = clamp((b.rgb - f * (vec3<f32>(1.0) - s.rgb)) / (1.0 - f), vec3<f32>(0.0), vec3<f32>(1.0)); }
    if (special) {
        let wb = ab * (1.0 - over) / ao;
        let wbs = ab * over / ao;
        let ws = (1.0 - ab) * alone / ao;
        if (!gamma) { return vec4<f32>(wb * b.rgb + wbs * bl + ws * s.rgb, ao); }
        let g = text_gamma;
        return vec4<f32>(blend_text_decode(wb * blend_text_encode(b.rgb, g) + wbs * blend_text_encode(bl, g) + ws * blend_text_encode(s.rgb, g), g), ao);
    }
    if (!gamma) { return vec4<f32>(((1.0 - over) * ab * b.rgb + (1.0 - ab) * over * s.rgb + over * ab * bl) / ao, ao); }
    if (ab <= 0.0) { return vec4<f32>(s.rgb, ao); }
    let w = blend_weights(ab, over);
    let g = text_gamma;
    return vec4<f32>(blend_text_decode(w.x * blend_text_encode(b.rgb, g) + w.y * blend_text_encode(s.rgb, g) + w.z * blend_text_encode(bl, g), g), ao);
}

// Blend::composite_atop (straight).
fn blend_atop(mode: u32, b: vec4<f32>, s: vec4<f32>, opacity: f32, float_range: bool) -> vec4<f32> {
    let as_ = s.a * opacity;
    if (as_ <= 0.0) { return b; }
    let bl = blend_rgb(mode, b.rgb, s.rgb, float_range);
    return vec4<f32>((1.0 - as_) * b.rgb + as_ * bl, b.a);
}

fn blend_is_stencil(mode: u32) -> bool {
    return mode == BLEND_STENCIL_ALPHA || mode == BLEND_STENCIL_LUMA || mode == BLEND_SILHOUETTE_ALPHA || mode == BLEND_SILHOUETTE_LUMA;
}

fn blend_reads_backdrop(mode: u32) -> bool {
    return !(mode == BLEND_PASS_THROUGH || mode == BLEND_NORMAL || mode == BLEND_DISSOLVE || mode == BLEND_DANCING_DISSOLVE || mode == BLEND_REPLACE);
}

fn blend_after_effects_keeps_over_range(mode: u32) -> bool {
    var keep = false;
    switch mode {
        case BLEND_NORMAL, BLEND_DISSOLVE, BLEND_DANCING_DISSOLVE, BLEND_DARKEN, BLEND_MULTIPLY, BLEND_COLOR_BURN, BLEND_CLASSIC_COLOR_BURN,
            BLEND_LINEAR_BURN, BLEND_DARKER_COLOR, BLEND_ADD, BLEND_LIGHTEN, BLEND_SCREEN, BLEND_LINEAR_DODGE, BLEND_LIGHTER_COLOR,
            BLEND_DIFFERENCE, BLEND_CLASSIC_DIFFERENCE, BLEND_OVERLAY, BLEND_EXCLUSION, BLEND_LUMINOSITY, BLEND_SUBTRACT, BLEND_DIVIDE,
            BLEND_STENCIL_ALPHA, BLEND_STENCIL_LUMA, BLEND_SILHOUETTE_ALPHA, BLEND_SILHOUETTE_LUMA, BLEND_ALPHA_ADD, BLEND_LUMINESCENT_PREMUL: { keep = true; }
        default: {}
    }
    return keep;
}

// B(Cb, Cs) for a premultiplied composite: in display-encoded sRGB (clamped back) when
// `srgb_blend` (Premiere: every mode but Normal blends encoded values).
fn blend_in(mode: u32, cb: vec3<f32>, cs: vec3<f32>, float_range: bool, srgb_blend: bool) -> vec3<f32> {
    if (!srgb_blend || !blend_reads_backdrop(mode)) { return blend_rgb(mode, cb, cs, float_range); }
    return srgb_to_linear3(clamp(blend_rgb(mode, linear_to_srgb3(cb), linear_to_srgb3(cs), float_range), vec3<f32>(0.0), vec3<f32>(1.0)));
}

// Blend::composite with Alpha::Premultiplied and a Linear mix space. `src` already carries the
// layer's opacity and fill. `noise` decides Dissolve (the pixel shows where noise < alpha).
fn blend_premul(mode: u32, dst: vec4<f32>, src: vec4<f32>, noise: f32, float_range: bool, srgb_blend: bool) -> vec4<f32> {
    let sa = src.a;
    let da = dst.a;
    var alpha_only = true;
    var out = dst;
    switch mode {
        case BLEND_NORMAL, BLEND_PASS_THROUGH: { out = vec4<f32>(src.rgb + dst.rgb * (1.0 - sa), sa + da * (1.0 - sa)); }
        case BLEND_DISSOLVE, BLEND_DANCING_DISSOLVE: {
            if (sa > 0.0 && noise < sa) { out = vec4<f32>(src.rgb * (1.0 / sa), 1.0); }
        }
        case BLEND_STENCIL_ALPHA: { out = dst * sa; }
        case BLEND_SILHOUETTE_ALPHA: { out = dst * (1.0 - sa); }
        case BLEND_STENCIL_LUMA: { out = dst * luminance(src.rgb); }
        case BLEND_SILHOUETTE_LUMA: { out = dst * (1.0 - luminance(src.rgb)); }
        case BLEND_ALPHA_ADD: { out = vec4<f32>(src.rgb + dst.rgb * (1.0 - sa), min(sa + da, 1.0)); }
        case BLEND_LUMINESCENT_PREMUL: { out = vec4<f32>(src.rgb + dst.rgb * (1.0 - sa), sa + da - sa * da); }
        default: { alpha_only = false; }
    }
    if (alpha_only) { return out; }
    if (sa <= 0.0) { return dst; }
    if (da <= 0.0) { return src; }
    var cs = src.rgb / sa;
    var cb = dst.rgb / da;
    if (BLEND_VARIANT == BLEND_AFTER_EFFECTS && !blend_after_effects_keeps_over_range(mode)) {
        cs = clamp(cs, vec3<f32>(0.0), vec3<f32>(1.0));
        cb = clamp(cb, vec3<f32>(0.0), vec3<f32>(1.0));
    }
    let ao = sa + da - sa * da;
    // Classic Color Dodge / Difference (neutral 0) and Classic Color Burn (neutral 1): the source
    // fades toward the neutral colour by its coverage, then the formula applies.
    if (mode == BLEND_CLASSIC_COLOR_DODGE || mode == BLEND_CLASSIC_DIFFERENCE || mode == BLEND_CLASSIC_COLOR_BURN) {
        let neutral = select(0.0, 1.0, mode == BLEND_CLASSIC_COLOR_BURN);
        let bc = blend_in(mode, cb, neutral + sa * (cs - neutral), float_range, srgb_blend);
        return vec4<f32>((1.0 - da) * src.rgb + da * bc, ao);
    }
    let bl = blend_in(mode, cb, cs, float_range, srgb_blend);
    return vec4<f32>((1.0 - da) * src.rgb + (1.0 - sa) * dst.rgb + sa * da * bl, ao);
}
