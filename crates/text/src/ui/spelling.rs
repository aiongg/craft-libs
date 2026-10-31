//! [`CheckSpellingDialog`] (F14.06): Edit › Check Spelling on the dialog stack, and
//! [`SpellingSettings`], the user's dictionary as a settings section.
//!
//! The dialog walks a list of misspellings the app found (with [`crate::spell::Dictionary`]
//! over its text objects). Each button the app has to act on comes back from
//! [`CheckSpellingDialog::take_action`]; the app changes its text, finds the misspellings again
//! and hands the dialog the new list with [`CheckSpellingDialog::set_items`]:
//!
//! ```ignore
//! dialogs.open(CheckSpellingDialog::new(find_misspellings(doc, false, &[])));
//! // each pass, while open:
//! if let Some(dialog) = dialogs.get_mut::<CheckSpellingDialog>() {
//!     if let Some(action) = dialog.take_action() {
//!         run_spelling_action(doc, &action, &mut settings);
//!         dialog.set_items(find_misspellings(doc, dialog.all_layers(), dialog.ignored()));
//!     }
//! }
//! let _ = dialogs.take::<CheckSpellingDialog>(); // Done or the close box
//! ```

use std::ops::Range;

use craft_host::Section;
use craft_ui::behavior::{FieldTab, RowInfo, Selection};
use craft_ui::controls::{Button, Checkbox, TextField};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, Outcome, StackDialog, Width};
use craft_ui::list::{List, ListEvent, ListRow};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Surface, SurfaceKind};
use egui::Context;
use serde::{Deserialize, Serialize};

craft_ui::words! {
    /// The dialog's words, English by default (PhotoCraft's strings).
    #[non_exhaustive]
    pub struct CheckSpellingLabels {
        title: "checkSpelling.title" = "Check Spelling";
        not_in_dictionary: "checkSpelling.notInDictionary" = "Not in Dictionary:";
        change_to: "checkSpelling.changeTo" = "Change To:";
        suggestions: "checkSpelling.suggestions" = "Suggestions:";
        all_layers: "checkSpelling.allLayers" = "Check All Layers";
        complete: "checkSpelling.complete" = "Spell check complete.";
        done: "checkSpelling.done" = "Done";
        ignore: "checkSpelling.ignore" = "Ignore";
        ignore_all: "checkSpelling.ignoreAll" = "Ignore All";
        change: "checkSpelling.change" = "Change";
        change_all: "checkSpelling.changeAll" = "Change All";
        add: "checkSpelling.add" = "Add";
    }
}

/// How many suggestions the app asks [`Dictionary::suggest`](crate::spell::Dictionary::suggest)
/// for (PhotoCraft's `"suggestions": 8`).
pub const SUGGESTION_COUNT: usize = 8;

/// Width of the word and Change To fields and of the suggestions list (PhotoCraft's 240 pt).
const FIELD_WIDTH: f32 = 240.0;
/// Height of the suggestions list (PhotoCraft's 110 pt).
const SUGGESTIONS_HEIGHT: f32 = 110.0;
/// Row height of the suggestions list.
const SUGGESTION_ROW: f32 = 22.0;

/// A button of the right-hand column that the dialog or the app acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Ignore,
    IgnoreAll,
    Change,
    ChangeAll,
    Add,
    /// Check All Layers changed.
    Relist,
}

/// The user's dictionary, saved under `[spelling]`. Words are kept as added; read them through
/// [`UserWords::from_words`](crate::spell::UserWords::from_words).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpellingSettings {
    pub user_words: Vec<String>,
}

impl Section for SpellingSettings {
    const KEY: &'static str = "spelling";
}

/// One misspelling in the app's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpellingItem {
    /// The text object holding it (PhotoCraft: the layer id).
    pub object: u64,
    /// Byte range of the word in that object's text.
    pub range: Range<usize>,
    pub word: String,
    /// At most [`SUGGESTION_COUNT`], best first.
    pub suggestions: Vec<String>,
}

/// A button the app acts on. Ignore is the dialog's own (it moves to the next item).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpellingAction {
    /// Replace this one occurrence with `to`.
    Change { item: SpellingItem, to: String },
    /// Replace every occurrence of `word` with `to`, in every text object when `all_layers`.
    ChangeAll { word: String, to: String, all_layers: bool },
    /// Add `word` to the user's dictionary ([`SpellingSettings`]).
    Add(String),
    /// Find the misspellings again: Ignore All added a word to [`CheckSpellingDialog::ignored`],
    /// or Check All Layers changed.
    Relist,
}

/// Edit › Check Spelling. Photoshop's and PhotoCraft's layout: at the left, "Not in Dictionary:"
/// with the word (read only), "Change To:" (a text field, opening with the first suggestion),
/// "Suggestions:" (a list 240 × 110 pt), Check All Layers, and "Spell check complete." once the
/// list is empty; at the right, Done (default), then Ignore, Ignore All, Change, Change All, Add,
/// each 100 pt wide and disabled when the list is empty.
///
/// - A click on a suggestion puts it in Change To; a double-click also changes.
/// - Enter is Done unless Change To has focus; Esc and the close box are Done too (nothing to
///   undo: every change was made as it was asked for). Tab walks Change To, the suggestions
///   (one stop, arrows inside), the check box and the buttons.
/// - Result: `()`; the dialog always ends with [`DialogResult::Ok`] or, closed from outside,
///   `Cancel`.
pub struct CheckSpellingDialog {
    items: Vec<SpellingItem>,
    index: usize,
    /// Ignore All words for this check.
    ignored: Vec<String>,
    change_to: String,
    all_layers: bool,
    action: Option<SpellingAction>,
    labels: Option<CheckSpellingLabels>,
    state: DialogState,
}

impl CheckSpellingDialog {
    /// Over `items` from a first search (the current layer: Check All Layers off).
    pub fn new(items: Vec<SpellingItem>) -> Self {
        let change_to = items.first().and_then(|i| i.suggestions.first()).cloned().unwrap_or_default();
        CheckSpellingDialog {
            items,
            index: 0,
            ignored: Vec::new(),
            change_to,
            all_layers: false,
            action: None,
            labels: None,
            state: DialogState::new(),
        }
    }

    /// Open with Check All Layers on.
    pub fn all_layers_on(mut self, on: bool) -> Self {
        self.all_layers = on;
        self
    }

    pub fn labels(mut self, labels: CheckSpellingLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The button pressed this pass that the app must run, once.
    pub fn take_action(&mut self) -> Option<SpellingAction> {
        self.action.take()
    }

    /// The misspellings found again after an action. The dialog keeps its position in the list
    /// (after Change, Ignore All and Add those words are gone, so the same position is the next
    /// misspelling; earlier ones skipped with Ignore stay skipped), except after Check All Layers
    /// changed, when it starts from the first. Change To gets the current item's first suggestion.
    // pc: crates/ui-egui/src/panels/type_panels_ui.rs:759 `refresh_spelling` (index kept) and the
    // "relist" arm of `spelling_window` (:778, index 0).
    pub fn set_items(&mut self, items: Vec<SpellingItem>) {
        self.items = items;
        self.change_to = self.items.get(self.index).and_then(|i| i.suggestions.first()).cloned().unwrap_or_default();
    }

    /// Whether Check All Layers is on.
    pub fn all_layers(&self) -> bool {
        self.all_layers
    }

    /// The words Ignore All added during this check, for the app's next search to skip.
    pub fn ignored(&self) -> &[String] {
        &self.ignored
    }
}

impl StackDialog for CheckSpellingDialog {
    type Result = ();

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("check-spelling")
    }

    // pc: crates/ui-egui/src/panels/type_panels_ui.rs:778 `spelling_window` (layout, sizes,
    // double-click on a suggestion, the action match). On `Dialog` + `Footer` (Done as OK, no
    // Cancel) with the right-hand button column.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<()>> {
        let labels = self.labels.clone().unwrap_or_else(|| CheckSpellingLabels::of_ctx(ctx));
        let tokens = craft_ui::theme::current(ctx);
        let gap = tokens.component.dialog.button_gap;
        let item = self.items.get(self.index).cloned();
        let has_item = item.is_some();

        let mut change_to = std::mem::take(&mut self.change_to);
        let mut all_layers = self.all_layers;
        let mut body_press: Option<Press> = None;
        let mut button_press: Option<Press> = None;

        let footer = {
            let buttons = [
                (labels.ignore.clone(), Press::Ignore),
                (labels.ignore_all.clone(), Press::IgnoreAll),
                (labels.change.clone(), Press::Change),
                (labels.change_all.clone(), Press::ChangeAll),
                (labels.add.clone(), Press::Add),
            ];
            let press = &mut button_press;
            Footer::side().ok(labels.done.clone()).without_cancel().below(move |ui| {
                let width = ui.available_width();
                for (index, (label, which)) in buttons.into_iter().enumerate() {
                    let response = ui.add(Button::new(label).enabled(has_item).min_width(width).id_salt(("check-spelling", "button", index)));
                    if has_item {
                        FieldTab::register_stop(ui, response.id);
                    }
                    if response.clicked() {
                        *press = Some(which);
                    }
                }
            })
        };

        let shown = Dialog::new("check-spelling", labels.title.as_str())
            .stack(stack)
            .width(Width::Fixed(FIELD_WIDTH + tokens.component.dialog.side_min + 3.0 * gap))
            .footer(footer)
            .show(ctx, &mut self.state, &mut (), |ui, ()| {
                ui.vertical(|ui| {
                    ui.label(labels.not_in_dictionary.as_str());
                    let word = item.as_ref().map_or("", |i| i.word.as_str());
                    let _ = TextField::new(("check-spelling", "word"), labels.not_in_dictionary.as_str(), word)
                        .enabled(false)
                        .width(FIELD_WIDTH)
                        .show(ui);
                    ui.label(labels.change_to.as_str());
                    let edit = TextField::new(("check-spelling", "change-to"), labels.change_to.as_str(), &change_to)
                        .enabled(has_item)
                        .width(FIELD_WIDTH)
                        .show(ui);
                    if let Some(value) = edit.event.as_ref().and_then(|e| e.value()) {
                        change_to.clone_from(value);
                    }
                    ui.label(labels.suggestions.as_str());
                    let suggestions: &[String] = item.as_ref().map_or(&[], |i| i.suggestions.as_slice());
                    let rows: Vec<RowInfo<usize>> = (0..suggestions.len()).map(RowInfo::leaf).collect();
                    let mut selection = suggestions.iter().position(|s| *s == change_to).map_or_else(Selection::new, Selection::only);
                    Surface::new(SurfaceKind::Sunken).show(ui, |ui| {
                        ui.set_min_size(egui::vec2(FIELD_WIDTH, SUGGESTIONS_HEIGHT));
                        ui.set_max_width(FIELD_WIDTH);
                        let out = List::new(("check-spelling", "suggestions"), labels.suggestions.as_str(), rows.as_slice())
                            .selection(&mut selection)
                            .row_height(SUGGESTION_ROW)
                            .max_height(SUGGESTIONS_HEIGHT)
                            .show(ui, |_, &key| ListRow::new(key, suggestions[key].as_str()).accessible_name(suggestions[key].as_str()));
                        for event in out.events {
                            match event {
                                ListEvent::Selection(change) => {
                                    if let Some(row) = change.row {
                                        change_to.clone_from(&suggestions[row]);
                                    }
                                }
                                ListEvent::Activate(row) => {
                                    change_to.clone_from(&suggestions[row]);
                                    body_press = Some(Press::Change);
                                }
                                _ => {}
                            }
                        }
                    });
                    if Checkbox::bool(labels.all_layers.as_str(), &mut all_layers).show(ui).event.is_some() {
                        body_press = Some(Press::Relist);
                    }
                    if !has_item {
                        ui.label(labels.complete.as_str());
                    }
                });
            });

        self.change_to = change_to;
        self.all_layers = all_layers;
        match body_press.or(button_press) {
            Some(Press::Ignore) => {
                self.index = (self.index + 1).min(self.items.len());
                self.change_to = self.items.get(self.index).and_then(|i| i.suggestions.first()).cloned().unwrap_or_default();
            }
            Some(Press::IgnoreAll) => {
                if let Some(item) = &item {
                    self.ignored.push(item.word.clone());
                    self.action = Some(SpellingAction::Relist);
                }
            }
            Some(Press::Change) => {
                if let Some(item) = item {
                    self.action = Some(SpellingAction::Change { item, to: self.change_to.clone() });
                }
            }
            Some(Press::ChangeAll) => {
                if let Some(item) = item {
                    self.action = Some(SpellingAction::ChangeAll { word: item.word, to: self.change_to.clone(), all_layers: self.all_layers });
                }
            }
            Some(Press::Add) => {
                if let Some(item) = item {
                    self.action = Some(SpellingAction::Add(item.word));
                }
            }
            Some(Press::Relist) => {
                self.index = 0;
                self.action = Some(SpellingAction::Relist);
            }
            None => {}
        }
        match shown.outcome? {
            Outcome::Ok | Outcome::Cancel | Outcome::Close => Some(DialogResult::Ok(())),
            _ => None,
        }
    }
}
