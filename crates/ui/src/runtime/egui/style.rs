//! `ui.disable()` for env scopes and gates, the theme styles' `right_to_left` and a scope's child `Ui` direction.

use egui::{Align, Context, Layout, Ui, UiBuilder};

/// Marks `ui` and everything below it disabled.
#[doc(hidden)]
pub fn disable(ui: &mut Ui) {
    ui.disable();
}

/// Whether egui considers `ui` enabled.
pub(crate) fn is_enabled(ui: &Ui) -> bool {
    ui.is_enabled()
}

/// Writes the direction into both theme styles and into `root`, whose style was copied before the begin hook ran.
pub(crate) fn set_theme_direction(ctx: &Context, root: &mut Ui, right_to_left: bool) {
    ctx.all_styles_mut(|style| {
        if style.right_to_left != right_to_left {
            style.right_to_left = right_to_left;
        }
    });
    if root.style().right_to_left != right_to_left {
        root.style_mut().right_to_left = right_to_left;
    }
}

/// The physical layout of a body with no layout of its own: top down, items hugging the reading start.
fn stack_layout(right_to_left: bool) -> Layout {
    Layout::top_down(if right_to_left { Align::Max } else { Align::Min })
}

/// Runs `body` in a child `Ui` that reads `right_to_left`: a cloned style carrying the direction and an
/// explicit layout, never the parent's.
pub(crate) fn scope_direction<R>(ui: &mut Ui, right_to_left: bool, body: impl FnOnce(&mut Ui) -> R) -> R {
    let mut style = (**ui.style()).clone();
    style.right_to_left = right_to_left;
    let builder = UiBuilder::new().layout(stack_layout(right_to_left)).style(style);
    ui.scope_builder(builder, body).inner
}
