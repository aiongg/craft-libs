//! The Properties panel: a stub until the panel step fills it.

use craft_ui::composites::EmptyState;
use craft_ui::patterns::Panel;
use egui::Ui;

pub fn show(ui: &mut Ui) {
    Panel::new("properties").title("Properties").show(ui, |ui| {
        EmptyState::new("Properties come here.").show(ui);
    });
}
