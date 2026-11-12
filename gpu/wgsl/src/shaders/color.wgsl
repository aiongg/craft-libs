// craft-wgsl color: luma, the sRGB transfer curve and HSL. Each function has a Rust twin of the
// same name in craft_wgsl, operation for operation. Uses MATH (fract_euclid).

// Rec. 709 luma weights.
fn luminance(c: vec3<f32>) -> f32 {
    return 0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z;
}

fn srgb_to_linear(v: f32) -> f32 {
    if (v <= 0.04045) {
        return v / 12.92;
    }
    return pow((v + 0.055) / 1.055, 2.4);
}

fn linear_to_srgb(v: f32) -> f32 {
    if (v <= 0.0031308) {
        return v * 12.92;
    }
    return 1.055 * pow(max(v, 0.0), 1.0 / 2.4) - 0.055;
}

// Per component, so a vector gives the same bits as three scalar calls.
fn srgb_to_linear3(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(srgb_to_linear(c.x), srgb_to_linear(c.y), srgb_to_linear(c.z));
}

fn linear_to_srgb3(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(linear_to_srgb(c.x), linear_to_srgb(c.y), linear_to_srgb(c.z));
}

fn rgb_to_hsl(c: vec3<f32>) -> vec3<f32> {
    let mx = max(max(c.x, c.y), c.z);
    let mn = min(min(c.x, c.y), c.z);
    let l = (mx + mn) * 0.5;
    if (abs(mx - mn) < 1e-7) {
        return vec3<f32>(0.0, 0.0, l);
    }
    let d = mx - mn;
    var denom = mx + mn;
    if (l > 0.5) {
        denom = 2.0 - mx - mn;
    }
    // Over-range (HDR) or negative inputs can make the denominator zero or negative.
    let s = select(1.0, d / denom, denom > 0.0);
    var h: f32;
    if (mx == c.x) {
        h = fract_euclid(((c.y - c.z) / d + select(0.0, 6.0, c.y < c.z)) / 6.0);
    } else if (mx == c.y) {
        h = ((c.z - c.x) / d + 2.0) / 6.0;
    } else {
        h = ((c.x - c.y) / d + 4.0) / 6.0;
    }
    return vec3<f32>(h, s, l);
}

fn hsl_channel(t0: f32, p: f32, q: f32) -> f32 {
    let t = fract_euclid(t0);
    if (t < 1.0 / 6.0) {
        return p + (q - p) * 6.0 * t;
    }
    if (t < 0.5) {
        return q;
    }
    if (t < 2.0 / 3.0) {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    return p;
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> vec3<f32> {
    if (s <= 0.0) {
        return vec3<f32>(l);
    }
    var q: f32;
    if (l < 0.5) {
        q = l * (1.0 + s);
    } else {
        q = l + s - l * s;
    }
    let p = 2.0 * l - q;
    return vec3<f32>(hsl_channel(h + 1.0 / 3.0, p, q), hsl_channel(h, p, q), hsl_channel(h - 1.0 / 3.0, p, q));
}
