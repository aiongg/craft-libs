//! `TileZoom`: the steps of the ladder and of a continuous range.

use craft_ui::testing::trial;
use craft_ui::tile_grid::{TileZoom, ZoomStep};

const SIZE_MIN: f64 = 48.0;
const SIZE_MAX: f64 = 400.0;
const SIZE_STEP: f64 = 16.0;
const SIZE_DEFAULT: f64 = 120.0;

#[trial]
fn the_ladder_steps_to_the_next_rung_and_stops_at_its_ends() {
    let ladder = TileZoom::Ladder;
    assert_eq!(ladder.step(1.1, ZoomStep::In), 1.25, "a value between rungs goes to the next one");
    assert_eq!(ladder.step(1.1, ZoomStep::Out), 1.0);
    assert_eq!(ladder.step(0.6, ZoomStep::Out), 0.6);
    assert_eq!(ladder.step(5.0, ZoomStep::In), 5.0);
    assert_eq!(ladder.step(3.7, ZoomStep::Reset), 1.0);
}

#[trial]
fn a_range_zoom_steps_by_its_step_inside_its_ends_and_resets_to_its_default() {
    let range = TileZoom::bounded(SIZE_MIN, SIZE_MAX, SIZE_STEP, SIZE_DEFAULT);
    assert_eq!(range.step(50.0, ZoomStep::Out), SIZE_MIN);
    assert_eq!(range.step(395.0, ZoomStep::In), SIZE_MAX);
    assert_eq!(range.step(100.0, ZoomStep::In), 116.0);
    assert_eq!(range.step(300.0, ZoomStep::Reset), SIZE_DEFAULT);
    assert_eq!(range.range(), SIZE_MIN..=SIZE_MAX);
}
