//! Stories of `StartScreen`.

use super::StartScreen;
use crate::contract::Bind;
use crate::primitives::{LoadFailure, Revision, TextureState, TextureStore, TextureStoreOptions};
use crate::shell::{RecentFile, RecentSort, RecentView, Template, ThumbKey};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_i18n::Collator;
use egui::{Color32, ColorImage, Ui};
use std::path::PathBuf;

/// The time the age lines count from, in seconds since the Unix epoch.
const NOW: i64 = 1_800_000_000;
const DAY: i64 = 86_400;
/// The size of the stored pictures.
const PICTURE: [usize; 2] = [64, 36];

/// The view and sort the stories edit.
#[derive(Clone, Debug, Default, PartialEq)]
struct Choice {
    view: RecentView,
    sort: RecentSort,
}

fn recent_files() -> Vec<RecentFile> {
    let file = |name: &str, folder: &str, days: i64, thumbnail: Option<Revision>| {
        let mut file = RecentFile::new(PathBuf::from(folder).join(name), name);
        file.opened = Some(NOW - days * DAY);
        file.created = Some(NOW - (days + 30) * DAY);
        file.thumbnail = thumbnail;
        file
    };
    vec![
        file("Poster 2.fig", "/home/ana/Work", 0, Some(Revision::new(1))),
        file("Poster 10.fig", "/home/ana/Work", 2, Some(Revision::new(2))),
        file("Brochure.fig", "/home/ana/Print", 5, Some(Revision::new(3))),
        file("Broken.fig", "/home/ana/Print", 9, Some(Revision::new(4))),
        file("Notes.fig", "/home/ana/Notes", 30, None),
    ]
}

fn templates() -> Vec<Template> {
    let mut a4 = Template::new("a4", "A4", "210 × 297 mm");
    a4.thumbnail = Some(Revision::new(1));
    vec![a4, Template::new("hd", "Full HD", "1920 × 1080 px")]
}

/// Fills the store with the pictures the stories name: ready for revisions 1 to 3 and the A4 template, failed for 4.
fn fill(ui: &Ui) -> TextureStore<ThumbKey> {
    let ctx = ui.ctx();
    let store = TextureStore::<ThumbKey>::in_context(ctx, "start-stories", TextureStoreOptions::default());
    let tints = [Color32::from_rgb(0x8a, 0x6f, 0x4d), Color32::from_rgb(0x4a, 0x7f, 0xc1), Color32::from_rgb(0x5f, 0x9e, 0x6e)];
    for (file, tint) in recent_files().into_iter().zip(tints) {
        if let Some(revision) = file.thumbnail {
            let _ = store.get_or_put(ctx, ThumbKey::Recent(file.path), revision, || ColorImage::filled(PICTURE, tint));
        }
    }
    let _ = store.get_or_put(ctx, ThumbKey::Template("a4".to_owned()), Revision::new(1), || ColorImage::filled(PICTURE, tints[0]));
    let broken = ThumbKey::Recent(PathBuf::from("/home/ana/Print").join("Broken.fig"));
    if matches!(store.state(ctx, &broken, Revision::new(4)), TextureState::Missing) {
        store.put_failure(ctx, broken, Revision::new(4), LoadFailure::Unreadable);
    }
    store
}

fn screen(ui: &mut Ui, data: &mut StoryData, choice: Choice, files: &[RecentFile], pinned: &[PathBuf]) -> StoryOutput {
    let store = fill(ui);
    let mut choice = data.value("choice", choice).clone();
    let collator = Collator::new("en");
    let shown = StartScreen::new(Bind::bound(&mut choice.view), Bind::bound(&mut choice.sort), files, &collator)
        .pinned(pinned)
        .templates(&templates())
        .now(NOW)
        .thumbnails(store)
        .show(ui)
        .into();
    *data.value("choice", Choice::default()) = choice;
    shown
}

#[story(StartScreen<'static>, covers = [], drive = Drive::Never)]
fn start_screen(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let pinned = [PathBuf::from("/home/ana/Work"), PathBuf::from("/home/ana/Print")];
    screen(ui, data, Choice::default(), &recent_files(), &pinned)
}

#[story(StartScreen<'static>, covers = [], drive = Drive::Never)]
fn start_screen_list_sorted_by_name(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let choice = Choice { view: RecentView::List, sort: RecentSort::Name };
    screen(ui, data, choice, &recent_files(), &[])
}

#[story(StartScreen<'static>, covers = [], drive = Drive::Never)]
fn start_screen_without_files(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    screen(ui, data, Choice::default(), &[], &[])
}
