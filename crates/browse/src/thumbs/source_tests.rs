//! `FileThumbSource` through `Thumbnails` with `ManualJobs` and a fake decoder (task XT).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use craft_host::ManualJobs;
use craft_host::queue::Cancel;
use craft_ui::canvas::textures::{Priority, TextureState, ThumbPaint, ThumbRequest, ThumbnailOptions, Thumbnails};
use egui::{Context, Rect, pos2, vec2};

use super::{DecodeRequest, DecodedThumb, FileThumbKey, FileThumbSource, StoreKey, ThumbDecoder, ThumbError, ThumbStore};

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
}

impl Decoder {
    fn new(quick_side: Option<u32>) -> Arc<Self> {
        Arc::new(Self {
            answer: Mutex::new(Answer::Picture),
            full_side: AtomicU32::new(4),
            quick_side,
            decoded: Mutex::default(),
            quick_calls: AtomicUsize::new(0),
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
    fn decode(&self, request: &DecodeRequest, _: &Cancel) -> Result<DecodedThumb, ThumbError> {
        self.decoded.lock().unwrap().push(request.key.path.clone());
        match self.answer.lock().unwrap().clone() {
            Answer::Picture => Ok(picture(self.full_side.load(Ordering::Relaxed))),
            Answer::Fail(message) => Err(ThumbError::Failed(message.to_owned())),
            Answer::Unsupported => Err(ThumbError::Unsupported),
        }
    }

    fn quick(&self, _: &DecodeRequest, _: &Cancel) -> Option<DecodedThumb> {
        self.quick_calls.fetch_add(1, Ordering::Relaxed);
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
