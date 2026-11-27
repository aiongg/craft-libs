//! Count trials of the font stack facts on craft-ui-rt (INT-3f): the window root measures them again only when the
//! text engine's stack moves, and then only the nodes that read the env run again. Relations only, no times.

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
