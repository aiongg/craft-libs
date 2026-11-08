//! A length field keeps the document's unit and shows the user's.

use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::runtime::env::{LengthUnits, UnitKind};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::{Frames, trial};
use craft_ui::text::NumberField;
use egui::{Event, Key, Modifiers, Pos2, Ui};
use std::cell::{Cell, RefCell};

/// One inch in the document's points.
const INCH: f64 = 72.0;
/// How far two lengths may differ and still count as the same.
const SAME: f64 = 1e-6;

/// A field over points that shows inches, driven by `drive` after it holds focus.
fn in_inches(start: f64, drive: impl FnOnce(&mut Frames, &mut dyn FnMut(&mut Ui))) -> f64 {
    let mut frames = Frames::new();
    let value = RefCell::new(Some(start));
    let at = Cell::new(Pos2::ZERO);
    let mut show = |ui: &mut Ui| {
        let mut guard = value.borrow_mut();
        let lengths = LengthUnits::new(UnitKind::Points, UnitKind::Inches, INCH);
        at.set(NumberField::length("Width", Bind::bound(&mut *guard), lengths).id_salt("width").show(ui).response.rect.center());
    };
    frames.idle(3, &mut show);
    click(&mut frames, at.get(), &mut show);
    drive(&mut frames, &mut show);
    frames.idle(3, &mut show);
    value.into_inner().unwrap_or_default()
}

fn type_and_commit(text: &'static str) -> impl FnOnce(&mut Frames, &mut dyn FnMut(&mut Ui)) {
    move |frames, show| {
        frames.frame(vec![key_press(Key::A, Modifiers::COMMAND)], &mut *show);
        frames.frame(vec![Event::Text(text.into())], &mut *show);
        frames.idle(1, &mut *show);
        frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], &mut *show);
    }
}

/// Typed units convert to the document's unit, a bare number is in the shown unit, and an arrow steps by the shown
/// unit's step.
#[trial]
fn a_length_field_types_and_steps_in_the_shown_unit() {
    let typed = in_inches(INCH, type_and_commit("10 mm"));
    assert!((typed - 10.0 / 25.4 * INCH).abs() < SAME, "10 mm is stored in points: {typed}");

    let bare = in_inches(INCH, type_and_commit("2"));
    assert!((bare - 2.0 * INCH).abs() < SAME, "a bare 2 is two inches: {bare}");

    let stepped = in_inches(INCH, |frames, show| {
        frames.frame(vec![key_press(Key::ArrowUp, Modifiers::NONE)], show);
    });
    assert!((stepped - 1.1 * INCH).abs() < SAME, "an arrow moves a tenth of an inch: {stepped}");
}
