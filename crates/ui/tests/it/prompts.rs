//! Prompts: the letters shown on answers, and which answer a key gives.

use craft_host::target::Os;
use craft_ui::dialog::{Prompt, accelerator_labels};
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

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
fn enter_gives_the_accept_answer() {
    let (mut fixture, picked) = rig(Os::Windows);
    fixture.key(Key::Enter, Modifiers::NONE).idle(2);
    assert_eq!(picked.get(), Some(SAVE));
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
