//! The status readout.

use std::time::Duration;

use egui::accesskit::Role;
use egui::{Context, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::localized::{Labels as _, LocaleNumbers, Words};
use crate::theme::{Base, install};

#[derive(Default)]
struct App {
    zoom: f32,
    item: StatusItem,
    out: StatusReadoutOutput,
}

fn harness() -> Harness<'static, App> {
    let mut harness =
        Harness::builder().with_size([640.0, 80.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app| {
                StatusBar::new("bar").show(ui, |bar| {
                    bar.leading(|ui| {
                        let values = StatusValues { scratch_sizes: Some((1 << 30, 8 << 30)), ..StatusValues::default() };
                        let out = StatusReadout::new("readout").show(ui, app.zoom, app.item, &values);
                        app.out.zoom = app.out.zoom.or(out.zoom);
                        app.out.item = app.out.item.or(out.item);
                    });
                });
            },
            App { zoom: 100.0, ..App::default() },
        );
    harness.run();
    harness
}

fn text(item: StatusItem, values: &StatusValues) -> String {
    let ctx = Context::default();
    item.text(values, &StatusReadoutLabels::english(), &LocaleNumbers::from_tag("en-US"), &Words::of_ctx(&ctx))
}

#[test]
// The ▸ menu lists every StatusItem as radio rows; choosing Scratch Sizes returns item Some(ScratchSizes).
fn readout_menu_chooses_the_item() {
    let mut harness = harness();
    harness.get_by_label("Show").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.get_all_by_role(Role::MenuItemRadio).count(), StatusItem::ALL.len());
    // Opened from the keyboard, the first row is lit; Scratch Sizes is the fifth.
    for _ in 0..4 {
        harness.key_press(Key::ArrowDown);
        harness.run();
    }
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().out.item, Some(StatusItem::ScratchSizes));
}

#[test]
// Typing 200 and Enter in the zoom field returns zoom Some(200.0); Esc reverts.
fn zoom_field_commits_a_typed_percentage() {
    let mut harness = harness();
    let field = harness.get_by_label("Zoom");
    field.focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    harness.get_by_label("Zoom").type_text("300");
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().out.zoom, None, "Esc reverts");

    harness.get_by_label("Zoom").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    harness.get_by_label("Zoom").type_text("200");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().out.zoom, Some(200.0));
}

#[test]
// DocumentSizes reads Doc: 24.9M/24.9M from bytes; a missing value reads —.
fn item_text_formats_sizes_and_missing_values() {
    let mib = 1024 * 1024;
    let values = StatusValues {
        document_sizes: Some((26_109_542, 26_109_542)),
        scratch_sizes: Some((1_288_490_189, 8 << 30)),
        efficiency: Some(100),
        timing: Some(Duration::from_millis(420)),
        layer_count: Some(1),
        ..StatusValues::default()
    };
    assert_eq!(text(StatusItem::DocumentSizes, &values), "Doc: 24.9M/24.9M");
    assert_eq!(
        text(StatusItem::DocumentSizes, &StatusValues { document_sizes: Some((412 * 1024 + 512, 3 * mib)), ..StatusValues::default() }),
        "Doc: 412.5K/3.0M"
    );
    assert_eq!(text(StatusItem::ScratchSizes, &values), "Scratch: 1.2G/8.0G");
    assert_eq!(text(StatusItem::Efficiency, &values), "Efficiency: 100%");
    assert_eq!(text(StatusItem::Timing, &values), "Timing: 0.4 s");
    assert_eq!(text(StatusItem::LayerCount, &values), "1 Layer");
    assert_eq!(text(StatusItem::DocumentProfile, &values), "—");
    assert_eq!(text(StatusItem::DocumentSizes, &StatusValues::default()), "—");
}
