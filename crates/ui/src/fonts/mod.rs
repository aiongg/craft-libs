//! Font definitions and the on-demand script coverage policy; the egui calls sit in `runtime::egui::fonts`.

use crate::runtime::egui::fonts as egui_fonts;
use crate::runtime::layout_memory;
use egui::{Context, FontDefinitions};

/// Gives the UI `definitions`, as when on-demand script coverage loads the glyphs of another script. Text
/// measured in this pass used the old metrics, so the pass asks for one discard pass: layout memory and the
/// new metrics agree when the frame is shown.
pub fn load(ctx: &Context, definitions: FontDefinitions) {
    egui_fonts::set_definitions(ctx, definitions);
    layout_memory::ask_discard(ctx);
}
