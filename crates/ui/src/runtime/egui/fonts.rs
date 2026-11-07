//! The egui font calls only; the top-level `fonts` module owns the definitions and the coverage policy.

use egui::{Context, FontDefinitions};

/// Gives egui `definitions`; the next pass lays text out with them.
pub(crate) fn set_definitions(ctx: &Context, definitions: FontDefinitions) {
    ctx.set_fonts(definitions);
}

/// Lays glyphs out without grid-fitting their advances: a text is as wide in points at every pixel density, so a
/// layout measured at one interface scale holds at another.
pub(crate) fn lay_out_unhinted(ctx: &Context) {
    ctx.all_styles_mut(|style| style.visuals.text_options.font_hinting = false);
}
