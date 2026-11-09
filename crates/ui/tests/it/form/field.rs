//! `Field`: the help line is part of the field, and a field without help shows none.

use craft_ui::contract::{Bind, Props as _, Role, Show as _, ShowContainer as _, Validity};
use craft_ui::display::Text;
use craft_ui::form::{Field, Help};
use craft_ui::runtime::env::Message;
use craft_ui::testing::access::focused_key;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::NumberField;
use std::cell::Cell;
use std::rc::Rc;

const SIZE: f64 = 12.0;
const HELP: &str = "In points";

fn shown(help: Option<Help>) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let field = Field::new("Size");
        let field = match help.clone() {
            Some(help) => field.help(help),
            None => field,
        };
        field.show(ui, |ui, _| NumberField::new("Size", Bind::given(Some(SIZE))).show(ui));
    });
    fixture.settle();
    fixture
}

fn help_shown(fixture: &Fixture) -> bool {
    fixture.by_role(Role::LABEL).into_iter().any(|label| fixture.tree().name_of(label.node) == Some(HELP))
}

#[trial]
fn help_text_shows_only_when_the_field_has_it_and_a_message_replaces_it() {
    {
        assert!(help_shown(&shown(Some(Help::Text(HELP.into())))), "the help line is shown");
        assert!(!help_shown(&shown(None)), "a field without help shows no help line");
    }
    {
        // The help line gives way to a validity message on the next frame and returns when the validity clears.
        let value = Rc::new(Cell::new(SIZE));
        let mut fixture = judged(&value, |size| if size > 100.0 { Validity::Refuse(Message::new(MESSAGE)) } else { Validity::Ok });
        assert!(line_shown(&fixture, HELP) && !line_shown(&fixture, MESSAGE), "a valid value shows the help line");
        value.set(500.0);
        fixture.settle();
        assert!(line_shown(&fixture, MESSAGE) && !line_shown(&fixture, HELP), "a refused value shows its message in place of the help");
        value.set(SIZE);
        fixture.settle();
        assert!(line_shown(&fixture, HELP) && !line_shown(&fixture, MESSAGE), "the help returns once the message clears");
    }
}

const MESSAGE: &str = "Size is too large";

/// A field with help over a number the test sets between frames; `check` judges it.
fn judged(value: &Rc<Cell<f64>>, check: fn(f64) -> Validity<f64>) -> Fixture {
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let current = shown.get();
        Field::new("Size").help(Help::Text(HELP.into())).show(ui, |ui, _| {
            NumberField::new(
                "Size",
                Bind::given(Some(current)).check(move |size: &Option<f64>| {
                    size.map_or(Validity::Ok, |size| match check(size) {
                        Validity::Fix(fixed) => Validity::Fix(Some(fixed)),
                        Validity::Warn(message) => Validity::Warn(message),
                        Validity::Refuse(message) => Validity::Refuse(message),
                        _ => Validity::Ok,
                    })
                }),
            )
            .show(ui)
        });
    });
    fixture.settle();
    fixture
}

fn line_shown(fixture: &Fixture, text: &str) -> bool {
    fixture.by_role(Role::LABEL).into_iter().any(|label| fixture.tree().name_of(label.node) == Some(text))
}

/// A click on a field's label moves focus to its control, as on any form label.
#[trial]
fn a_click_on_the_label_focuses_the_control() {
    let mut fixture = Fixture::new(|ui| {
        Field::new("Spacing").show(ui, |ui, _| {
            NumberField::new("Spacing", Bind::given(Some(8.0))).show(ui);
        });
    });
    fixture.settle();
    let control = fixture.by_label("Spacing").expect("the field's control");
    let label = fixture.by_role(Role::LABEL).into_iter().next().expect("the field's label");
    assert!(!label.rect.intersects(control.rect), "the label sits beside the control");
    assert_ne!(focused_key(fixture.tree()), control.node, "the control starts unfocused");
    fixture.click(&label);
    assert_eq!(focused_key(fixture.tree()), control.node, "the label's click focused the control");
}

const READOUT: &str = "12 mm";

#[trial]
fn a_readout_field_reports_its_value() {
    let mut fixture = Fixture::new(|ui| {
        Field::new("Length").value(READOUT).show(ui, |ui, _| {
            Text::new(READOUT).show(ui);
        });
    });
    fixture.settle();
    let tree = fixture.tree();
    let valued = fixture.by_role(Role::GROUP).into_iter().any(|group| tree.value_of(group.node) == Some(READOUT));
    assert!(valued, "the field node carries the readout as its value");
}
