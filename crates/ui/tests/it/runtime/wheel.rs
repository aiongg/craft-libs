//! The wheel service: the begin hook splits the pass's wheel input among last pass's takers.

use craft_behavior::{Sense, WheelTaker};
use craft_ui::testing::pointer;
use craft_ui::testing::sensing::{ProbeRoot, show};
use craft_ui::testing::wheel::{focus, register, take};
use craft_ui::testing::{Frames, trial};
use egui::{Id, Pos2, Rect, Vec2};

/// The scroll the panel and the slider read in the frame that carried the wheel event.
fn wheel_over(gated: bool, focused: bool) -> (Option<Vec2>, Option<Vec2>) {
    let panel = Id::new("wheel_panel");
    let slider = ProbeRoot::id("slider");
    let panel_rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.0));
    let slider_rect = Rect::from_min_size(Pos2::new(50.0, 50.0), Vec2::new(100.0, 20.0));
    let probe = ProbeRoot::new("slider", slider_rect, Sense::FOCUS | Sense::DRAG);
    let mut read = (None, None);
    let mut show_frame = |ui: &mut egui::Ui| {
        show(ui, std::slice::from_ref(&probe));
        register(ui.ctx(), WheelTaker::new(panel, panel_rect).vertical(true));
        register(ui.ctx(), WheelTaker::new(slider, slider_rect).vertical(true).gated(gated));
        read = (take(ui.ctx(), panel), take(ui.ctx(), slider));
    };
    let mut frames = Frames::new();
    frames.idle(2, &mut show_frame);
    frames.frame(vec![pointer::moved(Pos2::new(70.0, 60.0))], &mut show_frame);
    if focused {
        focus(&frames.ctx, slider);
        frames.idle(1, &mut show_frame);
    }
    frames.frame(vec![pointer::wheel(-30.0)], &mut show_frame);
    read
}

/// Skeleton: `frames_state::slider_takes_the_wheel_after_a_press_until_the_pointer_leaves`, first half: a value
/// control takes the wheel only while focused; otherwise the enclosing scroller does.
#[trial]
fn a_gated_taker_passes_the_wheel_on_until_it_is_focused() {
    let scroll = Some(Vec2::new(0.0, -30.0));
    assert_eq!(wheel_over(true, false), (scroll, None), "the panel scrolls");
    assert_eq!(wheel_over(true, true), (None, scroll), "the focused slider takes the wheel");
    assert_eq!(wheel_over(false, false), (None, scroll), "an ungated taker is the innermost one");
}
