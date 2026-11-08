//! A two-body split area: the bound share follows the divider's drag and keys.

use craft_behavior::Axis;
use craft_ui::contract::ShowContainer as _;
use craft_ui::contract::{Role, SizeLimits};
use craft_ui::docking::SplitArea;
use craft_ui::docking::tree::FrameRules;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Rect, vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const START: f32 = 0.5;

/// A pair of bodies stacked along Y whose share the host keeps, and the bodies' rects of the last pass.
fn pair() -> (Fixture, Rc<Cell<f32>>, Rc<RefCell<Vec<Rect>>>) {
    let share = Rc::new(Cell::new(START));
    let rects: Rc<RefCell<Vec<Rect>>> = Rc::default();
    let (kept, seen) = (Rc::clone(&share), Rc::clone(&rects));
    let fixture = Fixture::new(move |ui| {
        let limits = |_: &u8| SizeLimits::default();
        let area = SplitArea::pair("Bodies", Axis::Y, (1u8, 2u8), kept.get(), FrameRules::new(&limits)).show(ui, |_, body| {
            *seen.borrow_mut() = body.group_frames::<u8>().into_iter().map(|(_, rect)| rect).collect();
        });
        if let Some(next) = area.output.share().as_ref().and_then(|event| event.value()) {
            kept.set(*next);
        }
    });
    (fixture, share, rects)
}

#[trial]
fn the_share_follows_the_dragged_divider_and_the_arrow_keys() {
    let (mut fixture, share, rects) = pair();
    fixture.settle();
    assert_eq!(rects.borrow().len(), 2, "two bodies, one divider");
    let divider = fixture.by_role(Role::SPLITTER).into_iter().next().expect("one divider between the bodies");
    let before = rects.borrow()[0].height();
    fixture.drag(&divider, vec2(0.0, 40.0));
    fixture.settle();
    assert!(share.get() > START, "dragging down gives the first body more: {}", share.get());
    assert!(rects.borrow()[0].height() > before, "the first body grew with the share");

    let kept = share.get();
    let divider = fixture.by_role(Role::SPLITTER).into_iter().next().expect("the divider stays");
    fixture.click(&divider);
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    fixture.settle();
    assert!(share.get() < kept, "Up on the focused divider moves it up: {} -> {}", kept, share.get());
}
