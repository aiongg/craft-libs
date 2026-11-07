//! Bypass detection: a raw egui widget fails the pass-end walk, a component's own widgets pass it.

use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::controls::check::Checkbox;
use craft_ui::testing::{Fixture, bypass, trial};
use egui::{Rect, pos2};

#[trial]
fn a_component_alone_leaves_no_bypass_finding() {
    let mut checked = false;
    let mut fixture = Fixture::new(move |ui| {
        Checkbox::new(Bind::bound(&mut checked)).label("Show grid").show(ui);
    });
    fixture.settle();
    assert_eq!(bypass::findings(&fixture), Vec::<String>::new());
}

#[trial]
fn a_raw_egui_widget_next_to_a_component_is_a_finding() {
    let mut checked = false;
    let mut fixture = Fixture::new(move |ui| {
        Checkbox::new(Bind::bound(&mut checked)).label("Show grid").show(ui);
        let raw = Rect::from_min_max(pos2(0.0, 100.0), pos2(40.0, 120.0));
        craft_ui::testing::contract::raw_widget(ui, raw);
    });
    fixture.settle();
    let found = bypass::findings(&fixture);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("egui widget"), "{found:?}");
}
