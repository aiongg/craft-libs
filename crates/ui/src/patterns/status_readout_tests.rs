//! The status readout.

use std::time::Duration;

use egui::accesskit::Role;
use egui::{Context, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::edit::EditEvent;
use crate::localized::{Labels as _, LocaleNumbers};
use crate::theme::{Base, install};

#[derive(Default)]
struct App {
    zoom: f64,
    range: Option<std::ops::RangeInclusive<f64>>,
    item: StatusItem,
    out: StatusReadoutOutput,
    zoom_events: Vec<EditEvent<f64>>,
}

fn harness() -> Harness<'static, App> {
    let mut harness =
        Harness::builder().with_size([640.0, 80.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app| {
                StatusBar::new("bar").show(ui, |bar| {
                    bar.leading(|ui| {
                        let values = StatusValues { scratch_sizes: Some((1 << 30, 8 << 30)), ..StatusValues::default() };
                        let mut readout = StatusReadout::new("readout");
                        if let Some(range) = app.range.clone() {
                            readout = readout.range(range);
                        }
                        let out = readout.show(ui, app.zoom, app.item, &values);
                        if let Some(event) = out.zoom {
                            if let Some(&value) = event.value() {
                                app.zoom = value;
                            }
                            app.zoom_events.push(event);
                        }
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
    item.text(values, &StatusReadoutLabels::english(), &LocaleNumbers::from_tag("en-US"), &ctx)
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
// Typing 200 and Enter in the zoom field commits 200; Esc after typing cancels.
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
    assert!(
        matches!(harness.state().zoom_events.last(), Some(EditEvent::Cancel { .. })),
        "Esc cancels the preview: {:?}",
        harness.state().zoom_events
    );

    harness.get_by_label("Zoom").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    harness.get_by_label("Zoom").type_text("200");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(matches!(harness.state().zoom_events.last(), Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) if *value == 200.0));
}

#[test]
// A drag on the field's text zooms while it goes on: Begin and Update reach the host before the
// button is released, and the range limits them.
fn zoom_field_reports_a_scrub_live_within_the_range() {
    let mut harness = harness();
    harness.state_mut().range = Some(50.0..=150.0);
    harness.run();
    let rect = harness.get_by_label("Zoom").rect();
    let from = rect.center();
    harness.event(egui::Event::PointerMoved(from));
    harness.run();
    harness.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.run();
    for dx in [10.0, 40.0, 300.0] {
        harness.event(egui::Event::PointerMoved(from + egui::vec2(dx, 0.0)));
        harness.run();
    }
    let events = &harness.state().zoom_events;
    assert!(events.iter().any(|e| matches!(e, EditEvent::Update { .. } | EditEvent::Begin { value: Some(_), .. })), "live: {events:?}");
    assert!(events.iter().all(|e| !matches!(e, EditEvent::Commit { .. })), "not released yet: {events:?}");
    assert!(events.iter().filter_map(EditEvent::value).all(|v| (50.0..=150.0).contains(v)), "{events:?}");
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

#[test]
// A new StatusItem needs its ▸ menu name: every item has a non-empty English label.
fn every_item_has_an_english_label() {
    let labels = StatusReadoutLabels::english();
    for item in StatusItem::ALL {
        assert!(!item.label(&labels).trim().is_empty(), "{item:?}");
    }
}

#[test]
// The tooltip set for the shown item appears over its text.
fn the_shown_items_tooltip_appears_over_its_text() {
    let mut harness =
        Harness::builder().with_size([640.0, 80.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui(|ui| {
            StatusBar::new("bar").show(ui, |bar| {
                bar.leading(|ui| {
                    let values = StatusValues { scratch_sizes: Some((1 << 30, 8 << 30)), ..StatusValues::default() };
                    let readout = StatusReadout::new("readout")
                        .tooltip(StatusItem::ScratchSizes, "Memory used and available")
                        .tooltip(StatusItem::Efficiency, "Not shown");
                    readout.show(ui, 100.0, StatusItem::ScratchSizes, &values);
                });
            });
        });
    harness.run();
    assert!(harness.query_by_label_contains("Memory used").is_none(), "no tooltip before hovering");
    harness.get_by_label_contains("Scratch:").hover();
    for _ in 0..30 {
        harness.step();
    }
    harness.get_by_label_contains("Memory used and available");
    assert!(harness.query_by_label_contains("Not shown").is_none());
}
