//! The Character and Paragraph controls: the changes they report for typed values, presets, toggles and the colour well.

use craft_text::ui::DocUnit;
use craft_text::ui::character::{
    COLOR_KEY, Caps, CharacterChange, CharacterControls, CharacterValues, Kerning, Leading, ParagraphChange, ParagraphControls, ParagraphValues,
    TextAlign,
};
use craft_theme::{Rgba, Srgba};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

/// A panel in a fixture; the app's side applies each change to the values the next pass shows.
struct Panel {
    fixture: Fixture,
    edits: Rc<RefCell<Vec<craft_behavior::EditEvent<CharacterChange>>>>,
    requests: Rc<RefCell<Vec<craft_ui::runtime::requests::ColorRequest>>>,
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
    CharacterValues { size: Some(12.0), tracking: Some(0.0), kerning: Some(Kerning::Metrics), caps: Some(Caps::Normal), ..CharacterValues::default() }
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
        Panel { fixture, edits, requests }
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

/// Typing 24 in the size field reports `Size(24.0)`; in pixels at 144 ppi the field shows 48 and typing 36 reports
/// `Size(18.0)`.
#[trial]
fn size_field_converts_the_type_unit() {
    let mut panel = Panel::new(base(), DocUnit::points());
    panel.type_into("Font size", "24");
    assert_eq!(panel.last(), Some(CharacterChange::Size(24.0)));

    let values = CharacterValues { size: Some(24.0), ..base() };
    let mut panel = Panel::new(values, DocUnit::new(UnitKind::Pixels, 144.0));
    assert!(panel.shown_value("Font size").contains("48"), "{}", panel.shown_value("Font size"));
    panel.type_into("Font size", "36");
    assert_eq!(panel.last(), Some(CharacterChange::Size(18.0)));
}

/// All Caps and Small Caps are one choice: pressing the pressed one reports `Normal`, pressing the other reports it.
#[trial]
fn caps_toggles_are_one_choice() {
    let mut panel = Panel::new(base(), DocUnit::points());
    panel.click("All Caps");
    assert_eq!(panel.last(), Some(CharacterChange::Caps(Caps::AllCaps)));
    panel.click("Small Caps");
    assert_eq!(panel.last(), Some(CharacterChange::Caps(Caps::SmallCaps)));
    panel.click("Small Caps");
    assert_eq!(panel.last(), Some(CharacterChange::Caps(Caps::Normal)));
    assert_eq!(panel.edits.borrow().len(), 3);
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

    let mixed = CharacterValues { kerning: None, tracking: None, ..base() };
    let panel = Panel::new(mixed, DocUnit::points());
    assert_eq!(panel.shown_value("Kerning"), "");
    assert_eq!(panel.shown_value("Tracking"), "");
}

/// The colour well files a request with the current colour for the pass of the click only.
#[trial]
fn swatch_requests_a_colour_for_one_pass() {
    let values = CharacterValues { color: Some(Rgba([10, 20, 30, 255])), ..base() };
    let mut panel = Panel::new(values, DocUnit::points());
    assert!(panel.requests.borrow().is_empty());
    panel.click("Text color");
    let seen = panel.requests.borrow().clone();
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert_eq!(seen[0].key, COLOR_KEY);
    assert_eq!(seen[0].color, Srgba::from(Rgba([10, 20, 30, 255])));
    panel.fixture.settle();
    assert_eq!(panel.requests.borrow().len(), 1, "a request is made once per click");
}

/// The justify buttons do nothing on point text; Left reports `Align(Left)`.
#[trial]
fn justify_buttons_need_a_box() {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let seen = changes.clone();
    let values = ParagraphValues { align: Some(TextAlign::Center), ..ParagraphValues::default() };
    let mut fixture = Fixture::new(move |ui| {
        let out = ParagraphControls::new("Paragraph", &values).point_text(true).show(ui);
        seen.borrow_mut().extend(out.edits.iter().filter_map(|edit| edit.value().copied()));
    });
    fixture.settle();
    for name in ["Justify last left", "Justify last centered", "Justify last right", "Justify all"] {
        let target = fixture.by_label(name).unwrap_or_else(|| panic!("`{name}` is shown"));
        fixture.click(&target);
    }
    assert!(changes.borrow().is_empty(), "{:?}", changes.borrow());
    let left = fixture.by_label("Left align text").expect("Left is shown");
    fixture.click(&left);
    assert_eq!(*changes.borrow(), [ParagraphChange::Align(TextAlign::Left)]);
}

/// The leading menu's Auto row reports `Leading(Auto)` as the one edit of the pass.
#[trial]
fn leading_menu_has_an_auto_row() {
    let values = CharacterValues { leading: Some(Leading::Points(20.0)), ..base() };
    let mut panel = Panel::new(values, DocUnit::points());
    panel.pick_preset("Leading", "Auto");
    assert_eq!(panel.last(), Some(CharacterChange::Leading(Leading::Auto)));
    assert_eq!(panel.edits.borrow().len(), 1);
}
