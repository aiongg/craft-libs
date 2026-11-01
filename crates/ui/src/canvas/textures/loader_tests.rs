//! `TextureLoader` over a `JobQueue` driven by `ManualJobs` (task XT, after Q1, X1 and X3).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use craft_host::ManualJobs;
use egui::{Color32, ColorImage, Context, RawInput};

use super::{LoadedPicture, LoaderOptions, Priority, Revision, TextureBudget, TextureCacheOptions, TextureLoader, TextureState};

fn pass(ctx: &Context, run: impl FnOnce()) {
    let mut run = Some(run);
    ctx.run_ui(RawInput::default(), |_| {
        if let Some(run) = run.take() {
            run();
        }
    })
    .drop_without_applying_deltas();
}

fn loader(ctx: &Context, lanes: usize, uploads_per_pass: usize) -> (TextureLoader<u32>, Arc<ManualJobs>) {
    let jobs = Arc::new(ManualJobs::new());
    let budget = TextureBudget { uploads_per_pass, ..TextureCacheOptions::thumbnails().budget };
    let options = LoaderOptions { cache: TextureCacheOptions::thumbnails().budget(budget), lanes, jobs: Some(jobs.clone()) };
    (TextureLoader::in_context(ctx, "test", options), jobs)
}

fn counted(runs: &Arc<AtomicUsize>, side: usize) -> impl FnOnce(&craft_host::queue::Cancel) -> Result<LoadedPicture, String> + Send + 'static {
    let runs = Arc::clone(runs);
    move |_| {
        runs.fetch_add(1, Ordering::Relaxed);
        Ok(LoadedPicture::Image(ColorImage::filled([side, side], Color32::RED)))
    }
}

fn ready(state: &TextureState) -> bool {
    matches!(state, TextureState::Ready(_))
}

// A queued job whose key is not asked for in two passes never runs.
#[test]
fn loader_drops_jobs_nobody_asked_for() {
    let ctx = Context::default();
    let (loader, jobs) = loader(&ctx, 1, 16);
    let runs = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
    pass(&ctx, || {
        for key in [1, 2] {
            loader.request(&ctx, &key, Revision(1), Priority::VISIBLE, || counted(&runs[key as usize - 1], 4));
        }
    });
    pass(&ctx, || {
        loader.request(&ctx, &1, Revision(1), Priority::VISIBLE, || counted(&runs[0], 4));
    });
    pass(&ctx, || {
        loader.request(&ctx, &3, Revision(1), Priority::VISIBLE, || counted(&runs[2], 4));
    });
    jobs.run_all();
    pass(&ctx, || {
        for key in [1, 2, 3] {
            loader.ask(&ctx, &key, Revision(1), Priority::VISIBLE);
        }
    });
    assert_eq!(runs.each_ref().map(|runs| runs.load(Ordering::Relaxed)), [1, 0, 1]);
    assert!(ready(&loader.cache().state(&ctx, &1, Revision(1))));
    assert!(!ready(&loader.cache().state(&ctx, &2, Revision(1))));
    assert!(ready(&loader.cache().state(&ctx, &3, Revision(1))));
}

// With 20 results finished, 16 are uploaded this pass, the rest in the next, and a repaint is requested.
#[test]
fn loader_uploads_at_most_budget_per_pass() {
    let ctx = Context::default();
    let (loader, jobs) = loader(&ctx, 1, 16);
    let runs = Arc::new(AtomicUsize::new(0));
    let ask_all = |states: &mut Vec<TextureState>| {
        *states = (0..20).map(|key| loader.request(&ctx, &key, Revision(1), Priority::VISIBLE, || counted(&runs, 4))).collect();
    };
    let mut states = Vec::new();
    pass(&ctx, || ask_all(&mut states));
    jobs.run_all();
    assert_eq!(runs.load(Ordering::Relaxed), 20);
    // Settle the repaint request that finishing the jobs made.
    pass(&ctx, || {});
    pass(&ctx, || ask_all(&mut states));
    assert_eq!(states.iter().filter(|state| ready(state)).count(), 16);
    assert_eq!(loader.cache().statistics().uploads, 16);
    assert!(ctx.has_requested_repaint());
    pass(&ctx, || ask_all(&mut states));
    assert!(states.iter().all(ready));
    assert_eq!(runs.load(Ordering::Relaxed), 20);
}

// A result for a revision replaced before it finished is not uploaded.
#[test]
fn loader_discards_results_of_older_revisions() {
    let ctx = Context::default();
    // One upload per pass keeps the finished revision-1 result waiting behind another key's.
    let (loader, jobs) = loader(&ctx, 1, 1);
    let runs = Arc::new(AtomicUsize::new(0));
    pass(&ctx, || {
        loader.request(&ctx, &1, Revision(1), Priority::VISIBLE, || counted(&runs, 4));
        loader.request(&ctx, &2, Revision(1), Priority::NEAR, || counted(&runs, 4));
    });
    jobs.run_all();
    // Key 1 uploads, key 2's result waits; key 2 is then asked at revision 2.
    pass(&ctx, || {
        loader.ask(&ctx, &1, Revision(1), Priority::VISIBLE);
        loader.request(&ctx, &2, Revision(2), Priority::NEAR, || counted(&runs, 8));
    });
    assert!(!ready(&loader.cache().state(&ctx, &2, Revision(1))));
    pass(&ctx, || {
        loader.ask(&ctx, &2, Revision(2), Priority::NEAR);
    });
    assert_eq!(loader.cache().statistics().entries, 1);
    jobs.run_all();
    let mut state = TextureState::Missing;
    pass(&ctx, || state = loader.ask(&ctx, &2, Revision(2), Priority::NEAR).expect("settled"));
    let TextureState::Ready(texture) = state else { panic!("expected Ready, got {state:?}") };
    assert_eq!(texture.size, [8, 8]);
    assert_eq!(runs.load(Ordering::Relaxed), 3);
}

// With `lanes: 0` the first call of a pass runs one job inline.
#[test]
fn loader_runs_one_job_per_pass_without_lanes() {
    let ctx = Context::default();
    let (loader, jobs) = loader(&ctx, 0, 16);
    let runs = Arc::new(AtomicUsize::new(0));
    let ask_all = || (0..3).map(|key| loader.request(&ctx, &key, Revision(1), Priority::VISIBLE, || counted(&runs, 4))).collect::<Vec<_>>();
    pass(&ctx, || {
        ask_all();
    });
    assert_eq!(runs.load(Ordering::Relaxed), 0);
    for expected in 1..=3 {
        let mut states = Vec::new();
        pass(&ctx, || states = ask_all());
        assert_eq!(runs.load(Ordering::Relaxed), expected);
        assert_eq!(states.iter().filter(|state| ready(state)).count(), expected);
    }
    assert_eq!(jobs.waiting(), 0);
}

// A job that answers `LoadedPicture::Blank` leaves the key `Empty` at its revision, with no texture and no second job.
#[test]
fn loader_stores_a_blank_answer_as_empty() {
    let ctx = Context::default();
    let (loader, jobs) = loader(&ctx, 1, 16);
    let runs = Arc::new(AtomicUsize::new(0));
    let blank = |runs: &Arc<AtomicUsize>| {
        let runs = Arc::clone(runs);
        move |_: &craft_host::queue::Cancel| {
            runs.fetch_add(1, Ordering::Relaxed);
            Ok(LoadedPicture::Blank)
        }
    };
    pass(&ctx, || {
        loader.request(&ctx, &1, Revision(1), Priority::VISIBLE, || blank(&runs));
    });
    jobs.run_all();
    for _ in 0..2 {
        let mut state = TextureState::Missing;
        pass(&ctx, || state = loader.request(&ctx, &1, Revision(1), Priority::VISIBLE, || blank(&runs)));
        assert_eq!(state, TextureState::Empty);
    }
    assert_eq!(jobs.waiting(), 0);
    assert_eq!(runs.load(Ordering::Relaxed), 1);
    assert_eq!(loader.cache().statistics().bytes, 0);
}
