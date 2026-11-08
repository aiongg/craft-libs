//! Story of the import offer's prompt.

use super::ImportOfferWords;
use craft_i18n::WordSet as _;
use craft_ui::contract::Bind;
use craft_ui::dialog::Prompt;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(Prompt<'static>, covers = [], drive = Drive::Never)]
fn import_offer(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let words = ImportOfferWords::english();
    let found = words.found.fill(&[("app", "PhotoCraft"), ("set", "Mine"), ("n", "12")]);
    Prompt::new(words.title.clone(), format!("{found}\n\n{}", words.undo.as_str()))
        .answers([FooterButton::new(words.import.clone(), DialogRole::Accept), FooterButton::new(words.dont_import.clone(), DialogRole::Cancel)])
        .opened(Bind::bound(data.value("open", true)))
        .show(ui)
        .into()
}
