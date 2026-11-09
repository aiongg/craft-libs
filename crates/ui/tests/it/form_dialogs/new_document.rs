//! New Document's own logic: the built-in cards, the recent list and what a card, the orientation control, Reset and the
//! Save Preset row do to the settings.

use craft_behavior::PixelSize;
use craft_ui::contract::{Bind, SaveKey};
use craft_ui::patterns::form_dialogs::{DocumentOptions, DocumentPreset, NewDocumentDialog};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, Target, trial};
use egui::Vec2;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The resolution a print card is asked to open at, in pixels per inch.
const PRINT_PPI: f64 = 600.0;
/// The resolution a screen card is asked to open at, in pixels per inch.
const SCREEN_PPI: f64 = 96.0;
/// The room of an app window the dialog opens in: wide and tall enough for the cards beside the whole details column.
const WINDOW: Vec2 = Vec2::new(1100.0, 900.0);

/// A print card keeps its physical size at the app's print resolution and opens in inches; a screen card keeps its
/// pixels and takes the screen resolution.
#[trial]
fn a_card_opens_at_the_apps_resolution() {
    let letter = DocumentPreset::new("letter", "Letter", 2550, 3300, 300.0).options(PRINT_PPI, SCREEN_PPI);
    assert_eq!((letter.size, letter.pixels_per_inch, letter.unit), (PixelSize::new(5100, 6600), PRINT_PPI, UnitKind::Inches));
    let hdtv = DocumentPreset::new("hdtv-1080p", "HDTV 1080p", 1920, 1080, 72.0).options(PRINT_PPI, SCREEN_PPI);
    assert_eq!((hdtv.size, hdtv.pixels_per_inch, hdtv.unit), (PixelSize::new(1920, 1080), SCREEN_PPI, UnitKind::Pixels));
}

/// A fixture with the dialog over its settings, which collects what each pass of Create answers with.
struct Rig {
    fixture: Fixture,
    created: Rc<RefCell<Vec<DocumentOptions>>>,
    closed: Rc<Cell<bool>>,
}

fn rig() -> Rig {
    let options = Rc::new(RefCell::new(DocumentOptions::default()));
    let created = Rc::new(RefCell::new(Vec::new()));
    let closed = Rc::new(Cell::new(false));
    let (held, answers, done) = (Rc::clone(&options), Rc::clone(&created), Rc::clone(&closed));
    let mut fixture = Fixture::new(move |ui| {
        let mut options = held.borrow_mut();
        let out = NewDocumentDialog::new(Bind::bound(&mut *options)).show(ui);
        if let Some(result) = out.result {
            answers.borrow_mut().push(result.options);
        }
        if out.closed.is_some() {
            done.set(true);
        }
    });
    fixture.room(WINDOW);
    fixture.settle();
    Rig { fixture, created, closed }
}

/// The node named `label`, or one named `label` followed by its details.
fn named(fixture: &Fixture, label: &str) -> Option<Target> {
    let tree = fixture.tree();
    let key = tree.keys().into_iter().find(|key| tree.name_of(*key).is_some_and(|name| name == label || name.starts_with(&format!("{label}, "))))?;
    fixture.target(key)
}

/// Picks the Letter card of the Print tab.
fn pick_letter(fixture: &mut Fixture) {
    let print = named(fixture, "Print").expect("the Print tab");
    fixture.click(&print);
    fixture.settle();
    let letter = named(fixture, "Letter").expect("the Letter card");
    fixture.click(&letter);
    fixture.settle();
}

/// Enter on a card creates the document once, with the card's settings.
#[trial]
fn enter_on_a_card_creates_once() {
    let Rig { mut fixture, created, closed, .. } = rig();
    pick_letter(&mut fixture);
    assert!(created.borrow().is_empty(), "picking a card does not create");
    fixture.key(egui::Key::Enter, egui::Modifiers::NONE);
    fixture.settle();
    assert!(closed.get(), "Enter on a card closes the dialog");
    let answers = created.borrow();
    assert_eq!(answers.len(), 1, "one answer");
    assert_eq!(answers[0].size, PixelSize::new(2550, 3300));
}

/// Types `name` into the Save Preset row and presses Enter.
fn save_preset_as(fixture: &mut Fixture, name: &str) {
    let save = named(fixture, "Save Preset…").expect("the Save Preset button");
    fixture.click(&save);
    fixture.settle();
    fixture.type_text(name);
    fixture.key(egui::Key::Enter, egui::Modifiers::NONE);
    fixture.settle();
}

/// Enter in the preset name saves the preset under Saved and does not create the document.
#[trial]
fn enter_in_the_preset_name_saves_without_creating() {
    let Rig { mut fixture, created, closed, .. } = rig();
    save_preset_as(&mut fixture, "My size");
    assert!(created.borrow().is_empty() && !closed.get(), "Enter in the name row does not create");
    assert!(named(&fixture, "My size").is_some(), "the preset is listed under Saved");
}

/// A card's name carries its size line, which gives the size in the unit the card opens in and its resolution: inches for
/// a print card, pixels for a screen card, with at most two decimals.
#[trial]
fn a_card_is_named_with_its_size_line() {
    let Rig { mut fixture, .. } = rig();
    let print = named(&fixture, "Print").expect("the Print tab");
    fixture.click(&print);
    fixture.settle();
    let letter = named(&fixture, "Letter").expect("the Letter card");
    let name = fixture.tree().name_of(letter.node).expect("named").to_owned();
    assert_eq!(name, "Letter, 8.5 × 11 in @ 300 ppi");
    let a4 = named(&fixture, "A4").expect("the A4 card");
    assert!(fixture.tree().name_of(a4.node).is_some_and(|name| name.starts_with("A4, 8.27 × 11.") && name.ends_with(" in @ 300 ppi")));
}

const RECENT: SaveKey = SaveKey::new("craft_ui_tests.new_document_recent");

/// A document created from a preset and remembered is the first card of Recent when the dialog opens again under the
/// same key; a result that is not remembered leaves the list as it was.
#[trial]
fn the_created_preset_leads_recent_in_the_next_dialog() {
    let open = Rc::new(Cell::new(true));
    let options = Rc::new(RefCell::new(DocumentOptions::default()));
    let remember = Rc::new(Cell::new(false));
    let (shown, held, keep) = (Rc::clone(&open), Rc::clone(&options), Rc::clone(&remember));
    let mut fixture = Fixture::new(move |ui| {
        let mut options = held.borrow_mut();
        let mut is_open = shown.get();
        let out = NewDocumentDialog::new(Bind::bound(&mut *options)).remember_recent(RECENT).opened(Bind::bound(&mut is_open)).show(ui);
        shown.set(is_open);
        if let Some(result) = out.result
            && keep.get()
        {
            result.remember(ui);
        }
    });
    fixture.room(WINDOW);
    fixture.settle();
    pick_letter(&mut fixture);
    let create = fixture.by_label("Create").expect("the Create button");
    fixture.click(&create);
    fixture.settle();
    assert!(!open.get(), "Create closes the dialog");

    open.set(true);
    fixture.settle();
    let recent = named(&fixture, "Recent").expect("the Recent tab");
    fixture.click(&recent);
    fixture.settle();
    assert!(named(&fixture, "Letter").is_none(), "a result nobody remembered adds no recent card");

    open.set(false);
    fixture.settle();
    remember.set(true);
    open.set(true);
    fixture.settle();
    pick_letter(&mut fixture);
    let create = fixture.by_label("Create").expect("the Create button");
    fixture.click(&create);
    fixture.settle();
    open.set(true);
    fixture.settle();
    let recent = named(&fixture, "Recent").expect("the Recent tab");
    fixture.click(&recent);
    fixture.settle();
    assert!(named(&fixture, "Letter").is_some(), "the remembered preset is listed in Recent");
}
