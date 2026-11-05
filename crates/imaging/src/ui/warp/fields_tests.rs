//! Tests of the Warp options-bar group.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::Rect;

use craft_ui::edit::EditEvent;
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};

use super::{WarpFieldValues, WarpFields, WarpFieldsEdit, WarpFieldsLabels, WarpFieldsOutput, WarpGridChoice, WarpMesh, WarpSplit, WarpStyleEntry};

const RECT: Rect = Rect::new(0.0, 0.0, 100.0, 100.0);

struct Host {
    styles: Vec<WarpStyleEntry>,
    values: WarpFieldValues,
    split: Option<WarpSplit>,
    on_type: bool,
    labels: Option<WarpFieldsLabels>,
    edits: Vec<WarpFieldsEdit>,
}

fn entry(key: &str, label: &str, preset: bool, custom: bool) -> WarpStyleEntry {
    WarpStyleEntry { key: key.into(), label: label.into(), preset, custom }
}

fn host(style: &str) -> Host {
    Host {
        styles: vec![entry("none", "None", false, false), entry("arc", "Arc", true, false), entry("custom", "Custom", false, true)],
        values: WarpFieldValues {
            style: style.into(),
            vertical: false,
            bend: 50.0,
            horizontal: 0.0,
            vertical_distortion: 0.0,
            grid: WarpGridChoice::Default,
        },
        split: None,
        on_type: false,
        labels: None,
        edits: Vec::new(),
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut h = Harness::builder().with_size([900.0, 120.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let mut out = WarpFieldsOutput::default();
            OptionsBar::new("bar").label("Options").show(ui, |slots| {
                let mut fields =
                    WarpFields::new("warp-fields", &host.styles, host.values.clone()).split(host.split).on_type(host.on_type).output(&mut out);
                if let Some(labels) = host.labels.clone() {
                    fields = fields.labels(labels);
                }
                fields.add_to(slots);
            });
            for edit in out.edits {
                if let WarpFieldsEdit::Split(split) = &edit {
                    host.split = *split;
                }
                host.edits.push(edit);
            }
        },
        host,
    );
    h.run_steps(3);
    h
}

// A one-patch mesh reads Default, even 3 × 3 to 5 × 5 read their entries, a split mesh Custom;
// picking 4 x 4 reports Grid(4).
#[test]
fn the_grid_menu_reads_default_even_and_custom() {
    for (n, want) in
        [(1, WarpGridChoice::Default), (3, WarpGridChoice::Three), (4, WarpGridChoice::Four), (5, WarpGridChoice::Five), (2, WarpGridChoice::Custom)]
    {
        assert_eq!(WarpGridChoice::of(&WarpMesh::identity(RECT, n, n)), want, "{n} x {n}");
    }
    assert_eq!(WarpGridChoice::of(&WarpMesh::identity(RECT, 3, 4)), WarpGridChoice::Custom);
    let uneven = WarpMesh::identity_with_knots(RECT, vec![0.0, 0.2, 1.0], vec![0.0, 0.5, 1.0]).expect("valid knots");
    assert_eq!(WarpGridChoice::of(&uneven), WarpGridChoice::Custom);
    assert_eq!(WarpGridChoice::Five.size(), Some(5));
    assert_eq!(WarpGridChoice::Custom.size(), None);

    let mut h = harness(host("arc"));
    h.get_by_role_and_label(Role::ComboBox, "Grid").click();
    h.run_steps(2);
    h.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("4 x 4")).click();
    h.run_steps(3);
    assert_eq!(h.state().edits, vec![WarpFieldsEdit::Grid(4)]);
}

// On a type layer Split and Grid are disabled and the style select offers no Custom.
#[test]
fn a_type_layer_greys_split_and_grid_and_hides_custom() {
    let mut on_type = host("arc");
    on_type.on_type = true;
    let mut h = harness(on_type);
    assert!(h.get_by_label("Split Warp Crosswise").accesskit_node().is_disabled());
    assert!(h.get_by_role_and_label(Role::ComboBox, "Grid").accesskit_node().is_disabled());
    h.get_by_role_and_label(Role::ComboBox, "Warp").click();
    h.run_steps(2);
    assert!(h.query_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Arc")).is_some());
    assert!(h.query_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Custom")).is_none());

    let mut h = harness(host("arc"));
    assert!(!h.get_by_label("Split Warp Crosswise").accesskit_node().is_disabled());
    h.get_by_role_and_label(Role::ComboBox, "Warp").click();
    h.run_steps(2);
    assert!(h.query_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Custom")).is_some());
}

// On a preset style the orientation and Bend, H, V report edits; on Custom they are disabled.
#[test]
fn bend_and_distortion_fields_act_only_on_a_preset_style() {
    let mut h = harness(host("arc"));
    h.get_by_label("Change warp orientation").click();
    h.run_steps(3);
    assert_eq!(h.state().edits, vec![WarpFieldsEdit::Orientation { vertical: true }]);

    h.get_by_role_and_label(Role::SpinButton, "Horizontal distortion").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("-30".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert!(
        matches!(h.state().edits.last(), Some(WarpFieldsEdit::Horizontal(EditEvent::Commit { value, .. })) if *value == -30.0),
        "{:?}",
        h.state().edits
    );

    let h = harness(host("custom"));
    assert!(h.get_by_label("Change warp orientation").accesskit_node().is_disabled());
    for name in ["Bend", "Horizontal distortion", "Vertical distortion"] {
        assert!(h.get_by_role_and_label(Role::SpinButton, name).accesskit_node().is_disabled(), "{name}");
    }
}

// A press on a Split icon reports it armed; a press on the armed one reports None.
#[test]
fn an_armed_split_tool_is_reported_and_toggles_off() {
    let mut h = harness(host("arc"));
    h.get_by_label("Split Warp Vertically").click();
    h.run_steps(3);
    assert_eq!(h.state().edits, vec![WarpFieldsEdit::Split(Some(WarpSplit::Vertical))]);
    h.get_by_label("Split Warp Vertically").click();
    h.run_steps(3);
    assert_eq!(h.state().edits.last(), Some(&WarpFieldsEdit::Split(None)));
    assert_eq!(h.state().split, None);
}

// The Bend field is named by its own word, not by its caption with the colon cut off.
#[test]
fn the_bend_field_is_named_by_its_name_word() {
    let labels = WarpFieldsLabels { bend: "Courbure :".into(), bend_name: "Courbure".into(), ..WarpFieldsLabels::default() };
    let mut with_words = host("arc");
    with_words.labels = Some(labels);
    let h = harness(with_words);
    assert!(h.query_by_role_and_label(Role::SpinButton, "Courbure").is_some());
    assert!(h.query_by_role_and_label(Role::SpinButton, "Courbure :").is_none());
}
