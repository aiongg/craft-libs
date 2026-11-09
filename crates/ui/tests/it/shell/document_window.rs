//! `DocumentWindow`: where a further window opens, and what the window reports of its OS window.

use crate::windows_kit::Windows;
use craft_behavior::Monitor;
use craft_ui::contract::{Props as _, Show as _, ShowContainer as _};
use craft_ui::shell::DocumentWindow;
use craft_ui::testing::layout::Block;
use craft_ui::testing::trial;
use craft_window::{WindowId, WindowList};
use egui::{Pos2, Rect, Vec2, ViewportCommand, ViewportId, ViewportInfo};
use std::cell::RefCell;

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

#[trial]
fn raise_brings_the_window_forward_in_the_pass_it_is_set() {
    let windows = Windows::new();
    windows.child_info(reporting());
    let frame = |raise: bool| {
        windows.frame(vec![], vec![], |ui| {
            let window = DocumentWindow::new("Landscape.psd").id_salt("document");
            let window = if raise { window.raise() } else { window };
            window.show(ui, |ui, _| {
                Block::new(BODY).show(ui);
            });
        })
    };
    for _ in 0..SETTLED_FRAMES {
        frame(false);
    }
    let child_of = |out: &crate::windows_kit::Out| {
        out.titles.iter().map(|(viewport, _)| *viewport).find(|viewport| *viewport != ViewportId::ROOT).expect("the document window is open")
    };
    let quiet = frame(false);
    assert!(!quiet.commands.contains(&(child_of(&quiet), ViewportCommand::Focus)), "no focus command without a request");
    let raised = frame(true);
    assert!(raised.commands.contains(&(child_of(&raised), ViewportCommand::Focus)), "the request raised the window: {:?}", raised.commands);
}

#[trial]
fn cascade_in_places_the_next_window_on_the_right_monitor() {
    {
        let main = Rect::from_min_size(Pos2::new(40.0, 30.0), WINDOW);
        let monitors = [monitor(MONITOR)];
        let next = DocumentWindow::cascade_in(main, WINDOW, &monitors, STEP);
        assert_eq!(next.min, main.min + Vec2::splat(STEP));
        assert_eq!(next.size(), WINDOW);

        let wrapped = DocumentWindow::cascade_in(NEAR_CORNER, WINDOW, &monitors, STEP);
        assert_eq!(wrapped.min, MONITOR.min, "past the edge the window starts again at the monitor's corner");

        let free = DocumentWindow::cascade_in(NEAR_CORNER, WINDOW, &[], STEP);
        assert_eq!(free.min, NEAR_CORNER.min + Vec2::splat(STEP), "no monitor is known: the step is not bounded");
    }
    {
        let right = Rect::from_min_size(Pos2::new(1000.0, 0.0), Vec2::new(1000.0, 800.0));
        let monitors = [monitor(MONITOR), monitor(right)];
        // Maximized on the right monitor, overhanging it by a few points on every side.
        let maximized = right.expand(8.0);
        let next = DocumentWindow::cascade_in(maximized, WINDOW, &monitors, STEP);
        assert!(right.contains_rect(next), "the next window opens on the right monitor: {next:?}");
    }
}
