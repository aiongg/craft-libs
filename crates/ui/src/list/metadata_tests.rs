//! `MetadataTable` in a harness.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::theme::{Base, install};

fn row(name: &str, value: &str) -> MetadataRow {
    MetadataRow { name: name.into(), value: value.into() }
}

fn sample() -> Vec<MetadataGroup> {
    vec![
        MetadataGroup { name: "Camera".into(), rows: vec![row("Exposure Time", "1/250 s"), row("Model", "Alpha 7")] },
        MetadataGroup { name: "XMP".into(), rows: vec![row("dc:creator", "Mara Vale")] },
    ]
}

// `filter_metadata` keeps name or value matches, any case, and drops empty groups.
#[test]
fn the_filter_keeps_fields_whose_name_or_value_match() {
    let groups = sample();
    assert_eq!(filter_metadata(&groups, "  ").len(), 2, "an empty query keeps everything");
    let by_name = filter_metadata(&groups, "EXPOSURE");
    assert_eq!(by_name.len(), 1);
    assert_eq!(by_name[0].rows, [row("Exposure Time", "1/250 s")]);
    let by_value = filter_metadata(&groups, "vale");
    assert_eq!((by_value.len(), by_value[0].name.as_str()), (1, "XMP"));
    assert!(filter_metadata(&groups, "zzz").is_empty());
}

type State = (Vec<MetadataGroup>, String, Vec<String>, bool);

fn harness(chosen: Vec<String>, choose: bool) -> Harness<'static, State> {
    Harness::<State>::builder().with_size([420.0, 300.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        move |ui, (groups, query, chosen, changed)| {
            let mut table = MetadataTable::new("meta", groups).filter(query);
            if choose {
                table = table.choose(chosen);
            }
            *changed |= table.show(ui).chosen_changed;
        },
        (sample(), String::new(), chosen, false),
    )
}

// Typing in the filter field hides rows; with none left the body reads "No fields match".
#[test]
fn typing_in_the_filter_hides_rows_and_says_when_none_match() {
    let mut harness = harness(Vec::new(), false);
    harness.run();
    harness.get_by_label("Exposure Time");
    harness.get_by_label("dc:creator");
    harness.get_by_role_and_label(Role::SearchInput, "Filter fields").focus();
    harness.run();
    harness.get_by_role_and_label(Role::SearchInput, "Filter fields").type_text("model");
    harness.run();
    harness.get_by_label("Model");
    assert!(harness.query_by_label("Exposure Time").is_none());
    assert!(harness.query_by_label("dc:creator").is_none());
    harness.get_by_role_and_label(Role::SearchInput, "Filter fields").type_text("zzz");
    harness.run();
    harness.get_by_label("No fields match");
}

// With `choose`, a check adds the name at the end and unchecking removes it; `chosen_changed` is set.
#[test]
fn checking_a_field_adds_it_to_the_chosen_columns() {
    let mut harness = harness(vec!["Model".into()], true);
    harness.run();
    harness.get_by_role_and_label(Role::CheckBox, "Show Model");
    harness.get_by_role_and_label(Role::CheckBox, "Show dc:creator").click();
    harness.run();
    assert_eq!(harness.state().2, ["Model", "dc:creator"]);
    assert!(harness.state().3, "the table reports the change");
    harness.get_by_role_and_label(Role::CheckBox, "Show Model").click();
    harness.run();
    assert_eq!(harness.state().2, ["dc:creator"]);
}

// Down from the filter field moves into the rows; Down moves the lead and Space toggles its check.
#[test]
fn the_keys_move_the_lead_and_space_toggles_its_check() {
    let mut harness = harness(Vec::new(), true);
    harness.run();
    harness.get_by_role_and_label(Role::SearchInput, "Filter fields").focus();
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    harness.key_press(egui::Key::Space);
    harness.run();
    assert_eq!(harness.state().2, ["Model"]);
    assert!(harness.state().3);
}
