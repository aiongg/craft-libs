//! The args of every story shown so far, and the Properties panel that edits the target story's.
use std::collections::HashMap;

use craft_theme::{ControlSize, TypeRole};
use craft_ui::composites::PropertyRow;
use craft_ui::controls::{Checkbox, IconButton, NumberField, Select, TextField};
use craft_ui::direction;
use craft_ui::icons::Icon;
use craft_ui::localized::LocaleNumbers;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{ArgValue, Args, Story};
use egui::Ui;

use crate::fields;
use crate::persist;

/// Edited args by story id; a story gets its entry when first shown, from the saved values.
pub(super) struct ArgStore {
    edited: HashMap<String, Args>,
    saved: HashMap<String, Vec<persist::SavedArg>>,
}

impl ArgStore {
    pub(super) fn new(saved: HashMap<String, Vec<persist::SavedArg>>) -> Self {
        Self { edited: HashMap::new(), saved }
    }

    /// The args of `story`, from the saved values on first use.
    pub(super) fn get(&mut self, story: &Story) -> &mut Args {
        args_of(&mut self.edited, &self.saved, story)
    }

    /// The edited and the saved args, for code that draws several stories at once.
    pub(super) fn parts(&mut self) -> (&mut HashMap<String, Args>, &HashMap<String, Vec<persist::SavedArg>>) {
        (&mut self.edited, &self.saved)
    }

    /// Puts the args of `story` back to its defaults.
    pub(super) fn reset(&mut self, story: &Story) {
        self.edited.insert(story.id.to_owned(), story.defaults());
        self.saved.remove(story.id);
    }

    /// The saved args with the edited ones written over them.
    pub(super) fn to_saved(&self) -> HashMap<String, Vec<persist::SavedArg>> {
        let mut saved = self.saved.clone();
        for (id, args) in &self.edited {
            saved.insert(id.clone(), persist::save_args(args));
        }
        saved
    }

    /// The target story's args, under a header with its name and Reset.
    pub(super) fn show(&mut self, ui: &mut Ui, story: &Story) {
        let changed = *self.get(story) != story.defaults();
        let mut reset = false;
        direction::horizontal(ui, |ui| {
            ui.add(Text::new(story.component).role(TypeRole::Heading));
            ui.add(Text::new(story.name).content(Content::Muted));
            if changed {
                ui.with_layout(direction::trailing(ui), |ui| {
                    reset = ui.add(IconButton::new(Icon::Undo, "Reset args").size(ControlSize::Sm)).clicked()
                });
            }
        });
        if reset {
            self.reset(story);
        }
        let numbers = LocaleNumbers::of(ui);
        let args = self.get(story);
        for arg in args.iter_mut() {
            let id = ("gallery.arg", story.id, arg.name);
            PropertyRow::word(arg.name).show(ui, |ui, _| match &mut arg.value {
                ArgValue::Bool(b) => Checkbox::bool("", b).show(ui).response,
                ArgValue::Choice { options, index } => {
                    let mut select = Select::new(id, arg.name, index).fit(true);
                    for (i, option) in options.iter().enumerate() {
                        select = select.option(i, *option);
                    }
                    select.policy(fields::policy()).show(ui).response
                }
                ArgValue::Text(t) => {
                    let edit = TextField::new(id, arg.name, t).policy(fields::policy()).show(ui);
                    fields::apply(&edit, t);
                    edit.response
                }
                ArgValue::Number { value, min, max } => {
                    let edit = NumberField::new(id, arg.name, *value, &numbers)
                        .range(*min..=*max)
                        .step((*max - *min) / 200.0)
                        .policy(fields::policy())
                        .show(ui);
                    fields::apply(&edit, value);
                    edit.response
                }
            });
        }
    }
}

/// The args of `story` in `args`, from the saved values on first use.
pub(crate) fn args_of<'a>(args: &'a mut HashMap<String, Args>, saved: &HashMap<String, Vec<persist::SavedArg>>, story: &Story) -> &'a mut Args {
    args.entry(story.id.to_owned()).or_insert_with(|| match saved.get(story.id) {
        Some(values) => persist::restore_args(story.defaults(), values),
        None => story.defaults(),
    })
}
