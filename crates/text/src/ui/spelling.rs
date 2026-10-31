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

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::Range;

use craft_host::Section;
use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
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
}

impl CheckSpellingDialog {
    /// Over `items` from a first search (the current layer: Check All Layers off).
    pub fn new(items: Vec<SpellingItem>) -> Self {
        let change_to = items.first().and_then(|i| i.suggestions.first()).cloned().unwrap_or_default();
        CheckSpellingDialog { items, index: 0, ignored: Vec::new(), change_to, all_layers: false, action: None, labels: None }
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
        todo!()
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
        todo!()
    }
}
