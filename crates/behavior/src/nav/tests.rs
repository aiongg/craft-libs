//! Tests for navigation, roving and value stepping.

use super::*;
use crate::intent::Granularity;

struct Rows {
    depth: Vec<usize>,
    open: Vec<bool>,
    children: Vec<bool>,
    disabled: Vec<usize>,
}

impl Rows {
    fn flat(n: usize) -> Rows {
        Rows { depth: vec![0; n], open: vec![false; n], children: vec![false; n], disabled: vec![] }
    }
}

impl NavOutline for Rows {
    fn len(&self) -> usize {
        self.depth.len()
    }
    fn depth(&self, at: usize) -> usize {
        self.depth[at]
    }
    fn has_children(&self, at: usize) -> bool {
        self.children[at]
    }
    fn open(&self, at: usize) -> bool {
        self.open[at]
    }
    fn is_section(&self, _: usize) -> bool {
        false
    }
    fn enabled(&self, at: usize) -> bool {
        !self.disabled.contains(&at)
    }
}

fn go(shape: NavShape, rows: &Rows, state: &mut NavState, mv: NavMove) -> NavStep {
    step(shape, rows, state, mv, MoveMods::PLAIN)
}

#[test]
fn flat_moves_skip_disabled_rows_and_stop_or_wrap_at_the_ends() {
    let mut rows = Rows::flat(5);
    rows.disabled = vec![2];
    let mut state = NavState::at(1);
    assert_eq!(go(NavShape::Flat, &rows, &mut state, NavMove::Down), NavStep::Moved(3));
    assert_eq!(go(NavShape::Flat, &rows, &mut state, NavMove::End), NavStep::Moved(4));
    assert_eq!(go(NavShape::Flat, &rows, &mut state, NavMove::Forward), NavStep::Stay);
    state.wrap = true;
    assert_eq!(go(NavShape::Flat, &rows, &mut state, NavMove::Forward), NavStep::Moved(0));
    assert_eq!(go(NavShape::Flat, &rows, &mut state, NavMove::PageDown), NavStep::Moved(4));
    let mut none = NavState::new();
    assert_eq!(go(NavShape::Flat, &rows, &mut none, NavMove::Up), NavStep::Moved(4), "the first backward move lands on the last row");
}

#[test]
fn a_page_move_from_the_last_row_wraps_only_when_the_state_wraps() {
    let rows = Rows::flat(5);
    let mut stops = NavState::at(4);
    assert_eq!(go(NavShape::Flat, &rows, &mut stops, NavMove::PageDown), NavStep::Stay);
    let mut wraps = NavState::at(4).wrap(true);
    assert_eq!(go(NavShape::Flat, &rows, &mut wraps, NavMove::PageDown), NavStep::Moved(0));
    assert_eq!(go(NavShape::Flat, &rows, &mut wraps, NavMove::PageUp), NavStep::Moved(4));
}

#[test]
fn extending_keeps_the_anchor_at_the_first_lead() {
    let rows = Rows::flat(5);
    let mut state = NavState::at(1);
    let extend = MoveMods::new(true, false, Granularity::Unit);
    step(NavShape::Flat, &rows, &mut state, NavMove::Down, extend);
    step(NavShape::Flat, &rows, &mut state, NavMove::Down, extend);
    assert_eq!((state.anchor, state.lead), (Some(1), Some(3)));
    step(NavShape::Flat, &rows, &mut state, NavMove::Up, MoveMods::PLAIN);
    assert_eq!(state.anchor, None);
}

#[test]
fn tree_forward_expands_then_enters_and_back_collapses_then_leaves() {
    let mut rows = Rows::flat(3);
    rows.depth = vec![0, 1, 1];
    rows.children = vec![true, false, false];
    let mut state = NavState::at(0);
    assert_eq!(go(NavShape::Tree, &rows, &mut state, NavMove::Forward), NavStep::Expand(0));
    rows.open[0] = true;
    assert_eq!(go(NavShape::Tree, &rows, &mut state, NavMove::Forward), NavStep::Moved(1));
    assert_eq!(go(NavShape::Tree, &rows, &mut state, NavMove::Back), NavStep::Moved(0), "Back on a child goes to its parent");
    assert_eq!(go(NavShape::Tree, &rows, &mut state, NavMove::Back), NavStep::Collapse(0));
    rows.open[0] = false;
    assert_eq!(go(NavShape::Tree, &rows, &mut state, NavMove::Back), NavStep::Stay);
}

#[test]
fn wrapped_grids_move_by_columns_and_pages_land_in_the_same_column() {
    let rows = Rows::flat(10);
    let shape = NavShape::wrapped_grid(3);
    let mut state = NavState::at(4).page(2);
    assert_eq!(go(shape, &rows, &mut state, NavMove::Down), NavStep::Moved(7));
    assert_eq!(go(shape, &rows, &mut state, NavMove::Down), NavStep::Stay, "no row below");
    assert_eq!(go(shape, &rows, &mut state, NavMove::Up), NavStep::Moved(4));
    assert_eq!(go(shape, &rows, &mut state, NavMove::PageDown), NavStep::Moved(9), "a short last row takes the last position");
    assert_eq!(go(shape, &rows, &mut state, NavMove::Forward), NavStep::Stay);
    assert_eq!(go(shape, &rows, &mut state, NavMove::Back), NavStep::Moved(8));
}

#[test]
fn bounded_grids_stop_left_and_right_at_the_row_ends() {
    let rows = Rows::flat(9);
    let shape = NavShape::bounded_grid(3);
    let mut state = NavState::at(2);
    assert_eq!(go(shape, &rows, &mut state, NavMove::Forward), NavStep::Stay, "the end of a row does not continue on the next");
    assert_eq!(go(shape, &rows, &mut state, NavMove::Down), NavStep::Moved(5));
    let mut state = NavState::at(3);
    assert_eq!(go(shape, &rows, &mut state, NavMove::Back), NavStep::Stay, "the start of a row does not continue on the previous");
    assert_eq!(go(shape, &rows, &mut state, NavMove::Forward), NavStep::Moved(4));
    assert_eq!(go(shape, &rows, &mut NavState::at(0), NavMove::Up), NavStep::Stay, "no row above");
}

/// Rows of 3, 1 and 2 positions; a sheet of 3 rows by 4 columns; four items at free positions.
struct Shapes;

impl NavOutline for Shapes {
    fn len(&self) -> usize {
        6
    }
    fn depth(&self, _: usize) -> usize {
        0
    }
    fn has_children(&self, _: usize) -> bool {
        false
    }
    fn open(&self, _: usize) -> bool {
        false
    }
    fn is_section(&self, _: usize) -> bool {
        false
    }
    fn enabled(&self, _: usize) -> bool {
        true
    }
    fn row_len(&self, row: usize) -> Option<usize> {
        [3, 1, 2].get(row).copied()
    }
    fn columns(&self) -> usize {
        4
    }
    fn rect(&self, at: usize) -> Option<Rect> {
        let spots = [(0.0, 0.0), (100.0, 10.0), (10.0, 100.0), (110.0, 110.0), (200.0, 0.0), (0.0, 200.0)];
        spots.get(at).map(|&(x, y)| Rect::from_min_size(emath::Pos2::new(x, y), emath::Vec2::splat(20.0)))
    }
}

#[test]
fn ragged_rows_keep_the_column_where_the_row_is_long_enough() {
    let mut state = NavState::at(2).page(5);
    let go = |state: &mut NavState, mv| step(NavShape::RaggedGrid, &Shapes, state, mv, MoveMods::PLAIN);
    assert_eq!(go(&mut state, NavMove::Down), NavStep::Moved(3), "the single-item row takes any column");
    assert_eq!(go(&mut state, NavMove::Down), NavStep::Moved(4), "the lead of the single-item row sits in column 0");
    assert_eq!(go(&mut state, NavMove::Down), NavStep::Stay);
    assert_eq!(go(&mut state, NavMove::PageUp), NavStep::Moved(0));
    assert_eq!(go(&mut state, NavMove::Forward), NavStep::Moved(1));
}

#[test]
fn sheet_cells_move_by_column_and_rows_stop_at_the_ends() {
    let mut state = NavState::at(1);
    let go = |state: &mut NavState, mv, lead| step(NavShape::Cell, &Shapes, state, mv, MoveMods::new(false, lead, Granularity::Unit));
    assert_eq!(go(&mut state, NavMove::Forward, false), NavStep::Column(1));
    assert_eq!(go(&mut state, NavMove::End, false), NavStep::Column(3));
    assert_eq!(go(&mut state, NavMove::Forward, false), NavStep::Stay);
    assert_eq!((state.lead, state.column), (Some(1), 3));
    assert_eq!(go(&mut state, NavMove::Up, false), NavStep::Moved(0));
    assert_eq!(go(&mut state, NavMove::End, true), NavStep::Moved(5), "Ctrl+End is the last row");
    assert_eq!(state.column, 3, "a row move keeps the column");
}

#[test]
fn spatial_moves_go_to_the_nearest_item_in_that_direction() {
    let mut state = NavState::at(0);
    let go = |state: &mut NavState, mv| step(NavShape::Spatial, &Shapes, state, mv, MoveMods::PLAIN);
    assert_eq!(go(&mut state, NavMove::Forward), NavStep::Moved(1), "the near item at the right, not the far one in line");
    assert_eq!(go(&mut state, NavMove::Down), NavStep::Moved(3));
    assert_eq!(go(&mut state, NavMove::Back), NavStep::Moved(2));
    assert_eq!(go(&mut state, NavMove::Up), NavStep::Moved(0));
    assert_eq!(go(&mut state, NavMove::Up), NavStep::Stay);
}

#[test]
fn type_ahead_builds_a_prefix_for_one_second_and_repeated_letters_cycle() {
    let labels = ["apple", "apricot", "banana", "avocado"];
    let label = |at: usize| std::borrow::Cow::Borrowed(labels[at]);
    let mut buffer = TypeAhead::new();
    assert_eq!(buffer.push('a', 0.0), "a");
    assert_eq!(buffer.push('p', 0.5), "ap");
    assert_eq!(buffer.push('r', 1.4), "apr");
    assert_eq!(buffer.push('x', 3.0), "x", "a pause starts a new prefix");
    assert_eq!(TypeAhead::pick("ap", Some(0), 4, label), Some(0));
    assert_eq!(TypeAhead::pick("apr", Some(0), 4, label), Some(1));
    assert_eq!(TypeAhead::pick("a", Some(0), 4, label), Some(1), "one letter moves past the lead");
    assert_eq!(TypeAhead::pick("aa", Some(3), 4, label), Some(0), "a repeated letter cycles and wraps");
    assert_eq!(TypeAhead::pick("z", None, 4, label), None);
}

#[test]
fn alt_steps_finely_and_shift_with_alt_steps_by_small() {
    let steps = ValueSteps::new(5.0, 0.0, 10.0, 1.0, 4.0).fine(0.25);
    let fine = MoveMods::PLAIN.fine(true);
    assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::Forward, fine), StepResult::Set(5.25));
    assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::Down, fine), StepResult::Set(4.75));
    let both = MoveMods::new(true, false, Granularity::Unit).fine(true);
    assert_eq!(step_value(&steps, ValueAxes::Both, NavMove::Forward, both), StepResult::Set(6.0));
    let StepResult::Set(default_fine) = step_value(&ValueSteps::new(5.0, 0.0, 10.0, 1.0, 4.0), ValueAxes::Both, NavMove::Forward, fine) else {
        panic!("a forward move sets the value")
    };
    assert!((default_fine - 5.1).abs() < 1e-9);
}

#[test]
fn wheel_steps_ignore_zero_flip_under_rtl_and_scale_the_notch() {
    use crate::wheel::{WheelPhase, WheelUnit};
    use craft_keymap::Mods;
    use emath::vec2;
    let steps = ValueSteps::new(5.0, 0.0, 10.0, 0.01, 0.1).fine(0.001).notch(0.01);
    let wheel = |scroll, mods| Wheel::new(scroll, mods, WheelUnit::Point, WheelPhase::Move);
    let set = |wheel: Wheel, rtl| match wheel_step(&steps, ValueAxes::Both, &wheel, rtl) {
        StepResult::Set(value) => value - 5.0,
        StepResult::Ignored => f64::NAN,
    };
    assert_eq!(wheel_step(&steps, ValueAxes::Both, &wheel(vec2(0.0, 0.0), Mods::NONE), false), StepResult::Ignored);
    assert!((set(wheel(vec2(0.0, 3.0), Mods::NONE), false) - 0.01).abs() < 1e-9);
    assert!((set(wheel(vec2(0.0, -3.0), Mods::NONE), false) + 0.01).abs() < 1e-9);
    assert!((set(wheel(vec2(3.0, 0.0), Mods::NONE), false) - 0.01).abs() < 1e-9);
    assert!((set(wheel(vec2(3.0, 0.0), Mods::NONE), true) + 0.01).abs() < 1e-9);
    assert!((set(wheel(vec2(0.0, 1.0), Mods::SHIFT), false) - 0.1).abs() < 1e-9);
    assert!((set(wheel(vec2(0.0, 1.0), Mods::ALT), false) - 0.001).abs() < 1e-9);
    assert_eq!(wheel_step(&steps, ValueAxes::Vertical, &wheel(vec2(3.0, 0.0), Mods::NONE), false), StepResult::Ignored);
}
