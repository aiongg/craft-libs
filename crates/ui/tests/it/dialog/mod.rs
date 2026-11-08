//! Dialog bodies and footer buttons.

use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::contract::{Bind, Entry, Role, SelectMode, Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::display::Text;
use craft_ui::list::{List, ListRow};
use craft_ui::pickers::Select;
use craft_ui::runtime::focus::Initial;
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::TextField;
use egui::{Key, Modifiers, Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const ROWS: usize = 40;

struct Rows;

impl DisplayOrder<usize> for Rows {
    fn len(&self) -> usize {
        ROWS
    }
    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for Rows {}

#[trial]
fn a_dialog_with_a_collection_body_shows_the_list_without_a_body_scroll() {
    let mut fixture = Fixture::new(|ui| {
        let mut open = true;
        Dialog::new("Pick").collection_body().opened(Bind::bound(&mut open)).show(ui, |ui, _| {
            List::new("Rows").select(SelectMode::Single).show_items(ui, &Rows, &mut |key, _| ListRow::new(format!("Row {key}")));
        });
    });
    fixture.settle();
    assert!(fixture.by_label("Row 0").is_some(), "the list shows its first row inside the dialog");
}

/// Shows a dialog with Apply (stays open), OK and Cancel; `allow` answers the guard for OK. Returns what the dialog
/// reported on the pass the button was clicked and whether it is still open after.
fn click_footer(label: &str, allow: bool) -> (Option<CloseReason>, Option<usize>, bool) {
    let seen = Rc::new(Cell::new((None, None)));
    let open = Rc::new(Cell::new(true));
    let (sink, state) = (seen.clone(), open.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut now = state.get();
        let shown = Dialog::new("Options")
            .opened(Bind::bound(&mut now))
            .buttons([
                FooterButton::new("Apply", DialogRole::Other).stays_open(true),
                FooterButton::new("OK", DialogRole::Accept),
                FooterButton::new("Cancel", DialogRole::Cancel),
            ])
            .close_guard(move |_, _| allow)
            .show(ui, |_, _| {});
        if shown.output.picked.is_some() {
            sink.set((shown.output.closed, shown.output.picked));
        }
        state.set(now);
    });
    fixture.idle(30);
    let button = fixture.by_label(label).expect("the button is shown");
    fixture.click(&button);
    fixture.idle(5);
    let (closed, picked) = seen.get();
    (closed, picked, open.get())
}

#[trial]
fn a_footer_button_that_stays_open_reports_its_pick_and_keeps_the_dialog() {
    assert_eq!(click_footer("Apply", true), (None, Some(0), true), "Apply picks and the dialog stays open");
}

#[trial]
fn the_host_can_refuse_the_close_of_ok_but_not_of_cancel() {
    assert_eq!(click_footer("OK", true), (Some(CloseReason::Activated), Some(1), false), "OK closes when the host allows it");
    assert_eq!(click_footer("OK", false), (None, Some(1), true), "OK stays open when the host refuses");
    assert_eq!(click_footer("Cancel", false), (Some(CloseReason::Requested), Some(2), false), "Cancel closes whatever the host says");
}

/// The size the dialogs below open at, and the screen they are shown on.
const OPENING: Vec2 = Vec2::new(420.0, 300.0);
const LINES: usize = 200;
const SCREEN: Vec2 = Vec2::new(800.0, 600.0);

/// The rect of a resizable dialog opened at `OPENING` whose body is `body` tall.
fn opened_rect(lines: usize) -> Rect {
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        Dialog::new("Sized").resizable(true).overlay(|overlay| overlay.size(OPENING)).opened(Bind::bound(&mut open)).show(ui, |ui, _| {
            for _ in 0..lines {
                Text::new("Line").show(ui);
            }
        });
    });
    fixture.room(SCREEN);
    fixture.settle();
    fixture.by_role(Role::DIALOG).first().expect("the dialog is shown").rect
}

#[trial]
fn a_resizable_dialog_opens_at_its_default_size_within_the_screen() {
    let screen = Rect::from_min_size(egui::Pos2::ZERO, SCREEN);
    let small = opened_rect(1);
    assert!(small.width() >= OPENING.x && small.height() >= OPENING.y, "short content does not shrink the dialog: {small:?}");
    let tall = opened_rect(LINES);
    assert!(screen.contains_rect(tall), "tall content does not push the dialog off the screen: {tall:?}");
}

/// Enter pressed in a text field of a dialog with an OK button: what the dialog reported and whether it is open.
fn enter_in_field(allow: bool) -> (Option<CloseReason>, Option<usize>, bool) {
    let seen = Rc::new(Cell::new((None, None)));
    let open = Rc::new(Cell::new(true));
    let (sink, state) = (seen.clone(), open.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut now = state.get();
        let mut name = String::new();
        let shown = Dialog::new("Options")
            .opened(Bind::bound(&mut now))
            .initial_focus(Initial::First)
            .close_on_enter(true)
            .buttons([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)])
            .close_guard(move |_, _| allow)
            .show(ui, |ui, _| {
                TextField::new("Name", Bind::bound(&mut name)).show(ui);
            });
        if shown.output.picked.is_some() {
            sink.set((shown.output.closed, shown.output.picked));
        }
        state.set(now);
    });
    fixture.idle(30);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(5);
    let (closed, picked) = seen.get();
    (closed, picked, open.get())
}

#[trial]
fn enter_activates_ok_through_the_close_guard() {
    assert_eq!(enter_in_field(true), (Some(CloseReason::Activated), Some(0), false), "Enter closes as OK when the host allows it");
    assert_eq!(enter_in_field(false), (None, Some(0), true), "Enter leaves the dialog open when the host refuses, and reports the pick");
}

/// A Select in a dialog's scrolling body opens its list by a click and by Alt+Down and picks an item: the list's
/// layer lies outside the body's scroll.
#[trial]
fn a_select_in_a_scrolling_dialog_body_opens_its_list_and_picks() {
    let value = Rc::new(Cell::new(0u8));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let entries = [Entry::new(0u8, "Normal"), Entry::new(1, "Multiply"), Entry::new(2, "Screen")];
        let mut mode = shown.get();
        let mut open = true;
        Dialog::new("Blend").opened(Bind::bound(&mut open)).show(ui, |ui, _| {
            Select::new("Blend mode", Bind::bound(&mut mode), &entries).show(ui);
        });
        shown.set(mode);
    });
    fixture.settle();
    let select = fixture.by_label("Blend mode").expect("the select is shown");
    fixture.click(&select);
    fixture.settle();
    assert!(!fixture.by_role(Role::LIST_BOX).is_empty(), "a click opens the list");
    let screen = fixture.by_label("Screen").expect("the list shows its options");
    fixture.click(&screen);
    fixture.settle();
    assert_eq!(value.get(), 2, "a click on an option picks it");
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.settle();
    assert!(!fixture.by_role(Role::LIST_BOX).is_empty(), "Alt+Down opens the list");
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.settle();
    assert_eq!(value.get(), 1, "Up and Enter pick the option above");
}
