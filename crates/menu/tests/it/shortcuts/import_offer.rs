//! The first-launch import offer.

use std::cell::RefCell;
use std::rc::Rc;

use craft_keymap::import::kys;
use craft_keymap::{Bindable, Binding, CommandRef, ForeignSet, ImportOptions, Keymap, Overrides, Platform, Scope, Scopes};
use craft_menu::shortcuts::{ImportOffer, NoOffer, OfferAnswer, OfferRecord};
use craft_ui::dialog::{DialogHost, DialogResult, StartupStage};
use craft_ui::testing::{Fixture, trial};

const PSP: &str = "<photoshop-keyboard-shortcuts version=\"4\" filename=\"$$$/FileName/Presets/KeyboardCustomization/Mine=Mine\">\
    <command name=\"Save\"><shortcut>Cmd+S</shortcut></command>\
    <command name=\"Rulers\"><shortcut>Cmd+K</shortcut></command></photoshop-keyboard-shortcuts>";

fn keymap() -> Keymap {
    let command = |id: &str, label: &str, key: &str| Bindable::new(id, label, Scope::app()).with_default(Binding::parse(key).expect("a binding"));
    Keymap::new(vec![command("file.save", "Save", "Cmd+S"), command("view.rulers", "Rulers", "Cmd+R")], Scopes::new(), Platform::Mac)
}

fn set(text: &str) -> ForeignSet {
    kys::read(text).expect("a set")
}

fn prepare(km: &Keymap, record: Option<&OfferRecord>) -> Result<ImportOffer, NoOffer> {
    ImportOffer::prepare("/ps/Keyboard Shortcuts.psp", &set(PSP), km, &ImportOptions::default(), record)
}

/// No record and no overrides offers; a record, own overrides or a set equal to the defaults gives `NoOffer` with
/// the record to store. Only commands that end up off their defaults count.
#[trial]
fn the_offer_is_made_once_and_only_for_a_set_that_changes_something() {
    let offer = prepare(&keymap(), None).expect("an offer");
    assert_eq!((offer.set_name.as_str(), offer.changes), ("Mine", 1), "Save keeps its key; Rulers changes");

    let answered = OfferRecord::new("/ps/Keyboard Shortcuts.psp", false);
    let refused = prepare(&keymap(), Some(&answered)).expect_err("answered before");
    assert_eq!((refused.clone(), refused.record("/ps")), (NoOffer::Answered, None));

    let mut own = keymap();
    let mut overrides = Overrides::new();
    overrides.insert("file.save", vec!["Cmd+Shift+S".into()]);
    own.set_overrides(overrides);
    let refused = prepare(&own, None).expect_err("own shortcuts");
    assert_eq!(refused, NoOffer::OwnShortcuts);
    assert_eq!(refused.record("/ps"), Some(OfferRecord::new("/ps", false)));

    let defaults = ImportOffer::prepare("/ps/x.psp", &set(&PSP.replace("Cmd+K", "Cmd+R")), &keymap(), &ImportOptions::default(), None);
    assert_eq!(defaults.expect_err("no change"), NoOffer::NothingChanges);

    let unnamed = ImportOffer::prepare("/ps/My Keys.psp", &set(&PSP.replace(" filename=", " x=")), &keymap(), &ImportOptions::default(), None)
        .expect("an offer");
    assert_eq!(unnamed.set_name, "My Keys.psp", "without a name in the file the offer names the file");
    {
        let command = |id: &str, label: &str, key: &str| Bindable::new(id, label, Scope::app()).with_default(Binding::parse(key).expect("a binding"));
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
        assert_eq!(prepare(file("Cmd+S", "Cmd+R", "Cmd+G")).expect_err("no change"), NoOffer::NothingChanges);
        assert_eq!(prepare(file("Cmd+S", "Cmd+K", "Cmd+J")).expect("an offer").changes, 2);
    }
}

/// Import on the shown prompt answers `Import`, and applying the plan changes the counted shortcuts.
#[trial]
fn the_import_answer_applies_the_plan() {
    let offer = prepare(&keymap(), None).expect("an offer");
    let host = Rc::new(RefCell::new(DialogHost::new()));
    host.borrow_mut().enqueue(StartupStage::Offer, offer);
    let answer = Rc::new(RefCell::new(None));
    let (shown, given) = (Rc::clone(&host), Rc::clone(&answer));
    let mut fixture = Fixture::new(move |ui| {
        shown.borrow_mut().run(ui);
        if let Some(DialogResult::Ok(done)) = shown.borrow_mut().take::<ImportOffer>() {
            *given.borrow_mut() = Some(done);
        }
    });
    fixture.settle();
    assert!(fixture.by_label("Import keyboard shortcuts?").is_some());
    let import = fixture.by_label("Import").expect("the Import button");
    fixture.click(&import).settle();
    assert_eq!(*answer.borrow(), Some(OfferAnswer::Import));

    let offer = prepare(&keymap(), None).expect("an offer");
    assert!(offer.record(OfferAnswer::Import).imported && !offer.record(OfferAnswer::DontImport).imported);
    let mut km = keymap();
    let report = offer.plan.apply(&mut km);
    assert_eq!(report.imported, offer.changes);
    let rulers: Vec<String> = km.bindings(&CommandRef::new("view.rulers")).iter().map(|b| b.trigger.to_string()).collect();
    assert_eq!(rulers, ["Cmd+K"]);
}

/// A foreign name that fits an alias command and the menu command prefers the one with a default shortcut.
#[trial]
fn a_name_shared_with_an_alias_command_matches_the_one_with_a_default() {
    let undo = |cmd: &str| {
        format!(
            "<photoshop-keyboard-shortcuts version=\"4\"><command name=\"Undo\"><shortcut>{cmd}</shortcut></command></photoshop-keyboard-shortcuts>"
        )
    };
    let km = Keymap::new(
        vec![
            Bindable::new("history.undo", "Undo", Scope::app()),
            Bindable::new("edit.undo", "Undo", Scope::app())
                .with_path(vec!["Edit".into(), "Undo".into()])
                .with_default(Binding::parse("Cmd+Z").expect("a binding")),
        ],
        Scopes::new(),
        Platform::Mac,
    );
    let offer = |text: &str| ImportOffer::prepare("/ps/u.psp", &set(text), &km, &ImportOptions::default(), None);
    assert_eq!(offer(&undo("Cmd+Z")).expect_err("no change"), NoOffer::NothingChanges);
    assert_eq!(offer(&undo("Cmd+Shift+Z")).expect("an offer").changes, 1);
}
