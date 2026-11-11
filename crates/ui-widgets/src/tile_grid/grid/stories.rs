//! Stories of `TileGrid`.

use super::{GridEvent, TileGrid, TileGroup};
use crate::contract::ShowCollection as _;
use crate::list::ThumbAspect;
use crate::testing::pictures::StoryPictures;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use crate::tile_grid::{BadgeKind, BadgeSlot, Tile, TileBadge, TileFlag};
use craft_behavior::{DisplayOrder, ItemSource, SizeHint};
use craft_theme::Srgba;
use craft_theme::variant::Tone;
use egui::Ui;

/// The keys of the group header rows, apart from the tiles' own.
const GROUP_KEYS: [usize; 3] = [1000, 1001, 1002];

/// Tiles keyed by position, each with a picture aspect that cycles through the usual photo shapes.
struct Photos(usize);

/// Width over height of the picture of tile `at`.
fn aspect(at: usize) -> f32 {
    [1.5, 1.0, 0.75, 1.78][at % 4]
}

/// [`aspect`] as a thumbnail shape.
fn thumb_aspect(at: usize) -> ThumbAspect {
    [ThumbAspect::new(3, 2), ThumbAspect::SQUARE, ThumbAspect::new(3, 4), ThumbAspect::new(16, 9)][at % 4]
}

/// A hash of `key` that spreads small keys.
fn mix(key: usize) -> usize {
    (key.wrapping_mul(2_654_435_761) >> 7) & 0xff_ffff
}

/// The label colours the photos cycle through.
const LABELS: [Srgba; 4] = [Srgba::rgb(0.86, 0.27, 0.24), Srgba::rgb(0.9, 0.67, 0.16), Srgba::rgb(0.27, 0.63, 0.35), Srgba::rgb(0.27, 0.47, 0.86)];

/// Photo `key`: its picture, a rating, and from the key the odd pick, reject, colour label and Duplicate badge.
fn photo(pictures: &StoryPictures, key: usize) -> Tile<'static> {
    let mut tile = Tile::new(format!("Photo {}", key + 1)).thumb(thumb_aspect(key), pictures.thumb(key as u64)).rating((mix(key) % 6) as u8);
    if key.is_multiple_of(11) {
        tile = tile.flag(TileFlag::Picked);
    }
    if key.is_multiple_of(17) {
        tile = tile.flag(TileFlag::Rejected).rejected();
    }
    if key.is_multiple_of(5) {
        tile = tile.label_color(LABELS[key / 5 % 4]);
    }
    if key.is_multiple_of(13) {
        tile = tile.badge(TileBadge::new(
            BadgeSlot::EndTop,
            BadgeKind::Text("Duplicate".into()),
            Tone::Neutral,
            Some("Duplicate of another photo".into()),
        ));
    }
    tile
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
    let pictures = StoryPictures::of(ui.ctx());
    grid.show_items(ui, &Photos(count), &mut |key, _| photo(&pictures, *key)).into()
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
        TileGroup::new(GROUP_KEYS[0], 0, 12, "October 2026").detail("12 photos").open(open[0]),
        TileGroup::new(GROUP_KEYS[1], 12, 8, "September 2026").detail("8 photos").open(open[1]),
        TileGroup::new(GROUP_KEYS[2], 20, 10, "August 2026").detail("10 photos").open(open[2]),
    ];
    let pictures = StoryPictures::of(ui.ctx());
    let out = TileGrid::new("Library").groups(&groups).collapsible().zoom(2.0).show_items(ui, &Photos(30), &mut |key, _| photo(&pictures, *key));
    for event in &out.events {
        if let GridEvent::GroupOpen { group, open: wanted } = event {
            open[*group] = *wanted;
        }
    }
    out.into()
}
