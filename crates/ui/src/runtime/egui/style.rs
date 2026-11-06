//! The theme styles' `right_to_left`.

use egui::{Context, Ui};

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
