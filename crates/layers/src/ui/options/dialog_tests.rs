//! Screen tests of Layers Panel Options (L2).

#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::list::ThumbSize;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::*;

#[derive(Default)]
struct App {
    host: DialogHost,
    answers: Vec<DialogResult<LayersOptions>>,
}

fn open(initial: LayersOptions) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(LayersOptionsDialog::new().initial(initial));
    let mut harness =
        HarnessBuilder::default().with_size(vec2(1000.0, 800.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(answer) = app.host.take::<LayersOptionsDialog>() {
                    app.answers.push(answer);
                }
            },
            App { host, answers: Vec::new() },
        );
    harness.run();
    harness
}

// OK returns the edited `LayersOptions`; Esc and Cancel return `Cancel` and change nothing; a click
// on a size's sample picks that size.
#[test]
fn options_dialog_ok_returns_settings_and_esc_cancels() {
    let mut harness = open(LayersOptions::default());
    harness.get_by_label("Layer Bounds").click();
    harness.get_by_label("Show layer mask badges").click();
    harness.get_by_label("Expand New Effects").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "OK").click();
    harness.run();
    let Some(DialogResult::Ok(options)) = harness.state().answers.last().cloned() else { panic!("OK answers") };
    assert_eq!(options.thumbnail_contents, ThumbContents::LayerBounds);
    assert!(options.show_layer_mask_badges);
    assert!(!options.expand_new_effects);
    assert_eq!(options.thumbnail_size, ThumbSize::Medium);

    let mut harness = open(LayersOptions::default());
    harness.get_by_label("Show layer mask badges").click();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(matches!(harness.state().answers.last(), Some(DialogResult::Cancel)), "Esc cancels");

    let mut harness = open(LayersOptions::default());
    harness.get_by_role_and_label(Role::Button, "Cancel").click();
    harness.run();
    assert!(matches!(harness.state().answers.last(), Some(DialogResult::Cancel)), "Cancel cancels");

    let mut harness = open(LayersOptions::default());
    harness.get_by_label_contains("Small:").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "OK").click();
    harness.run();
    let Some(DialogResult::Ok(options)) = harness.state().answers.last().cloned() else { panic!("OK answers") };
    assert_eq!(options.thumbnail_size, ThumbSize::Small, "a click on a sample picks its size");
}
