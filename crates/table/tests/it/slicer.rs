//! `Slicer`: a tile click toggles its value, the funnel clears.

use std::cell::RefCell;
use std::rc::Rc;

use craft_table::ui::{Slicer, SlicerAction};
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, Sense, vec2};

#[trial]
fn clicks_toggle_and_clear() {
    let actions: Rc<RefCell<Vec<SlicerAction>>> = Rc::default();
    let shown = actions.clone();
    let mut fixture = Fixture::new(move |ui| {
        let items: Vec<(String, bool)> = ["Red", "Green", "Blue"].iter().map(|name| ((*name).to_owned(), *name == "Green")).collect();
        let rect = Rect::from_min_size(ui.cursor().min, vec2(240.0, 200.0));
        ui.allocate_rect(rect, Sense::hover());
        let out = Slicer::new("Colour", &items).columns(1).filtered(true).show(ui, rect);
        shown.borrow_mut().extend(out.action);
    });
    fixture.settle();
    let blue = fixture.by_label("Blue").expect("a tile for the value");
    fixture.click(&blue);
    let funnel = fixture.by_label("Clear Filter").expect("the funnel button");
    fixture.click(&funnel);
    assert_eq!(*actions.borrow(), [SlicerAction::Toggle("Blue".into()), SlicerAction::Clear]);
}
