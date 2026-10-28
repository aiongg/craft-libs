//! `foundations/stock-egui`: unmigrated egui widgets under the generated `Style`.

use egui::{Id, Ui};

use super::{Args, Level, Story};
use crate::ComboBox;

pub(super) fn stories() -> Vec<Story> {
    vec![Story {
        id: "foundations/stock-egui/widgets",
        level: Level::Foundations,
        component: "Stock egui",
        name: "Widgets",
        args: || Args::new().with_bool("enabled", true),
        show,
    }]
}

/// A value that lives in the context between frames, keyed under `ui`.
fn kept<T: Clone + Send + Sync + 'static>(ui: &Ui, name: &str, init: T) -> T {
    let id = ui.scope_id().with(("stock-egui", name));
    ui.data_mut(|d| d.get_temp_mut_or_insert_with(id, || init).clone())
}

fn keep<T: Clone + Send + Sync + 'static>(ui: &Ui, name: &str, value: T) {
    let id: Id = ui.scope_id().with(("stock-egui", name));
    ui.data_mut(|d| d.insert_temp(id, value));
}

fn show(ui: &mut Ui, args: &mut Args) {
    ui.add_enabled_ui(args.bool("enabled"), |ui| {
        ui.horizontal(|ui| {
            let _ = ui.button("Button");
            ui.add(egui::Button::new("Selected").selected(true));
            let _ = ui.small_button("Small");
        });

        let mut checked = kept(ui, "check", true);
        if ui.checkbox(&mut checked, "Checkbox").changed() {
            keep(ui, "check", checked);
        }

        let mut radio = kept(ui, "radio", 0usize);
        ui.horizontal(|ui| {
            for (i, label) in ["One", "Two", "Three"].into_iter().enumerate() {
                ui.radio_value(&mut radio, i, label);
            }
        });
        keep(ui, "radio", radio);

        let mut value = kept(ui, "slider", 40.0f32);
        ui.add(egui::Slider::new(&mut value, 0.0..=100.0).text("Slider"));
        keep(ui, "slider", value);

        let mut text = kept(ui, "text", String::from("Text edit"));
        ui.text_edit_singleline(&mut text);
        keep(ui, "text", text);

        let mut choice = kept(ui, "combo", 0usize);
        let options = ["Alpha", "Beta", "Gamma"];
        ComboBox::new("combo", "Combo box").show_index(ui, &mut choice, options.len(), |i| options[i]);
        keep(ui, "combo", choice);

        ui.collapsing("Collapsing header", |ui| {
            ui.label("Contents");
        });
        ui.separator();
        ui.hyperlink_to("Hyperlink", "https://example.com");
        ui.add(egui::ProgressBar::new(0.6).show_percentage());
    });
}
