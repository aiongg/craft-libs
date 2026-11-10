//! `Field`: the help line is part of the field, and a field without help shows none.

use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _, Validity};
use craft_ui::form::{Field, Help};
use craft_ui::runtime::env::Message;
use craft_ui::testing::access::focused_key;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::NumberField;
use std::cell::Cell;
use std::rc::Rc;

const SIZE: f64 = 12.0;
const HELP: &str = "In points";

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

/// The help line gives way to a validity message on the next frame and returns when the validity clears.
#[trial]
fn a_message_replaces_the_help_line_until_the_validity_clears() {
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

/// A field's label and its control's text sit on one baseline, whatever the control's own padding. Guards against a
/// label placed from the control's top while the control's text sits lower in a taller box.
#[trial]
fn a_label_and_its_control_text_share_a_baseline() {
    use craft_ui::contract::Entry;
    use craft_ui::pickers::Select;
    use craft_ui::text::TextField;
    let mut fixture = Fixture::new(|ui| {
        let entries = [Entry::new(0u8, "Hamburg"), Entry::new(1, "Multiply")];
        let mut mode = 0u8;
        let mut text = String::from("Hamburg");
        Field::new("Name").show(ui, |ui, _| TextField::new("Name", Bind::bound(&mut text)).show(ui));
        Field::new("Blend").show(ui, |ui, _| Select::new("Blend", Bind::bound(&mut mode), &entries).show(ui).response);
        Field::new("Size").show(ui, |ui, _| NumberField::new("Size", Bind::bound(&mut Some(12.0f64))).show(ui));
    });
    fixture.settle();
    fixture.idle(3);
    let texts = fixture.texts();
    for (label, control) in [("Name", "Hamburg"), ("Blend", "Hamburg"), ("Size", "12")] {
        let label_at = texts
            .iter()
            .find(|text| text.text == label)
            .unwrap_or_else(|| panic!("no label {label}: {:?}", texts.iter().map(|t| (&t.text, t.rect)).collect::<Vec<_>>()));
        let control_at = texts
            .iter()
            .filter(|text| text.text.starts_with(control) && text.rect.left() > label_at.rect.right())
            .min_by(|a, b| (a.rect.center().y - label_at.rect.center().y).abs().total_cmp(&(b.rect.center().y - label_at.rect.center().y).abs()))
            .unwrap_or_else(|| panic!("no control text for {label}"));
        assert!(
            label_at.rect.bottom().round() == control_at.rect.bottom().round(),
            "{label}: label {:?} control text {:?}",
            label_at.rect,
            control_at.rect
        );
    }
}
