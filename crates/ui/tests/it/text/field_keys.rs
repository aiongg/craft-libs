//! The keys a field passes on: the modifiers of a submitting Enter, Cmd/Ctrl+Enter in a multi-line field, and the
//! Escape an emptied clearing field leaves to its body.

use craft_host::target::Os;
use craft_keymap::Mods;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::runtime::focus::Initial;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::TextField;
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A field with `build`'s options, focused, holding "hi": the text it shows and the modifiers of the last submit.
fn focused_field(os: Os, build: impl for<'a> Fn(TextField<'a>) -> TextField<'a> + 'static) -> (Fixture, Rc<RefCell<String>>, Rc<Cell<Option<Mods>>>) {
    let text = Rc::new(RefCell::new("hi".to_owned()));
    let submitted = Rc::new(Cell::new(None));
    let (shown, seen) = (text.clone(), submitted.clone());
    let mut fixture = Fixture::new(move |ui| {
        let out = build(TextField::new("Note", Bind::bound(&mut *shown.borrow_mut()))).show(ui);
        if out.submitted {
            seen.set(Some(out.submit_mods));
        }
    });
    fixture.os(os).settle();
    fixture.key(Key::Tab, Modifiers::NONE).idle(2);
    (fixture, text, submitted)
}

/// Cmd+Enter (Ctrl+Enter off macOS) submits a multi-line field that submits and adds no line; Enter adds a line and
/// does not submit.
#[trial]
fn cmd_enter_submits_a_multi_line_field_and_enter_adds_a_line() {
    for os in [Os::Linux, Os::Mac] {
        let (mut fixture, text, submitted) = focused_field(os, |field| field.lines(3).submits());
        fixture.key(Key::Enter, Modifiers::COMMAND).idle(3);
        assert_eq!(submitted.get().map(|mods| mods.cmd), Some(true), "{os:?}: Cmd+Enter submits");
        assert_eq!(*text.borrow(), "hi", "{os:?}: Cmd+Enter adds no line break");
        submitted.set(None);
        fixture.key(Key::Enter, Modifiers::NONE).idle(3);
        assert_eq!(submitted.get(), None, "{os:?}: Enter does not submit");
        assert!(text.borrow().contains('\n'), "{os:?}: Enter adds a line");
    }
}

/// A body holding a clearing search field: the first Escape clears the text and the body gets none; the second goes
/// on to the body, which takes it, so the dialog around it stays open.
#[trial]
fn the_escape_an_emptied_search_field_passes_on_reaches_its_body() {
    let text = Rc::new(RefCell::new("abc".to_owned()));
    let open = Rc::new(Cell::new(true));
    let taken = Rc::new(Cell::new(0));
    let (shown, shown_open, counted) = (text.clone(), open.clone(), taken.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut is_open = shown_open.get();
        Dialog::new("Find")
            .opened(Bind::bound(&mut is_open))
            .initial_focus(Initial::First)
            .buttons([FooterButton::new("Close", DialogRole::Cancel)])
            .show(ui, |ui, body| {
                TextField::search("Find", Bind::bound(&mut *shown.borrow_mut())).show(ui);
                counted.set(counted.get() + usize::from(body.take_cancel()));
            });
        shown_open.set(is_open);
    });
    fixture.settle();
    fixture.key(Key::Escape, Modifiers::NONE).idle(3);
    assert_eq!(*text.borrow(), "", "the first Escape cleared the field");
    assert_eq!(taken.get(), 0, "the field took it: the body gets none");
    assert!(open.get(), "the dialog stays open");
    fixture.key(Key::Escape, Modifiers::NONE).idle(3);
    assert_eq!(taken.get(), 1, "the second Escape reached the body once");
    assert!(open.get(), "the body took it, so the dialog did not close");
}
