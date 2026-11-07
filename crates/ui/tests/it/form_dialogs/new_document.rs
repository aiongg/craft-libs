//! New Document's own logic: the built-in cards, the recent list, the preset-name rule and what a card, the orientation
//! control and the Save Preset row do to the settings.

use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{
    DocumentOptions, DocumentPreset, NameError, NewDocumentDialog, Orientation, PresetCategory, PresetWords, preset_name_error, push_recent,
};
use craft_ui::runtime::env::{Env, UnitKind};
use craft_ui::testing::{Fixture, Target, trial};
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
        let words = Env::of(ui).words::<PresetWords>();
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

/// A preset name is trimmed, must have 1 to 255 characters and must differ from a saved name in ASCII case; 256 presets
/// is the most.
#[trial]
fn a_preset_name_is_checked_against_the_saved_ones() {
    let saved = vec![recent(1, "Poster")];
    assert_eq!(preset_name_error(&saved, "  "), Some(NameError::Length));
    assert_eq!(preset_name_error(&saved, &"x".repeat(256)), Some(NameError::Length));
    assert_eq!(preset_name_error(&saved, " poster "), Some(NameError::Taken("poster".into())));
    assert_eq!(preset_name_error(&saved, "Flyer"), None);
    let full = vec![recent(1, "Poster"); 256];
    assert_eq!(preset_name_error(&full, "Flyer"), Some(NameError::TooMany));
}

/// A fixture with the dialog over `options`, which collects what Create answers with.
fn rig() -> (Fixture, Rc<RefCell<DocumentOptions>>, Rc<RefCell<Option<DocumentOptions>>>, Rc<Cell<bool>>) {
    let options = Rc::new(RefCell::new(DocumentOptions::default()));
    let created = Rc::new(RefCell::new(None));
    let closed = Rc::new(Cell::new(false));
    let (held, answer, done) = (Rc::clone(&options), Rc::clone(&created), Rc::clone(&closed));
    let mut fixture = Fixture::new(move |ui| {
        let mut options = held.borrow_mut();
        let out = NewDocumentDialog::new().options(Bind::bound(&mut *options)).show(ui);
        if out.result.is_some() {
            *answer.borrow_mut() = out.result;
        }
        if out.closed.is_some() {
            done.set(true);
        }
    });
    fixture.settle();
    (fixture, options, created, closed)
}

/// The node named `label`, or one named `label` followed by its details.
fn named(fixture: &Fixture, label: &str) -> Option<Target> {
    let tree = fixture.tree();
    let key = tree.keys().into_iter().find(|key| tree.name_of(*key).is_some_and(|name| name == label || name.starts_with(&format!("{label}, "))))?;
    fixture.target(key)
}

/// A click on a card fills the size and unit; Create then answers with the settings under the default name.
#[trial]
fn a_card_fills_the_details_and_create_answers_with_them() {
    let (mut fixture, options, created, closed) = rig();
    let print = named(&fixture, "Print").expect("the Print tab");
    fixture.click(&print);
    fixture.settle();
    let letter = named(&fixture, "Letter").expect("the Letter card");
    fixture.click(&letter);
    fixture.settle();
    {
        let options = options.borrow();
        assert_eq!((options.size, options.unit), (PixelSize::new(2550, 3300), UnitKind::Inches), "a print card applies inches");
    }
    let create = fixture.by_label("Create").expect("the Create button");
    fixture.click(&create);
    fixture.settle();
    assert!(closed.get(), "Create closes the dialog");
    let answer = created.borrow().clone().expect("Create answers");
    assert_eq!((answer.size, answer.name.as_str()), (PixelSize::new(2550, 3300), "Untitled-1"));
}

/// The orientation control swaps width and height.
#[trial]
fn the_orientation_control_swaps_the_sides() {
    let (mut fixture, options, _, _) = rig();
    assert_eq!(options.borrow().size, PixelSize::new(2100, 1500), "the default is landscape");
    let portrait = named(&fixture, "Portrait").expect("the Portrait segment");
    fixture.click(&portrait);
    fixture.settle();
    assert_eq!(options.borrow().size, PixelSize::new(1500, 2100));
}

/// Enter in the preset name saves the preset under Saved and does not create the document.
#[trial]
fn enter_in_the_preset_name_saves_without_creating() {
    let (mut fixture, _, created, closed) = rig();
    let save = named(&fixture, "Save Preset…").expect("the Save Preset button");
    fixture.click(&save);
    fixture.settle();
    fixture.type_text("My size");
    fixture.key(egui::Key::Enter, egui::Modifiers::NONE);
    fixture.settle();
    assert!(created.borrow().is_none() && !closed.get(), "Enter in the name row does not create");
    assert!(named(&fixture, "My size").is_some(), "the preset is listed under Saved");
}
