//! egui focus, `move_focus` and `surrender_focus_on`.

use egui::{Context, Id, SurrenderFocusOn};

/// Stops egui from taking focus away on its own: focus on press is the runtime's (8.3).
pub(crate) fn never_surrender_focus(ctx: &Context) {
    ctx.options_mut(|options| options.input_options.surrender_focus_on = SurrenderFocusOn::Never);
}

/// The widget id egui holds key focus on.
pub(crate) fn focused(ctx: &Context) -> Option<Id> {
    ctx.memory(|memory| memory.focused())
}
