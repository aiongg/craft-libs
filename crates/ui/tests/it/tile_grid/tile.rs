//! `TileZoom`: the steps of the ladder and of a continuous range of factors on the medium thumbnail.

use craft_ui::testing::trial;
use craft_ui::tile_grid::{TileZoom, ZoomStep};

const ZOOM_MIN: f64 = 0.5;
const ZOOM_MAX: f64 = 4.0;
const ZOOM_STEP: f64 = 0.25;
const ZOOM_DEFAULT: f64 = 1.0;

#[trial]
fn zoom_steps_along_a_ladder_or_a_range() {
    {
        let ladder = TileZoom::Ladder;
        assert_eq!(ladder.step(1.1, ZoomStep::In), 1.25, "a value between rungs goes to the next one");
        assert_eq!(ladder.step(1.1, ZoomStep::Out), 1.0);
        assert_eq!(ladder.step(0.6, ZoomStep::Out), 0.6);
        assert_eq!(ladder.step(5.0, ZoomStep::In), 5.0);
        assert_eq!(ladder.step(3.7, ZoomStep::Reset), 1.0);
    }
    {
        let range = TileZoom::bounded(ZOOM_MIN, ZOOM_MAX, ZOOM_STEP, ZOOM_DEFAULT);
        assert_eq!(range.step(0.55, ZoomStep::Out), ZOOM_MIN);
        assert_eq!(range.step(3.9, ZoomStep::In), ZOOM_MAX);
        assert_eq!(range.step(1.0, ZoomStep::In), 1.25);
        assert_eq!(range.step(3.0, ZoomStep::Reset), ZOOM_DEFAULT);
        assert_eq!(range.range(), ZOOM_MIN..=ZOOM_MAX);
    }
}
