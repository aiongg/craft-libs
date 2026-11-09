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
    let english = Words::new(|_| None).set::<Trial>();
    let set = german().set::<Trial>();
    assert_ne!(set.save.as_str(), english.save.as_str(), "a translated row shows the translation");
    assert_ne!(set.corners[0].as_str(), english.corners[0].as_str());
    assert_eq!(set.corners[1].as_str(), english.corners[1].as_str(), "a missing row is English");
    assert_eq!(set.plain.as_str(), english.plain.as_str());
    assert_eq!(set.size.fill(&[("n", "12")]), english.size.fill(&[("n", "12")]));
    assert!(set.saved.fill(&[("name", "a.txt")]).contains("a.txt"));
    assert_eq!(Trial::english().rows().text(SAVE), Some(english.save.as_str()));
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

// An overlay scope reuses its words across passes: the word set built under it is built once.
#[trial]
fn an_overlay_scope_builds_its_word_set_once_across_passes() {
    let mut frames = Frames::new();
    let mut sets = Vec::new();
    for _ in 0..2 {
        frames.frame(vec![], |ui| {
            with_env(
                ui,
                |o: &mut Overrides| o.words = Some(WordsOverride::Overlay(WordRows::new().row(SAVE, "Sichern"))),
                |ui| sets.push(Env::of(ui).words::<Trial>()),
            );
        });
    }
    assert!(std::sync::Arc::ptr_eq(&sets[0], &sets[1]));
}

// The words of a `Host`'s catalogs reach a scope's word set in the host's language, and a language switch
// (a new `Host`) changes them.
#[trial]
fn the_words_of_the_host_catalog_reach_the_word_set() {
    use craft_host::{AppInfo, Host};
    use std::sync::Arc;
    let (i18n, problems) = craft_i18n::I18n::from_owned([("de".to_owned(), String::new(), "@id\ttrial.save\tSpeichern\n".to_owned())], &["de"]);
    assert!(problems.is_empty(), "{problems:?}");
    let i18n = Arc::new(i18n);
    let german = i18n.from_code("de").expect("offered");
    let english = Arc::new(Host::builder(AppInfo { id: "trial", name: "Trial", version: "0" }).i18n(i18n).build());
    let mut frames = Frames::new();
    let read = |frames: &mut Frames, host: &Arc<Host>| {
        frames.host(host.clone());
        let mut text = String::new();
        frames.frame(vec![], |ui| text = Env::of(ui).words::<Trial>().save.as_str().to_owned());
        text
    };
    assert_eq!(read(&mut frames, &english), "Save");
    assert_eq!(read(&mut frames, &Arc::new(english.with_lang(german))), "Speichern");
}
