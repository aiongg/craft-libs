//! `Field`: the help line is part of the field, and a field without help shows none.

use craft_theme::variant::Tone;
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _, Validity};
use craft_ui::display::Text;
use craft_ui::form::{Field, Help};
use craft_ui::runtime::env::Message;
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
fn help_text_shows_only_when_the_field_has_it() {
    assert!(help_shown(&shown(Some(Help::Text(HELP.into())))), "the help line is shown");
    assert!(!help_shown(&shown(None)), "a field without help shows no help line");
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

/// The colours the texts of the field paint with.
fn inks(fixture: &Fixture) -> Vec<craft_ui::style::Stateful<craft_ui::style::StyleColor>> {
    fixture.styles_of(std::any::type_name::<Text>()).iter().flat_map(|seen| seen.looks.texts.iter().map(|look| look.color.clone())).collect()
}

/// The ink a `Text` in `tone` reads in.
fn tone_ink(tone: Tone) -> craft_ui::style::Stateful<craft_ui::style::StyleColor> {
    let mut fixture = Fixture::new(move |ui| {
        Text::new("Reference").tone(tone).id_salt("reference").show(ui);
    });
    fixture.settle();
    inks(&fixture).into_iter().next().expect("the reference text resolved a style")
}

/// A warning shows in the warn tone and a refusal in the error tone.
#[trial]
fn a_warning_reads_in_the_warn_tone_and_a_refusal_in_the_error_tone() {
    let value = Rc::new(Cell::new(500.0));
    let warned = judged(&value, |size| if size > 100.0 { Validity::Warn(Message::new(MESSAGE)) } else { Validity::Ok });
    let refused = judged(&value, |size| if size > 100.0 { Validity::Refuse(Message::new(MESSAGE)) } else { Validity::Ok });
    let (warn, bad) = (tone_ink(Tone::Warn), tone_ink(Tone::Bad));
    assert!(inks(&warned).contains(&warn) && !inks(&warned).contains(&bad), "a warning is not painted as an error");
    assert!(inks(&refused).contains(&bad) && !inks(&refused).contains(&warn), "a refusal is painted as an error");
}
