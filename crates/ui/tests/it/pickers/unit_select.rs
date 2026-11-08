//! `UnitSelect`: the number field is the first Tab stop, so typing after Tab lands in it.

use craft_ui::contract::{Bind, Entry, Show as _};
use craft_ui::pickers::UnitSelect;
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

/// The digits the trial types into the number field.
const TYPED: &str = "42";

/// Tab focuses the number field first, as it is first on screen: typed digits and Enter set the number, and the unit
/// is left as it was.
#[trial]
fn tab_focuses_the_number_field_first() {
    let width = Rc::new(Cell::new(Some(210.0)));
    let shown = width.clone();
    let mut fixture = Fixture::new(move |ui| {
        let units = [Entry::new(UnitKind::Millimetres, "mm"), Entry::new(UnitKind::Inches, "in")];
        let mut value = shown.get();
        let mut unit = UnitKind::Millimetres;
        UnitSelect::new("Width", Bind::bound(&mut value), Bind::bound(&mut unit), &units).show(ui);
        shown.set(value);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text(TYPED);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(4);
    assert_eq!(width.get(), TYPED.parse().ok());
}

/// With the number disabled the unit select still picks a unit, and typing reaches nothing.
#[trial]
fn a_disabled_number_leaves_the_unit_select_working() {
    let width = Rc::new(Cell::new(Some(210.0)));
    let unit = Rc::new(Cell::new(UnitKind::Millimetres));
    let (shown_width, shown_unit) = (width.clone(), unit.clone());
    let mut fixture = Fixture::new(move |ui| {
        let units = [Entry::new(UnitKind::Millimetres, "mm"), Entry::new(UnitKind::Inches, "in")];
        let (mut value, mut kind) = (shown_width.get(), shown_unit.get());
        UnitSelect::new("Width", Bind::bound(&mut value), Bind::bound(&mut kind), &units).number_enabled(false).show(ui);
        shown_width.set(value);
        shown_unit.set(kind);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.type_text(TYPED);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(4);
    assert_eq!(width.get(), Some(210.0), "typing reaches no number");
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(10);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(10);
    assert_eq!(unit.get(), UnitKind::Inches, "the unit still changes");
}
