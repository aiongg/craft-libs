//! The format option dialogs in a harness.

use egui::accesskit::Role;
use egui::{Key, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::dialog::DialogHost;
use crate::theme::{Base, install};

struct App<D: StackDialog> {
    host: DialogHost,
    answers: Vec<DialogResult<D::Result>>,
}

fn harness<D: StackDialog>(dialog: D) -> Harness<'static, App<D>> {
    let mut host = DialogHost::default();
    host.open(dialog);
    HarnessBuilder::default().with_size(vec2(640.0, 480.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App<D>| {
            app.host.show(ui.ctx());
            if let Some(answer) = app.host.take::<D>() {
                app.answers.push(answer);
            }
        },
        App { host, answers: Vec::new() },
    )
}

// JPEG Options: picking High in the preset list sets 8; typing 6 shows Medium; OK returns the
// quality and format.
#[test]
fn jpeg_presets_and_typed_quality_agree() {
    let mut harness = harness(JpegOptionsDialog::new(10));
    harness.run();
    harness.get_by_role_and_label(Role::ComboBox, "Preset").click();
    harness.run();
    harness.get_by_label("High").click();
    harness.run();
    assert_eq!(harness.get_by_role_and_label(Role::SpinButton, "Quality").accesskit_node().numeric_value(), Some(8.0));

    harness.get_by_role_and_label(Role::SpinButton, "Quality").focus();
    harness.run();
    harness.event(egui::Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND });
    harness.event(egui::Event::Text("6".into()));
    harness.run();
    harness.get_by_role_and_label(Role::RadioButton, "Progressive").click();
    harness.run();
    assert_eq!(harness.get_by_role_and_label(Role::ComboBox, "Preset").accesskit_node().value().as_deref(), Some("Medium"));
    harness.get_by_label("OK").click();
    harness.run();
    assert_eq!(harness.state().answers, [DialogResult::Ok(JpegOptions { quality: 6, format: JpegFormat::Progressive })]);
}

// Enter is OK unless Cancel has focus; Esc cancels (every option dialog).
#[test]
fn enter_confirms_unless_cancel_is_focused() {
    let mut harness = harness(TargaOptionsDialog::new(32));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().answers, [DialogResult::Ok(32)]);

    let mut harness = self::harness(TiffOptionsDialog::new());
    harness.run();
    harness.get_by_label("Cancel").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().answers, [DialogResult::Cancel]);

    let mut harness = self::harness(DdsOptionsDialog::new());
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().answers, [DialogResult::Cancel]);
}

// DDS Options opens at None with mipmaps off and returns the choices.
#[test]
fn dds_defaults_and_result() {
    let mut harness = harness(DdsOptionsDialog::new());
    harness.run();
    assert_eq!(harness.get_by_role_and_label(Role::RadioButton, "None").accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    assert_eq!(harness.get_by_label("Generate Mipmaps").accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
    harness.get_by_role_and_label(Role::RadioButton, "BC3 / DXT5").click();
    harness.get_by_label("Generate Mipmaps").click();
    harness.run();
    harness.get_by_label("OK").click();
    harness.run();
    assert_eq!(harness.state().answers, [DialogResult::Ok(DdsOptions { compression: DdsCompression::Bc3, mipmaps: true })]);
}
