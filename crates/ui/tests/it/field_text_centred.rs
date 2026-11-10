//! Vertical placement of the line in a control.

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Entry, Show as _};
use craft_ui::pickers::Select;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::{NumberField, TextField};
use std::cell::RefCell;
use std::rc::Rc;

/// The single line of a button, select, text field and number field sits at the vertical centre of its box. Guards
/// against a field style whose padding put the top inset where the end inset belongs, and a select that painted top aligned.
#[trial]
fn a_single_line_is_centred_in_its_control() {
    let boxes: Rc<RefCell<Vec<(&'static str, egui::Rect)>>> = Rc::default();
    let b = boxes.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut v = b.borrow_mut();
        v.clear();
        v.push(("button", Button::new("Hamburg").show(ui).rect));
        let entries = [Entry::new(0u8, "Hamburg"), Entry::new(1, "Multiply")];
        let mut mode = 0u8;
        v.push(("select", Select::new("Blend", Bind::bound(&mut mode), &entries).show(ui).response.rect));
        let mut t = String::from("Hamburg");
        v.push(("text", TextField::new("T", Bind::bound(&mut t)).show(ui).response.rect));
        v.push(("number", NumberField::new("N", Bind::bound(&mut Some(12.0f64))).show(ui).response.rect));
    });
    fixture.settle();
    fixture.idle(3);
    for (name, r) in boxes.borrow().iter() {
        let text = fixture.texts().iter().find(|text| r.contains_rect(text.rect)).unwrap_or_else(|| panic!("{name} paints no text"));
        let (above, below) = (text.rect.top() - r.top(), r.bottom() - text.rect.bottom());
        assert!((above - below).abs() < above.min(below), "{name}: {above} above the text, {below} below it");
    }
}

/// A caption and the controls beside it in a bar share one baseline: their line boxes have the same height, so
/// the bottoms of their text coincide.
#[trial]
fn a_caption_in_a_bar_shares_the_baseline_of_the_control_beside_it() {
    use craft_ui::bars::{Bar, BarKind};
    use craft_ui::contract::ShowContainer as _;
    use craft_ui::display::Text;
    let mut fixture = Fixture::new(move |ui| {
        Bar::new("Opts", BarKind::Options).show(ui, |ui, _| {
            Text::new("Theme").show(ui);
            let entries = [Entry::new(0u8, "Hamburg"), Entry::new(1, "Multiply")];
            let mut mode = 0u8;
            Select::new("Blend", Bind::bound(&mut mode), &entries).show(ui);
        });
    });
    fixture.settle();
    fixture.idle(3);
    let texts = fixture.texts();
    let bottom = |name: &str| texts.iter().find(|text| text.text == name).map(|text| text.rect.bottom()).unwrap();
    eprintln!("BASE {} {}", bottom("Theme"), bottom("Hamburg"));
    assert!((bottom("Theme") - bottom("Hamburg")).abs() < 0.5);
}
