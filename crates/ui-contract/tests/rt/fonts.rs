//! Count trials of the font stack on craft-ui-rt: the window root measures the stack facts again only when the text
//! engine's stack moves, and then only the nodes that read the env run again; a served face arriving offers the stack
//! once. Relations only, no times.

use craft_theme::FontSlot;
use craft_ui_contract::contract::{MemoView, Shown, Ui};
use craft_ui_contract::runtime::env::{Env, EnvCells};
use craft_ui_contract::runtime::windows::{WindowEnv, window_root_with};
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{FontFace, FontStack, Renderer, StackFace, StepReport, TextEngine, Vec2};
use craft_ui_test_kit::rt::Rig;

/// What each scope returned in the last build, and whether its closure ran there.
#[derive(Default)]
struct Seen {
    reader: Option<Shown<u32>>,
    quiet: Option<Shown<u32>>,
}

/// Reads the env: the line ratio control heights grow by.
fn reader(_: &mut (), ui: &mut Ui) -> u32 {
    Env::of(ui).line_ratio.to_bits()
}

/// Reads nothing of the env.
fn quiet(_: &mut (), _: &mut Ui) -> u32 {
    0
}

fn step(rig: &mut Rig<RecordingRenderer>, cells: EnvCells, seen: &mut Seen) -> StepReport {
    let host = WindowEnv { cells: Some(cells), jobs: None, commands: None };
    rig.frame_with(0.0, &mut |_, cx| {
        window_root_with(cx, host.clone(), |ui| {
            seen.reader = MemoView::new("reader", (), reader).show(ui);
            seen.quiet = MemoView::new("quiet", (), quiet).show(ui);
        })
    })
}

fn fresh(shown: &Option<Shown<u32>>) -> bool {
    shown.as_ref().is_some_and(|shown| shown.fresh)
}

fn face(name: &str) -> StackFace {
    StackFace::from_static(name, b"", 0)
}

#[test]
fn stack_facts_are_written_only_when_the_stack_moves() {
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let main = rig.main;
    let tokens = rig.driver.runtime.env(main).tokens;
    let cells = EnvCells::in_store(rig.driver.runtime.store_mut(), tokens);
    let mut seen = Seen::default();
    let settled = (0..6).any(|_| !step(&mut rig, cells, &mut seen).ran);
    assert!(settled, "the rig did not settle");
    let version = |rig: &Rig<RecordingRenderer>| cells.fonts.version(rig.driver.runtime.store());
    let before = version(&rig);
    for _ in 0..3 {
        step(&mut rig, cells, &mut seen);
    }
    assert_eq!(version(&rig), before, "no write to the facts cell while the stack stays");
    assert_eq!(rig.driver.runtime.store().peek(cells.fonts).line_ratio, 0.0, "no stack, no line ratio");

    let mut stack = FontStack::default();
    stack.set_family(FontFace::Slot(FontSlot::Ui), vec![face("ui")]);
    stack.set_family(FontFace::Slot(FontSlot::UiSemibold), vec![face("ui-semibold")]);
    rig.driver.renderer.text().set_stack(&stack);
    let changed = step(&mut rig, cells, &mut seen);
    let facts = *rig.driver.runtime.store().peek(cells.fonts);
    assert!(facts.semibold && !facts.tab_title, "the facts are the new stack's: {facts:?}");
    assert_eq!(facts.line_ratio, 1.25, "measured with the engine's probe layout");
    assert!(fresh(&seen.reader), "the scope that read the env ran again: {changed:?}");
    assert!(!fresh(&seen.quiet), "the scope that reads no env did not: {changed:?}");
    let written = version(&rig);
    assert_ne!(written, before);

    for _ in 0..3 {
        let idle = step(&mut rig, cells, &mut seen);
        assert_eq!((idle.frame.runs, idle.frame.scopes), (0, 0), "nothing reruns once the facts are measured: {idle:?}");
    }
    assert_eq!(version(&rig), written, "and the facts cell stays as written");
}

mod served {
    use std::sync::{Arc, Mutex};

    use craft_fonts_build::ServedFace;
    use craft_theme::FontSlot;
    use craft_ui_contract::fonts::{FetchDone, FetchWake, FontFetcher, FontHost, I18nScripts, SystemFonts, UiFonts, default_stack};
    use craft_ui_rt::record::FixedText;
    use craft_ui_rt::{FontFace, TextEngine};

    const ARABIC: &[u8] = include_bytes!("../fonts/NotoSansArabic-test.ttf");
    const URL: &str = "fonts/noto-sans-arabic/NotoSansArabic.ttf";
    /// Coverage of every character.
    const ALL: &[u8] = &[0, 0xFF, 0xFF, 0x43];
    const SERVED: &[ServedFace] =
        &[ServedFace { family: "Noto Sans Arabic", style: "Regular", scripts: &["Arab"], index: 0, url: URL, size: 1, integrity: "", coverage: ALL }];

    /// Fetches nothing until the trial answers.
    #[derive(Default)]
    struct Fetch(Mutex<Vec<(&'static str, FetchDone)>>);

    impl FontFetcher for Fetch {
        fn fetch(&self, url: &'static str, _integrity: &'static str, done: FetchDone) {
            self.0.lock().expect("lock").push((url, done));
        }

        fn now(&self) -> f64 {
            0.0
        }

        fn after(&self, _seconds: f64, _wake: FetchWake) {}
    }

    fn arabic_in(engine: &FixedText) -> bool {
        engine
            .stack()
            .and_then(|stack| stack.family(&FontFace::Slot(FontSlot::Ui)))
            .is_some_and(|faces| faces.iter().any(|face| face.name.contains("Arabic") && !face.data().is_empty()))
    }

    /// The UI language's served face is fetched at install; until it arrives the given stack lacks it. Its arrival
    /// offers the stack once: the engine's generation moves by one, the stack carries the face, and a frame after that
    /// gives nothing more.
    #[test]
    fn a_served_face_arriving_moves_the_generation_once() {
        let fetch = Arc::new(Fetch::default());
        let latin = default_stack().family(&FontFace::Slot(FontSlot::Ui)).expect("the suite's interface family")[0].clone();
        let host = FontHost::new(None);
        UiFonts::new(&I18nScripts)
            .app_face("ui", latin, &[craft_ui_contract::fonts::FontRef::proportional()])
            .served_faces(SERVED)
            .languages(&["en", "ar"])
            .ui_language("ar")
            .system_fonts(SystemFonts::Off)
            .fetcher(Arc::clone(&fetch) as Arc<dyn FontFetcher>)
            .install(&host);
        let (mut engine, mut store) = (FixedText::default(), craft_ui_rt::Store::default());
        assert!(host.give(&mut engine, &mut store), "the install offers a stack");
        assert_eq!(engine.stack_generation(), 1);
        assert!(!arabic_in(&engine), "the served face is not there before it arrives");
        assert!(host.loading(), "the UI language's face is being fetched");
        assert!(!host.give(&mut engine, &mut store), "nothing more is offered while it is fetched");

        let (url, done) = fetch.0.lock().expect("lock").pop().expect("the Arabic face is fetched at install");
        assert_eq!(url, URL);
        done(Ok(ARABIC.to_vec()));
        assert!(!host.loading());
        assert!(host.give(&mut engine, &mut store), "the arrival offers the stack");
        assert_eq!(engine.stack_generation(), 2, "one arrival moves the generation once");
        assert!(arabic_in(&engine), "the given stack sets Arabic in the arrived face");
        assert!(!host.give(&mut engine, &mut store), "and offers nothing more");
        assert_eq!(engine.stack_generation(), 2);
    }
}

/// Document faces on the session's registry: a load drawn alone stops when its last asker leaves, and eviction past
/// the budget takes only the faces no node reads.
mod documents {
    use std::sync::{Arc, Mutex, OnceLock};

    use craft_ui_contract::contract::{MemoView, Ui};
    use craft_ui_contract::fonts::{FaceBytes, FaceState, FontHost, default_stack};
    use craft_ui_contract::primitives::FacesCx;
    use craft_ui_contract::runtime::env::EnvCells;
    use craft_ui_contract::runtime::pending::JobBoard;
    use craft_ui_contract::runtime::windows::{WindowEnv, window_root_with};
    use craft_ui_contract::testing::document_face_budget;
    use craft_ui_rt::record::{FixedText, RecordingRenderer};
    use craft_ui_rt::{FaceInstall, Pos2, Rect, StepReport, TextEngine, TextLayout, TextSpec, Vec2};
    use craft_ui_test_kit::rt::Rig;

    /// A text engine that installs every face it is given and logs what it takes out.
    #[derive(Default)]
    struct Faces {
        text: FixedText,
        installed: Vec<String>,
        removed: Vec<String>,
    }

    impl TextEngine for Faces {
        fn layout(&mut self, spec: &TextSpec, scale: f32) -> TextLayout {
            self.text.layout(spec, scale)
        }

        fn hit_index(&mut self, layout: &TextLayout, pos: Pos2) -> usize {
            self.text.hit_index(layout, pos)
        }

        fn caret_rect(&mut self, layout: &TextLayout, index: usize) -> Rect {
            self.text.caret_rect(layout, index)
        }

        fn install_face(&mut self, face: FaceInstall) -> bool {
            self.installed.push(face.family.to_string());
            true
        }

        fn has_face(&self, family: &str) -> bool {
            self.installed.iter().any(|name| name == family)
        }

        fn remove_face(&mut self, family: &str) -> bool {
            let had = self.has_face(family);
            self.installed.retain(|name| name != family);
            self.removed.push(family.to_owned());
            had
        }
    }

    /// Jobs that run when the trial says so.
    #[derive(Clone, Default)]
    struct Manual(Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>);

    impl craft_host::Jobs for Manual {
        fn spawn(&self, _: &str, job: Box<dyn FnOnce() + Send>) {
            self.0.lock().expect("lock").push(job);
        }
    }

    impl Manual {
        fn run_all(&self) {
            let jobs = std::mem::take(&mut *self.0.lock().expect("lock"));
            for job in jobs {
                job();
            }
        }
    }

    /// The keys whose load was made (`job` called) and whose load ran, in order; each trial asks for its own keys.
    static MADE: Mutex<Vec<char>> = Mutex::new(Vec::new());
    static LOADED: Mutex<Vec<char>> = Mutex::new(Vec::new());

    fn count(log: &Mutex<Vec<char>>, key: char) -> usize {
        log.lock().expect("lock").iter().filter(|&&made| made == key).count()
    }

    fn font() -> &'static [u8] {
        static FONT: OnceLock<Vec<u8>> = OnceLock::new();
        FONT.get_or_init(|| default_stack().families[0].1[0].data().to_vec())
    }

    /// Asks for the document face `key` drawn alone: 0 unavailable, 1 loading, 2 ready. Each load is a file of its own.
    fn asker(key: &mut char, ui: &mut Ui) -> u32 {
        let key = *key;
        let state = ui.document_face(&key.to_string(), || {
            MADE.lock().expect("lock").push(key);
            move || {
                LOADED.lock().expect("lock").push(key);
                Some(FaceBytes::new(Arc::from(font()), 0))
            }
        });
        match state {
            FaceState::Ready(_) => 2,
            FaceState::Loading => 1,
            _ => 0,
        }
    }

    struct World {
        rig: Rig<RecordingRenderer>,
        cells: EnvCells,
        board: JobBoard,
        queue: Manual,
        host: FontHost,
        engine: Faces,
        shown: Vec<char>,
        states: Vec<(char, u32)>,
    }

    impl World {
        fn new() -> World {
            let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
            let tokens = rig.driver.runtime.env(rig.main).tokens;
            let cells = EnvCells::in_store(rig.driver.runtime.store_mut(), tokens);
            let queue = Manual::default();
            let board = JobBoard::new(Arc::new(queue.clone()), rig.driver.waker());
            let deliver = board.clone();
            rig.driver.set_wake_handler(move |store, images| {
                deliver.deliver(store, images);
            });
            let host = FontHost::new(Some(rig.driver.waker()));
            World { rig, cells, board, queue, host, engine: Faces::default(), shown: Vec::new(), states: Vec::new() }
        }

        /// One frame, the host's hand-over first, as an app's host runs it.
        fn frame(&mut self) -> StepReport {
            self.host.give(&mut self.engine, self.rig.driver.runtime.store_mut());
            let env = WindowEnv { cells: Some(self.cells), jobs: Some(self.board.clone()), commands: None };
            let (host, shown, states) = (self.host.clone(), self.shown.clone(), &mut self.states);
            self.rig.frame_with(0.0, &mut |_, cx| {
                cx.provide(host.clone());
                window_root_with(cx, env.clone(), |ui| {
                    for &key in &shown {
                        if let Some(state) = MemoView::new(key, key, asker).show(ui) {
                            states.retain(|(seen, _)| *seen != key);
                            states.push((key, state.value));
                        }
                    }
                })
            })
        }

        /// A few frames: a landing takes two (the delivery wakes the host, whose next hand-over gives the face), and a
        /// wake waiting in the platform shows in no report, so the trial does not stop at the first quiet frame.
        fn settle(&mut self) {
            for _ in 0..6 {
                self.frame();
            }
        }

        /// Shows an asker for each of `keys` from the next frame on.
        fn show(&mut self, keys: &[char]) {
            self.shown = keys.to_vec();
            let main = self.rig.main;
            self.rig.driver.runtime.mark_root(main);
            self.settle();
        }

        /// Runs the waiting loads, then frames until their faces are given.
        fn land(&mut self) {
            self.queue.run_all();
            self.settle();
        }

        fn state(&self, key: char) -> u32 {
            self.states.iter().find(|(seen, _)| *seen == key).map_or(0, |(_, state)| *state)
        }
    }

    /// A load waits on the board while its asker shows; the asker leaving cancels it, so it never runs, and the registry
    /// forgets the face: asking again makes a new load.
    #[test]
    fn a_face_load_stops_when_its_last_asker_leaves() {
        let mut world = World::new();
        world.show(&['x']);
        assert_eq!(world.state('x'), 1, "the face loads");
        assert_eq!(count(&MADE, 'x'), 1);

        world.show(&[]);
        world.land();
        assert_eq!(count(&LOADED, 'x'), 0, "the cancelled load never runs");
        assert!(world.engine.installed.is_empty(), "nothing lands");

        world.show(&['x']);
        assert_eq!(count(&MADE, 'x'), 2, "asking again makes a new load");
        world.land();
        assert_eq!(count(&LOADED, 'x'), 1);
        assert_eq!(world.state('x'), 2, "the face is ready in the frame it is given");
    }

    /// Past the budget, eviction takes only the faces no node reads; faces in use stay even while the held files pass
    /// the budget.
    #[test]
    fn eviction_takes_only_unread_faces() {
        let mut world = World::new();
        let file = font().len();
        document_face_budget(&world.host, file * 5 / 2);
        world.show(&['a', 'b']);
        world.land();
        assert_eq!((world.state('a'), world.state('b')), (2, 2), "both faces are ready");
        assert!(world.engine.removed.is_empty(), "two files fit the budget");

        // `a` loses its asker; `c` lands and takes the held files past the budget.
        world.show(&['b', 'c']);
        world.land();
        assert_eq!(world.state('c'), 2);
        assert_eq!(world.engine.removed, ["face:a"], "only the unread face goes");
        assert!(world.engine.has_face("face:b") && world.engine.has_face("face:c"), "faces in use stay");

        // Both remaining faces are read: a fourth landing takes none of them, though the files pass the budget.
        world.show(&['b', 'c', 'd']);
        world.land();
        assert_eq!(world.state('d'), 2);
        assert_eq!(world.engine.removed, ["face:a"], "a face in use is never taken out");
    }
}
