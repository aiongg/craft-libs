//! Screen tests of Layers Panel Options (L2).

#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::list::ThumbSize;
use craft_ui::testing::HarnessExt as _;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
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
    let hit = harness.get_by_role_and_label(Role::RadioButton, "Small").rect();
    harness.click_at(egui::pos2(hit.right() - 3.0, hit.center().y));
    harness.run();
    harness.get_by_role_and_label(Role::Button, "OK").click();
    harness.run();
    let Some(DialogResult::Ok(options)) = harness.state().answers.last().cloned() else { panic!("OK answers") };
    assert_eq!(options.thumbnail_size, ThumbSize::Small, "a click on a sample picks its size");
}

// A size's sample is part of its radio: four radios, no node of its own, the description worded with
// the side in points, and the dialog's id is `layers-options`.
#[test]
fn samples_are_not_separate_widgets() {
    let harness = open(LayersOptions::default());
    let radios = harness
        .query_all_by_role(Role::RadioButton)
        .filter(|n| n.accesskit_node().label().is_some_and(|l| ["None", "Small", "Medium", "Large"].contains(&l.as_str())))
        .count();
    assert_eq!(radios, 4);
    let buttons: Vec<_> = harness.query_all_by_role(Role::Button).filter_map(|n| n.accesskit_node().label()).collect();
    assert!(buttons.iter().all(|label| !label.contains("point") && !label.contains("Small")), "{buttons:?}");
    let medium = harness.get_by_role_and_label(Role::RadioButton, "Medium");
    let description = medium.accesskit_node().data().description().map(str::to_owned).expect("a description");
    assert!(description.ends_with("point thumbnails") && !description.contains('{'), "{description}");
    assert_eq!(LayersOptionsDialog::new().id(), dialog_id("layers-options"));
}
