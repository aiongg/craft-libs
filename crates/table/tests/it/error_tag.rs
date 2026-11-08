//! The error checking tag.

use std::cell::Cell;
use std::rc::Rc;

use craft_table::ui::ErrorTag;
use craft_ui::contract::Show as _;
use craft_ui::runtime::env::Label;
use craft_ui::testing::{Fixture, trial};
use egui::vec2;

/// Choosing a fix in the tag's menu reports its index.
#[trial]
fn fix_menu_reports_its_index() {
    let fixes = [Label::Static("Convert to Number"), Label::Static("Ignore Error")];
    let fix = Rc::new(Cell::new(None));
    let sink = fix.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = ErrorTag::new("Number Stored as Text", &fixes).show(ui);
        sink.set(sink.get().or(out.fix));
    });
    fixture.room(vec2(300.0, 200.0));
    fixture.settle();
    let tag = fixture.by_label("Error checking options").expect("the tag is a named button");
    fixture.click(&tag);
    fixture.settle();
    let ignore = fixture.by_label("Ignore Error").expect("the menu lists the fixes");
    fixture.click(&ignore);
    assert_eq!(fix.get(), Some(1));
}
