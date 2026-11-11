//! Trials of a slider shown at rects the layout should never hand out.

use super::Slider;
use crate::contract::{Bind, Props as _, Show as _};
use crate::testing::{Fixture, pointer, trial};
use craft_behavior::MotionMode;
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// A leaf with parts shown at a rect that is empty, not a number, infinite, over the extent cap, or one point wide
/// does not panic and answers presses and keys with nothing: its value stays as it was. It runs on a slider, because
/// only shared code can break it.
#[trial]
fn a_leaf_at_a_degenerate_rect_neither_panics_nor_changes_its_value() {
    let nan = f32::NAN;
    let rects = [
        Rect::ZERO,
        Rect::from_min_max(Pos2::new(nan, nan), Pos2::new(nan, nan)),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(f32::INFINITY, f32::INFINITY)),
        Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0e9)),
        Rect::from_min_size(Pos2::new(5.0, 5.0), Vec2::splat(1.0)),
    ];
    for (index, rect) in rects.into_iter().enumerate() {
        let inert = index < 4;
        let value = Rc::new(RefCell::new(40.0_f64));
        let shown = value.clone();
        let mut fixture = Fixture::new(move |ui| {
            let mut value = shown.borrow_mut();
            Slider::new(Bind::bound(&mut *value), 0.0..=100.0).label("Opacity").show_at(ui, rect);
        });
        fixture.motion(MotionMode::Off).settle();
        if !inert {
            continue;
        }
        for at in [Pos2::ZERO, Pos2::new(5.5, 5.5), Pos2::new(0.5, 0.5)] {
            fixture.frame(vec![pointer::moved(at)]);
            fixture.frame(vec![pointer::press(at)]);
            fixture.frame(vec![pointer::release(at)]);
        }
        fixture.key(Key::Tab, Modifiers::NONE);
        fixture.key(Key::ArrowRight, Modifiers::NONE);
        fixture.settle();
        assert!((*value.borrow() - 40.0).abs() <= f64::EPSILON, "a slider shown at {rect:?} changed its value to {}", value.borrow());
    }
}
