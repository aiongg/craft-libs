//! `Stack` header and footer regions: the footer stays in the room under a collection that fills it.

use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::contract::{Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::layout_components::{Row, Stack};
use craft_ui::list::{List, ListRow};
use craft_ui::testing::layout::Block;
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::Cell;

const AREA: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 210.0) };
const ROWS: usize = 50;

/// Fifty rows keyed by position.
struct Rows;

impl DisplayOrder<usize> for Rows {
    fn len(&self) -> usize {
        ROWS
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for Rows {}

