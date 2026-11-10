//! `Select`: opens from the keyboard and from a click, picks through the listbox, closes on Escape without a change and
//! reports its popup to assistive technology.

use craft_ui::contract::{Bind, Entry, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::form::Field;
use craft_ui::pickers::Select;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

const FADE_FRAMES: usize = 10;

struct Rig {
    fixture: Fixture,
    value: Rc<Cell<u8>>,
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
    Rig { fixture, value }
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

/// A row of the dropdown under the pointer has a fill.
#[trial]
fn a_select_dropdown_row_under_the_pointer_has_a_fill() {
    let mut rig = rig();
    rig.open();
    let row = rig.fixture.by_label("Multiply").expect("the option shows").rect;
    for step in [20.0, 8.0, 2.0, 0.0] {
        rig.fixture.frame(vec![egui::Event::PointerMoved(row.center() + egui::vec2(step, 0.0))]);
        rig.fixture.idle(2);
    }
    rig.fixture.advance(0.5);
    rig.fixture.idle(6);
    let row = rig.fixture.by_label("Multiply").expect("the option shows").rect;
    assert!(row_filled(&rig.fixture, row), "the hovered option has a fill {:?}", rig.fixture.shape_bounds());
}

/// Whether a fill (not a focus ring or other stroke) covers `row` and is no taller than the row plus its rounding.
fn row_filled(fixture: &Fixture, row: egui::Rect) -> bool {
    let overshoot = fixture.shape_overshoot();
    fixture
        .shape_bounds()
        .iter()
        .enumerate()
        .any(|(at, shape)| overshoot.get(at).is_none_or(|stroke| *stroke == 0.0) && shape.contains_rect(row) && shape.height() < row.height() * 1.5)
}

/// A popup list keeps its lead row filled while the field holds keyboard focus: the first Down moves the lead onto the
/// option after the picked one and that row is filled.
#[trial]
fn the_lead_row_of_an_open_select_is_filled_after_an_arrow_key() {
    let mut rig = rig();
    rig.open();
    rig.fixture.key(Key::ArrowDown, Modifiers::NONE);
    rig.fixture.idle(4);
    let row = rig.fixture.by_label("Multiply").expect("the option shows").rect;
    assert!(row_filled(&rig.fixture, row), "the lead option has a fill {:?}", rig.fixture.shape_bounds());
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

/// A focused, searchable select over `names` with its list open.
fn searchable(names: &'static [&'static str]) -> Rig {
    let (value, id) = (Rc::new(Cell::new(0u8)), Rc::new(Cell::new(Id::NULL)));
    let (shown_value, shown_id) = (value.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        let entries: Vec<Entry<u8>> = names.iter().zip(0u8..).map(|(name, at)| Entry::new(at, *name)).collect();
        let mut picked = shown_value.get();
        let out = Select::new("Fruit", Bind::bound(&mut picked), &entries).searchable().id_salt("fruit").show(ui);
        shown_value.set(picked);
        shown_id.set(out.response.id);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    let mut rig = Rig { fixture, value };
    rig.open();
    rig
}

impl Rig {}

/// Typing "b" narrows the list to the entries that match, Down moves the lead to the first of them and Enter picks it.
#[trial]
fn typing_then_down_and_enter_picks_the_first_match() {
    let mut rig = searchable(&["Apple", "Banana", "Cherry", "Blueberry"]);
    rig.fixture.type_text("b");
    rig.fixture.idle(4);
    rig.press(Key::ArrowDown);
    rig.press(Key::Enter);
    assert_eq!(rig.value.get(), 1, "Banana is the first entry that matches");
}

/// A select beside the options of a two-valued radio group (a `bool` value, off) opens and picks by click: the group's
/// value is no gate on its content.
#[trial]
fn a_select_in_a_radio_option_s_field_group_picks_by_click() {
    use craft_ui::choice::{Radio, RadioGroup};
    use craft_ui::form::FieldGroup;
    use craft_ui::layout_components::Row;
    let value = Rc::new(Cell::new(0u8));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let entries = [Entry::new(0u8, "Blue"), Entry::new(1, "Red"), Entry::new(2, "Green")];
        FieldGroup::new("Filter").show(ui, |ui, _| {
            RadioGroup::new("Filter", Bind::given(false)).show(ui, |ui, _| {
                Field::new("Preset").id_salt("preset").show(ui, |ui, _| {
                    Row::new().show(ui, |ui, _| {
                        Radio::new(false).label("Preset").show(ui);
                        let mut color = shown.get();
                        Select::new("Preset", Bind::bound(&mut color), &entries).id_salt("select").show(ui);
                        shown.set(color);
                    });
                });
            });
        });
    });
    fixture.settle();
    let select = fixture.by_role(Role::COMBO_BOX).into_iter().next().expect("the select is shown");
    fixture.click(&select);
    assert!(!fixture.by_role(Role::LIST_BOX).is_empty(), "a click opens the list");
    let red = fixture.by_label("Red").expect("the open list shows its rows");
    fixture.click(&red);
    fixture.idle(FADE_FRAMES);
    assert_eq!(value.get(), 1, "a click on a row picks it");
}
