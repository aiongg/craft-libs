//! `ViewportWindow`: the close contract it shares with `DocumentWindow`.

use crate::windows_kit::Windows;
use craft_ui::contract::Show as _;
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::shell::ViewportWindow;
use craft_ui::testing::layout::Block;
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Vec2, ViewportEvent, ViewportInfo};
use std::cell::{Cell, RefCell};

const BODY: Vec2 = Vec2::new(120.0, 80.0);
const WINDOW: Vec2 = Vec2::new(400.0, 300.0);
const SETTLED_FRAMES: usize = 3;

fn reporting() -> ViewportInfo {
    ViewportInfo {
        focused: Some(true),
        inner_rect: Some(Rect::from_min_size(Pos2::new(100.0, 100.0), WINDOW)),
        native_pixels_per_point: Some(1.0),
        ..ViewportInfo::default()
    }
}

#[trial]
fn an_os_close_is_requested_and_the_window_stays_while_the_app_shows_it() {
    let windows = Windows::new();
    windows.child_info(reporting());
    let (reasons, body_runs) = (RefCell::new(Vec::new()), Cell::new(0));
    let frame = || {
        windows.frame(vec![], vec![], |ui| {
            let shown = ViewportWindow::new("Export Log").show(ui, |ui, _| {
                body_runs.set(body_runs.get() + 1);
                Block::new(BODY).show(ui);
            });
            reasons.borrow_mut().extend(shown.close_requested);
        })
    };
    for _ in 0..SETTLED_FRAMES {
        frame();
    }
    assert!(reasons.borrow().is_empty());
    windows.child_info(ViewportInfo { events: vec![ViewportEvent::Close], ..reporting() });
    frame();
    windows.child_info(reporting());
    let after_request = body_runs.get();
    for _ in 0..SETTLED_FRAMES {
        frame();
    }
    assert_eq!(*reasons.borrow(), [CloseReason::Requested], "one request for one press of the close button");
    assert!(body_runs.get() > after_request, "the window stays and its body runs while the app shows it");
}
