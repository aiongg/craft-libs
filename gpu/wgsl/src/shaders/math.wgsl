// craft-wgsl math: binding-free numeric helpers that give the CPU's f32 results on every backend.
// Rust twins: f32::rem_euclid, asin, acos, sin, cos, division, f32::round, powf, craft_wgsl::hash_noise.

// `a` mod `n` in [0, n) for n > 0 (floored). Never `%` on a negative operand: WGSL's `%` truncates,
// but naga's GLSL output is plain `%`, undefined for negative operands (wrong on Mesa's GL).
fn imod(a: i32, n: i32) -> i32 {
    if (a >= 0) {
        return a % n;
    }
    return n - 1 - (-(a + 1)) % n;
}

// asin / acos to f32 precision on every backend (Cephes' asinf polynomial). WGSL leaves their
// precision to the implementation, and Mesa's GL drivers use a coarse approximation (errors near
// 1e-4 rad), which shows in every effect that maps through a sphere or a lens.
fn asin_p(x: f32) -> f32 {
    let a = abs(x);
    let big = a > 0.5;
    let z = select(a * a, 0.5 * (1.0 - a), big);
    let s = select(a, sqrt(z), big);
    var p = ((((4.2163199048e-2 * z + 2.4181311049e-2) * z + 4.5470025998e-2) * z + 7.4953002686e-2) * z + 1.6666752422e-1) * z * s + s;
    if (big) {
        p = 1.5707963267948966 - 2.0 * p;
    }
    return select(p, -p, x < 0.0);
}

fn acos_p(x: f32) -> f32 {
    if (x > 0.5) {
        return 2.0 * asin_p(sqrt(0.5 * (1.0 - x)));
    }
    if (x < -0.5) {
        return 3.141592653589793 - 2.0 * asin_p(sqrt(0.5 * (1.0 + x)));
    }
    return 1.5707963267948966 - asin_p(x);
}

// Some hardware's native trig instructions saturate cos near a full turn and
// have errors large enough to move particles and sphere samples visibly. Reduce
// to [-pi/4, pi/4] with a split pi/2, then evaluate the Taylor polynomials with
// explicit fused operations. The effect angles use this bounded range; retain
// the builtins for non-finite or very large arguments rather than cast them.
fn sincos_p(x: f32) -> vec2<f32> {
    if (!(abs(x) <= 8192.0)) {
        return vec2<f32>(sin(x), cos(x));
    }
    let k = round(x * 0.6366197723675814);
    // Keep the coarse reduction in integers. Some drivers reassociate even
    // explicit fma chains into x - k*pi/2, losing low bits for large angles.
    // 3216/2048 is exact; the bounded scaled input and product fit in i32.
    let scaled = x * 2048.0;
    let whole = floor(scaled);
    var r = f32(i32(whole) - i32(k) * 3216) / 2048.0;
    r += (scaled - whole) / 2048.0;
    r = fma(-k, 0.0004838267893064022, r);
    r = fma(-k, 5.564431554416771e-12, r);
    let z = r * r;
    var sp = fma(z, -1.0 / 39916800.0, 1.0 / 362880.0);
    sp = fma(z, sp, -1.0 / 5040.0);
    sp = fma(z, sp, 1.0 / 120.0);
    sp = fma(z, sp, -1.0 / 6.0);
    let sn = fma(r * z, sp, r);
    var cp = fma(z, -1.0 / 3628800.0, 1.0 / 40320.0);
    cp = fma(z, cp, -1.0 / 720.0);
    cp = fma(z, cp, 1.0 / 24.0);
    cp = fma(z, cp, -0.5);
    let cs = fma(z, cp, 1.0);
    switch imod(i32(k), 4) {
        case 1: { return vec2<f32>(cs, -sn); }
        case 2: { return vec2<f32>(-sn, -cs); }
        case 3: { return vec2<f32>(-cs, sn); }
        default: { return vec2<f32>(sn, cs); }
    }
}

fn sin_p(x: f32) -> f32 { return sincos_p(x).x; }
fn cos_p(x: f32) -> f32 { return sincos_p(x).y; }

// Correctly-rounded positive x/a, clamped to [0,1]. Native division and fma
// need not have IEEE precision on every shader backend. Discontinuous colour
// effects need the exact CPU f32 value, not just an approximation within an ulp.
// Long division uses only bounded 24-bit significands (at most 24 iterations).
fn unit_div(x: f32, a: f32) -> f32 {
    if (x <= 0.0 || a <= 0.0) { return 0.0; }
    if (x >= a) { return 1.0; }
    if (a == 1.0) { return x; }
    let xb = bitcast<u32>(x);
    let ab = bitcast<u32>(a);
    var nx = xb & 0x7fffffu;
    var na = ab & 0x7fffffu;
    var ex = i32((xb >> 23u) & 255u) - 127;
    var ea = i32((ab >> 23u) & 255u) - 127;
    if (ex == -127) {
        let shift = 23u - firstLeadingBit(nx);
        nx <<= shift;
        ex = -126 - i32(shift);
    } else { nx |= 0x800000u; }
    if (ea == -127) {
        let shift = 23u - firstLeadingBit(na);
        na <<= shift;
        ea = -126 - i32(shift);
    } else { na |= 0x800000u; }
    var exponent = ex - ea;
    if (nx < na) { nx <<= 1u; exponent -= 1; }
    var rem = nx - na;
    if (exponent < -150) { return 0.0; }
    if (exponent == -150) { return bitcast<f32>(select(0u, 1u, rem != 0u)); }
    let fraction = u32(min(23, exponent + 149));
    var q = 1u;
    for (var i = 0u; i <= fraction; i++) {
        rem <<= 1u;
        q <<= 1u;
        if (rem >= na) { rem -= na; q |= 1u; }
    }
    let guard = q & 1u;
    q >>= 1u;
    if (guard != 0u && (rem != 0u || (q & 1u) != 0u)) { q += 1u; }
    if (exponent < -126) { return bitcast<f32>(q); }
    if (q == 0x1000000u) { q >>= 1u; exponent += 1; }
    return bitcast<f32>((u32(exponent + 127) << 23u) | (q & 0x7fffffu));
}


// raster::hash_noise.
fn hash_noise(x: u32, y: u32, seed: u32) -> f32 {
    var h = (x * 0x8da6b343u) ^ (y * 0xd8163841u) ^ (seed * 0xcb1ab31fu);
    h = h ^ (h >> 13u);
    h = h * 0x5bd1e995u;
    h = h ^ (h >> 15u);
    return f32(h & 0x00ffffffu) / 16777216.0;
}

// powf for x >= 0 (0^y = 0 for y > 0; WGSL leaves pow(0, y) to the implementation).
fn powz(x: f32, y: f32) -> f32 {
    if (x <= 0.0) {
        return select(0.0, 1.0, y == 0.0);
    }
    return pow(x, y);
}

// Rust's f32::round (half away from zero; WGSL's round() is half to even). Decided by comparing
// `x` with `t ± 0.5` (exact for integer `t`) rather than subtracting: drivers may fuse a product
// that feeds `x - t` into one fma (AMD does), skipping the product's rounding, so `5/6 · 3`,
// which rounds to exactly 2.5 on the CPU, came out 0.49999994 above 2 and rounded down (#1605).
fn round_away(x: f32) -> f32 {
    // From 2^23 up every f32 is an integer, and `t + 0.5` would round to even (8388608 → 8388609).
    if (abs(x) >= 8388608.0) {
        return x;
    }
    let t = trunc(x);
    if (x >= 0.0) {
        return select(t, t + 1.0, x >= t + 0.5);
    }
    return select(t, t - 1.0, x <= t - 0.5);
}

// f32::rem_euclid(1.0).
fn fract_euclid(x: f32) -> f32 {
    return x - floor(x);
}
