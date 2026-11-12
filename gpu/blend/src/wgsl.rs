//! The blend formulas in WGSL, one module per [`Variant`].
//!
//! A shader module serves one app, so the variant is a module constant (`BLEND_VARIANT`) and
//! drivers drop the other variants' branches. [`Range`](crate::Range) is the runtime argument
//! `float_range`, because PhotoCraft switches it per document. Every function and constant is
//! named `blend_*` / `BLEND_*`.

use craft_wgsl::{COLOR, MATH, Module};

use crate::mode::{MODE_IDS_WGSL, Variant};

/// `BLEND_<MODE>` for every [`BlendMode`](crate::BlendMode), from the Rust enum.
pub static IDS: Module = Module { name: "craft-blend/ids", source: MODE_IDS_WGSL, deps: &[] };

/// `BLEND_STANDARD` …: the variant numbers of [`Variant::id`].
static VARIANT_IDS: Module = Module {
    name: "craft-blend/variant-ids",
    source: "const BLEND_STANDARD: u32 = 0u;\nconst BLEND_PHOTOSHOP: u32 = 1u;\nconst BLEND_AFTER_EFFECTS: u32 = 2u;\nconst BLEND_SUBSTANCE: u32 = 3u;\n",
    deps: &[],
};

// The numbers in VARIANT_IDS, checked against the enum when the crate compiles.
const _: () = assert!(Variant::Standard.id() == 0 && Variant::Photoshop.id() == 1 && Variant::AfterEffects.id() == 2 && Variant::Substance.id() == 3);

static FORMULAS: Module =
    Module { name: "craft-blend/formulas", source: include_str!("shaders/blend.wgsl"), deps: &[&IDS, &VARIANT_IDS, &COLOR, &MATH] };

/// The formulas with W3C and Premiere's rules.
pub static STANDARD: Module = Module { name: "craft-blend/standard", source: "const BLEND_VARIANT: u32 = BLEND_STANDARD;\n", deps: &[&FORMULAS] };
/// The formulas with Photoshop's rules.
pub static PHOTOSHOP: Module = Module { name: "craft-blend/photoshop", source: "const BLEND_VARIANT: u32 = BLEND_PHOTOSHOP;\n", deps: &[&FORMULAS] };
/// The formulas with After Effects' rules.
pub static AFTER_EFFECTS: Module =
    Module { name: "craft-blend/after-effects", source: "const BLEND_VARIANT: u32 = BLEND_AFTER_EFFECTS;\n", deps: &[&FORMULAS] };
/// The formulas with Substance's rules.
pub static SUBSTANCE: Module = Module { name: "craft-blend/substance", source: "const BLEND_VARIANT: u32 = BLEND_SUBSTANCE;\n", deps: &[&FORMULAS] };

/// The module of `variant`: pass it to [`craft_wgsl::assemble`] with the shader's own code.
pub fn module(variant: Variant) -> &'static Module {
    match variant {
        Variant::Standard => &STANDARD,
        Variant::Photoshop => &PHOTOSHOP,
        Variant::AfterEffects => &AFTER_EFFECTS,
        Variant::Substance => &SUBSTANCE,
    }
}
