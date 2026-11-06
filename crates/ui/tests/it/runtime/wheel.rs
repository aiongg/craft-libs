//! The wheel service: the begin hook splits the pass's wheel input among last pass's takers.

use craft_behavior::WheelTaker;
use craft_ui::testing::pointer;
use craft_ui::testing::wheel::{focus as focus_widget, register, take};
use craft_ui::testing::{Frames, trial};
use egui::{Id, Pos2, Rect, Vec2};

/// The scroll each taker read in the frame that carried the wheel event.
fn wheel_over(inner_gated: bool, focused: bool) -> (Option<Vec2>, Option<Vec2>) {
    let panel = Id::new("wheel_panel");
    let slider = Id::new("wheel_slider");
    let panel_rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.0));
    let slider_rect = Rect::from_min_size(Pos2::new(50.0, 50.0), Vec2::new(100.0, 20.0));
    let mut read = (None, None);
    let mut show_frame = |ui: &mut egui::Ui| {
        register(ui.ctx(), WheelTaker::new(panel, panel_rect).vertical(true));
        register(ui.ctx(), WheelTaker::new(slider, slider_rect).vertical(true).gated(inner_gated));
        read = (take(ui.ctx(), panel), take(ui.ctx(), slider));
    };
    let mut frames = Frames::new();
    frames.idle(2, &mut show_frame);
    frames.frame(vec![pointer::moved(Pos2::new(70.0, 60.0))], &mut show_frame);
    if focused {
        focus_widget(&frames.ctx, slider);
        frames.idle(1, &mut show_frame);
    }
    frames.frame(vec![pointer::wheel(-30.0)], &mut show_frame);
    read
}

/// Skeleton: `frames_state::slider_takes_the_wheel_after_a_press_until_the_pointer_leaves`, first half: a value
/// control takes the wheel only while focused or armed; otherwise the enclosing scroller does.
#[trial]
fn a_gated_taker_passes_the_wheel_on_until_it_is_focused() {
    let (panel, slider) = wheel_over(true, false);
    assert_eq!((panel, slider), (Some(Vec2::new(0.0, -30.0)), None), "the panel scrolls");
    let (panel, slider) = wheel_over(true, true);
    assert_eq!((panel, slider), (None, Some(Vec2::new(0.0, -30.0))), "the focused slider takes the wheel");
    let (panel, slider) = wheel_over(false, false);
    assert_eq!((panel, slider), (None, Some(Vec2::new(0.0, -30.0))), "an ungated taker is the innermost one");
}
