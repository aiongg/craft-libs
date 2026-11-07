//! `Select`: opens from the keyboard, picks through the listbox, closes on Escape without a change and reports its
//! popup to assistive technology.

use craft_ui::contract::{Bind, Entry, Props as _, Role, Show as _};
use craft_ui::pickers::Select;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

const FADE_FRAMES: usize = 10;

struct Rig {
    fixture: Fixture,
    value: Rc<Cell<u8>>,
    id: Rc<Cell<Id>>,
}

/// A select over three options with focus on it.
fn rig() -> Rig {
    let (value, id) = (Rc::new(Cell::new(0u8)), Rc::new(Cell::new(Id::NULL)));
    let (shown_value, shown_id) = (value.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        let entries = [Entry::new(0u8, "Normal"), Entry::new(1, "Multiply"), Entry::new(2, "Screen")];
        let mut mode = shown_value.get();
        let out = Select::new("Blend mode", Bind::bound(&mut mode), &entries).id_salt("mode").show(ui);
        shown_value.set(mode);
        shown_id.set(out.response.id);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    Rig { fixture, value, id }
}

impl Rig {
    fn open(&mut self) {
        self.fixture.key(Key::ArrowDown, Modifiers::ALT);
        self.fixture.idle(2);
    }

    /// A key, then enough frames for a closing listbox to finish fading out.
    fn press(&mut self, key: Key) {
        self.fixture.key(key, Modifiers::NONE);
        self.fixture.idle(FADE_FRAMES);
    }

    fn listbox_shown(&self) -> bool {
        !self.fixture.by_role(Role::LIST_BOX).is_empty()
    }
}

/// Alt+Down opens the listbox, Down and Enter pick the next option as one change and close it, and focus is back on
/// the field: an arrow on the closed field steps the value.
#[trial]
fn a_select_opens_from_the_keyboard_and_picks_with_arrows_and_enter() {
    let mut rig = rig();
    assert!(!rig.listbox_shown());
    rig.open();
    assert!(rig.listbox_shown(), "Alt+Down opens the listbox");
    rig.press(Key::ArrowDown);
    assert_eq!(rig.value.get(), 0, "moving through the list changes nothing until a pick");
    rig.press(Key::Enter);
    assert_eq!(rig.value.get(), 1, "the lead started on the picked option, so one Down reaches the next");
    assert!(!rig.listbox_shown(), "a pick closes the listbox");
    rig.press(Key::ArrowUp);
    assert_eq!(rig.value.get(), 0, "focus returned to the field, whose arrows step the value");
}

/// Escape closes the listbox and keeps the value, whatever the list's highlight reached.
#[trial]
fn escape_closes_a_select_without_a_change() {
    let mut rig = rig();
    rig.open();
    rig.press(Key::ArrowDown);
    rig.press(Key::Escape);
    assert!(!rig.listbox_shown(), "Escape closes the listbox");
    assert_eq!(rig.value.get(), 0);
    rig.open();
    rig.press(Key::Enter);
    assert_eq!(rig.value.get(), 0, "the listbox opens on the picked option, not on the highlight Escape abandoned");
}

/// Typing a letter in the open list moves to the option that starts with it.
#[trial]
fn typing_in_a_select_list_moves_to_the_matching_option() {
    let mut rig = rig();
    rig.open();
    rig.fixture.type_text("S");
    rig.fixture.idle(2);
    rig.press(Key::Enter);
    assert_eq!(rig.value.get(), 2);
}

/// The field is a combo box that reports whether its listbox is shown.
#[trial]
fn a_select_reports_its_popup_state() {
    let mut rig = rig();
    let id = rig.id.get();
    assert_eq!(rig.fixture.tree().expanded(id), Some(false));
    rig.open();
    assert_eq!(rig.fixture.tree().expanded(id), Some(true));
    rig.press(Key::Escape);
    assert_eq!(rig.fixture.tree().expanded(id), Some(false));
}

/// The lead of the list starts again at every opening: a lead moved to the last row and abandoned with Escape does
/// not come back when the list opens next.
#[trial]
fn a_reopened_select_list_starts_its_lead_again() {
    let mut rig = rig();
    rig.open();
    rig.press(Key::ArrowDown);
    rig.press(Key::ArrowDown);
    rig.press(Key::Escape);
    rig.open();
    rig.press(Key::Enter);
    assert_eq!(rig.value.get(), 0, "the lead began on the picked option, not on the last row");
}

/// Arrows on the closed field step over entries that cannot be picked and over separators: Down from the first entry
/// lands on the entry after the disabled one, and Down from the last stays.
#[trial]
fn arrows_on_a_closed_select_skip_disabled_entries() {
    let value = Rc::new(Cell::new(0u8));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let entries = [Entry::new(0u8, "Normal"), Entry::new(1, "Multiply").enabled(false), Entry::separator(), Entry::new(2, "Screen")];
        let mut mode = shown.get();
        Select::new("Blend mode", Bind::bound(&mut mode), &entries).id_salt("mode").show(ui);
        shown.set(mode);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(value.get(), 2, "the disabled entry and the separator are passed over");
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(value.get(), 2, "the last entry that can be picked is the end");
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(value.get(), 0);
}
