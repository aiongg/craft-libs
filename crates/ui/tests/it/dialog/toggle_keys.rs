//! Keys on a toggle control inside a dialog.

use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::runtime::focus::Initial;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

/// Space toggles the focused check box and nothing else: the dialog stays open and no button is picked. Enter on it is the
/// default button's (the full-window dialog trials cover that).
#[trial]
fn space_on_a_focused_check_box_toggles_it_and_leaves_the_dialog_open() {
    let checked = Rc::new(Cell::new(false));
    let picked = Rc::new(Cell::new(None));
    let (shown_checked, shown_picked) = (checked.clone(), picked.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut value = shown_checked.get();
        let mut open = true;
        let shown = Dialog::new("Options")
            .opened(Bind::bound(&mut open))
            .initial_focus(Initial::First)
            .buttons([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)])
            .show(ui, |ui, _| {
                Checkbox::new(Bind::bound(&mut value)).label("Show the Grid").show(ui);
            });
        if shown.output.picked.is_some() {
            shown_picked.set(shown.output.picked);
        }
        shown_checked.set(value);
    });
    fixture.settle();
    assert!(fixture.focused().is_some(), "the check box holds focus");
    fixture.key(Key::Space, Modifiers::NONE).settle();
    assert!(checked.get(), "Space toggles the check box");
    assert_eq!(picked.get(), None, "Space picks no button");
}
