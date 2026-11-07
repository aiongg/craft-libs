//! `Field`: the help line is part of the field, and a field without help shows none.

use craft_ui::contract::X;
use craft_ui::form::{Field, Help};
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::NumberField;

const SIZE: f64 = 12.0;
const HELP: &str = "In points";

fn shown(help: Option<Help<'static>>) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let field = Field::new("Size");
        let field = match help {
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
fn help_text_shows_only_when_the_field_has_it() {
    assert!(help_shown(&shown(Some(Help::Text(HELP)))), "the help line is shown");
    assert!(!help_shown(&shown(None)), "a field without help shows no help line");
}
