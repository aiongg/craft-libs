//! The Brush Preset picker's data: the tip scale, the parts and the persisted view.

use craft_draw::ui::brush_presets::{BrushCardPart, BrushCardParts, BrushPick, BrushPickerSettings, BrushPickerState, tip_thumb_scale};
use craft_ui::testing::trial;

/// Thumbnails grow with the preset size, never exceed the cell and stay finite for a degenerate size.
#[trial]
fn tip_thumbnails_scale_with_the_preset_size() {
    let (small, middle, large) = (tip_thumb_scale(1.0), tip_thumb_scale(20.0), tip_thumb_scale(400.0));
    assert!(small < middle && middle <= large);
    assert!(large <= 1.0);
    assert!(tip_thumb_scale(f64::NAN).is_finite());
}

/// The gear's part boxes keep one part on.
#[trial]
fn the_last_part_stays_on() {
    let tip_only = BrushCardParts::ALL.set(BrushCardPart::Name, false).set(BrushCardPart::Stroke, false);
    assert!(tip_only.tip_only());
    assert_eq!(tip_only.set(BrushCardPart::Tip, false), tip_only);
    assert_eq!(BrushCardParts::ALL.set(BrushCardPart::Tip, false).tip, false);
}

/// Loaded settings are made safe: the scale stays in its range and a view with no part gets every part.
#[trial]
fn restored_settings_clamp_the_scale_and_keep_a_part() {
    let mut none = BrushCardParts::ALL;
    none.name = false;
    none.stroke = false;
    none.tip = false;
    let restored = BrushPickerSettings::new(none, 9.0).restored();
    assert_eq!(restored.parts, BrushCardParts::ALL);
    assert_eq!(restored.scale, 2.0);
    assert_eq!(BrushPickerSettings::new(BrushCardParts::ALL, f32::NAN).restored().scale, 1.0);
    let mut state = BrushPickerState::<u32>::new();
    state.restore(&BrushPickerSettings::new(BrushCardParts::ALL.set(BrushCardPart::Stroke, false), 0.5));
    assert_eq!(state.settings(), BrushPickerSettings::new(BrushCardParts::ALL.set(BrushCardPart::Stroke, false), 0.5));
}

/// A pick that opens something else closes the picker; a plain click does not.
#[trial]
fn picks_that_open_something_else_close_the_picker() {
    assert!(BrushPick::<u32>::OpenSettings.closes());
    assert!(!BrushPick::Select(1u32).closes());
}
