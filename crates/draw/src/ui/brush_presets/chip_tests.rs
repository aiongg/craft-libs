//! Tests of the options-bar brush chip (task BR-BP3).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Context, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::BrushPresetChip;

fn harness() -> Harness<'static, u32> {
    let mut harness =
        Harness::builder().with_size([200.0, 80.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, clicks: &mut u32| {
                let chip = BrushPresetChip::new("chip").diameter(1234.4).show(ui, |_, _| {});
                *clicks += u32::from(chip.clicked);
            },
            0,
        );
    harness.run();
    harness
}

// A click, Space and Enter each report `clicked`; the chip is a Button named "Brush Preset picker"
// showing the rounded size (four digits fit inside 44 pt).
#[test]
fn chip_click_and_keys_report_clicked() {
    let mut harness = harness();
    let chip = harness.get_by_role_and_label(Role::Button, "Brush Preset picker");
    let rect = chip.rect();
    assert!(rect.width() >= 44.0 && rect.height() >= 30.0, "the chip is 44 x 30: {rect:?}");

    harness.get_by_role_and_label(Role::Button, "Brush Preset picker").click();
    harness.run();
    assert_eq!(*harness.state(), 1, "a click");

    for (n, key) in [(2, Key::Space), (3, Key::Enter)] {
        harness.key_press(Key::Tab);
        harness.run();
        harness.key_press(key);
        harness.run();
        assert_eq!(*harness.state(), n, "{key:?} reports clicked");
    }
}
