//! `TileZoom`: the steps of the ladder and of a continuous range of factors on the medium thumbnail.

use craft_ui::testing::trial;
use craft_ui::tile_grid::{TileZoom, ZoomStep};

const ZOOM_MIN: f64 = 0.5;
const ZOOM_MAX: f64 = 4.0;
const ZOOM_STEP: f64 = 0.25;
const ZOOM_DEFAULT: f64 = 1.0;

