//! `Select`: opens from the keyboard and from a click, picks through the listbox, closes on Escape without a change and
//! reports its popup to assistive technology.

use craft_ui::contract::{Bind, Entry, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::form::Field;
use craft_ui::pickers::Select;
use craft_ui::testing::{Fixture, Frames, trial};
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

/// A disabled select shows its text and chevron in a different ink from an enabled one.
#[trial]
fn a_disabled_selects_ink_differs_from_an_enabled_ones() {
    let inks = |enabled: bool| {
        let mut value = 1u8;
        let mut show = move |ui: &mut egui::Ui| {
            let entries = [Entry::new(0u8, "Normal"), Entry::new(1, "Multiply")];
            Select::new("Blend mode", Bind::bound(&mut value), &entries).enabled(enabled).show(ui);
        };
        let mut frames = Frames::new();
        frames.idle(2, &mut show);
        frames
            .frame_shapes(&mut show)
            .iter()
            .filter_map(|shape| match shape {
                egui::Shape::Text(text) => text.galley.job.sections.first().map(|section| section.format.color),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let (on, off) = (inks(true), inks(false));
    assert!(!on.is_empty() && on.len() == off.len(), "both show the same text");
    assert_ne!(on, off, "a disabled select's ink is the disabled ink");
}
