//! The theme styles' `right_to_left`.

use egui::{Context, Ui};

/// Turns off egui's own caret blink in `ui`, and hides the caret stroke unless `visible`: the runtime blinks it.
pub(crate) fn set_text_cursor(ui: &mut Ui, visible: bool) {
    ui.visuals_mut().text_cursor.blink = false;
    if !visible {
        ui.visuals_mut().text_cursor.stroke.color = egui::Color32::TRANSPARENT;
    }
}

/// Paints the selected text of `ui`'s text edits in `fill`.
pub(crate) fn set_text_selection(ui: &mut Ui, fill: egui::Color32) {
    ui.visuals_mut().selection.bg_fill = fill;
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

/// Sets the spacing between the children `ui` lays out.
pub(crate) fn set_item_spacing(ui: &mut Ui, gap: egui::Vec2) {
    ui.spacing_mut().item_spacing = gap;
}
