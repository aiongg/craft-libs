//! Prompts: the letters shown on answers, and which answer a key gives.

use craft_host::target::Os;
use craft_ui::contract::Role;
use craft_ui::contract::tooltip::DELAY;
use craft_ui::dialog::{Alert, Prompt, RecoveredDocument, RecoveryDialog, StandardPrompt, accelerator_labels};
use craft_ui::runtime::env::{Overrides, Word, WordRows, WordsOverride, with_env};
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::pointer::moved;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// Frames a modal layer needs to open and place focus.
const OPEN_FRAMES: usize = 30;

const SAVE: usize = 0;
const DONT_SAVE: usize = 1;
const CANCEL: usize = 2;

/// A prompt of three answers on `os`; `picked` holds the last answer a pass reported.
fn rig(os: Os) -> (Fixture, Rc<Cell<Option<usize>>>) {
    let picked = Rc::new(Cell::new(None));
    let shown = picked.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = Prompt::new("Unsaved changes", "Save the changes?")
            .answers([
                FooterButton::new("Save", DialogRole::Accept).letter('S'),
                FooterButton::new("Don't Save", DialogRole::Other).letter('D'),
                FooterButton::new("Cancel", DialogRole::Cancel).letter('C'),
            ])
            .show(ui);
        if out.picked.is_some() {
            shown.set(out.picked);
        }
    });
    fixture.os(os).idle(OPEN_FRAMES);
    (fixture, picked)
}

#[trial]
fn escape_gives_the_cancel_answer() {
    let (mut fixture, picked) = rig(Os::Linux);
    fixture.key(Key::Escape, Modifiers::NONE).idle(2);
    assert_eq!(picked.get(), Some(CANCEL));
}

#[trial]
fn enter_gives_the_accept_answer_on_every_os() {
    for os in [Os::Windows, Os::Linux, Os::Mac] {
        let (mut fixture, picked) = rig(os);
        fixture.key(Key::Enter, Modifiers::NONE).idle(2);
        assert_eq!(picked.get(), Some(SAVE), "{os:?}");
    }
}

#[trial]
fn a_bare_letter_gives_its_answer() {
    let (mut fixture, picked) = rig(Os::Linux);
    fixture.key(Key::D, Modifiers::NONE).idle(2);
    assert_eq!(picked.get(), Some(DONT_SAVE));
}

#[trial]
fn labels_show_the_letter_in_one_style_per_prompt() {
    let english = [("Yes", Some('Y')), ("No", Some('N')), ("Cancel", Some('C'))];
    assert_eq!(accelerator_labels(&english, Os::Linux), ["(Y)es", "(N)o", "(C)ancel"]);
    let german = [("Ja", Some('Y')), ("Nein", Some('N')), ("Abbrechen", Some('C'))];
    assert_eq!(accelerator_labels(&german, Os::Windows), ["Ja (Y)", "Nein (N)", "Abbrechen (C)"]);
    let mixed = [("Retry", Some('R')), ("Skip", None)];
    assert_eq!(accelerator_labels(&mixed, Os::Linux), ["(R)etry", "Skip"]);
    let wide = [("Écraser", Some('E')), ("Annuler", Some('C'))];
    assert_eq!(accelerator_labels(&wide, Os::Linux), ["Écraser (E)", "Annuler (C)"]);
}

#[trial]
fn macos_shows_plain_labels() {
    let save = [("Don't Save", Some('D')), ("Cancel", Some('C'))];
    assert_eq!(accelerator_labels(&save, Os::Mac), ["Don't Save", "Cancel"]);
}

const SAVE_WORD: Word = Word::keyed("trial.prompt.save", "Save");
const DELETE_WORD: Word = Word::keyed("prompt.delete", "Delete");
const CANCEL_WORD: Word = Word::keyed("common.cancel", "Cancel");

/// Runs `show` in a scope whose words are German for the rows the trials read.
fn german(ui: &mut egui::Ui, show: impl FnOnce(&mut egui::Ui)) {
    let rows = WordRows::new().row(SAVE_WORD, "Sichern").row(DELETE_WORD, "Löschen").row(CANCEL_WORD, "Abbrechen");
    with_env(ui, |o: &mut Overrides| o.words = Some(WordsOverride::Overlay(rows)), show);
}

#[trial]
fn german_labels_get_their_letters_from_their_own_text() {
    let picked = Rc::new(Cell::new(None));
    let shown = picked.clone();
    let mut fixture = Fixture::new(move |ui| {
        german(ui, |ui| {
            let out = StandardPrompt::confirm_delete(["Ebene"], 1).show(ui);
            if let Some(answer) = out.answer {
                shown.set(Some(answer.choice));
            }
        });
    });
    fixture.os(Os::Linux).idle(OPEN_FRAMES);
    fixture.key(Key::C, Modifiers::NONE).idle(2);
    assert_eq!(picked.get(), None, "Abbrechen has no letter C");
    fixture.key(Key::A, Modifiers::NONE).idle(2);
    assert_eq!(picked.get(), Some(craft_ui::dialog::PromptChoice::Cancel));
}

#[trial]
fn a_prompt_built_from_words_shows_translated_labels() {
    let mut fixture = Fixture::new(|ui| {
        german(ui, |ui| {
            Prompt::new("Title", "Message").answers([FooterButton::new(SAVE_WORD, DialogRole::Accept)]).show(ui);
        });
    });
    fixture.os(Os::Linux).idle(OPEN_FRAMES);
    assert!(fixture.texts().iter().any(|text| text.text == "Sichern"));
}

const BLOCKED: &str = "Stop recording first";

#[trial]
fn a_blocked_recover_button_is_disabled_with_its_reason_and_enter_does_not_answer_it() {
    let answered = Rc::new(Cell::new(false));
    let shown = answered.clone();
    let mut fixture = Fixture::new(move |ui| {
        let document = RecoveredDocument::new("poster", "poster.psd", Duration::from_secs(300)).blocked(BLOCKED);
        if RecoveryDialog::new(document).show(ui).answer.is_some() {
            shown.set(true);
        }
    });
    fixture.os(Os::Linux).idle(OPEN_FRAMES);
    fixture.key(Key::Enter, Modifiers::NONE).idle(2);
    assert!(!answered.get(), "Enter does not answer a blocked Recover");
    let recover = fixture.by_label("Recover").expect("the Recover button");
    fixture.click(&recover).idle(2);
    assert!(!answered.get(), "a click does not answer a blocked Recover");
    fixture.frame(vec![moved(recover.center())]);
    fixture.advance(DELAY);
    fixture.idle(3);
    let tips = fixture.by_role(Role::TOOLTIP);
    assert_eq!(tips.first().and_then(|tip| fixture.tree().name_of(tip.node)), Some(BLOCKED));
}

#[trial]
fn an_alert_is_an_alert_dialog_described_by_its_message() {
    let id = Rc::new(Cell::new(None::<Id>));
    let shown = id.clone();
    let mut fixture = Fixture::new(move |ui| {
        shown.set(Some(Alert::new("About", "Version 1.0").show(ui).response.id));
    });
    fixture.os(Os::Linux).idle(OPEN_FRAMES);
    let id = id.get().expect("the alert showed");
    assert_eq!(fixture.tree().role(id), Some(Role::ALERT_DIALOG));
    assert_eq!(fixture.tree().description(id), Some("Version 1.0"));
}
