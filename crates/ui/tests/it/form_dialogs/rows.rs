//! The unit rows of a parameter form: a unit's number shows the digits and steps by the rule of its unit kind.

use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{DialogFields, NumberUnit, Param, ParamRows};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

const INCH_PER_CENTIMETRE: f64 = 2.54;
/// The next whole number of pixels/cm above 72 pixels/inch.
const STEPPED: f64 = 29.0;

/// A resolution row in pixels per centimetre shows two digits and an arrow press steps by one unit of that kind,
/// writing the base value back in pixels per inch.
#[trial]
fn a_resolution_row_shows_the_digits_and_step_of_its_unit_kind() {
    let units = vec![
        NumberUnit::new("ppi", UnitKind::PixelsPerInch, "pixels/inch", 1.0),
        NumberUnit::new("ppcm", UnitKind::PixelsPerCentimetre, "pixels/cm", INCH_PER_CENTIMETRE),
    ];
    let params = vec![Param::number_with_units("resolution", "Resolution", 72.0, 1.0, 0, units).default_unit("ppcm")];
    let fields = Rc::new(RefCell::new(DialogFields::new()));
    let held = Rc::clone(&fields);
    let mut fixture = Fixture::new(move |ui| {
        let mut fields = held.borrow_mut();
        ParamRows::new("Options", &params, Bind::bound(&mut *fields)).show(ui);
    });
    fixture.settle();
    assert!(fixture.texts().iter().any(|text| text.text.contains("28.35")), "72 pixels/inch is 28.35 pixels/cm");

    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    fixture.idle(2);
    let base = fields.borrow().number("resolution").unwrap_or_default();
    assert_eq!(base, STEPPED * INCH_PER_CENTIMETRE, "the arrow steps to the next whole pixel/cm");
}
