//! Reset in the footer step: Alt-Reset restores the opening value of an unbound, a bound and a typed form dialog,
//! and what the body keeps; a host's cancel drops a pending request.

use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, Props as _, Remember as _, Show as _};
use craft_ui::patterns::form_dialogs::{FormDialog, Param, ParamDialog, ParamKind};
use craft_ui::runtime::env::Label;
use craft_ui::testing::{Fixture, trial};
use egui::Modifiers;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn smooth() -> Vec<Param> {
    vec![Param::new("smooth", "Smooth", ParamKind::bool(false))]
}

/// A dialog with no binding keeps its values in its own memory; Reset restores the opening ones and the next pass
/// still shows them.
#[trial]
fn alt_reset_in_an_unbound_param_dialog_restores_the_opening_values() {
    let shown = Rc::new(Cell::new(None));
    let seen = Rc::clone(&shown);
    let params = smooth();
    let mut fixture = Fixture::new(move |ui| {
        let seen = Rc::clone(&seen);
        ParamDialog::new("blur", "Blur", params.clone()).below(move |_, fields| seen.set(fields.bool("smooth"))).show(ui);
    });
    fixture.settle();
    assert_eq!(shown.get(), Some(false), "opens with the default");
    let target = fixture.by_label("Smooth").expect("the row is shown");
    fixture.click(&target);
    fixture.settle();
    assert_eq!(shown.get(), Some(true), "the edit stays between passes");
    fixture.hold(Modifiers::ALT);
    fixture.settle();
    let reset = fixture.by_label("Reset").expect("Alt turns Cancel into Reset");
    fixture.click(&reset);
    fixture.settle();
    assert_eq!(shown.get(), Some(false), "Reset restores the opening value and the next pass keeps it");
}

/// What the body of the form below keeps beside its value.
#[derive(Clone, Default)]
struct Extra(bool);

/// A typed form restores its value and runs `on_reset` for what the body keeps in its memory.
#[trial]
fn alt_reset_in_a_form_dialog_restores_the_value_and_the_body_memory() {
    let value = Rc::new(RefCell::new(false));
    let extra = Rc::new(Cell::new(false));
    let (held, kept) = (Rc::clone(&value), Rc::clone(&extra));
    let mut fixture = Fixture::new(move |ui| {
        let kept = Rc::clone(&kept);
        let mut value = held.borrow_mut();
        FormDialog::new("Flags", Bind::bound(&mut *value)).on_reset(|cx| cx.memory::<Extra>().0 = false).show(ui, move |ui, cx, value| {
            Checkbox::new(Bind::bound(value)).label(Label::from("Flag")).show(ui);
            let note = cx.memory::<Extra>();
            Checkbox::new(Bind::bound(&mut note.0)).label(Label::from("Extra")).show(ui);
            kept.set(note.0);
            true
        });
    });
    fixture.settle();
    let flag = fixture.by_label("Flag").expect("the value's control is shown");
    fixture.click(&flag);
    fixture.settle();
    let note = fixture.by_label("Extra").expect("the body's control is shown");
    fixture.click(&note);
    fixture.settle();
    assert!(*value.borrow() && extra.get(), "both were edited");
    fixture.hold(Modifiers::ALT);
    fixture.settle();
    let reset = fixture.by_label("Reset").expect("Alt turns Cancel into Reset");
    fixture.click(&reset);
    fixture.settle();
    assert!(!extra.get(), "Reset cleared what the body keeps");
    assert!(!*value.borrow(), "Reset restored the opening value");
}
