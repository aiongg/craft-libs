use egui::accesskit::Role;
use egui::{Id, Sense, vec2};

use super::RadioSelection;
use crate::primitives::Interactive;

fn items(n: usize) -> Vec<(Id, bool)> {
    (0..n).map(|i| (Id::unique(("item", i)), true)).collect()
}

#[test]
fn a_selection_past_the_last_item_selects_none_and_the_first_item_stays_the_tab_stop() {
    egui::__run_test_ui(|ui| {
        let radios = RadioSelection::begin(ui, items(3), Some(3));
        assert_eq!((radios.selected(), radios.changed()), (None, false));
        assert_eq!([radios.sense(0), radios.sense(1)], [Sense::click(), Sense::CLICK]);
        let mut empty = RadioSelection::begin(ui, Vec::new(), Some(0));
        empty.sensed(ui, &[]);
        assert_eq!(empty.selected(), None);
    });
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "one sensed element per item")]
fn sensing_more_elements_than_items_is_caught_in_debug_builds() {
    egui::__run_test_ui(|ui| {
        let mut radios = RadioSelection::begin(ui, items(1), Some(0));
        let sensed = vec![
            Interactive::new(0, Role::RadioButton, "A").interact(ui, vec2(10.0, 10.0)),
            Interactive::new(1, Role::RadioButton, "B").interact(ui, vec2(10.0, 10.0)),
        ];
        radios.sensed(ui, &sensed);
    });
}
