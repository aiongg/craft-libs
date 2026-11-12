//! The display pass in WGSL: the [`DISPLAY`] module, the bytes of its uniform block and of its
//! two lookup textures. No device here: the view that runs the pass (craft-ui-gpu's display
//! bake) creates the textures and the pipeline from these.
//!
//! A shader built on the pass assembles [`DISPLAY`] with [`texture_access`] (or its own
//! definitions of `display_table` and `display_lut`) and calls
//! `display_pixel(params, texel)` with a `DisplayParams` uniform filled from [`params`].

use craft_cms::GpuMatrixShaper;

use crate::pass::{DisplayPass, PassColor};

/// The pass: `DisplayParams`, `display_pixel` and their helpers.
pub static DISPLAY: craft_wgsl::Module = craft_wgsl::Module {
    name: "craft-display/display",
    source: include_str!("shaders/display.wgsl"),
    deps: &[&craft_wgsl::TRANSFER, &craft_wgsl::COLOR, &craft_wgsl::MATH],
};

/// Size of the `DisplayParams` uniform block.
pub const PARAMS_BYTES: usize = 160;

/// Texels per row of the shaper-table texture (short rows keep within WebGL2's size limits).
pub const TABLE_ROW: u32 = 1024;

/// The `DisplayParams` block (std140) for `pass`.
pub fn params(pass: &DisplayPass) -> [u8; PARAMS_BYTES] {
    const IDENTITY: [[f32; 4]; 3] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]];
    let (form, quantize, second, lut_size, gamut, m1, m2) = match &pass.color {
        None => (0u32, false, None, 2u32, [0.0f32; 4], IDENTITY, IDENTITY),
        Some(PassColor::Shapers { first, quantize, second }) => {
            (1, *quantize, second.as_deref(), 2, [0.0; 4], first.matrix_rows(), second.as_ref().map_or(IDENTITY, |s| s.matrix_rows()))
        }
        Some(PassColor::Lut { lut, gamut }) => {
            let g = gamut.map_or([0.0; 4], |g| [g.color[0], g.color[1], g.color[2], g.opacity]);
            (2, false, None, lut.size() as u32, g, IDENTITY, IDENTITY)
        }
    };
    let curve = pass.encoding.curve;
    let mut b = Vec::with_capacity(PARAMS_BYTES);
    for v in [pass.channel_id(), pass.colorized as u32, curve.id(), pass.hdr_id(), form, quantize as u32, second.is_some() as u32, lut_size] {
        b.extend(v.to_le_bytes());
    }
    for v in [pass.gain, curve.decode_param(), curve.encode_param(), pass.inverse_gamma()] {
        b.extend(v.to_le_bytes());
    }
    for v in gamut.into_iter().chain(m1.into_iter().flatten()).chain(m2.into_iter().flatten()) {
        b.extend(v.to_le_bytes());
    }
    let mut out = [0u8; PARAMS_BYTES];
    out.copy_from_slice(&b);
    out
}

/// The shaper tables in the order the shader reads them: the first shaper's input and output
/// entries, then the second's. Empty without shapers.
pub fn table_entries(color: Option<&PassColor>) -> Vec<[f32; 4]> {
    let Some(PassColor::Shapers { first, second, .. }) = color else { return Vec::new() };
    let shaper = |s: &GpuMatrixShaper| s.input_table().iter().chain(s.output_table()).copied().collect::<Vec<_>>();
    let mut e = shaper(first);
    if let Some(s) = second {
        e.extend(shaper(s));
    }
    e
}

/// The shaper-table texture: width, height and RGBA32Float bytes ([`TABLE_ROW`] entries per
/// row, the last row padded with zeros). One zero texel without shapers.
pub fn table_texels(color: Option<&PassColor>) -> (u32, u32, Vec<u8>) {
    let entries = table_entries(color);
    if entries.is_empty() {
        return (1, 1, vec![0; 16]);
    }
    let width = TABLE_ROW as usize;
    let height = entries.len().div_ceil(width);
    let mut bytes: Vec<u8> = entries.iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
    bytes.resize(width * height * 16, 0);
    (width as u32, height as u32, bytes)
}

/// The LUT texture: edge length and RGBA32Float bytes, x = red. One zero texel without a LUT.
pub fn lut_texels(color: Option<&PassColor>) -> (u32, Vec<u8>) {
    match color {
        Some(PassColor::Lut { lut, .. }) => (lut.size() as u32, lut.data().iter().flatten().flat_map(|v| v.to_le_bytes()).collect()),
        _ => (1, vec![0; 16]),
    }
}

/// WGSL declaring the two lookup textures at `@group(group)`, bindings `tables` (2D,
/// unfilterable float) and `lut` (3D, unfilterable float), and the `display_table` and
/// `display_lut` functions [`DISPLAY`] calls.
pub fn texture_access(group: u32, tables: u32, lut: u32) -> String {
    format!(
        "@group({group}) @binding({tables}) var display_tables: texture_2d<f32>;\n\
         @group({group}) @binding({lut}) var display_luts: texture_3d<f32>;\n\
         fn display_table(i: u32) -> vec3<f32> {{\n\
         \x20   let w = textureDimensions(display_tables).x;\n\
         \x20   return textureLoad(display_tables, vec2<u32>(i % w, i / w), 0).xyz;\n\
         }}\n\
         fn display_lut(r: u32, g: u32, b: u32) -> vec4<f32> {{\n\
         \x20   return textureLoad(display_luts, vec3<u32>(r, g, b), 0);\n\
         }}\n"
    )
}
