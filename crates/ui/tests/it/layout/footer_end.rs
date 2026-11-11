//! A footer bar spans the footer region of a container sized from its content, so its buttons sit at the end edge
//! the body reaches, mirrored under RTL.

use craft_ui::buttons::Button;
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::layout_components::Stack;
use craft_ui::runtime::env::Direction;
use craft_ui::sections::FooterBar;
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, Vec2};

// Edges snap to whole pixels, so two edges that meet may differ by half a point.
const EDGE_SLACK: f32 = 0.5;
const WIDE: &str = "A body button whose label is much wider than the footer's";

/// The wide body button and the footer's OK button of a content-sized stack.
fn shown(direction: Direction) -> (Rect, Rect) {
    let mut fixture = Fixture::new(|ui| {
        Stack::new()
            .footer(|ui, _| {
                FooterBar::body().show(ui, |ui, _| {
                    let _ = Button::new("OK").show(ui);
                });
            })
            .show(ui, |ui, _| {
                let _ = Button::new(WIDE).show(ui);
            });
    });
    fixture.room(Vec2::new(800.0, 300.0)).direction(direction).settle();
    let body = fixture.by_label(WIDE).expect("the body shows its button").rect;
    let ok = fixture.by_label("OK").expect("the footer shows its button").rect;
    (body, ok)
}

#[trial]
fn a_footer_bar_puts_its_buttons_at_the_end_edge_of_a_content_sized_container() {
    let (body, ok) = shown(Direction::Ltr);
    assert!((ok.right() - body.right()).abs() <= EDGE_SLACK, "OK ends where the body does: {ok:?} under {body:?}");
    assert!(ok.left() > body.left() + EDGE_SLACK, "OK does not start at the start edge: {ok:?} under {body:?}");
    let (body, ok) = shown(Direction::Rtl);
    assert!((ok.left() - body.left()).abs() <= EDGE_SLACK, "under RTL OK ends at the left edge: {ok:?} under {body:?}");
    assert!(ok.right() < body.right() - EDGE_SLACK, "under RTL OK does not start at the right edge: {ok:?} under {body:?}");
}
