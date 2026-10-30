//! The Adjustments panel: a grid of buttons, each adding an adjustment layer above the selected
//! layer. The mock has one kind of adjustment, so each button adds that kind under its own name.

use craft_theme::ControlSize;
use craft_ui::controls::IconButton;
use craft_ui::icons::Icon;
use craft_ui::patterns::Panel;
use craft_ui::primitives::{Content, Text};
use egui::Ui;

use super::super::model::Mock;

/// PhotoCraft's order and glyphs for the adjustments the mock draws.
const ADJUSTMENTS: [(Icon, &str); 6] = [
    (Icon::Sun, "Brightness/Contrast"),
    (Icon::Adjustments, "Levels"),
    (Icon::Droplet, "Hue/Saturation"),
    (Icon::Contrast, "Black & White"),
    (Icon::Palette, "Gradient Map"),
    (Icon::AdjustmentLayer, "Photo Filter"),
];

/// Returns whether a button added a layer, which the Properties panel then shows.
pub fn show(ui: &mut Ui, mock: &mut Mock) -> bool {
    Panel::new("adjustments")
        .title("Adjustments")
        .show(ui, |ui| {
            ui.add(Text::new("Add an adjustment").content(Content::Muted));
            let mut added = None;
            ui.horizontal_wrapped(|ui| {
                for (icon, name) in ADJUSTMENTS {
                    if ui.add(IconButton::new(icon, name).size(ControlSize::Md)).clicked() {
                        added = Some(name);
                    }
                }
            });
            added.is_some_and(|name| {
                mock.add_adjustment(name);
                true
            })
        })
        .inner
}
