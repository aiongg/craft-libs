//! Trials of the dialog shells' layer behaviour: initial focus, Escape and a closed dialog's keys.

use super::Dialog;
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use crate::controls::check::Checkbox;
use crate::runtime::overlay::{OpenFilter, any_open};
use crate::testing::focus::is_focused;
use crate::testing::input::key_press;
use crate::testing::{Frames, trial};
use egui::{Id, Key, Modifiers, Ui};
use std::cell::Cell;

/// What a shown dialog reports: its open state and the ids of its two check boxes.
#[derive(Default)]
struct Shown {
    open: Cell<bool>,
    fields: Cell<[Option<Id>; 2]>,
}

fn show(shown: &Shown) -> impl FnMut(&mut Ui) + '_ {
    move |ui| {
        let mut open = shown.open.get();
        let (mut snap, mut rulers) = (false, false);
        Dialog::new("Options").opened(Bind::bound(&mut open)).show(ui, |ui, _| {
            let first = Checkbox::new(Bind::bound(&mut snap)).label("Snap").show(ui).response.id;
            let second = Checkbox::new(Bind::bound(&mut rulers)).label("Rulers").show(ui).response.id;
            shown.fields.set([Some(first), Some(second)]);
        });
        shown.open.set(open);
    }
}

#[trial]
fn a_dialog_takes_its_first_field_as_initial_focus() {
    let shown = Shown::default();
    shown.open.set(true);
    let mut frames = Frames::new();
    let mut ui = show(&shown);
    frames.idle(30, &mut ui);
    let [first, second] = shown.fields.get();
    assert!(first.is_some_and(|id| is_focused(&frames.ctx, id)), "the first field has the focus");
    assert!(!second.is_some_and(|id| is_focused(&frames.ctx, id)), "the second field does not");
}

#[trial]
fn escape_closes_a_dialog_with_the_focus_inside() {
    let shown = Shown::default();
    shown.open.set(true);
    let mut frames = Frames::new();
    let mut ui = show(&shown);
    frames.idle(30, &mut ui);
    assert!(shown.open.get(), "the dialog is open");
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut ui);
    frames.idle(2, &mut ui);
    assert!(!shown.open.get(), "Escape wrote the open state");
}

#[trial]
fn a_closed_dialogs_layer_owns_no_keys() {
    let shown = Shown::default();
    shown.open.set(true);
    let mut frames = Frames::new();
    let mut ui = show(&shown);
    frames.idle(30, &mut ui);
    assert!(any_open(&frames.ctx, OpenFilter::Keys), "the open dialog's layer takes keys");
    shown.open.set(false);
    frames.idle(30, &mut ui);
    assert!(!any_open(&frames.ctx, OpenFilter::Keys), "a closed dialog's layer takes no keys");
    assert!(!any_open(&frames.ctx, OpenFilter::Modal), "a closed dialog blocks nothing");
}
