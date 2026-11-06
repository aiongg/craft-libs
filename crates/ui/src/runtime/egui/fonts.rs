//! The egui font calls only; the top-level `fonts` module owns the definitions and the coverage policy.

use egui::{Context, FontDefinitions};

/// Gives egui `definitions`; the next pass lays text out with them.
pub(crate) fn set_definitions(ctx: &Context, definitions: FontDefinitions) {
    ctx.set_fonts(definitions);
}
