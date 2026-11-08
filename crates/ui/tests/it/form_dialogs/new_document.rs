//! New Document's own logic: the built-in cards, the recent list and what a card, the orientation control, Reset and the
//! Save Preset row do to the settings.

use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{
    DocumentOptions, DocumentPreset, NewDocumentDialog, NewDocumentPresetWords, Orientation, PresetCategory, push_recent,
};
use craft_ui::runtime::env::{Env, UnitKind};
use craft_ui::testing::{Fixture, Target, trial};
use egui::Vec2;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The most the Recent list keeps.
const RECENT_KEPT: usize = 20;
/// More documents than Recent keeps.
const CREATED: u32 = 25;
/// The side of a recent document, in pixels.
const SIDE: u32 = 10;
/// The resolution a print card is asked to open at, in pixels per inch.
const PRINT_PPI: f64 = 600.0;
/// The resolution a screen card is asked to open at, in pixels per inch.
const SCREEN_PPI: f64 = 96.0;
/// The room of an app window the dialog opens in: wide and tall enough for the cards beside the whole details column.
const WINDOW: Vec2 = Vec2::new(1100.0, 900.0);

fn recent(width: u32, name: &str) -> DocumentOptions {
    let mut options = DocumentOptions::default();
    options.size = PixelSize::new(width, SIDE);
    options.name = name.to_owned();
    options
}

/// The categories come in Photoshop's order and every built-in card is named by the word set under its id, in the
/// English the data carries.
#[trial]
fn the_standard_categories_keep_their_order_and_every_card_has_a_word() {
    let ids: Vec<&str> = PresetCategory::standard().iter().map(|category| category.id).collect();
    assert_eq!(ids, ["photo", "print", "art", "web", "mobile", "film"]);

    let unnamed = Rc::new(RefCell::new(Vec::new()));
    let found = Rc::clone(&unnamed);
    let mut fixture = Fixture::new(move |ui| {
        let words = Env::of(ui).words::<NewDocumentPresetWords>();
        let mut missing = found.borrow_mut();
        missing.clear();
        for preset in PresetCategory::standard().iter().flat_map(|category| category.presets.iter()) {
            if words.name_of(preset.id).map(|name| name.as_str()) != Some(preset.name) {
                missing.push(preset.id);
            }
        }
    });
    fixture.settle();
    assert!(unnamed.borrow().is_empty(), "cards without a word of their name: {:?}", unnamed.borrow());
}

/// A print card keeps its physical size at the app's print resolution and opens in inches; a screen card keeps its
/// pixels and takes the screen resolution.
#[trial]
fn a_card_opens_at_the_apps_resolution() {
    let letter = DocumentPreset::new("letter", "Letter", 2550, 3300, 300.0).options(PRINT_PPI, SCREEN_PPI);
    assert_eq!((letter.size, letter.pixels_per_inch, letter.unit), (PixelSize::new(5100, 6600), PRINT_PPI, UnitKind::Inches));
    let hdtv = DocumentPreset::new("hdtv-1080p", "HDTV 1080p", 1920, 1080, 72.0).options(PRINT_PPI, SCREEN_PPI);
    assert_eq!((hdtv.size, hdtv.pixels_per_inch, hdtv.unit), (PixelSize::new(1920, 1080), SCREEN_PPI, UnitKind::Pixels));
}

/// A built-in card turns with the orientation; a square card stays as it is.
#[trial]
fn a_card_turns_to_the_orientation() {
    let landscape = DocumentPreset::new("landscape-6x4", "Landscape, 6 x 4", 1800, 1200, 300.0);
    let turned = landscape.turned(Orientation::Portrait);
    assert_eq!((turned.width, turned.height), (1200, 1800));
    assert_eq!(landscape.turned(Orientation::Landscape), landscape);
    let square = DocumentPreset::new("square-5x5", "Square, 5 x 5", 1500, 1500, 300.0);
    assert_eq!(square.turned(Orientation::Landscape), square);
    assert_eq!(Orientation::of(PixelSize::new(3, 3)), Orientation::Portrait);
}

/// The recent list puts the newest first, keeps twenty and moves an equal document to the front instead of repeating
/// it.
#[trial]
fn recent_documents_keep_twenty_and_do_not_repeat() {
    let mut list = Vec::new();
    for width in 1..=CREATED {
        push_recent(&mut list, recent(width, ""), RECENT_KEPT);
    }
    assert_eq!((list.len(), list[0].size.width), (RECENT_KEPT, CREATED));
    push_recent(&mut list, recent(SIDE, "Other"), RECENT_KEPT);
    assert_eq!((list.len(), list[0].size.width), (RECENT_KEPT, SIDE), "an equal document moves to the front");
    assert_eq!(list.iter().filter(|held| held.size.width == SIDE).count(), 1);
}

/// A fixture with the dialog over its settings, which collects what each pass of Create answers with.
struct Rig {
    fixture: Fixture,
    options: Rc<RefCell<DocumentOptions>>,
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
            answers.borrow_mut().push(result);
        }
        if out.closed.is_some() {
            done.set(true);
        }
    });
    fixture.room(WINDOW);
    fixture.settle();
    Rig { fixture, options, created, closed }
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

/// A click on a card fills the size and unit; Create then answers with the settings under the default name.
#[trial]
fn a_card_fills_the_details_and_create_answers_with_them() {
    let Rig { mut fixture, options, created, closed } = rig();
    pick_letter(&mut fixture);
    {
        let options = options.borrow();
        assert_eq!((options.size, options.unit), (PixelSize::new(2550, 3300), UnitKind::Inches), "a print card applies inches");
    }
    let create = fixture.by_label("Create").expect("the Create button");
    fixture.click(&create);
    fixture.settle();
    assert!(closed.get(), "Create closes the dialog");
    let answers = created.borrow();
    let answer = answers.last().expect("Create answers");
    assert_eq!((answer.size, answer.name.as_str()), (PixelSize::new(2550, 3300), "Untitled-1"));
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

/// The orientation control swaps width and height.
#[trial]
fn the_orientation_control_swaps_the_sides() {
    let Rig { mut fixture, options, .. } = rig();
    assert_eq!(options.borrow().size, PixelSize::new(2100, 1500), "the default is landscape");
    let portrait = named(&fixture, "Portrait").expect("the Portrait segment");
    fixture.click(&portrait);
    fixture.settle();
    assert_eq!(options.borrow().size, PixelSize::new(1500, 2100), "the Portrait segment: {}", where_is(&fixture, &portrait));
}

/// Where `target` sits after a click on it: its bounds then and now, its ancestors' bounds, the layer under its
/// centre and the focused node.
fn where_is(fixture: &Fixture, target: &Target) -> String {
    let tree = fixture.tree();
    let mut chain = Vec::new();
    let mut key = Some(target.node);
    while let Some(at) = key {
        chain.push(format!("{:?} {:?} {:?}", tree.role_of(at), tree.name_of(at), tree.bounds_of(at)));
        key = tree.parent_of_key(at);
    }
    let layer = fixture.ctx().layer_id_at(target.center());
    let focused = fixture.focused().map(|id| tree.name_of(craft_ui::testing::access::NodeKey::of(id)).map(str::to_owned));
    format!(
        "clicked at {:?}; now {:?}; layer {layer:?}; focused {focused:?}; chain {chain:#?}",
        target.rect,
        fixture.target(target.node).map(|now| now.rect)
    )
}

/// Alt turns Close into Reset, which puts back the settings the dialog opened with.
#[trial]
fn alt_reset_restores_the_details() {
    let Rig { mut fixture, options, closed, .. } = rig();
    let portrait = named(&fixture, "Portrait").expect("the Portrait segment");
    fixture.click(&portrait);
    fixture.settle();
    assert_eq!(options.borrow().size, PixelSize::new(1500, 2100), "the edit reached the settings");
    fixture.hold(egui::Modifiers::ALT);
    fixture.settle();
    let reset = fixture.by_label("Reset").expect("Alt turns Close into Reset");
    fixture.click(&reset);
    fixture.settle();
    assert_eq!(options.borrow().size, PixelSize::new(2100, 1500), "Reset restored the opening settings");
    assert!(!closed.get(), "Reset keeps the dialog open");
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

/// A name that differs from a saved one only in case is refused by the library's name check: the row stays open and
/// the list keeps the one preset.
#[trial]
fn a_preset_name_that_differs_only_in_case_is_refused() {
    let Rig { mut fixture, .. } = rig();
    save_preset_as(&mut fixture, "Poster");
    assert!(named(&fixture, "Poster").is_some(), "the first preset is listed");
    save_preset_as(&mut fixture, "poster");
    assert!(named(&fixture, "poster").is_none(), "the same name in another case is not saved");
    assert!(fixture.by_label("Preset Name").is_some(), "the refused name leaves the Save Preset row open");
}
