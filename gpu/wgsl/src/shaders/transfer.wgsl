// craft-wgsl transfer: PQ, HLG and the curve switch (Rust twins: craft_wgsl::{pq_*, hlg_*, curve_*}).
// Uses COLOR (sRGB) and MATH (powz).

// ColorSpace transfer curves: 1 = sRGB (sRGB, Display P3), 2 = 2.4 power (Rec. 709 / 2020),
// 3 = SMPTE ST 2084 PQ and 4 = BT.2100 HLG (linear 1.0 = the BT.2408 reference white).
const PQ_M1: f32 = 0.1593017578125;
const PQ_M2: f32 = 78.84375;
const PQ_C1: f32 = 0.8359375;
const PQ_C2: f32 = 18.8515625;
const PQ_C3: f32 = 18.6875;
const PQ_WHITE: f32 = 0.0203;
const HLG_A: f32 = 0.17883277;
const HLG_B: f32 = 0.28466892;
const HLG_C: f32 = 0.55991073;
const HLG_REF: f32 = 0.26496256;

fn pq_decode(e: f32) -> f32 {
    let p = pow(clamp(e, 0.0, 1.0), 1.0 / PQ_M2);
    return pow(max(p - PQ_C1, 0.0) / (PQ_C2 - PQ_C3 * p), 1.0 / PQ_M1) / PQ_WHITE;
}

fn pq_encode(l: f32) -> f32 {
    let y = pow(max(l * PQ_WHITE, 0.0), PQ_M1);
    return pow((PQ_C1 + PQ_C2 * y) / (1.0 + PQ_C3 * y), PQ_M2);
}

fn hlg_decode(v: f32) -> f32 {
    if (v <= 0.5) {
        return v * v / 3.0 / HLG_REF;
    }
    return (exp((v - HLG_C) / HLG_A) + HLG_B) / 12.0 / HLG_REF;
}

fn hlg_encode(l: f32) -> f32 {
    let e = max(l * HLG_REF, 0.0);
    if (e <= 1.0 / 12.0) {
        return sqrt(3.0 * e);
    }
    return HLG_A * log(12.0 * e - HLG_B) + HLG_C;
}

fn curve_decode(curve: u32, v: f32) -> f32 {
    let a = abs(v);
    var l = a;
    if (curve == 1u) {
        l = srgb_to_linear(a);
    } else if (curve == 2u) {
        l = powz(a, 2.4);
    } else if (curve == 3u) {
        l = pq_decode(a);
    } else if (curve == 4u) {
        l = hlg_decode(a);
    }
    return select(l, -l, v < 0.0);
}

fn curve_encode(curve: u32, v: f32) -> f32 {
    let a = abs(v);
    var e = a;
    if (curve == 1u) {
        e = linear_to_srgb(a);
    } else if (curve == 2u) {
        e = powz(a, 1.0 / 2.4);
    } else if (curve == 3u) {
        e = pq_encode(a);
    } else if (curve == 4u) {
        e = hlg_encode(a);
    }
    return select(e, -e, v < 0.0);
}
