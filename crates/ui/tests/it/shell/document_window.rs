//! `DocumentWindow`: where a further window opens, and what the window reports of its OS window.

use crate::windows_kit::Windows;
use craft_behavior::Monitor;
use craft_ui::contract::{Props as _, Show as _, ShowContainer as _};
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::shell::DocumentWindow;
use craft_ui::testing::layout::Block;
use craft_ui::testing::trial;
use craft_window::{WindowId, WindowList};
use egui::{Pos2, Rect, Vec2, ViewportCommand, ViewportEvent, ViewportId, ViewportInfo};
use std::cell::{Cell, RefCell};

const BODY: Vec2 = Vec2::new(120.0, 80.0);
const WINDOW: Vec2 = Vec2::new(400.0, 300.0);
const MONITOR: Rect = Rect { min: Pos2::ZERO, max: Pos2::new(1000.0, 800.0) };
/// The offset between cascaded windows the trials use.
const STEP: f32 = 28.0;
/// Near the monitor's bottom right, where one more cascade step leaves it on both axes.
const NEAR_CORNER: Rect = Rect { min: Pos2::new(600.0, 480.0), max: Pos2::new(1000.0, 780.0) };
const SETTLED_FRAMES: usize = 3;

fn monitor(rect: Rect) -> Monitor {
    Monitor::new(None, rect, 1.0)
}

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
    let (reasons, focused, body_runs) = (RefCell::new(Vec::new()), Cell::new(false), Cell::new(0));
    let frame = || {
        windows.frame(vec![], vec![], |ui| {
            let shown = DocumentWindow::new("Landscape.psd").id_salt("document").show(ui, |ui, _| {
                body_runs.set(body_runs.get() + 1);
                Block::new(BODY).show(ui);
            });
            reasons.borrow_mut().extend(shown.output.close_requested);
            focused.set(shown.output.focused);
        })
    };
    for _ in 0..SETTLED_FRAMES {
        frame();
    }
    assert!(focused.get(), "the window reports the OS focus its viewport has");
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

#[trial]
fn gaining_focus_makes_the_window_the_active_one_of_its_list_once() {
    let windows = Windows::new();
    windows.child_info(reporting());
    let mut list = WindowList::new();
    let (first, second) = (list.open("Landscape.psd"), list.open("Portrait.psd"));
    assert_eq!(list.active(), Some(second));
    let list = RefCell::new(list);
    let frames = |id: WindowId, title: &'static str, key: &'static str| {
        for _ in 0..SETTLED_FRAMES {
            windows.frame(vec![], vec![], |ui| {
                DocumentWindow::new(title).id_salt(key).in_list(&mut *list.borrow_mut(), id).show(ui, |ui, _| {
                    Block::new(BODY).show(ui);
                });
            });
        }
    };
    frames(first, "Landscape.psd", "first");
    assert_eq!(list.borrow().active(), Some(first), "the window that gained focus is the active one");
    frames(second, "Portrait.psd", "second");
    assert_eq!(list.borrow().active(), Some(second), "the check mark moves to the window focused next");
    list.borrow_mut().focus(first);
    frames(second, "Portrait.psd", "second");
    assert_eq!(list.borrow().active(), Some(first), "only the edge activates: a window that stays focused does not take the mark back");
}

