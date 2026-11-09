//! `TextureStore` and `TextureLoader`: revisions, eviction, region updates and background jobs.

use craft_behavior::PixelSize;
use craft_host::ManualJobs;
use craft_host::queue::Cancel;
use craft_ui::primitives::{
    LoadFailure, LoadResult, LoadedPicture, LoaderOptions, Priority, Revision, TextureBudget, TextureLoader, TextureState, TextureStore,
    TextureStoreOptions, texture_side,
};
use craft_ui::testing::{Frames, trial};
use egui::{Color32, ColorImage, Context};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const SMALL: usize = 4;
const LARGE: usize = 8;
const KEPT_ENTRIES: usize = 2;
const QUIET_FRAMES: usize = 3;

fn shade(level: u8) -> Color32 {
    Color32::from_gray(level)
}

fn square(side: usize, color: Color32) -> ColorImage {
    ColorImage::filled([side, side], color)
}

fn next_frame(frames: &mut Frames) {
    frames.idle(1, |_| {});
}

fn ready(state: &TextureState) -> bool {
    matches!(state, TextureState::Ready(_))
}

fn store_with(ctx: &Context, salt: &str, budget: TextureBudget) -> TextureStore<u32> {
    TextureStore::in_context(ctx, salt, TextureStoreOptions::thumbnails().budget(budget))
}

#[trial]
fn a_new_revision_keeps_the_texture_and_the_same_revision_uploads_nothing() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let store = TextureStore::<u32>::in_context(&ctx, "pictures", TextureStoreOptions::thumbnails());
    let first = store.put(&ctx, 1, Revision::new(1), square(SMALL, shade(40)));
    next_frame(&mut frames);
    let second = store.put(&ctx, 1, Revision::new(2), square(LARGE, shade(90)));
    assert_eq!(first.id, second.id, "the texture is updated in place");
    assert_eq!(second.size, PixelSize::new(LARGE as u32, LARGE as u32));
    assert_eq!(store.state(&ctx, &1, Revision::new(2)), TextureState::Ready(second));
    assert!(matches!(store.state(&ctx, &1, Revision::new(3)), TextureState::Loading { stand_in: Some(shown), .. } if shown == second));
    assert_eq!(store.statistics(&ctx).entries, 1);

    next_frame(&mut frames);
    let made = Cell::new(0);
    let again = store.get_or_put(&ctx, 1, Revision::new(2), || {
        made.set(made.get() + 1);
        square(LARGE, shade(90))
    });
    assert_eq!((again, made.get()), (second, 0), "the same revision makes no image");
    assert_eq!(store.statistics(&ctx).uploads, 0, "and uploads nothing");
}

#[trial]
fn over_budget_the_oldest_unseen_entry_goes_first_and_a_fresh_entry_stays() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let by_count = store_with(&ctx, "by-count", TextureBudget::new(usize::MAX, KEPT_ENTRIES));
    for key in 0..4 {
        by_count.put(&ctx, key, Revision::new(1), square(2, shade(40)));
        frames.idle(QUIET_FRAMES, |_| {});
    }
    let stats = by_count.statistics(&ctx);
    assert_eq!((stats.entries, stats.evicted), (KEPT_ENTRIES, 2));
    assert_eq!(by_count.state(&ctx, &0, Revision::new(1)), TextureState::Missing);
    assert!(ready(&by_count.state(&ctx, &3, Revision::new(1))));

    let crowded = store_with(&ctx, "crowded", TextureBudget::new(1, 1));
    for key in 0..3 {
        crowded.put(&ctx, key, Revision::new(1), square(2, shade(40)));
    }
    assert_eq!(crowded.statistics(&ctx).entries, 3, "entries put in this frame stay past the budget");
    next_frame(&mut frames);
    crowded.put(&ctx, 3, Revision::new(1), square(2, shade(40)));
    assert_eq!(crowded.statistics(&ctx).entries, 4, "entries of the previous frame stay too");
}

#[trial]
fn a_region_update_changes_part_of_a_texture_without_a_new_id() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let store = TextureStore::<u32>::in_context(&ctx, "pages", TextureStoreOptions::pages());
    // Twice the device limit wide, so the texture is reduced by the whole factor 2.
    let side = texture_side(&ctx) as usize;
    let whole = store.put(&ctx, 1, Revision::new(1), ColorImage::filled([2 * side, 4], shade(0)));
    assert_eq!(whole.size, PixelSize::new(side as u32, 2));
    next_frame(&mut frames);

    let patch = || square(2, shade(255));
    assert!(store.put_region(&ctx, &1, Revision::new(2), [1, 0], patch()).is_none(), "origin off the reduction grid");
    assert!(store.put_region(&ctx, &1, Revision::new(2), [2 * side as u32 - 2, 2], square(4, shade(255))).is_none(), "past the image");
    assert!(store.put_region(&ctx, &9, Revision::new(2), [0, 0], patch()).is_none(), "no picture under the key");
    assert_eq!(store.state(&ctx, &1, Revision::new(1)), TextureState::Ready(whole), "a refused update changes nothing");

    let updated = store.put_region(&ctx, &1, Revision::new(2), [2, 2], patch()).expect("a region on the grid");
    assert_eq!(updated, whole);
    assert_eq!(store.state(&ctx, &1, Revision::new(2)), TextureState::Ready(whole));
}

fn loader_with(ctx: &Context, lanes: usize, uploads: usize) -> (TextureLoader<u32>, Arc<ManualJobs>) {
    let jobs = Arc::new(ManualJobs::new());
    let budget = TextureBudget::new(usize::MAX, usize::MAX).uploads(uploads, usize::MAX);
    let options = LoaderOptions::new().store(TextureStoreOptions::thumbnails().budget(budget)).lanes(lanes).jobs(jobs.clone());
    (TextureLoader::in_context(ctx, "loader", options), jobs)
}

fn counted(runs: &Arc<AtomicUsize>, side: usize) -> impl FnOnce(&Cancel) -> LoadResult + Send + 'static {
    let runs = Arc::clone(runs);
    move |_| {
        runs.fetch_add(1, Ordering::Relaxed);
        Ok(LoadedPicture::Image(ColorImage::filled([side, side], shade(40))))
    }
}

/// One frame in which `run` asks the loader.
fn frame(frames: &mut Frames, run: impl FnOnce()) {
    let mut run = Some(run);
    frames.frame(Vec::new(), |_| {
        if let Some(run) = run.take() {
            run();
        }
    });
}

#[trial]
fn a_result_of_an_older_revision_is_discarded() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    // One upload per frame keeps key 2's finished revision-1 result waiting behind key 1's.
    let (loader, jobs) = loader_with(&ctx, 1, 1);
    let runs = Arc::new(AtomicUsize::new(0));
    frame(&mut frames, || {
        loader.request(&ctx, &1, Revision::new(1), Priority::VISIBLE, || counted(&runs, SMALL));
        loader.request(&ctx, &2, Revision::new(1), Priority::NEAR, || counted(&runs, SMALL));
    });
    jobs.run_all();
    frame(&mut frames, || {
        loader.ask(&ctx, &1, Revision::new(1), Priority::VISIBLE);
        loader.request(&ctx, &2, Revision::new(2), Priority::NEAR, || counted(&runs, LARGE));
    });
    frame(&mut frames, || {
        loader.ask(&ctx, &2, Revision::new(2), Priority::NEAR);
    });
    assert_eq!(loader.cache().statistics(&ctx).entries, 1, "the revision-1 result of key 2 was not stored");
    jobs.run_all();
    let state = Cell::new(TextureState::Missing);
    frame(&mut frames, || state.set(loader.ask(&ctx, &2, Revision::new(2), Priority::NEAR).expect("settled")));
    let size = PixelSize::new(LARGE as u32, LARGE as u32);
    assert!(matches!(state.into_inner(), TextureState::Ready(texture) if texture.size == size), "the revision-2 result is stored");
    assert_eq!(runs.load(Ordering::Relaxed), 3);
}

#[trial]
fn a_panicking_job_is_stored_as_a_failure() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let (loader, jobs) = loader_with(&ctx, 1, 16);
    frame(&mut frames, || {
        loader.submit(&ctx, 1, Revision::new(1), Priority::VISIBLE, |_| std::panic::panic_any("the decoder failed"));
    });
    jobs.run_all();
    let state = Cell::new(TextureState::Missing);
    frame(&mut frames, || state.set(loader.ask(&ctx, &1, Revision::new(1), Priority::VISIBLE).expect("settled")));
    assert!(matches!(state.into_inner(), TextureState::Failed { failure, .. } if failure == LoadFailure::Panicked));
}

#[trial]
fn without_a_host_a_job_runs_off_the_calling_thread() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let loader = TextureLoader::<u32>::in_context(&ctx, "no-host", LoaderOptions::new());
    let (sender, receiver) = std::sync::mpsc::channel();
    frame(&mut frames, || {
        let state = loader.request(&ctx, &1, Revision::new(1), Priority::VISIBLE, || {
            move |_: &Cancel| {
                sender.send(std::thread::current().id()).ok();
                Ok(LoadedPicture::Blank)
            }
        });
        assert!(matches!(state, TextureState::Loading { .. }), "the request returns before the picture exists");
    });
    let ran_on = receiver.recv_timeout(std::time::Duration::from_secs(10)).expect("the job runs");
    assert_ne!(ran_on, std::thread::current().id(), "the job does not run on the UI thread");
}

#[trial]
fn a_key_asked_in_several_frames_runs_one_job() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let (loader, jobs) = loader_with(&ctx, 1, 16);
    let runs = Arc::new(AtomicUsize::new(0));
    for _ in 0..QUIET_FRAMES {
        frame(&mut frames, || {
            let state = loader.request(&ctx, &1, Revision::new(1), Priority::VISIBLE, || counted(&runs, SMALL));
            assert!(matches!(state, TextureState::Loading { .. }));
        });
    }
    jobs.run_all();
    frame(&mut frames, || {
        assert!(ready(&loader.request(&ctx, &1, Revision::new(1), Priority::VISIBLE, || counted(&runs, SMALL))));
    });
    assert_eq!(runs.load(Ordering::Relaxed), 1);
}

#[trial]
fn jobs_nobody_asked_for_in_two_frames_are_dropped() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let (loader, jobs) = loader_with(&ctx, 1, 16);
    let runs = [0, 1, 2].map(|_| Arc::new(AtomicUsize::new(0)));
    let ask = |key: u32| {
        loader.request(&ctx, &key, Revision::new(1), Priority::VISIBLE, || counted(&runs[key as usize - 1], SMALL));
    };
    frame(&mut frames, || [1, 2].into_iter().for_each(ask));
    frame(&mut frames, || ask(1));
    frame(&mut frames, || ask(3));
    jobs.run_all();
    frame(&mut frames, || {
        for key in [1, 2, 3] {
            loader.ask(&ctx, &key, Revision::new(1), Priority::VISIBLE);
        }
    });
    assert_eq!(runs.each_ref().map(|runs| runs.load(Ordering::Relaxed)), [1, 0, 1]);
    let stored = |key| ready(&loader.cache().state(&ctx, &key, Revision::new(1)));
    assert_eq!([stored(1), stored(2), stored(3)], [true, false, true]);
}
