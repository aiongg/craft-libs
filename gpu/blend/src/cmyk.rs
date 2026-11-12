//! Blending CMYK colour (Illustrator and PDF semantics).
//!
//! The blend formulas work on additive values, so inks blend as their complements, as
//! subtractive colour spaces blend in PDF: a CMYK colour splits into two additive planes, each
//! blended with the formulas, the complemented C, M and Y as one colour and the complemented K
//! as a grey. Multiply then adds inks (cyan over magenta prints both: blue), and on the grey
//! plane Hue, Saturation and Color keep the backdrop's black while Luminosity takes the
//! source's, as PDF has it for K.

/// The two additive planes of a CMYK colour: complemented C, M, Y, and complemented K as a grey.
pub fn cmyk_planes([c, m, y, k]: [f32; 4]) -> [[f32; 3]; 2] {
    [[1.0 - c, 1.0 - m, 1.0 - y], [1.0 - k; 3]]
}

/// The inks of blended [`cmyk_planes`]: the C, M, Y plane and the K plane's grey, clamped to 0..1.
pub fn planes_cmyk(cmy: [f32; 3], k: f32) -> [f32; 4] {
    [1.0 - cmy[0], 1.0 - cmy[1], 1.0 - cmy[2], 1.0 - k].map(|v| v.clamp(0.0, 1.0))
}
