//! Stories of `FreeBin`.

use super::{FreeBin, FreeformCard};
use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::contract::ShowCollection as _;
use craft_ui::list::ThumbAspect;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use craft_ui::tile_grid::Tile;
use egui::{Ui, vec2};

/// Cards keyed by position.
struct Cards(usize);

impl DisplayOrder<usize> for Cards {
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

impl ItemSource<usize> for Cards {}

fn bin(ui: &mut Ui, bin: FreeBin<'_, usize>, count: usize) -> StoryOutput {
    bin.show_items(ui, &Cards(count), &mut |key, _| {
        let tile = Tile::new(format!("Clip {}", key + 1)).thumb(ThumbAspect::new(1, 1), |_, _| {});
        if *key == 0 { tile.stack(2) } else { tile }
    })
    .into()
}

#[story(FreeBin<'static, usize>, covers = [Default], drive = Drive::Never)]
fn free_bin(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let cards = [
        FreeformCard::new(0, vec2(24.0, 24.0)).in_stack(1),
        FreeformCard::new(1, vec2(48.0, 40.0)).in_stack(1),
        FreeformCard::new(2, vec2(220.0, 90.0)),
    ];
    bin(ui, FreeBin::new("Footage", &cards), 5)
}

#[story(FreeBin<'static, usize>, covers = [Empty], drive = Drive::Never)]
fn free_bin_empty(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    bin(ui, FreeBin::new("Footage", &[]).empty("No clips"), 0)
}

#[story(FreeBin<'static, usize>, covers = [Many], drive = Drive::Never)]
fn free_bin_many(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    bin(ui, FreeBin::new("Footage", &[]).zoom(0.5), 60)
}
