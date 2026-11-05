//! The Insert Table grid.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::size_picker::*;

#[derive(Default)]
struct App {
    last: TableSizeResponse,
    picked: Vec<(usize, usize)>,
}

fn open() -> Harness<'static, App> {
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(vec2(300.0, 300.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                app.last = TableSizePicker::new("size").show(ui);
                app.picked.extend(app.last.picked);
            },
            App::default(),
        )
}

// Hovering row 3, column 4 reports (3, 4) and labels "4×3 Table"; clicking returns it; arrows and Enter do the same from the keyboard.
#[test]
fn hover_sizes_and_click_picks() {
    let mut harness = open();
    harness.run();
    let grid = harness.get_by_role_and_label(Role::Grid, "Insert Table").rect();
    let step = grid.width() / COLUMNS as f32;
    // Square (3, 4) is the fourth to the right and third down; its centre is in the middle of its step.
    let at: Pos2 = grid.min + vec2(3.5 * step, 2.5 * step);

    harness.event(Event::PointerMoved(at));
    harness.step();
    assert_eq!(harness.state().last.hovered, Some((3, 4)));
    assert!(harness.query_by_label("4×3 Table").is_some());

    for pressed in [true, false] {
        harness.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        harness.step();
    }
    assert_eq!(harness.state().picked, vec![(3, 4)]);

    // From the keyboard: the grid takes the focus, → and ↓ move the square, Enter picks it.
    harness.event(Event::PointerMoved(Pos2::new(-10.0, -10.0)));
    harness.state_mut().picked.clear();
    harness.get_by_role_and_label(Role::Grid, "Insert Table").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().last.hovered, Some((2, 2)));
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().picked, vec![(2, 2)]);
}

// A pointer in the gap between squares hovers none (WordCraft reports a hover only from a square's own response), and one on the square after the gap hovers that square.
#[test]
fn the_gap_between_squares_hovers_nothing() {
    let mut harness = open();
    harness.run();
    let grid = harness.get_by_role_and_label(Role::Grid, "Insert Table").rect();
    // The grid is `COLUMNS` steps wide and `ROWS` steps high, each less one gap.
    let step = (grid.width() - grid.height()) / (COLUMNS - ROWS) as f32;
    let gap = COLUMNS as f32 * step - grid.width();
    let cell = step - gap;
    let on_square = grid.min + vec2(0.5 * cell, 0.5 * cell);
    harness.event(Event::PointerMoved(on_square));
    harness.step();
    assert_eq!(harness.state().last.hovered, Some((1, 1)));
    // Midway between the first and the second column.
    harness.event(Event::PointerMoved(grid.min + vec2(cell + gap / 2.0, 0.5 * cell)));
    harness.step();
    assert_eq!(harness.state().last.hovered, None);
}

// A preset hover shows the square and its label without a pointer.
#[test]
fn a_preset_hover_shows_the_square_and_label() {
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(300.0, 300.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui, app: &mut App| app.last = TableSizePicker::new("size").hovered((3, 4)).show(ui), App::default());
    harness.run();
    assert_eq!(harness.state().last.hovered, Some((3, 4)));
    assert!(harness.query_by_label("4×3 Table").is_some());
}
