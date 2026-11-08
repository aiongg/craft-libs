//! A `Mixed` swatch with a picker key files the colour request, opened at the host's first value, and a pick from it
//! reaches the host.

use craft_behavior::Mixed;
use craft_theme::Srgba;
use craft_ui::choice::Swatch;
use craft_ui::contract::Show as _;
use craft_ui::patterns::form_dialogs::HostRequests;
use craft_ui::runtime::requests::HostAnswer;
use craft_ui::testing::{Fixture, trial};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
const GREEN: Srgba = Srgba::rgb(0.0, 1.0, 0.0);

#[trial]
fn a_click_on_a_mixed_swatch_files_a_request_and_the_pick_reaches_the_host() {
    let picked = Rc::new(Cell::new(None));
    let requests = Rc::new(RefCell::new(HostRequests::new()));
    let (result, filed) = (picked.clone(), requests.clone());
    let mut fixture = Fixture::new(move |ui| {
        let out = Swatch::new("Fill", Mixed::<Srgba>::Mixed).picker("fill").mixed_start(RED).show(ui);
        if let Some(request) = out.pick_color {
            filed.borrow_mut().request_color(request);
        }
        for answer in filed.borrow_mut().take_answers() {
            if let HostAnswer::Color { color, .. } = answer {
                result.set(Some(color));
            }
        }
    });
    fixture.settle();
    let well = fixture.by_label("Fill").expect("shown");
    fixture.click(&well);
    let request = requests.borrow_mut().take_color().expect("the click filed a request");
    assert_eq!((request.key.as_str(), request.color), ("fill", RED));
    requests.borrow_mut().answer_color("fill", GREEN);
    fixture.idle(2);
    assert_eq!(picked.get(), Some(GREEN));
}
