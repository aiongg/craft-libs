//! The Character and Paragraph controls: the changes they report for typed values, presets, toggles and the colour well.

use crate::set;
use craft_text::ui::DocUnit;
use craft_text::ui::character::{Caps, CharacterChange, CharacterControls, CharacterValues, Kerning};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

/// A panel in a fixture; the app's side applies each change to the values the next pass shows.
struct Panel {
    fixture: Fixture,
    edits: Rc<RefCell<Vec<craft_behavior::EditEvent<CharacterChange>>>>,
}

fn apply(values: &mut CharacterValues, change: &CharacterChange) {
    match change {
        CharacterChange::Size(v) => values.size = Some(*v),
        CharacterChange::Tracking(v) => values.tracking = Some(*v),
        CharacterChange::Kerning(k) => values.kerning = Some(*k),
        CharacterChange::Caps(c) => values.caps = Some(*c),
        _ => {}
    }
}

fn base() -> CharacterValues {
    set(CharacterValues::default(), |v| {
        v.size = Some(12.0);
        v.tracking = Some(0.0);
        v.kerning = Some(Kerning::Metrics);
        v.caps = Some(Caps::Normal);
    })
}

impl Panel {
    fn new(values: CharacterValues, unit: DocUnit) -> Panel {
        let values = Rc::new(RefCell::new(values));
        let edits = Rc::new(RefCell::new(Vec::new()));
        let requests = Rc::new(RefCell::new(Vec::new()));
        let (shown, kept, asked) = (values.clone(), edits.clone(), requests.clone());
        let mut fixture = Fixture::new(move |ui| {
            let families = ["Hack".to_owned(), "Ubuntu".to_owned()];
            let snapshot = shown.borrow().clone();
            let mut out = CharacterControls::new("Character", &snapshot, &families, &families).unit(unit).show(ui);
            for edit in out.edits {
                if let Some(change) = edit.value() {
                    apply(&mut shown.borrow_mut(), change);
                }
                kept.borrow_mut().push(edit);
            }
            asked.borrow_mut().extend(out.requests.take_color());
        });
        fixture.settle();
        Panel { fixture, edits }
    }

    /// The last change any edit carried.
    fn last(&self) -> Option<CharacterChange> {
        self.edits.borrow().iter().rev().find_map(|edit| edit.value().cloned())
    }

    fn click(&mut self, label: &str) {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("`{label}` is shown"));
        self.fixture.click(&target);
    }

    fn type_into(&mut self, label: &str, text: &str) {
        self.click(label);
        self.fixture.key(Key::A, Modifiers::COMMAND);
        self.fixture.type_text(text);
        self.fixture.key(Key::Enter, Modifiers::NONE);
    }

    fn shown_value(&self, label: &str) -> String {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("`{label}` is shown"));
        self.fixture.tree().value_of(target.node).unwrap_or_default().to_owned()
    }

    /// Opens the presets list of the field `label` with Alt+Down and picks the row `row`.
    fn pick_preset(&mut self, label: &str, row: &str) {
        self.click(label);
        self.fixture.key(Key::ArrowDown, Modifiers::ALT);
        self.click(row);
    }
}

/// The kerning menu's Optical row reports `Optical`, a typed number a manual value; a mixed kerning and tracking show
/// empty fields.
#[trial]
fn kerning_menu_and_mixed_values() {
    let mut panel = Panel::new(base(), DocUnit::points());
    panel.pick_preset("Kerning", "Optical");
    assert_eq!(panel.last(), Some(CharacterChange::Kerning(Kerning::Optical)));
    panel.type_into("Kerning", "-40");
    assert_eq!(panel.last(), Some(CharacterChange::Kerning(Kerning::Manual(-40.0))));

    let mixed = set(base(), |v| {
        v.kerning = None;
        v.tracking = None;
    });
    let panel = Panel::new(mixed, DocUnit::points());
    assert_eq!(panel.shown_value("Kerning"), "");
    assert_eq!(panel.shown_value("Tracking"), "");
}
