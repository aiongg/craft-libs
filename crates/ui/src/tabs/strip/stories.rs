//! Stories of `TabStrip`.

use super::{TabKind, TabStrip};
use crate::contract::{ReorderRule, ShowCollection as _};
use crate::tabs::Tab;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::{DisplayOrder, ItemSource};
use egui::Ui;
use std::borrow::Cow;

const TITLES: [&str; 12] = [
    "Untitled-1",
    "Photo.psd",
    "Poster final.psd",
    "Logo.ai",
    "Sketch",
    "Notes",
    "Brochure.indd",
    "Cover.psd",
    "Pattern",
    "Banner.psd",
    "Draft",
    "Export",
];

/// Tabs keyed by position and named by the title list.
struct Documents(usize);

impl DisplayOrder<usize> for Documents {
    fn len(&self) -> usize {
        self.0
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        TITLES.get(at).map(|title| Cow::Borrowed(*title))
    }
}

impl ItemSource<usize> for Documents {}

fn strip(ui: &mut Ui, strip: TabStrip<'_, usize>, count: usize, kind: TabKind) -> StoryOutput {
    strip.kind(kind).show_items(ui, &Documents(count), &mut |key, _| Tab::new(TITLES[*key]).closable(true)).into()
}

#[story(TabStrip<'static, usize>, covers = [Default, Many], drive = Drive::Never)]
fn tab_strip_many(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    strip(ui, TabStrip::new("Documents").selected(3).reorder(ReorderRule::new()), TITLES.len(), TabKind::Document)
}

#[story(TabStrip<'static, usize>, covers = [One], drive = Drive::Never)]
fn tab_strip_one(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    strip(ui, TabStrip::new("Documents").selected(0), 1, TabKind::Document)
}

#[story(TabStrip<'static, usize>, covers = [Empty], drive = Drive::Never)]
fn tab_strip_empty(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    strip(ui, TabStrip::new("Documents"), 0, TabKind::Document)
}

#[story(TabStrip<'static, usize>, covers = [], drive = Drive::Never)]
fn tab_strip_panel(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    strip(ui, TabStrip::new("Panels").selected(1), 4, TabKind::Panel)
}
