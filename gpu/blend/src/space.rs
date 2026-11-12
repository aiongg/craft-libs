//! The colour spaces a composite blends or mixes in.

/// A colour encoding a composite works in. [`Blend::composite`](crate::Blend::composite) takes two:
/// the blend space, where `B(Cb, Cs)` is computed, and the mix space, where coverage mixes the
/// backdrop, the source and the blend.
pub trait Space {
    /// True when `encode` and `decode` do nothing, so composites skip them.
    const IDENTITY: bool = false;
    /// Into the space.
    fn encode(&self, c: [f32; 3]) -> [f32; 3];
    /// Out of the space.
    fn decode(&self, c: [f32; 3]) -> [f32; 3];
}

/// The values as given.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Linear;

impl Space for Linear {
    const IDENTITY: bool = true;
    fn encode(&self, c: [f32; 3]) -> [f32; 3] {
        c
    }
    fn decode(&self, c: [f32; 3]) -> [f32; 3] {
        c
    }
}

/// sRGB-encoded display values: linear light in, encoded colour for the blend, and the blend
/// clamped to 0..1 on the way back. Premiere blends every mode but Normal this way ("as artists
/// expect from every editing app").
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Srgb;

impl Space for Srgb {
    fn encode(&self, c: [f32; 3]) -> [f32; 3] {
        c.map(craft_wgsl::linear_to_srgb)
    }
    fn decode(&self, c: [f32; 3]) -> [f32; 3] {
        c.map(|v| craft_wgsl::srgb_to_linear(v.clamp(0.0, 1.0)))
    }
}

/// Photoshop's "Blend Text Colors Using Gamma" (Color Settings › Advanced, 1.45 by default):
/// type layers mix their colour with the backdrop in linear light raised to `1 / gamma`, so
/// anti-aliased glyph edges come out lighter over dark backdrops than a mix of the encoded values
/// (psd-tools layer_effects.psd: a 69% edge pixel of a blue overlay over grey is 57/255 in red;
/// an encoded mix gives 40). Gamma 2.2 is close to mixing the encoded values, lower settings
/// approach linear light. Fitted on that file (every coverage level within 0.6/255; a plain
/// `v^1.45` power mix is 6/255 too light at 94% coverage) and ag-psd float-color.
///
/// Photoshop's setting 1 means "off": a mix of the encoded values, which is [`Linear`].
/// `TextGamma(1.0)` would mix in linear light instead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextGamma(pub f32);

/// Photoshop's default text blending gamma.
pub const TEXT_GAMMA: f32 = 1.45;

impl Space for TextGamma {
    fn encode(&self, c: [f32; 3]) -> [f32; 3] {
        c.map(|v| craft_wgsl::srgb_to_linear(v.max(0.0)).powf(1.0 / self.0))
    }
    fn decode(&self, c: [f32; 3]) -> [f32; 3] {
        c.map(|v| craft_wgsl::linear_to_srgb(v.max(0.0).powf(self.0)))
    }
}
