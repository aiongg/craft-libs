//! `ConstraintFields`: the style pick, the typed sides, the swap, the greyed state and the Fixed Size units.

use craft_behavior::PixelSize;
use craft_behavior::geometry::Constraint;
use craft_ui::bars::{Bar, BarKind, ConstraintFields};
use craft_ui::contract::{Bind, Entry, Role, ShowContainer as _};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const ROOM: Vec2 = Vec2::new(900.0, 80.0);
const DOCUMENT: PixelSize = PixelSize::new(1000, 500);

fn rig(start: Constraint, units: bool, enabled: bool) -> (Fixture, Rc<Cell<Constraint>>) {
    let state = Rc::new(Cell::new(start));
    let seen = state.clone();
    let mut fixture = Fixture::new(move |ui| {
        let list = [Entry::new(UnitKind::Pixels, "Pixels"), Entry::new(UnitKind::Inches, "Inches"), Entry::new(UnitKind::Centimetres, "Centimetres")];
        let mut value = seen.get();
        Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
            let fields = ConstraintFields::new(Bind::bound(&mut value)).enabled(enabled);
            let fields = if units { fields.size_units(UnitKind::Inches, &list, 100.0, DOCUMENT) } else { fields };
            fields.show(ui);
        });
        seen.set(value);
    });
    fixture.room(ROOM).settle();
    (fixture, state)
}

/// The Width field, then the Height field.
fn fields(fixture: &Fixture) -> (Target, Target) {
    let mut spins = fixture.by_role(Role::SPIN_BUTTON);
    assert_eq!(spins.len(), 2, "Width and Height are number fields");
    spins.sort_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x));
    let height = spins.pop().unwrap();
    (spins.pop().unwrap(), height)
}

fn type_into(fixture: &mut Fixture, field: &Target, text: &str) {
    fixture.click(field);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text(text);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.settle();
}

/// Picking a style resets the numbers to that style's start, whatever they were. The pick is made with the arrow
/// keys on the focused select, which steps through the styles without opening its list.
#[trial]
fn a_style_pick_resets_the_numbers() {
    let (mut fixture, state) = rig(Constraint::Ratio(16.0, 9.0), false, true);
    fixture.key(Key::Tab, Modifiers::NONE).settle();
    fixture.key(Key::ArrowDown, Modifiers::NONE).settle();
    let reset = Constraint::Size(ConstraintFields::SIZE_START.0, ConstraintFields::SIZE_START.1);
    assert_eq!(state.get(), reset);
}

/// A typed width commits on Enter and leaves the height.
#[trial]
fn a_typed_width_commits_on_enter() {
    let (mut fixture, state) = rig(Constraint::Ratio(1.0, 2.0), false, true);
    let (width, _) = fields(&fixture);
    type_into(&mut fixture, &width, "3");
    let expected = Constraint::Ratio(3.0, 2.0);
    assert_eq!(state.get(), expected);
}

/// The swap button exchanges width and height.
#[trial]
fn swap_exchanges_width_and_height() {
    let (mut fixture, state) = rig(Constraint::Size(640.0, 480.0), false, true);
    let swap = fixture.by_label("Swaps height and width").expect("the swap button");
    fixture.click(&swap).settle();
    let expected = Constraint::Size(480.0, 640.0);
    assert_eq!(state.get(), expected);
}

/// A disabled bar takes no input in the number cells or the swap.
#[trial]
fn disabled_cells_take_no_input() {
    let start = Constraint::Ratio(1.0, 2.0);
    let (mut greyed, state) = rig(start, false, false);
    let (width, _) = fields(&greyed);
    type_into(&mut greyed, &width, "5");
    let swap = greyed.by_label("Swaps height and width").unwrap();
    greyed.click(&swap).settle();
    assert_eq!(state.get(), start);
}

/// Fixed Size in inches: 3 in at 100 pixels per inch is 300 pixels; a secondary click offers the units menu only
/// when the host gave units.
#[trial]
fn fixed_size_converts_units_and_offers_the_menu_only_with_units() {
    let (mut fixture, state) = rig(Constraint::Size(200.0, 100.0), true, true);
    let (width, _) = fields(&fixture);
    type_into(&mut fixture, &width, "3");
    let expected = Constraint::Size(300.0, 100.0);
    assert_eq!(state.get(), expected);
    let at = width.center();
    fixture.frame(vec![craft_ui::testing::pointer::moved(at)]);
    fixture.frame(vec![craft_ui::testing::pointer::secondary(at, true)]);
    fixture.frame(vec![craft_ui::testing::pointer::secondary(at, false)]);
    fixture.settle();
    assert!(fixture.by_label("Centimetres").is_some(), "the units menu opens");

    let (mut plain, _) = rig(Constraint::Size(200.0, 100.0), false, true);
    let (width, _) = fields(&plain);
    let at = width.center();
    plain.frame(vec![craft_ui::testing::pointer::moved(at)]);
    plain.frame(vec![craft_ui::testing::pointer::secondary(at, true)]);
    plain.frame(vec![craft_ui::testing::pointer::secondary(at, false)]);
    plain.settle();
    assert!(plain.by_label("Centimetres").is_none(), "no units, no menu");
}
