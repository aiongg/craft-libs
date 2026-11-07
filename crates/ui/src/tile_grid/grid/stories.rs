//! Stories of `TileGrid`.

use super::{GridEvent, TileGrid, TileGroup};
use crate::contract::ShowCollection as _;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use crate::tile_grid::Tile;
use craft_behavior::{DisplayOrder, ItemSource, SizeHint};
use egui::Ui;

/// Tiles keyed by position, each with a picture aspect that cycles through the usual photo shapes.
struct Photos(usize);

/// Width over height of the picture of tile `at`.
fn aspect(at: usize) -> f32 {
    [1.5, 1.0, 0.75, 1.78][at % 4]
}

impl DisplayOrder<usize> for Photos {
    fn len(&self) -> usize {
        self.0
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0).then_some(*key)
    }
}

impl ItemSource<usize> for Photos {
    fn size_hint(&self, at: usize) -> SizeHint {
        SizeHint::Aspect(aspect(at))
    }
}

fn grid(ui: &mut Ui, grid: TileGrid<'_, usize>, count: usize) -> StoryOutput {
    grid.show_items(ui, &Photos(count), &mut |key, _| Tile::new(format!("Photo {}", key + 1))).into()
}

#[story(TileGrid<'static, usize>, covers = [Default, Many], drive = Drive::Never)]
fn tile_grid_many(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    grid(ui, TileGrid::new("Library").empty("No photos"), 60)
}

#[story(TileGrid<'static, usize>, covers = [One], drive = Drive::Never)]
fn tile_grid_one(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    grid(ui, TileGrid::new("Library").empty("No photos"), 1)
}

#[story(TileGrid<'static, usize>, covers = [Empty], drive = Drive::Never)]
fn tile_grid_empty(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    grid(ui, TileGrid::new("Library").empty("No photos"), 0)
}

#[story(TileGrid<'static, usize>, covers = [], drive = Drive::Never)]
fn tile_grid_justified(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    grid(ui, TileGrid::new("Library").justified(), 40)
}

#[story(TileGrid<'static, usize>, covers = [], drive = Drive::Never)]
fn tile_grid_groups(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let open = data.value("open", [true, false, true]);
    let groups = [
        TileGroup::new(0, 12, "October 2026").detail("12 photos").open(open[0]),
        TileGroup::new(12, 8, "September 2026").detail("8 photos").open(open[1]),
        TileGroup::new(20, 10, "August 2026").detail("10 photos").open(open[2]),
    ];
    let out = TileGrid::new("Library")
        .groups(&groups)
        .collapsible()
        .zoom(2.0)
        .show_items(ui, &Photos(30), &mut |key, _| Tile::new(format!("Photo {}", key + 1)));
    for event in &out.events {
        if let GridEvent::GroupOpen { group, open: wanted } = event {
            open[*group] = *wanted;
        }
    }
    out.into()
}
