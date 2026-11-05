//! The first-launch offer to bring in another editor's shortcut set (Photoshop's live set on a
//! machine that has Photoshop): asked once, never applied unasked.
//!
//! The app finds the file (it knows where its sibling editor keeps it), reads it with
//! [`crate::import::kys::read`], and hands the set to [`ImportOffer::prepare`] with what its
//! preferences remember. The offer is made only when the user has no shortcut overrides of their
//! own and the set changes at least one shortcut; every outcome is remembered so a later launch
//! never asks again.

use craft_ui::dialog::{ButtonRole, DialogId, DialogResult, DialogStack, Prompt, StackDialog, dialog_id};
use craft_ui::localized::Labels as _;
use craft_ui::localized::LocaleNumbers;
use serde::{Deserialize, Serialize};

use crate::import::{ForeignSet, ImportOptions, ImportPlan, plan};
use crate::keymap::Keymap;

/// What the app's preferences remember about the offer, under its own key (PhotoCraft's
/// `dialogs.photoshopShortcuts`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferRecord {
    /// Where the set was found.
    pub source: String,
    /// Whether the user imported it.
    pub imported: bool,
}

/// Why no offer is made.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NoOffer {
    /// Asked before (the record exists).
    Answered,
    /// The user has overrides of their own; remember the source as not imported.
    OwnShortcuts,
    /// The set changes nothing; remember the source as not imported.
    NothingChanges,
}

impl NoOffer {
    /// What to store so a later launch never asks again; `None` when the record already exists.
    pub fn record(&self, source: &str) -> Option<OfferRecord> {
        match self {
            NoOffer::Answered => None,
            NoOffer::OwnShortcuts | NoOffer::NothingChanges => Some(OfferRecord { source: source.to_owned(), imported: false }),
        }
    }
}

/// A set waiting for the user's answer.
#[derive(Clone, Debug)]
pub struct ImportOffer {
    /// Where the set was found (Photoshop's `Keyboard Shortcuts.psp`).
    pub source: String,
    /// The set's name, else its file name.
    pub set_name: String,
    /// The plan the answer Import applies.
    pub plan: ImportPlan,
    /// Commands whose shortcuts end up different from the app's defaults; the offer is made only
    /// when this is above 0.
    pub changes: usize,
    labels: Option<ImportOfferLabels>,
}

/// The user's answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferAnswer {
    /// Import the set.
    Import,
    /// Leave the keymap alone.
    DontImport,
}

impl ImportOffer {
    /// The offer for `set` found at `source`, or why none is made. `record` is what the
    /// preferences hold, `None` on first launch.
    // pc: crates/ui-egui/src/prefs/kys_import.rs:161 `offer_import`.
    pub fn prepare(
        source: &str,
        set: &ForeignSet,
        keymap: &Keymap,
        options: &ImportOptions,
        record: Option<&OfferRecord>,
    ) -> Result<ImportOffer, NoOffer> {
        if record.is_some() {
            return Err(NoOffer::Answered);
        }
        if !keymap.overrides().is_empty() {
            return Err(NoOffer::OwnShortcuts);
        }
        let plan = plan(set, keymap, options);
        // Counted against the defaults, not the import report: a command the set leaves on its
        // defaults is not a change, whatever the keymap passed in held before.
        let mut applied = keymap.clone();
        plan.apply(&mut applied);
        let changes = plan.commands.iter().filter(|p| applied.bindings(&p.command) != applied.default_bindings(&p.command)).count();
        if changes == 0 {
            return Err(NoOffer::NothingChanges);
        }
        let set_name = set
            .name
            .clone()
            .or_else(|| std::path::Path::new(source).file_name().map(|f| f.to_string_lossy().into_owned()))
            .unwrap_or_else(|| source.to_owned());
        Ok(ImportOffer { source: source.to_owned(), set_name, plan, changes, labels: None })
    }

    /// The record to store for `answer`.
    pub fn record(&self, answer: OfferAnswer) -> OfferRecord {
        OfferRecord { source: self.source.clone(), imported: answer == OfferAnswer::Import }
    }

    /// The offer's texts, instead of the ones installed on the context.
    pub fn labels(mut self, labels: ImportOfferLabels) -> Self {
        self.labels = Some(labels);
        self
    }
}

/// The prompt on the dialog stack, queued at [`StartupStage::Offer`](craft_ui::dialog::StartupStage::Offer):
/// "Import keyboard shortcuts?", the line naming the set and the count, the line on undoing it
/// (Reset All to Defaults), and Import (default, Enter) and Don't Import (Esc). Esc and the
/// button both answer [`OfferAnswer::DontImport`]; a dialog closed from outside gives
/// [`DialogResult::Cancel`], which the app does not record, so the next launch asks again. On
/// `Ok` the app applies the plan on Import ([`ImportPlan::apply`]), stores [`ImportOffer::record`]
/// either way and posts a notice that the set is in use.
impl StackDialog for ImportOffer {
    type Result = OfferAnswer;

    fn id(&self) -> DialogId {
        dialog_id(("keymap-import-offer", &self.source))
    }

    // pc: crates/ui-egui/src/prefs/kys_import.rs:214 `show_offer`, :187 `answer_offer`.
    fn show(&mut self, ctx: &egui::Context, stack: &DialogStack) -> Option<DialogResult<OfferAnswer>> {
        let labels = self.labels.clone().unwrap_or_else(|| ImportOfferLabels::of_ctx(ctx));
        let app = craft_host::Host::try_of_ctx(ctx).map_or("Craft", |host| host.app().name);
        let found = craft_i18n::fmt(
            &labels.found,
            &[("app", app), ("set", &self.set_name), ("n", &LocaleNumbers::of_ctx(ctx).format(self.changes as f64, 0..=0))],
        );
        let role = Prompt::new(("keymap-import-offer", &self.source), &labels.title, format!("{found}\n\n{}", labels.undo))
            .answer(ButtonRole::Default, &labels.import, None)
            .answer(ButtonRole::Cancel, &labels.dont_import, None)
            .stack(stack)
            .show(ctx)?;
        Some(DialogResult::Ok(if role == ButtonRole::Default { OfferAnswer::Import } else { OfferAnswer::DontImport }))
    }
}

craft_ui::words! {
    /// The offer's texts, English by default. `{set}` is the set's name, `{n}` the number of
    /// shortcuts that change, `{app}` the app's name, `{error}` why the import failed. The last
    /// three are the notice after an import and the status line after a failed one; the app
    /// writes them with `craft_i18n::fmt` and `LocaleNumbers`.
    #[non_exhaustive]
    pub struct ImportOfferLabels {
        title: "shortcuts.importOffer.title" = "Import keyboard shortcuts?";
        found: "shortcuts.importOffer.found" = "{app} found your keyboard shortcut set {set} from another image editor on this computer: {n} of its shortcuts differ from {app}'s. Import it?";
        undo: "shortcuts.importOffer.undo" = "Edit › Keyboard Shortcuts › Reset All to Defaults undoes an import.";
        import: "shortcuts.importOffer.import" = "Import";
        dont_import: "shortcuts.importOffer.dontImport" = "Don't Import";
        imported_title: "shortcuts.importOffer.importedTitle" = "Keyboard shortcuts imported";
        imported: "shortcuts.importOffer.imported" = "Your keyboard shortcut set {set} is in use here: {n} shortcuts differ from the defaults. Edit › Keyboard Shortcuts shows them; Reset All to Defaults undoes this.";
        not_imported: "shortcuts.importOffer.notImported" = "The keyboard shortcut set was not imported: {error}";
    }
}
