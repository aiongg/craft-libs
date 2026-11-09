//! A content-sized layer sizes from its whole body: fields nested in groups count at their content's width and
//! height from the first pass, so the layer holds them and settles where its buttons can be clicked.

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::form::{Field, FieldGroup};
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::relations::inside;
use craft_ui::testing::{Fixture, trial};
use std::cell::Cell;
use std::rc::Rc;

type Rig = (Fixture, Rc<Cell<bool>>, Rc<Cell<Option<CloseReason>>>);

/// A dialog titled `T` with `groups` field groups of three fields, each holding a button wider than the title and
/// the footer. Returns the fixture, whether the dialog is open and how it closed.
fn rig(groups: usize) -> Rig {
    let open = Rc::new(Cell::new(true));
    let closed = Rc::new(Cell::new(None));
    let (state, seen) = (Rc::clone(&open), Rc::clone(&closed));
    let mut fixture = Fixture::new(move |ui| {
        let mut now = state.get();
        let shown = Dialog::new("T")
            .opened(Bind::bound(&mut now))
            .buttons([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)])
            .show(ui, |ui, _| {
                for group in 0..groups {
                    FieldGroup::new(format!("Group {group}")).show(ui, |ui, _| {
                        for field in 0..3 {
                            Field::new(format!("Field {group}.{field}")).show(ui, |ui, _| {
                                Button::new(format!("A control much wider than the title {group}.{field}")).show(ui);
                            });
                        }
                    });
                }
            });
        if let Some(why) = shown.output.closed {
            seen.set(Some(why));
        }
        state.set(now);
    });
    fixture.settle();
    (fixture, open, closed)
}

#[trial]
fn a_content_sized_dialog_wider_than_its_header_holds_its_fields() {
    let (fixture, _, _) = rig(1);
    let dialog = fixture.by_role(Role::DIALOG).first().expect("the dialog is shown").rect;
    // A field names its control, so the controls are found by role: every button, the fields' and the footer's.
    let buttons = fixture.by_role(Role::BUTTON);
    assert!(buttons.len() >= 5, "the three controls and the footer buttons are shown");
    for button in buttons {
        assert!(inside(button.rect, dialog), "every button is inside the dialog: {:?} in {dialog:?}", button.rect);
    }
}

#[trial]
fn a_footer_click_on_a_tall_dialog_answers() {
    let (mut fixture, open, closed) = rig(3);
    let ok = fixture.by_label("OK").expect("the OK button is shown");
    fixture.click(&ok);
    assert_eq!(closed.get(), Some(CloseReason::Activated), "the OK click closes the dialog as its accept");
    assert!(!open.get(), "the dialog is closed");
}
