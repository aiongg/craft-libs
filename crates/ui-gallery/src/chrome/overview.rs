//! The centre tabs: the Overview and a component's tab. For now each shows every story of one component, stacked.

use crate::viewer::{Catalog, Cells};
use craft_theme::variant::Tone;
use craft_ui::contract::{Props as _, Show as _, ShowContainer as _};
use craft_ui::display::Text;
use craft_ui::layout_components::Stack;
use craft_ui::notices::Notice;
use craft_ui::style::TypeRole;
use egui::Ui;

/// Every story of the component registered as `type_name`, each live under the root's values. Each story is its own id
/// scope, so a story's `id_salt` and auto ids never meet another's; the reset `generation` is part of it, so a reset
/// mounts the stories anew.
pub fn stories(ui: &mut Ui, catalog: &Catalog, cells: &mut Cells, generation: u32, type_name: Option<&'static str>) {
    let Some(type_name) = type_name else {
        Notice::new(Tone::Info, "Select a component to see its stories.").show(ui);
        return;
    };
    for (index, entry) in catalog.entries.iter().enumerate().filter(|(_, entry)| entry.type_name == type_name) {
        let data = cells.entry(index).or_default();
        Stack::new().id_salt((index, generation)).show(ui, |ui, _| {
            Text::new(entry.registration.story.name).role(TypeRole::Heading).show(ui);
            let _ = (entry.registration.story.show)(ui, data);
        });
    }
}
