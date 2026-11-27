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
        let mut engine = FixedText::default();
        assert!(host.give(&mut engine), "the install offers a stack");
        assert_eq!(engine.stack_generation(), 1);
        assert!(!arabic_in(&engine), "the served face is not there before it arrives");
        assert!(host.loading(), "the UI language's face is being fetched");
        assert!(!host.give(&mut engine), "nothing more is offered while it is fetched");

        let (url, done) = fetch.0.lock().expect("lock").pop().expect("the Arabic face is fetched at install");
        assert_eq!(url, URL);
        done(Ok(ARABIC.to_vec()));
        assert!(!host.loading());
        assert!(host.give(&mut engine), "the arrival offers the stack");
        assert_eq!(engine.stack_generation(), 2, "one arrival moves the generation once");
        assert!(arabic_in(&engine), "the given stack sets Arabic in the arrived face");
        assert!(!host.give(&mut engine), "and offers nothing more");
        assert_eq!(engine.stack_generation(), 2);
    }
}
