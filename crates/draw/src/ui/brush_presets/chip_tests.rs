//! Tests of the options-bar brush chip (task BR-BP3).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click, Space and Enter each report `clicked`; the chip is a Button named "Brush Preset picker"
// showing the rounded size (four digits fit inside 44 pt).
#[test]
#[ignore = "skeleton"]
fn chip_click_and_keys_report_clicked() {
    todo!()
}
