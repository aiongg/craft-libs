//! New Guide and Edit Selected Guides: where the dialog opens, Alt-Reset, and the host's colour requests.

use craft_behavior::Axis;
use craft_theme::Srgba;
use craft_ui::canvas::{Guide, GuideColor, GuideKey};
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{GuideUnits, HostRequests, NewGuideDialog, NewGuideValues};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::access::NodeKey;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

const WIDTH: f64 = 1000.0;
const HEIGHT: f64 = 800.0;
const PPI: f64 = 72.0;
const EDITED_AT: f64 = 250.0;
/// Where the guide the binding holds stands.
const HELD_AT: f64 = 10.0;
const PICKED: Srgba = Srgba::rgb(0.07, 0.2, 0.34);

fn units() -> GuideUnits {
    GuideUnits::new(UnitKind::Pixels, WIDTH, HEIGHT, PPI)
}

fn preference() -> craft_theme::Rgba {
    GuideColor::Cyan.rgba()
}

/// A fixture showing New Guide over `values`, with the host's `requests` answered and refilled each pass.
fn new_guide(values: &Rc<RefCell<NewGuideValues>>, requests: &Rc<RefCell<HostRequests>>) -> Fixture {
    let (held, asked) = (Rc::clone(values), Rc::clone(requests));
    let mut fixture = Fixture::new(move |ui| {
        let mut values = held.borrow_mut();
        let answers = asked.borrow_mut().take_answers();
        let shown = NewGuideDialog::new(units(), Bind::bound(&mut *values)).answers(answers).opened(Bind::given(true)).show(ui);
        asked.borrow_mut().extend(shown.requests);
    });
    fixture.settle();
    fixture
}

/// The Position field has the focus when the dialog opens.
#[trial]
fn new_guide_opens_with_the_focus_on_position() {
    let fixture = new_guide(&Rc::new(RefCell::new(NewGuideValues::default())), &Rc::new(RefCell::new(HostRequests::new())));
    let focused = fixture.focused().expect("a field has the focus");
    assert_eq!(fixture.tree().name_of(NodeKey::of(focused)), Some("Position"));
}

/// Alt turns Cancel into Reset, which puts back the values the dialog opened with.
#[trial]
fn alt_reset_in_new_guide_restores_the_opening_values() {
    let opening = NewGuideValues::default();
    let values = Rc::new(RefCell::new(opening));
    let mut fixture = new_guide(&values, &Rc::new(RefCell::new(HostRequests::new())));
    let vertical = fixture.by_label("Vertical").expect("the orientation is shown");
    fixture.click(&vertical);
    fixture.settle();
    assert_ne!(*values.borrow(), opening, "the edit wrote through");
    fixture.hold(Modifiers::ALT);
    fixture.settle();
    let reset = fixture.by_label("Reset").expect("Alt turns Cancel into Reset");
    fixture.click(&reset);
    fixture.settle();
    assert_eq!(*values.borrow(), opening, "Reset wrote the opening values back");
}

/// Edit Selected Guides starts from the guide it edits, whatever the binding held.
#[trial]
fn edit_guide_opens_on_the_given_guide_while_the_bind_holds_another() {
    let mut guide = Guide::new(GuideKey::new(7), Axis::Y, EDITED_AT);
    guide.color = Some(GuideColor::Magenta.rgba());
    let values = Rc::new(RefCell::new(NewGuideValues::new(Axis::X, HELD_AT, GuideColor::Black)));
    let held = Rc::clone(&values);
    let edited = guide.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut values = held.borrow_mut();
        NewGuideDialog::edit(units(), edited.clone(), preference(), Bind::bound(&mut *values)).opened(Bind::given(true)).show(ui);
    });
    fixture.settle();
    assert_eq!(*values.borrow(), NewGuideValues::of_guide(&guide, preference()));
}

/// Picking Custom asks the host for a colour and keeps the colour until the host answers; a cancelled request changes
/// nothing, and an answer sets the colour.
#[trial]
fn a_host_cancel_keeps_the_colour_and_an_answer_sets_it() {
    let values = Rc::new(RefCell::new(NewGuideValues::new(Axis::X, 0.0, GuideColor::Black)));
    let requests = Rc::new(RefCell::new(HostRequests::new()));
    let mut fixture = new_guide(&values, &requests);
    let pick_custom = |fixture: &mut Fixture| {
        fixture.key(Key::Tab, Modifiers::NONE);
        fixture.key(Key::ArrowDown, Modifiers::ALT);
        fixture.idle(2);
        fixture.key(Key::Home, Modifiers::NONE);
        fixture.key(Key::Enter, Modifiers::NONE);
        fixture.settle();
    };
    pick_custom(&mut fixture);
    assert!(requests.borrow().clone().take_color().is_some(), "Custom asks the host for a colour");
    assert_eq!(values.borrow().color, GuideColor::Black, "the colour waits for the answer");
    requests.borrow_mut().cancel();
    fixture.settle();
    assert!(requests.borrow_mut().take_color().is_none(), "the cancelled request is gone");
    assert_eq!(values.borrow().color, GuideColor::Black, "a cancelled pick changes nothing");
    let key = "guides.newGuide.color";
    requests.borrow_mut().answer_color(key, PICKED);
    fixture.settle();
    assert_eq!(values.borrow().color, GuideColor::Custom(PICKED.to_rgba()), "the answer sets the colour");
}
