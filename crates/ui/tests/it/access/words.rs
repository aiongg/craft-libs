//! Words and labels: a missing translation shows the English text, and a scope overrides words one way.

use crate::contract::kit::{Probe, Seen};
use craft_ui::contract::{Props as _, Show as _};
use craft_ui::runtime::env::{Env, Overrides, Word, WordRows, WordSet, Words, WordsOverride, with_env};
use craft_ui::testing::{Frames, trial};
use std::cell::RefCell;

craft_ui::words! {
    /// The words the trials read.
    struct Trial {
        save: "trial.save" = "Save";
        saved: template "trial.saved" = "Saved {name}";
        size: unit "trial.size" = "{n} px";
        corners: array ["trial.cornerTl" = "Top left", "trial.cornerTr" = "Top right"];
        key: context "key" = "Option";
        plain: plain = "Cancel";
        items: count "trial.items" = "{n} item" | "{n} items";
    }
}

const SAVE: Word = Word::keyed("trial.save", "Save");

fn german() -> Words {
    Words::new(|word| match word.id {
        "trial.save" => Some("Speichern".to_owned()),
        "trial.saved" => Some("{name} gespeichert".to_owned()),
        "trial.cornerTl" => Some("Oben links".to_owned()),
        _ => None,
    })
}

#[trial]
fn a_missing_translation_shows_the_english_text() {
    let set = german().set::<Trial>();
    assert_eq!(set.save.as_str(), "Speichern");
    assert_eq!(set.corners[0].as_str(), "Oben links");
    assert_eq!(set.corners[1].as_str(), "Top right", "a missing row is English");
    assert_eq!(set.plain.as_str(), "Cancel");
    assert_eq!(set.saved.fill(&[("name", "a.txt")]), "a.txt gespeichert");
    assert_eq!(set.size.fill(&[("n", "12")]), "12 px");
    assert_eq!((set.items.text(1, "1"), set.items.text(2, "2")), ("1 item".to_owned(), "2 items".to_owned()));
    assert_eq!(Trial::WORDS.len(), 8);
    assert_eq!(Trial::COUNTS.len(), 1);
    assert_eq!(Trial::english().rows().text(SAVE), Some("Save"));
}

#[trial]
fn a_scope_overlays_words_and_a_word_label_is_resolved_at_step_one() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut read = String::new();
    frames.frame(vec![], |ui| {
        with_env(
            ui,
            |o: &mut Overrides| o.words = Some(WordsOverride::Overlay(WordRows::new().row(SAVE, "Sichern"))),
            |ui| {
                read = Env::of(ui).words::<Trial>().save.as_str().to_owned();
                Probe::new(&seen).label(SAVE).show(ui);
            },
        );
    });
    assert_eq!(read, "Sichern");
    let id = seen.borrow().id.expect("updated");
    assert_eq!(frames.tree().name(id), Some("Sichern"));
    let outside = RefCell::new(Seen::default());
    frames.frame(vec![], |ui| {
        Probe::new(&outside).label(SAVE).show(ui);
    });
    let id = outside.borrow().id.expect("updated");
    assert_eq!(frames.tree().name(id), Some("Save"), "outside the scope the label is English");
}
