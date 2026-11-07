//! The first-launch import offer.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::{ImportOffer, ImportOfferLabels, NoOffer, OfferAnswer, OfferRecord};
use craft_ui::dialog::{DialogHost, DialogResult, StartupStage};

use crate::chord::Platform;
use crate::import::kys;
use crate::keymap::{Bindable, Binding, Keymap, Overrides};
use crate::scope::{Scope, Scopes};
use crate::{CommandRef, ForeignSet, ImportOptions};

const PSP: &str = "<photoshop-keyboard-shortcuts version=\"4\" filename=\"$$$/FileName/Presets/KeyboardCustomization/Mine=Mine\">\
    <command name=\"Save\"><shortcut>Cmd+S</shortcut></command>\
    <command name=\"Rulers\"><shortcut>Cmd+K</shortcut></command></photoshop-keyboard-shortcuts>";

fn keymap() -> Keymap {
    let command = |id: &str, label: &str, key: &str| Bindable::new(id, label, Scope::app()).with_default(Binding::parse(key).unwrap());
    Keymap::new(vec![command("file.save", "Save", "Cmd+S"), command("view.rulers", "Rulers", "Cmd+R")], Scopes::new(), Platform::Mac)
}

fn set(text: &str) -> ForeignSet {
    kys::read(text).unwrap()
}

fn prepare(km: &Keymap, record: Option<&OfferRecord>) -> Result<ImportOffer, NoOffer> {
    ImportOffer::prepare("/ps/Keyboard Shortcuts.psp", &set(PSP), km, &ImportOptions::default(), record)
}

#[test]
// No record and no overrides offers; a record, own overrides or a set equal to the defaults gives NoOffer with the record to store.
fn offer_is_made_once_and_only_for_a_set_that_changes_something() {
    let offer = prepare(&keymap(), None).unwrap();
    assert_eq!((offer.set_name.as_str(), offer.changes), ("Mine", 1), "Save keeps its key; Rulers changes");

    let answered = OfferRecord { source: "/ps/Keyboard Shortcuts.psp".into(), imported: false };
    let refused = prepare(&keymap(), Some(&answered)).unwrap_err();
    assert_eq!((refused.clone(), refused.record("/ps")), (NoOffer::Answered, None));

    let mut own = keymap();
    let mut overrides = Overrides::new();
    overrides.insert("file.save", vec!["Cmd+Shift+S".into()]);
    own.set_overrides(overrides);
    let refused = prepare(&own, None).unwrap_err();
    assert_eq!(refused, NoOffer::OwnShortcuts);
    assert_eq!(refused.record("/ps"), Some(OfferRecord { source: "/ps".into(), imported: false }));

    let defaults = ImportOffer::prepare("/ps/x.psp", &set(&PSP.replace("Cmd+K", "Cmd+R")), &keymap(), &ImportOptions::default(), None).unwrap_err();
    assert_eq!(defaults, NoOffer::NothingChanges);
    assert_eq!(defaults.record("/ps/x.psp").map(|r| r.imported), Some(false));

    // Without a name in the file the offer names the file.
    let unnamed =
        ImportOffer::prepare("/ps/My Keys.psp", &set(&PSP.replace(" filename=", " x=")), &keymap(), &ImportOptions::default(), None).unwrap();
    assert_eq!(unnamed.set_name, "My Keys.psp");
}

#[test]
// Import from the shown prompt returns OfferAnswer::Import and applying the plan changes the counted shortcuts.
fn import_answer_applies_the_plan() {
    let offer = prepare(&keymap(), None).unwrap();
    let mut host = DialogHost::new();
    host.enqueue(StartupStage::Offer, offer.labels(ImportOfferLabels::default()));
    let answer = None;
    let mut harness = Harness::new_ui_state(
        |ui, (host, answer): &mut (DialogHost, Option<OfferAnswer>)| {
            host.show(ui.ctx());
            if let Some(DialogResult::Ok(given)) = host.take::<ImportOffer>() {
                *answer = Some(given);
            }
        },
        (host, answer),
    );
    harness.run();
    assert!(harness.get_all_by_label("Import keyboard shortcuts?").count() > 0);
    assert!(harness.get_all_by_label_contains("Mine").count() > 0);
    harness.get_by_role_and_label(Role::Button, "Import").click();
    harness.run();
    let answer = harness.state().1;
    assert_eq!(answer, Some(OfferAnswer::Import));

    let offer = prepare(&keymap(), None).unwrap();
    assert_eq!(offer.record(OfferAnswer::Import), OfferRecord { source: offer.source.clone(), imported: true });
    assert!(!offer.record(OfferAnswer::DontImport).imported);
    let mut km = keymap();
    let report = offer.plan.apply(&mut km);
    assert_eq!(report.imported, offer.changes);
    let rulers: Vec<String> = km.bindings(&CommandRef::new("view.rulers")).iter().map(|b| b.trigger.to_string()).collect();
    assert_eq!(rulers, ["Cmd+K"]);
}

#[test]
// Only commands that end up off their defaults count: a set equal to the defaults counts 0, one with two changed commands counts 2.
fn the_count_is_the_commands_that_differ_from_the_defaults() {
    let command = |id: &str, label: &str, key: &str| Bindable::new(id, label, Scope::app()).with_default(Binding::parse(key).unwrap());
    let km = Keymap::new(
        vec![command("file.save", "Save", "Cmd+S"), command("view.rulers", "Rulers", "Cmd+R"), command("view.grid", "Grid", "Cmd+G")],
        Scopes::new(),
        Platform::Mac,
    );
    let file = |save: &str, rulers: &str, grid: &str| {
        format!(
            "<photoshop-keyboard-shortcuts version=\"4\"><command name=\"Save\"><shortcut>{save}</shortcut></command>\
            <command name=\"Rulers\"><shortcut>{rulers}</shortcut></command>\
            <command name=\"Grid\"><shortcut>{grid}</shortcut></command></photoshop-keyboard-shortcuts>"
        )
    };
    let prepare = |text: String| ImportOffer::prepare("/ps/x.psp", &set(&text), &km, &ImportOptions::default(), None);
    assert_eq!(prepare(file("Cmd+S", "Cmd+R", "Cmd+G")).unwrap_err(), NoOffer::NothingChanges);
    assert_eq!(prepare(file("Cmd+S", "Cmd+K", "Cmd+J")).unwrap().changes, 2);
}

#[test]
// A foreign name that fits an alias command and the menu command prefers the one with a default shortcut.
fn a_name_shared_with_an_alias_command_matches_the_one_with_a_default() {
    let undo = |cmd: &str| format!("<photoshop-keyboard-shortcuts version=\"4\"><command name=\"Undo\"><shortcut>{cmd}</shortcut></command></photoshop-keyboard-shortcuts>");
    let km = Keymap::new(
        vec![
            Bindable::new("history.undo", "Undo", Scope::app()),
            Bindable::new("edit.undo", "Undo", Scope::app()).with_path(vec!["Edit".into(), "Undo".into()]).with_default(Binding::parse("Cmd+Z").unwrap()),
        ],
        Scopes::new(),
        Platform::Mac,
    );
    let offer = |text: &str| ImportOffer::prepare("/ps/u.psp", &set(text), &km, &ImportOptions::default(), None);
    assert_eq!(offer(&undo("Cmd+Z")).unwrap_err(), NoOffer::NothingChanges);
    assert_eq!(offer(&undo("Cmd+Shift+Z")).unwrap().changes, 1);
}
