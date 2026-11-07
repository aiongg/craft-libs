//! The per-shape cases: they run once, on one sample component, because only shared code can break them.

use super::{ANY, Case, Subject};
use crate::testing::fixture::Fixture;
use egui::{Pos2, Rect, Vec2};
use libtest_mimic::{Failed, Trial};

pub(super) const CASES: &[Case] = &[Case::new("paints-inside", ANY, small_rooms_paint_inside_the_shown_rect)];

/// The slack between a painted bound and the shown rect: the rounding of the layout, as the layout cases allow.
const ROUNDING: f32 = 0.5;

/// The rects a story may paint in: the area it allocated and the rect of every open layer. A layer that opened a
/// cover or is modal (a scrim, a full-window mount) covers the window.
fn painted_areas(fixture: &Fixture) -> Vec<Rect> {
    use crate::runtime::overlay;
    let ctx = fixture.ctx();
    let mut areas = vec![fixture.shown().expand(ROUNDING)];
    for entry in overlay::open_entries(ctx) {
        areas.push(entry.rect.expand(ROUNDING));
        if entry.cover.is_some() || entry.rank.is_modal() {
            areas.push(ctx.content_rect());
        }
    }
    areas
}

/// A story shown in a room of 1 by 1 and in a room of half its settled size does not panic, paints only inside the
/// rect it allocated (or the rect of a layer it opened) and registers finite part shapes.
fn small_rooms_paint_inside_the_shown_rect(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let settled = {
            let mut fixture = Fixture::story(story);
            fixture.settle();
            fixture.shown().size()
        };
        for room in [Vec2::splat(1.0), settled / 2.0] {
            let mut fixture = Fixture::story(story);
            fixture.room(room);
            fixture.settle();
            let areas = painted_areas(&fixture);
            if let Some(outside) = fixture.shape_bounds().iter().find(|bounds| !areas.iter().any(|area| area.contains_rect(**bounds))) {
                return Err(format!("{}: in a room of {room:?} a shape paints at {outside:?}, outside {:?}", story.name, fixture.shown()));
            }
            let tree = fixture.tree();
            if tree.keys().into_iter().filter_map(|key| tree.bounds_of(key)).any(|bounds| !(bounds.min.is_finite() && bounds.max.is_finite())) {
                return Err(format!("{}: in a room of {room:?} a part has non-finite bounds", story.name));
            }
        }
    }
    Ok(())
}

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
