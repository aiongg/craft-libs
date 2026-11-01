//! `FileThumbSource` through `Thumbnails` with `ManualJobs` and a fake decoder (task XT).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use craft_host::ManualJobs;
use craft_host::queue::Cancel;
use craft_ui::canvas::textures::{Priority, TextureState, ThumbPaint, ThumbRequest, ThumbnailOptions, Thumbnails};
use egui::{Context, Rect, pos2, vec2};

use super::{DecodeRequest, DecodedThumb, FileStat, FileThumbKey, FileThumbSource, StoreKey, ThumbDecoder, ThumbError, ThumbStore};

/// What the fake decoder answers.
#[derive(Clone)]
enum Answer {
    Picture,
    Fail(&'static str),
    Unsupported,
}

struct Decoder {
    answer: Mutex<Answer>,
    /// Side of the full picture; changed between decodes to tell pictures apart.
    full_side: AtomicU32,
    quick_side: Option<u32>,
    decoded: Mutex<Vec<PathBuf>>,
    quick_calls: AtomicUsize,
    /// Answer of the fake stat; the files the tests make are never read.
    stat: Mutex<Result<FileStat, ThumbError>>,
    /// Replaces the quick picture when set.
    bad_quick: Mutex<Option<DecodedThumb>>,
    requests: Mutex<Vec<DecodeRequest>>,
}

impl Decoder {
    fn new(quick_side: Option<u32>) -> Arc<Self> {
        Arc::new(Self {
            answer: Mutex::new(Answer::Picture),
            full_side: AtomicU32::new(4),
            quick_side,
            decoded: Mutex::default(),
            quick_calls: AtomicUsize::new(0),
            stat: Mutex::new(Ok(FileStat { len: 7, modified: None })),
            bad_quick: Mutex::default(),
            requests: Mutex::default(),
        })
    }

    fn decodes(&self) -> usize {
        self.decoded.lock().unwrap().len()
    }
}

fn picture(side: u32) -> DecodedThumb {
    DecodedThumb { width: side, height: side, rgba: vec![255; (side * side * 4) as usize] }
}

impl ThumbDecoder for Decoder {
    fn stat(&self, _: &Path) -> Result<FileStat, ThumbError> {
        self.stat.lock().unwrap().clone()
    }

    fn decode(&self, request: &DecodeRequest, _: &Cancel) -> Result<DecodedThumb, ThumbError> {
        self.requests.lock().unwrap().push(request.clone());
        self.decoded.lock().unwrap().push(request.key.path.clone());
        match self.answer.lock().unwrap().clone() {
            Answer::Picture => Ok(picture(self.full_side.load(Ordering::Relaxed))),
            Answer::Fail(message) => Err(ThumbError::Failed(message.to_owned())),
            Answer::Unsupported => Err(ThumbError::Unsupported),
        }
    }

    fn quick(&self, _: &DecodeRequest, _: &Cancel) -> Option<DecodedThumb> {
        self.quick_calls.fetch_add(1, Ordering::Relaxed);
        if let Some(bad) = self.bad_quick.lock().unwrap().clone() {
            return Some(bad);
        }
        self.quick_side.map(picture)
    }
}

struct Store {
    hit: DecodedThumb,
    saves: AtomicUsize,
}

impl ThumbStore for Store {
    fn load(&self, _: &StoreKey) -> Option<DecodedThumb> {
        Some(self.hit.clone())
    }

    fn save(&self, _: &StoreKey, _: &DecodedThumb) {
        self.saves.fetch_add(1, Ordering::Relaxed);
    }
}

struct Setup {
    ctx: Context,
    jobs: Arc<ManualJobs>,
    thumbs: Thumbnails<FileThumbKey>,
    dir: tempfile::TempDir,
}

impl Setup {
    fn new() -> Self {
        let ctx = Context::default();
        let jobs = Arc::new(ManualJobs::new());
        let options = ThumbnailOptions { lanes: 1, jobs: Some(jobs.clone()), ..ThumbnailOptions::default() };
        let thumbs = Thumbnails::in_context(&ctx, "files", options);
        Self { ctx, jobs, thumbs, dir: tempfile::tempdir().expect("temp dir") }
    }

    fn file(&self, name: &str) -> FileThumbKey {
        let path = self.dir.path().join(name);
        std::fs::write(&path, name).expect("write file");
        FileThumbKey::file(path)
    }

    fn pass(&self, run: impl FnOnce()) {
        let mut run = Some(run);
        self.ctx
            .run_ui(egui::RawInput::default(), |_| {
                if let Some(run) = run.take() {
                    run();
                }
            })
            .drop_without_applying_deltas();
    }

    fn ask(&self, source: &FileThumbSource, key: &FileThumbKey, priority: Priority) -> TextureState {
        let mut state = TextureState::Missing;
        self.pass(|| state = self.thumbs.state(&self.ctx, source, key, &ThumbRequest::for_cell(32.0, 1.0), priority));
        state
    }

    /// Asks `key` at VISIBLE.
    fn show(&self, source: &FileThumbSource, key: &FileThumbKey) -> TextureState {
        self.ask(source, key, Priority::VISIBLE)
    }
}

fn side(state: &TextureState) -> Option<usize> {
    state.shown().map(|texture| texture.size[0])
}

// Two keys asked VISIBLE and NEAR: ManualJobs runs the VISIBLE decode first.
#[test]
fn file_source_jobs_run_visible_before_near() {
    let setup = Setup::new();
    let decoder = Decoder::new(None);
    let source = FileThumbSource::new(decoder.clone(), None);
    let (near, visible) = (setup.file("near.png"), setup.file("visible.png"));
    setup.ask(&source, &near, Priority::NEAR);
    setup.ask(&source, &visible, Priority::VISIBLE);
    assert!(setup.jobs.run_next());
    assert_eq!(decoder.decoded.lock().unwrap().as_slice(), std::slice::from_ref(&visible.path));
    setup.jobs.run_all();
    assert_eq!(*decoder.decoded.lock().unwrap(), [visible.path, near.path]);
}

// After `invalidate(path)` the state is Loading with the old picture as stand-in, then Ready with the new.
#[test]
fn file_source_reloads_after_invalidate() {
    let setup = Setup::new();
    let decoder = Decoder::new(None);
    let source = FileThumbSource::new(decoder.clone(), None);
    let key = setup.file("a.png");
    assert!(matches!(setup.show(&source, &key), TextureState::Loading { stand_in: None }));
    setup.jobs.run_all();
    let state = setup.show(&source, &key);
    assert_eq!(side(&state), Some(4));
    let first = state.shown().expect("picture");

    decoder.full_side.store(6, Ordering::Relaxed);
    source.invalidate(&key.path);
    let state = setup.show(&source, &key);
    assert_eq!(state, TextureState::Loading { stand_in: Some(first) });
    setup.jobs.run_all();
    let state = setup.show(&source, &key);
    assert!(matches!(state, TextureState::Ready(_)));
    assert_eq!(side(&state), Some(6));
    assert_eq!(state.shown().map(|texture| texture.id), Some(first.id));
}

// A failing decode: Failed, and no second job on later passes until `invalidate`.
#[test]
fn file_source_fails_once_per_stamp() {
    let setup = Setup::new();
    let decoder = Decoder::new(None);
    *decoder.answer.lock().unwrap() = Answer::Fail("boom");
    let source = FileThumbSource::new(decoder.clone(), None);
    let key = setup.file("a.png");
    setup.show(&source, &key);
    setup.jobs.run_all();
    for _ in 0..3 {
        assert_eq!(setup.show(&source, &key), TextureState::Failed { message: "boom".to_owned() });
    }
    assert_eq!(setup.jobs.waiting(), 0);
    assert_eq!(decoder.decodes(), 1);

    *decoder.answer.lock().unwrap() = Answer::Picture;
    source.invalidate(&key.path);
    setup.show(&source, &key);
    setup.jobs.run_all();
    assert!(matches!(setup.show(&source, &key), TextureState::Ready(_)));
    assert_eq!(decoder.decodes(), 2);
}

// A decoder with `quick`: the quick picture shows, then stands in while `decode` runs, then the full one.
#[test]
fn file_source_shows_the_quick_preview_first() {
    let setup = Setup::new();
    let decoder = Decoder::new(Some(2));
    let source = FileThumbSource::new(decoder.clone(), None);
    let key = setup.file("a.png");
    setup.show(&source, &key);
    assert!(setup.jobs.run_next());
    assert_eq!(decoder.decodes(), 0);

    let state = setup.show(&source, &key);
    let TextureState::Loading { stand_in: Some(quick) } = state else { panic!("expected the quick picture as stand-in, got {state:?}") };
    assert_eq!(quick.size[0], 2);
    setup.jobs.run_all();
    let state = setup.show(&source, &key);
    assert!(matches!(state, TextureState::Ready(_)));
    assert_eq!(side(&state), Some(4));
    assert_eq!(state.shown().map(|texture| texture.id), Some(quick.id));
    assert_eq!((decoder.decodes(), decoder.quick_calls.load(Ordering::Relaxed)), (1, 1));
    // The full picture is final: no further job.
    setup.show(&source, &key);
    assert_eq!(setup.jobs.waiting(), 0);
}

// A store hit is final: `decode` is never called and no second revision is asked.
#[test]
fn a_stored_thumbnail_needs_no_decode() {
    let setup = Setup::new();
    let decoder = Decoder::new(Some(2));
    let store = Arc::new(Store { hit: picture(3), saves: AtomicUsize::new(0) });
    let source = FileThumbSource::new(decoder.clone(), Some(store.clone()));
    let key = setup.file("a.png");
    setup.show(&source, &key);
    setup.jobs.run_all();
    for _ in 0..3 {
        let state = setup.show(&source, &key);
        assert!(matches!(state, TextureState::Ready(_)));
        assert_eq!(side(&state), Some(3));
    }
    assert_eq!(setup.jobs.waiting(), 0);
    assert_eq!((decoder.decodes(), decoder.quick_calls.load(Ordering::Relaxed), store.saves.load(Ordering::Relaxed)), (0, 0, 0));
}

// A decoder answering `ThumbError::Unsupported`: the state is Empty (not Failed), `paint` draws nothing, and no second job runs.
#[test]
fn an_unsupported_file_shows_no_picture() {
    let setup = Setup::new();
    let decoder = Decoder::new(None);
    *decoder.answer.lock().unwrap() = Answer::Unsupported;
    let source = FileThumbSource::new(decoder.clone(), None);
    let key = setup.file("a.xyz");
    setup.show(&source, &key);
    setup.jobs.run_all();
    for _ in 0..3 {
        assert_eq!(setup.show(&source, &key), TextureState::Empty);
    }
    setup.pass(|| {
        let painter = egui::Painter::new(setup.ctx.clone(), egui::LayerId::background(), Rect::EVERYTHING);
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(32.0, 32.0));
        assert!(!setup.thumbs.paint(&painter, rect, &TextureState::Empty, &ThumbPaint::default()));
    });
    assert_eq!(setup.jobs.waiting(), 0);
    assert_eq!(decoder.decodes(), 1);
}

// The length and modification time come from the decoder's stat hook, not the file system.
#[test]
fn the_job_stats_through_the_decoder() {
    let setup = Setup::new();
    let decoder = Decoder::new(None);
    let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(99);
    *decoder.stat.lock().unwrap() = Ok(FileStat { len: 1234, modified: Some(modified) });
    let source = FileThumbSource::new(decoder.clone(), None);
    let key = FileThumbKey::file(setup.dir.path().join("not-on-disk.png"));
    setup.show(&source, &key);
    setup.jobs.run_all();
    assert!(matches!(setup.show(&source, &key), TextureState::Ready(_)));
    let requests = decoder.requests.lock().unwrap();
    assert_eq!((requests[0].len, requests[0].modified), (1234, Some(modified)));
}

// A stat that finds no file is the loader's "missing" reason, not a decode.
#[test]
fn a_failed_stat_is_missing() {
    let setup = Setup::new();
    let decoder = Decoder::new(None);
    *decoder.stat.lock().unwrap() = Err(ThumbError::Missing);
    let source = FileThumbSource::new(decoder.clone(), None);
    let key = setup.file("a.png");
    setup.show(&source, &key);
    setup.jobs.run_all();
    let state = setup.show(&source, &key);
    assert!(matches!(state, TextureState::Failed { .. }), "{state:?}");
    assert_eq!(decoder.decodes(), 0);
}

// A buffer that is not width x height x 4 fails as unreadable instead of panicking; a zero size shows no picture.
#[test]
fn a_malformed_decode_fails_and_a_zero_size_is_empty() {
    let setup = Setup::new();
    let key = setup.file("a.png");
    super::source::to_image(DecodedThumb { width: 2, height: 2, rgba: vec![0; 3] }).expect_err("short buffer");
    assert!(matches!(
        super::source::to_image(DecodedThumb { width: 0, height: 3, rgba: vec![] }),
        Ok(craft_ui::canvas::textures::LoadedPicture::Blank)
    ));
    assert!(matches!(
        super::source::to_image(DecodedThumb { width: 2, height: 2, rgba: vec![0; 16] }),
        Ok(craft_ui::canvas::textures::LoadedPicture::Image(_))
    ));

    let with_quick = Decoder::new(Some(2));
    *with_quick.bad_quick.lock().unwrap() = Some(DecodedThumb { width: 2, height: 2, rgba: vec![0; 3] });
    let source_quick = FileThumbSource::new(with_quick.clone(), None);
    setup.show(&source_quick, &key);
    setup.jobs.run_all();
    let state = setup.show(&source_quick, &key);
    assert!(matches!(state, TextureState::Failed { .. }), "{state:?}");
    // The failed quick job did not move the key to `NeedsFull`: no full job is asked.
    assert_eq!(setup.jobs.waiting(), 0);
}

// The stage is per (key, side): a second size starts from the quick preview again.
#[test]
fn stages_are_kept_per_side() {
    let setup = Setup::new();
    let decoder = Decoder::new(Some(2));
    let source = FileThumbSource::new(decoder.clone(), None);
    let key = setup.file("a.png");
    setup.show(&source, &key);
    setup.jobs.run_all();
    setup.show(&source, &key);
    setup.jobs.run_all();
    assert_eq!(decoder.quick_calls.load(Ordering::Relaxed), 1);
    setup.pass(|| {
        setup.thumbs.state(&setup.ctx, &source, &key, &ThumbRequest::for_cell(128.0, 1.0), Priority::VISIBLE);
    });
    setup.jobs.run_all();
    assert_eq!(decoder.quick_calls.load(Ordering::Relaxed), 2);
}
