//! Tests of the preview and Quick Mask colour rules.

use super::*;

// Every `PreviewMode` at coverage 0, 0.5 and 1 gives PhotoCraft's `overlay_pixel` values
// (grayscale 0/128/255 opaque, black matte alpha 255/128/0, white matte likewise, quick mask red at
// 128/64/0), and coverage outside 0..=1 is clamped.

// `QuickMaskLook::pixel`: masked areas tint the unselected pixels, selected areas tint the
// selected ones; the colour and the opacity carry through.
