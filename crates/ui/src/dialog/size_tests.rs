//! Width classes, resizable sizes, remembered sizes and the scrolling body's height.

use craft_theme::{Appearance, Contrast, DialogTokens, Mode, Size, TextSize, builtins, resolve};
use egui::vec2;

use super::*;

fn tokens(size: Size) -> DialogTokens {
    let appearance = Appearance { mode: Mode::Light, contrast: Contrast::Standard, size, text_size: TextSize::Default, reduced_motion: false };
    resolve(&builtins()[0], appearance, None).tokens.component.dialog
}

#[test]
fn width_classes_and_the_window_cap() {
    let t = tokens(Size::Compact);
    assert_eq!(Width::Compact.range(&t, 1280.0), (0.0, 260.0));
    assert_eq!(Width::Standard.range(&t, 1280.0), (380.0, 440.0));
    assert_eq!(Width::Wide.range(&t, 1280.0), (380.0, 600.0));
    assert_eq!(Width::Fixed(800.0).range(&t, 1280.0), (800.0, 800.0));
    // A narrow window caps the greatest width at the window less 24 on each side, and the least
    // with it.
    assert_eq!(Width::Standard.range(&t, 400.0), (352.0, 352.0));
    assert_eq!(Width::Wide.range(&t, 500.0), (380.0, 452.0));
    assert_eq!(Width::Compact.range(&t, 200.0), (0.0, 152.0));
    assert_eq!(Width::Fixed(800.0).range(&t, 600.0), (552.0, 552.0));
    assert_eq!(Width::Standard.range(&t, 30.0), (0.0, 0.0), "a window narrower than its margins leaves no width");
}

#[test]
fn widths_do_not_follow_the_size_but_vertical_gaps_do() {
    let (compact, comfortable) = (tokens(Size::Compact), tokens(Size::Comfortable));
    for width in [Width::Compact, Width::Standard, Width::Wide] {
        assert_eq!(width.range(&compact, 1280.0), width.range(&comfortable, 1280.0));
    }
    assert_eq!((compact.pad, compact.button_gap), (comfortable.pad, comfortable.button_gap));
    assert!(comfortable.body_gap > compact.body_gap && comfortable.footer_gap > compact.footer_gap);
}

#[test]
fn resizable_maximum_is_the_window_less_the_margins_and_never_below_the_minimum() {
    let t = tokens(Size::Compact);
    let resize = Resize { default: vec2(400.0, 300.0), min: vec2(300.0, 200.0), max: ResizeMax::Window };
    assert_eq!(resize.max_size(&t, vec2(1000.0, 800.0)), vec2(960.0, 760.0));
    assert_eq!(resize.max_size(&t, vec2(320.0, 220.0)), vec2(300.0, 200.0));
    let fixed = Resize { max: ResizeMax::Fixed(vec2(500.0, 900.0)), ..resize };
    assert_eq!(fixed.max_size(&t, vec2(1000.0, 800.0)), vec2(500.0, 760.0));
}

#[test]
fn remembered_size_keeps_only_a_users_size() {
    let (opened, max) = (vec2(400.0, 300.0), vec2(960.0, 760.0));
    assert_eq!(remembered_size(opened, opened, max), None, "unchanged");
    assert_eq!(remembered_size(vec2(960.0, 500.0), opened, max), None, "at the window's width");
    assert_eq!(remembered_size(vec2(500.0, 760.0), opened, max), None, "at the window's height");
    assert_eq!(remembered_size(vec2(500.0, 400.0), opened, max), Some(vec2(500.0, 400.0)));
    // A default larger than a small window opens at the window's size, which is not the user's.
    assert_eq!(remembered_size(vec2(300.0, 200.0), vec2(400.0, 300.0), vec2(300.0, 200.0)), None);
}

#[test]
fn scroll_height_leaves_room_for_the_chrome_and_reserve_but_not_below_the_minimum() {
    let t = tokens(Size::Compact);
    assert_eq!(scroll_height(&t, 800.0, 150.0, 0.0), 800.0 - 48.0 - 150.0);
    assert_eq!(scroll_height(&t, 800.0, 150.0, 200.0), 800.0 - 48.0 - 150.0 - 200.0);
    assert_eq!(scroll_height(&t, 300.0, 150.0, 200.0), 120.0);
}
