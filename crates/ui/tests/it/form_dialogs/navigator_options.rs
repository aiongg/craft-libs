//! Navigator Panel Options: the Color list and its Custom… colour.

use craft_theme::Srgba;
use craft_ui::canvas::{NavigatorOptions, ProxyColor};
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{HostRequests, NAVIGATOR_PROXY_COLOR_KEY, NavigatorOptionsDialog};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

const PICKED: Srgba = Srgba::rgb(0.07, 0.2, 0.34);

/// A fixture showing Panel Options over `options`, with the host's `requests` answered and refilled each pass.
fn panel_options(options: &Rc<RefCell<NavigatorOptions>>, requests: &Rc<RefCell<HostRequests>>) -> Fixture {
    let (held, asked) = (Rc::clone(options), Rc::clone(requests));
    let mut fixture = Fixture::new(move |ui| {
        let mut options = held.borrow_mut();
        let answers = asked.borrow_mut().take_answers();
        let shown = NavigatorOptionsDialog::new(Bind::bound(&mut *options)).answers(answers).opened(Bind::given(true)).show(ui);
        asked.borrow_mut().extend(shown.requests);
    });
    fixture.settle();
    fixture
}

/// Opens the Color list with Alt+Down on the focused select and picks its last entry, Custom….
fn pick_custom(fixture: &mut Fixture) {
    if fixture.focused().is_none() {
        fixture.key(Key::Tab, Modifiers::NONE);
    }
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.settle();
    fixture.key(Key::End, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.settle();
}

/// Every pick of Custom… asks the host, also while a custom colour is selected, and the picker opens at that colour;
/// the custom colour is still the value when the dialog opens again.
#[trial]
fn custom_asks_the_host_every_time_and_survives_reopening() {
    let options = Rc::new(RefCell::new(NavigatorOptions::new(ProxyColor::Blue)));
    let requests = Rc::new(RefCell::new(HostRequests::new()));
    let mut fixture = panel_options(&options, &requests);
    pick_custom(&mut fixture);
    let first = requests.borrow_mut().take_color().expect("Custom… asks the host for a colour");
    assert_eq!(first.key, NAVIGATOR_PROXY_COLOR_KEY);
    assert_eq!(options.borrow().proxy_color, ProxyColor::Blue, "the colour waits for the answer");
    requests.borrow_mut().answer_color(NAVIGATOR_PROXY_COLOR_KEY, PICKED);
    fixture.settle();
    let custom = ProxyColor::custom(PICKED);
    assert_eq!(options.borrow().proxy_color, custom, "the answer sets the colour");

    pick_custom(&mut fixture);
    let again = requests.borrow_mut().take_color().expect("Custom… asks again while it is selected");
    assert_eq!(again.color, custom.color(), "the picker opens at the custom colour");

    let mut reopened = panel_options(&options, &requests);
    pick_custom(&mut reopened);
    let after_reopen = requests.borrow_mut().take_color().expect("Custom… asks after the dialog opened again");
    assert_eq!(after_reopen.color, custom.color(), "the custom colour is the value when the dialog reopens");
}
