//! `IconPicker` search and its harness.

use super::*;

fn item(key: u32, label: &str, keywords: &[&str]) -> IconPickerItem<u32> {
    IconPickerItem { key, label: label.to_string(), keywords: keywords.iter().map(|k| k.to_string()).collect() }
}

// `icon_matches` matches a label or a keyword, any case; an empty query matches all.
#[test]
fn the_search_matches_labels_and_keywords() {
    let arrow = item(1, "Arrow Right", &["direction", "Next"]);
    assert!(icon_matches(&arrow, ""));
    assert!(icon_matches(&arrow, "  "));
    assert!(icon_matches(&arrow, "arrow"));
    assert!(icon_matches(&arrow, "RIGHT"));
    assert!(icon_matches(&arrow, " direct "), "the query is trimmed");
    assert!(icon_matches(&arrow, "next"), "a keyword in any case");
    assert!(!icon_matches(&arrow, "left"));
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod grid {
    use egui::accesskit::Role;
    use egui::{Color32, Key, Ui};
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    use egui_kittest::{Harness, HarnessBuilder};

    use super::super::{IconPicker, IconPickerGroup};
    use super::*;
    use crate::theme::{Base, install};

    struct Host {
        groups: Vec<IconPickerGroup<u32>>,
        picked: Vec<u32>,
    }

    fn harness() -> Harness<'static, Host> {
        let group = |name: &str, items: Vec<IconPickerItem<u32>>| IconPickerGroup { name: name.to_string(), items };
        let groups = vec![
            group("Arrows", vec![item(1, "Up", &["direction"]), item(2, "Down", &["direction"]), item(3, "Left", &[])]),
            group("Shapes", vec![item(4, "Circle", &[]), item(5, "Square", &[])]),
        ];
        HarnessBuilder::default().with_size([200.0, 360.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, host: &mut Host| {
                let out = IconPicker::new("icons", "Icons", &host.groups, |painter, rect, _, color: Color32| {
                    painter.rect_filled(rect, 0.0, color);
                })
                .show(ui);
                host.picked.extend(out.picked);
            },
            Host { groups, picked: Vec::new() },
        )
    }

    fn cells(harness: &Harness<'_, Host>) -> Vec<String> {
        harness.query_all_by_role(Role::GridCell).filter_map(|n| n.accesskit_node().label()).collect()
    }

    // Typing in the search hides the groups with no match and shows the no-match line when none is left.
    #[test]
    fn typing_hides_groups_without_a_match() {
        let mut harness = harness();
        harness.run();
        assert_eq!(cells(&harness).len(), 5);
        harness.event(egui::Event::Text("direct".to_string()));
        harness.run();
        harness.run();
        assert_eq!(cells(&harness), ["Up", "Down"], "keywords match; the Shapes group is gone");
        assert!(harness.query_by_label("Shapes").is_none());
        harness.event(egui::Event::Text("zzz".to_string()));
        harness.run();
        harness.run();
        assert!(cells(&harness).is_empty());
        assert!(harness.query_by_label("No icons match your search.").is_some());
    }

    // ↓ from the search goes to the first cell; the arrows move as a wrapped grid; Enter reports `picked`.
    #[test]
    fn arrows_move_over_the_cells_and_enter_picks() {
        let mut harness = harness();
        harness.run();
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        assert!(harness.get_by_label("Up").accesskit_node().is_focused(), "the first cell has the focus");
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert!(harness.get_by_label("Down").accesskit_node().is_focused());
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().picked, [5], "Down's column in the next group's row is Square");
        harness.key_press(Key::ArrowUp);
        harness.run();
        harness.key_press(Key::ArrowUp);
        harness.run();
        assert!(harness.get_by_role(Role::SearchInput).accesskit_node().is_focused(), "↑ from the first row goes back to the search field");
    }
}
