//! Dialog bodies and footer buttons.

use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::contract::{Bind, Props as _, SelectMode, ShowCollection as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::list::{List, ListRow};
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::{Fixture, trial};
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
            .close_guard(move |_| allow)
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
