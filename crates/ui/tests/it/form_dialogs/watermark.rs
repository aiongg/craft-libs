//! The own logic of the watermark rows: which insets apply to a position, and that switching the style keeps the
//! text and the image path apart.

use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{Watermark, WatermarkSettings, WatermarkStyle};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

/// Picking Graphic and then Text again brings the typed text back.
#[trial]
fn switching_the_style_restores_the_text() {
    let mut opening = WatermarkSettings::default();
    opening.style = WatermarkStyle::Text("Studio".to_owned());
    let value = Rc::new(RefCell::new(opening));
    let held = Rc::clone(&value);
    let mut fixture = Fixture::new(move |ui| {
        let mut value = held.borrow_mut();
        Watermark::new("Watermark", Bind::bound(&mut *value)).show(ui);
    });
    fixture.settle();
    let graphic = fixture.by_label("Graphic").expect("the style tabs are shown");
    fixture.click(&graphic);
    fixture.idle(3);
    assert_eq!(value.borrow().style, WatermarkStyle::Graphic(String::new()));
    let text = fixture.by_label("Text").expect("the Text tab is shown");
    fixture.click(&text);
    fixture.idle(3);
    assert_eq!(value.borrow().style, WatermarkStyle::Text("Studio".to_owned()));
}
