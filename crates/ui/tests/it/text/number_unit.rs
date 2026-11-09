//! A length field reports the unit the committed text named.

use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::runtime::env::{LengthUnits, UnitKind};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::{Frames, trial};
use craft_ui::text::NumberField;
use egui::{Event, Key, Modifiers, Pos2, Ui};
use std::cell::{Cell, RefCell};

/// The field's resolution in pixels per inch.
const PPI: f64 = 96.0;
/// How far two lengths may differ and still count as the same.
const SAME: f64 = 1e-6;

/// The stored pixels and the units reported in the passes after typing `text` and Enter in a pixel-stored field that
/// shows inches.
fn typed(text: &'static str) -> (f64, Vec<Option<UnitKind>>) {
    let mut frames = Frames::new();
    let value = RefCell::new(Some(96.0));
    let at = Cell::new(Pos2::ZERO);
    let reported = RefCell::new(Vec::new());
    let mut show = |ui: &mut Ui| {
        let mut guard = value.borrow_mut();
        let lengths = LengthUnits::new(UnitKind::Pixels, UnitKind::Inches, PPI);
        let out = NumberField::length("Width", Bind::bound(&mut *guard), lengths).id_salt("width").show(ui);
        at.set(out.response.rect.center());
        reported.borrow_mut().push(out.typed_unit);
    };
    frames.idle(3, &mut show);
    click(&mut frames, at.get(), &mut show);
    frames.idle(2, &mut show);
    reported.borrow_mut().clear();
    frames.frame(vec![key_press(Key::A, Modifiers::COMMAND)], &mut show);
    frames.frame(vec![Event::Text(text.into())], &mut show);
    frames.idle(1, &mut show);
    frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], &mut show);
    frames.idle(3, &mut show);
    (value.into_inner().unwrap_or_default(), reported.into_inner())
}

/// "50 mm" in an inch-shown field stores 50 mm in pixels and reports millimetres, once; a bare number reports none.
#[trial]
fn a_length_field_reports_the_unit_it_was_typed_in() {
    let (stored, reported) = typed("50 mm");
    let want = 50.0 / 25.4 * PPI;
    assert!((stored - want).abs() < SAME, "50 mm in pixels: {stored} against {want}");
    assert_eq!(reported.iter().flatten().collect::<Vec<_>>(), [&UnitKind::Millimetres], "reported: {reported:?}");

    let (stored, reported) = typed("3");
    assert!((stored - 3.0 * PPI).abs() < SAME, "3 in in pixels: {stored}");
    assert!(reported.iter().all(Option::is_none), "a bare number reports no unit: {reported:?}");
}
