//! The per-shape cases: they run once, on one sample component, because only shared code can break them.

use crate::testing::fixture::Fixture;
use egui::{Pos2, Rect, Vec2};
use libtest_mimic::{Failed, Trial};

/// The trials that run once, beside the per-type trials of `craft-ui`'s own package.
pub(super) fn trials(package: &'static str) -> Vec<Trial> {
    if package != "craft-ui" {
        return Vec::new();
    }
    vec![Trial::test(format!("suite::{package}::leaf-degenerate-rects"), leaf_degenerate_rects)]
}

/// A leaf with parts shown at a rect that is empty, not a number, infinite, over the extent cap, or one point wide
/// does not panic and answers presses and keys with nothing: its value stays as it was. This case runs once, on a
/// slider, because only shared code can break it.
fn leaf_degenerate_rects() -> Result<(), Failed> {
    use crate::contract::{Bind, Props as _, Show as _};
    use crate::controls::slider::Slider;
    use std::cell::RefCell;
    use std::rc::Rc;
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
        fixture.idle(3);
        if !inert {
            continue;
        }
        for at in [Pos2::ZERO, Pos2::new(5.5, 5.5), Pos2::new(0.5, 0.5)] {
            fixture.frame(vec![crate::testing::pointer::moved(at)]);
            fixture.frame(vec![crate::testing::pointer::press(at)]);
            fixture.frame(vec![crate::testing::pointer::release(at)]);
        }
        fixture.key(egui::Key::Tab, egui::Modifiers::NONE);
        fixture.key(egui::Key::ArrowRight, egui::Modifiers::NONE);
        fixture.idle(2);
        if (*value.borrow() - 40.0).abs() > f64::EPSILON {
            return Err(Failed::from(format!("a slider shown at {rect:?} changed its value to {}", value.borrow())));
        }
    }
    Ok(())
}
