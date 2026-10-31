//! Eyedropper: the sampling ring round the pointer. Its radius and band are the same at every
//! Size; the outer band's grey is overridable. The inner band shows the sampled and the current
//! colour, which are not tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Eyedropper tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct EyedropperTokens {
    /// Outer radius of the sampling ring: `eyedropper.ring_radius`.
    pub ring_radius: f32,
    /// Width of each of the ring's two bands: `eyedropper.ring_band`.
    pub ring_band: f32,
    /// The ring's opaque outer band: `eyedropper.ring_grey`, grey 127.
    pub ring_grey: Rgba,
}

/// An overridable eyedropper field: the ring's grey. Path: `ring_grey`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EyedropperField {
    /// `eyedropper.ring_grey`
    RingGrey,
}

impl Family for EyedropperTokens {
    type Field = EyedropperField;
    const FIELDS: &'static [EyedropperField] = &[EyedropperField::RingGrey];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { EyedropperTokens;
        "ring_radius" 0.0..=1024.0 => ring_radius;
        "ring_band" 0.0..=256.0 => ring_band;
    };

    fn defaults(_: &Semantic<'_>) -> EyedropperTokens {
        EyedropperTokens {
            ring_radius: values::EYEDROPPER_RING_RADIUS,
            ring_band: values::EYEDROPPER_RING_BAND,
            ring_grey: values::EYEDROPPER_RING_GREY,
        }
    }

    fn path(_: EyedropperField) -> String {
        "ring_grey".to_owned()
    }

    fn from_path(path: &str) -> Option<EyedropperField> {
        (path == "ring_grey").then_some(EyedropperField::RingGrey)
    }

    fn kind(_: EyedropperField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, _: EyedropperField) -> OverrideValue {
        OverrideValue::Color(self.ring_grey)
    }

    fn set(&mut self, _: EyedropperField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            self.ring_grey = c;
        }
    }

    /// The ring is an opaque band with hairlines on both edges; it states no pair.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
