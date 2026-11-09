//! The preferences window's answers: OK applies and closes, Apply applies and stays, Cancel and Escape drop the draft
//! and close, and a failed apply keeps the window open.

use craft_ui::contract::Bind;
use craft_ui::patterns::preferences::{Closed, PageSpec, PrefValue, PreferencesModel, PreferencesState, PreferencesWindow, RowKind, RowSpec};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Ui};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What the model saw and what the window reported.
#[derive(Default)]
struct Log {
    applied: Cell<u32>,
    cancelled: Cell<u32>,
    closed: Cell<Option<Closed>>,
}

struct Model {
    log: Rc<Log>,
    refuse: bool,
}

impl PreferencesModel for Model {
    fn pages(&self) -> Vec<PageSpec<'_>> {
        vec![PageSpec::new("general", "General", vec![RowSpec::new("general.snap", "Snap to grid", RowKind::Bool(true))])]
    }

    fn set(&mut self, _: &str, _: PrefValue) {}

    fn custom_row(&mut self, _: &mut Ui, _: &str) {}

    fn custom_page(&mut self, _: &mut Ui, _: &str) {}

    fn differs(&self) -> bool {
        true
    }

    fn apply(&mut self) -> Result<(), String> {
        self.log.applied.set(self.log.applied.get() + 1);
        if self.refuse { Err("Refused".to_owned()) } else { Ok(()) }
    }

    fn cancel(&mut self) {
        self.log.cancelled.set(self.log.cancelled.get() + 1);
    }

    fn reset_page(&mut self, _: &str) {}
}

fn rig(refuse: bool) -> (Fixture, Rc<Log>) {
    let log = Rc::new(Log::default());
    let model = Rc::new(RefCell::new(Model { log: log.clone(), refuse }));
    let state = Rc::new(RefCell::new(PreferencesState::default()));
    let seen = log.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut model = model.borrow_mut();
        let mut state = state.borrow_mut();
        let out = PreferencesWindow::new(Bind::bound(&mut state)).embedded().show(ui, &mut *model);
        if out.closed.is_some() {
            seen.closed.set(out.closed);
        }
    });
    fixture.settle();
    (fixture, log)
}

fn press(fixture: &mut Fixture, label: &str) {
    let button = fixture.by_label(label).unwrap_or_else(|| panic!("the window has a {label} button"));
    fixture.click(&button);
    fixture.idle(3);
}

#[trial]
fn ok_applies_and_closes_and_apply_keeps_the_window_open() {
    {
        // OK applies the draft once and closes the window as answered.
        let (mut fixture, log) = rig(false);
        press(&mut fixture, "OK");
        assert_eq!(log.applied.get(), 1);
        assert_eq!(log.closed.get(), Some(Closed::Ok));
    }
    {
        // Apply applies the draft and leaves the window open.
        let (mut fixture, log) = rig(false);
        press(&mut fixture, "Apply");
        assert_eq!(log.applied.get(), 1);
        assert_eq!(log.closed.get(), None);
    }
}

/// Escape drops the draft and closes the window as cancelled, without applying.
#[trial]
fn escape_cancels() {
    let (mut fixture, log) = rig(false);
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(3);
    assert_eq!(log.cancelled.get(), 1);
    assert_eq!(log.applied.get(), 0);
    assert_eq!(log.closed.get(), Some(Closed::Cancel));
}

/// A refused apply keeps the window open and shows the model's message.
#[trial]
fn a_refused_apply_keeps_the_window_open() {
    let (mut fixture, log) = rig(true);
    press(&mut fixture, "OK");
    assert_eq!(log.applied.get(), 1);
    assert_eq!(log.closed.get(), None);
    assert!(fixture.by_label("Refused").is_some(), "the message shows in the footer");
}
