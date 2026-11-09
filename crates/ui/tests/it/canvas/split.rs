//! `SplitViews`: the pane rects, the linked panes and the activation of another pane.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_ui::canvas::{Canvas, SplitAt, SplitLayout, SplitViews};
use craft_ui::contract::{Bind, ShowContainer as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const AREA: Vec2 = Vec2::new(400.0, 300.0);

fn full() -> Rect {
    Rect::from_min_size(Pos2::ZERO, AREA)
}

#[trial]
fn two_views_put_the_main_view_on_the_right_and_four_put_it_at_the_bottom_right() {
    let at = SplitAt::new(0.25, 0.5);
    let two = SplitLayout::Two.rects(at, full());
    assert_eq!((two[0].min.x, two[0].max, two[1].min, two[1].max.x), (100.0, full().max, full().min, 100.0));
    let four = SplitLayout::Four.rects(at, full());
    assert_eq!(four.len(), 4);
    assert_eq!((four[0].min, four[0].max), (Pos2::new(100.0, 150.0), full().max), "the main view is at the bottom right");
    assert_eq!(four[1].max, Pos2::new(100.0, 150.0));
    assert_eq!(SplitLayout::One.rects(at, full()), vec![full()]);
}

#[trial]
fn the_draw_order_moves_the_first_pane_forward_and_keeps_the_others_in_reading_order() {
    assert_eq!(SplitLayout::Four.reading_order(), vec![1, 2, 3, 0]);
    assert_eq!(SplitLayout::Four.draw_order(3), vec![3, 1, 2, 0]);
    assert_eq!(SplitLayout::Two.draw_order(0), vec![0, 1]);
}

#[trial]
fn a_click_on_an_inactive_view_activates_it_and_its_canvas_gets_no_press() {
    let seen: Rc<RefCell<(Option<usize>, bool)>> = Rc::default();
    let shared = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        let mut at = SplitAt::default();
        let views = SplitViews::new("Views", SplitLayout::Two, Bind::bound(&mut at)).show(ui, |ui, body| {
            for (pane, rect) in SplitLayout::Two.rects(SplitAt::default(), body.rect).into_iter().enumerate() {
                let view = CanvasView::new(rect, Point::new(0.0, 0.0), 1.0);
                let canvas = Canvas::new(if pane == 0 { "Main view" } else { "Other view" }).view(view).show_at(ui, rect, |_, _| ());
                if pane == 1 && canvas.output.press.phase.is_press() {
                    shared.borrow_mut().1 = true;
                }
            }
        });
        if let Some(pane) = views.output.activated {
            shared.borrow_mut().0 = Some(pane);
        }
    });
    fixture.settle();
    let left = fixture.by_label("View 1").expect("the left view is the first in reading order");
    fixture.click(&left);
    assert_eq!(*seen.borrow(), (Some(1), false));
}
