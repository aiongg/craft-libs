//! The Check Spelling dialog on the dialog stack in a harness.

use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::theme::{Base, install};
use egui::Key;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::spelling::*;

struct App {
    host: DialogHost,
    actions: Vec<SpellingAction>,
    results: Vec<DialogResult<()>>,
}

fn item(object: u64, word: &str, suggestions: &[&str]) -> SpellingItem {
    SpellingItem { object, range: 0..word.len(), word: word.to_owned(), suggestions: suggestions.iter().map(|s| (*s).to_owned()).collect() }
}

fn items() -> Vec<SpellingItem> {
    vec![item(1, "teh", &["the", "tea"]), item(1, "recieve", &["receive", "relieve"]), item(1, "wrold", &["world", "wold"])]
}

fn harness(items: Vec<SpellingItem>) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(CheckSpellingDialog::new(items));
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(640.0, 480.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(dialog) = app.host.get_mut::<CheckSpellingDialog>()
                    && let Some(action) = dialog.take_action()
                {
                    app.actions.push(action);
                }
                if let Some(result) = app.host.take::<CheckSpellingDialog>() {
                    app.results.push(result);
                }
            },
            App { host, actions: Vec::new(), results: Vec::new() },
        )
}

// Ignore moves to the next misspelling without an action; Ignore All returns `Relist` and adds
// the word to `ignored`.
#[test]
fn ignore_moves_on_and_ignore_all_relists() {
    let mut harness = harness(items());
    harness.run();
    harness.get_by_label("Ignore").click();
    harness.run();
    assert!(harness.state().actions.is_empty());
    harness.get_by_label("Change").click();
    harness.run();
    assert_eq!(harness.state().actions, [SpellingAction::Change { item: items()[1].clone(), to: "receive".into() }]);

    harness.state_mut().actions.clear();
    harness.get_by_label("Ignore All").click();
    harness.run();
    assert_eq!(harness.state().actions, [SpellingAction::Relist]);
    let dialog = harness.state().host.get::<CheckSpellingDialog>().unwrap();
    assert_eq!(dialog.ignored(), ["recieve"]);
}

// Change returns `Change` with the item and the Change To text; Enter on a suggestion (which the
// list reports as `Activate`, as it does a double-click) returns `Change` with that suggestion.
#[test]
fn change_uses_change_to_and_enter_on_a_suggestion_changes() {
    let mut harness = harness(items());
    harness.run();
    harness.get_by_label("Change").click();
    harness.run();
    assert_eq!(harness.state().actions, [SpellingAction::Change { item: items()[0].clone(), to: "the".into() }]);

    harness.state_mut().actions.clear();
    harness.get_by_label("tea").click();
    harness.run();
    assert!(harness.state().actions.is_empty(), "a click only fills Change To");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().actions, [SpellingAction::Change { item: items()[0].clone(), to: "tea".into() }]);
}

// An empty list shows "Spell check complete." and disables every button but Done; Done and the
// close box end the dialog.
#[test]
fn empty_list_shows_complete_and_done_closes() {
    let mut harness = harness(Vec::new());
    harness.run();
    harness.get_by_label("Spell check complete.");
    for label in ["Ignore", "Ignore All", "Change", "Change All", "Add"] {
        assert!(harness.get_by_label(label).accesskit_node().is_disabled(), "{label}");
    }
    assert!(!harness.get_by_label("Done").accesskit_node().is_disabled());
    harness.get_by_label("Done").click();
    harness.run();
    assert!(matches!(harness.state().results[..], [DialogResult::Ok(())]));

    let mut harness = self::harness(Vec::new());
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(matches!(harness.state().results[..], [DialogResult::Ok(())]));
}
