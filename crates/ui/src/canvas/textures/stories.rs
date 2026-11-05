//! Stories of the textures.
//!
//! Every story uses fakes: [`FakeThumbSource`](super::FakeThumbSource) and a
//! [`ManualJobs`](craft_host::ManualJobs) in the options' `jobs`, with a "Finish next" button that
//! calls `run_next`, so loading, stand-in and failure states show one step at a time. Every string
//! goes through story words (the gallery's pseudo-locale test).
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/texture-cache/cache` | `TextureCache` with a small budget: Ready, Loading with stand-in, Empty, Failed, eviction as the key count grows, a `TextureStatistics` readout |
//! | `primitives/thumbnails/grid` | `Thumbnails` over `FakeThumbSource`: synchronous and background renders, portrait and landscape frames, a transparent picture on the checkerboard, `hold` |
//! | `primitives/texture-queue/queue` | The order of visible, near and background requests with `ManualJobs`; requests scrolled away dropped |
//!
//! Story ids start with their level (`ids_are_unique_and_follow_the_level_component_story_form`),
//! so these are `Level::Primitives` stories with the component names in [`COMPONENTS`].

use std::sync::Arc;

use craft_host::ManualJobs;
use egui::{ColorImage, Context, Image, Rect, ScrollArea, Sense, Ui, pos2, vec2};

use super::{
    FakeThumbSource, Priority, Revision, TextureBudget, TextureCache, TextureCacheOptions, TextureState, ThumbFit, ThumbPaint, ThumbRender,
    ThumbRequest, ThumbSource as _, ThumbnailOptions, Thumbnails,
};
use crate::controls::Button;
use crate::localized::Labels as _;
use crate::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["TextureCache", "Thumbnails", "TextureQueue"];

crate::words! {
    /// The words of the texture stories.
    pub(crate) struct TextureStoryLabels {
        put: "textureStory.put" = "Put";
        put_next: "textureStory.putNext" = "Put next key";
        new_revision: "textureStory.newRevision" = "New revision";
        ready: "textureStory.ready" = "Ready";
        loading: "textureStory.loading" = "Loading";
        loading_stand_in: "textureStory.loadingStandIn" = "Loading, stand-in shown";
        missing: "textureStory.missing" = "Missing";
        empty: "textureStory.empty" = "Empty";
        failed: "textureStory.failed" = "Failed";
        key: "textureStory.key" = "Key {n}";
        readout: "textureStory.readout" = "Entries {entries}, {kib} KiB, evicted {evicted}";
        waiting_rows: "textureStory.waitingRows" = "Rows waiting: {n}";
        queue_row: "textureStory.queueRow" = "Row {n}: {priority}, {state}";
        not_asked: "textureStory.notAsked" = "Row {n}: not asked";
        visible: "textureStory.visible" = "visible";
        near: "textureStory.near" = "near";
        background: "textureStory.background" = "background";
        landscape: "textureStory.landscape" = "Landscape";
        square: "textureStory.square" = "Square";
        failing: "textureStory.failing" = "Fails";
        blank: "textureStory.blank" = "No picture";
    }
}

/// The three stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/texture-cache/cache",
            Level::Primitives,
            "TextureCache",
            "Cache",
            CacheArgs { entries: 4.0, visible: 3.0 },
            show_cache
        ),
        typed_story!(
            "primitives/thumbnails/grid",
            Level::Primitives,
            "Thumbnails",
            "Thumbnails",
            ThumbnailsArgs { background: true, fit: FitArg::Contain, checker: true, hold: false },
            show_thumbnails
        ),
        typed_story!("primitives/texture-queue/queue", Level::Primitives, "TextureQueue", "Queue", QueueArgs { prefetch: true }, show_queue),
    ]
}

/// The cell side of the cache and thumbnail tiles, in points.
const TILE: f32 = 64.0;

/// How many [`TILE`] cells fit in a row of `ui`. A nested `vertical` per cell doesn't wrap in
/// `horizontal_wrapped`, so the story breaks rows itself.
fn cells_per_row(ui: &Ui) -> usize {
    let gap = ui.spacing().item_spacing.x;
    (((ui.available_width() + gap) / (TILE + gap)).floor() as usize).max(1)
}

/// The jobs the story's thumbnails run on, kept in egui memory under the story's name.
fn manual_jobs(ui: &Ui, name: &str) -> Arc<ManualJobs> {
    let id = ui.make_persistent_id(("texture-story-jobs", name));
    ui.data_mut(|data| data.get_temp_mut_or_insert_with(id, || Arc::new(ManualJobs::new())).clone())
}

/// Thumbnails running one job at a time on `jobs`. The options are read when it is first made, so
/// `salt` includes everything the options depend on.
fn thumbnails(ctx: &Context, salt: impl std::hash::Hash + std::fmt::Debug, jobs: &Arc<ManualJobs>) -> Thumbnails<u32> {
    let options = ThumbnailOptions { lanes: 1, jobs: Some(jobs.clone()), ..ThumbnailOptions::default() };
    Thumbnails::in_context(ctx, ("texture-story-thumbnails", salt), options)
}

/// The "Finish next" button: runs the oldest waiting job.
fn finish_next(ui: &mut Ui, jobs: &ManualJobs) {
    if ui.add(Button::new(StoryLabels::of(ui).finish_next).enabled(jobs.waiting() > 0)).clicked() {
        jobs.run_next();
    }
}

fn state_word(words: &TextureStoryLabels, state: &TextureState) -> String {
    match state {
        TextureState::Missing => words.missing.clone(),
        TextureState::Loading { stand_in: None } => words.loading.clone(),
        TextureState::Loading { stand_in: Some(_) } => words.loading_stand_in.clone(),
        TextureState::Ready(_) => words.ready.clone(),
        TextureState::Empty => words.empty.clone(),
        TextureState::Failed { .. } => words.failed.clone(),
    }
}

/// A picture of the fake source for `key` at `revision`.
fn fake_image(key: u32, revision: u64) -> ColorImage {
    let mut source = FakeThumbSource::<u32>::new();
    source.set_revision(revision);
    match source.render(&key, &ThumbRequest { side: 64, pixels_per_point: 1.0, region: None }) {
        ThumbRender::Ready(image) => image,
        ThumbRender::Later(_) | ThumbRender::Blank => ColorImage::example(),
    }
}

// Cache

#[derive(StoryArgs, Debug, PartialEq)]
struct CacheArgs {
    /// The cache's entry budget.
    #[arg(name = "entry budget", range = 2.0..=8.0)]
    entries: f64,
    /// How many of the newest keys are on screen; older keys fall out of view and can be evicted.
    #[arg(name = "keys in view", range = 1.0..=8.0)]
    visible: f64,
}

/// The keys put so far and the revision every key is asked at.
#[derive(Clone)]
struct CacheDemo {
    puts: u32,
    revision: u64,
}

/// Puts what key `key` shows at `revision`: every fifth key from 2 is Empty, from 3 Failed.
fn put_key(cache: &TextureCache<u32>, ctx: &Context, key: u32, revision: u64, words: &TextureStoryLabels) {
    match key % 5 {
        2 => cache.put_empty(ctx, key, Revision(revision)),
        3 => cache.put_failure(ctx, key, Revision(revision), words.failed.clone()),
        _ => {
            cache.put(ctx, key, Revision(revision), fake_image(key, revision));
        }
    }
}

fn show_cache(ui: &mut Ui, args: &mut CacheArgs) {
    let words = TextureStoryLabels::of(ui);
    let ctx = ui.ctx().clone();
    let entries = args.entries.round().max(1.0) as usize;
    let visible = args.visible.round().max(1.0) as u32;
    let salt = ("texture-story-cache", entries);
    let state_id = ui.make_persistent_id(salt);
    let mut demo = ui.data_mut(|data| data.get_temp::<CacheDemo>(state_id)).unwrap_or(CacheDemo { puts: 4, revision: 0 });
    let budget = TextureBudget { max_entries: entries, max_bytes: 4 * 1024 * 1024, uploads_per_pass: 16, upload_bytes_per_pass: 16 * 1024 * 1024 };
    let cache = TextureCache::<u32>::in_context(&ctx, salt, TextureCacheOptions::thumbnails().budget(budget));

    ui.horizontal(|ui| {
        if ui.add(Button::new(words.put_next.clone())).clicked() {
            put_key(&cache, &ctx, demo.puts, demo.revision, &words);
            demo.puts += 1;
        }
        if ui.add(Button::new(words.new_revision.clone())).clicked() {
            demo.revision += 1;
        }
    });
    let keys: Vec<u32> = (demo.puts.saturating_sub(visible)..demo.puts).collect();
    for row in keys.chunks(cells_per_row(ui)) {
        ui.horizontal_top(|ui| {
            for &key in row {
                let state = cache.state(&ctx, &key, Revision(demo.revision));
                ui.vertical(|ui| {
                    ui.set_width(TILE);
                    let size = vec2(TILE, TILE);
                    match state.shown() {
                        Some(texture) => {
                            ui.add(Image::new((texture.id, size)));
                        }
                        None => {
                            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                            ui.painter().rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
                        }
                    }
                    ui.label(craft_i18n::fmt(&words.key, &[("n", &key.to_string())]));
                    ui.label(state_word(&words, &state));
                    let settled = matches!(state, TextureState::Ready(_) | TextureState::Empty | TextureState::Failed { .. });
                    if !settled && ui.add(Button::new(words.put.clone())).clicked() {
                        put_key(&cache, &ctx, key, demo.revision, &words);
                    }
                });
            }
        });
    }
    let statistics = cache.statistics();
    ui.label(craft_i18n::fmt(
        &words.readout,
        &[
            ("entries", &statistics.entries.to_string()),
            ("kib", &(statistics.bytes / 1024).to_string()),
            ("evicted", &statistics.evicted.to_string()),
        ],
    ));
    ui.data_mut(|data| data.insert_temp(state_id, demo));
}

// Thumbnails

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum FitArg {
    Contain,
    Cover,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ThumbnailsArgs {
    /// Render as jobs, one per "Finish next".
    background: bool,
    fit: FitArg,
    /// The checkerboard behind transparency.
    checker: bool,
    /// Freeze the shown pictures; a new revision then shows no new picture.
    hold: bool,
}

fn show_thumbnails(ui: &mut Ui, args: &mut ThumbnailsArgs) {
    let words = TextureStoryLabels::of(ui);
    let ctx = ui.ctx().clone();
    let jobs = manual_jobs(ui, "thumbnails");
    let thumbs = thumbnails(&ctx, ("thumbnails", args.background), &jobs);
    let revision_id = ui.make_persistent_id("texture-story-thumbnails-revision");
    let mut revision = ui.data_mut(|data| data.get_temp::<u64>(revision_id)).unwrap_or(0);
    ui.horizontal(|ui| {
        finish_next(ui, &jobs);
        if ui.add(Button::new(words.new_revision.clone())).clicked() {
            revision += 1;
        }
    });
    thumbs.hold(args.hold);
    let source = |width, height| {
        let mut source = FakeThumbSource::<u32>::new().later(args.background).aspect(width, height).failing([4]).blank([5]);
        source.set_revision(revision);
        source
    };
    let cells = [
        (1, source(3, 4), StoryLabels::of(ui).portrait),
        (2, source(4, 3), words.landscape.clone()),
        (3, source(1, 1), words.square.clone()),
        (4, source(1, 1), words.failing.clone()),
        (5, source(1, 1), words.blank.clone()),
    ];
    let fit = match args.fit {
        FitArg::Contain => ThumbFit::Contain,
        FitArg::Cover => ThumbFit::Cover,
    };
    let paint = ThumbPaint::default().fit(fit).checker(args.checker);
    let request = ThumbRequest::for_cell(TILE, ui.pixels_per_point());
    for row in cells.chunks(cells_per_row(ui)) {
        ui.horizontal_top(|ui| {
            for (key, source, caption) in row {
                ui.vertical(|ui| {
                    ui.set_width(TILE);
                    let (rect, _) = ui.allocate_exact_size(vec2(TILE, TILE), Sense::hover());
                    let state = thumbs.state(&ctx, source, key, &request, Priority::VISIBLE);
                    thumbs.paint(ui.painter(), rect, &state, &paint);
                    ui.label(caption.clone());
                    ui.label(state_word(&words, &state));
                });
            }
        });
    }
    ui.data_mut(|data| data.insert_temp(revision_id, revision));
}

// Queue

#[derive(StoryArgs, Debug, PartialEq)]
struct QueueArgs {
    /// Ask for rows beyond the near ones at background priority. Off, a row scrolled away is not
    /// asked and its queued job is dropped.
    prefetch: bool,
}

const QUEUE_ROWS: u32 = 14;
const QUEUE_ROW: f32 = 32.0;

fn show_queue(ui: &mut Ui, args: &mut QueueArgs) {
    let words = TextureStoryLabels::of(ui);
    let ctx = ui.ctx().clone();
    let jobs = manual_jobs(ui, "queue");
    let thumbs = thumbnails(&ctx, "queue", &jobs);
    let source = FakeThumbSource::<u32>::new().later(true);
    let request = ThumbRequest::for_cell(QUEUE_ROW - 4.0, ui.pixels_per_point());
    finish_next(ui, &jobs);
    let mut waiting = 0;
    ScrollArea::vertical().max_height(QUEUE_ROW * 4.0).id_salt("texture-story-queue").show_viewport(ui, |ui, viewport| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let near = viewport.expand2(vec2(0.0, viewport.height()));
        for row in 0..QUEUE_ROWS {
            let top = row as f32 * QUEUE_ROW;
            let band = Rect::from_min_max(pos2(0.0, top), pos2(1.0, top + QUEUE_ROW));
            let priority = if viewport.intersects(band) {
                Some((Priority::VISIBLE, &words.visible))
            } else if near.intersects(band) {
                Some((Priority::NEAR, &words.near))
            } else if args.prefetch {
                Some((Priority::BACKGROUND, &words.background))
            } else {
                None
            };
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(QUEUE_ROW - 4.0, QUEUE_ROW), Sense::hover());
                let rect = rect.shrink2(vec2(0.0, 2.0));
                let text = match priority {
                    Some((priority, name)) => {
                        let state = thumbs.state(&ctx, &source, &row, &request, priority);
                        thumbs.paint(ui.painter(), rect, &state, &ThumbPaint::default());
                        waiting += usize::from(matches!(state, TextureState::Loading { .. }));
                        craft_i18n::fmt(
                            &words.queue_row,
                            &[("n", &(row + 1).to_string()), ("priority", name), ("state", &state_word(&words, &state))],
                        )
                    }
                    None => craft_i18n::fmt(&words.not_asked, &[("n", &(row + 1).to_string())]),
                };
                ui.label(text);
            });
        }
    });
    ui.label(craft_i18n::fmt(&words.waiting_rows, &[("n", &waiting.to_string())]));
}
