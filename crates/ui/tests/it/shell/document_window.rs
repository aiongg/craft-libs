//! `DocumentWindow`: where a further window opens, and what the window reports of its OS window.

use crate::windows_kit::Windows;
use craft_ui::contract::{Props as _, Show as _, ShowContainer as _};
use craft_ui::shell::DocumentWindow;
use craft_ui::testing::layout::Block;
use craft_ui::testing::trial;
use craft_window::CASCADE_STEP;
use egui::{Pos2, Rect, Vec2, ViewportCommand, ViewportEvent, ViewportId, ViewportInfo};
use std::cell::Cell;

const BODY: Vec2 = Vec2::new(120.0, 80.0);
const WINDOW: Vec2 = Vec2::new(400.0, 300.0);
const MONITOR: Rect = Rect { min: Pos2::ZERO, max: Pos2::new(1000.0, 800.0) };
/// Near the monitor's bottom right, where one more cascade step leaves it.
const NEAR_CORNER: Rect = Rect { min: Pos2::new(560.0, 480.0), max: Pos2::new(960.0, 780.0) };
const SETTLED_FRAMES: usize = 3;

#[trial]
fn a_further_window_steps_down_and_across_and_wraps_at_the_monitors_edge() {
    let main = Rect::from_min_size(Pos2::new(40.0, 30.0), WINDOW);
    let next = DocumentWindow::cascade_in(main, WINDOW, &[MONITOR]);
    assert_eq!(next.min, main.min + Vec2::splat(CASCADE_STEP));
    assert_eq!(next.size(), WINDOW);

    let wrapped = DocumentWindow::cascade_in(NEAR_CORNER, WINDOW, &[MONITOR]);
    assert_eq!(wrapped.min, MONITOR.min, "past the edge the window starts again at the monitor's corner");

    let free = DocumentWindow::cascade_in(NEAR_CORNER, WINDOW, &[]);
    assert_eq!(free.min, NEAR_CORNER.min + Vec2::splat(CASCADE_STEP), "no monitor is known: the step is not bounded");
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
fn the_os_close_button_is_reported_once_and_the_window_focus_follows_the_os() {
    let windows = Windows::new();
    windows.child_info(reporting());
    let (requests, focused) = (Cell::new(0), Cell::new(false));
    let frame = || {
        windows.frame(vec![], vec![], |ui| {
            let shown = DocumentWindow::new("Landscape.psd").id_salt("document").show(ui, |ui, _| {
                Block::new(BODY).show(ui);
            });
            requests.set(requests.get() + usize::from(shown.output.close_requested));
            focused.set(shown.output.focused);
        });
    };
    for _ in 0..SETTLED_FRAMES {
        frame();
    }
    assert!(focused.get(), "the window reports the OS focus its viewport has");
    assert_eq!(requests.get(), 0);
    windows.child_info(ViewportInfo { events: vec![ViewportEvent::Close], ..reporting() });
    frame();
    windows.child_info(reporting());
    for _ in 0..SETTLED_FRAMES {
        frame();
    }
    assert_eq!(requests.get(), 1, "one request for one press of the close button");
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
    let raised = |out: &crate::windows_kit::Out| {
        out.commands.iter().any(|(viewport, command)| *viewport != ViewportId::ROOT && *command == ViewportCommand::Focus)
    };
    assert!(!raised(&frame(false)));
    assert!(raised(&frame(true)));
}
