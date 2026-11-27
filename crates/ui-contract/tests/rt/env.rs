//! Count trials of `Env::of` on craft-ui-rt (INT-3): a write to an env cell reruns the scopes that
//! called `Env::of` and no others. Relations only, no times.

use craft_ui_contract::contract::{MemoView, Shown, Ui};
use craft_ui_contract::runtime::env::{Env, EnvCells};
use craft_ui_contract::runtime::windows::{WindowEnv, window_root_with};
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{StepReport, Vec2};
use craft_ui_test_kit::rt::Rig;

/// What each scope returned in the last build, and whether its closure ran there.
#[derive(Default)]
struct Seen {
    reader: Option<Shown<u64>>,
    quiet: Option<Shown<u64>>,
    other: Option<Shown<u64>>,
}

/// Reads the env: the key of the tokens it resolves to.
fn reader(_: &mut (), ui: &mut Ui) -> u64 {
    Env::of(ui).tokens.key()
}

/// Reads nothing of the env.
fn quiet(_: &mut (), _: &mut Ui) -> u64 {
    0
}

fn view(ui: &mut Ui, seen: &mut Seen) {
    seen.reader = MemoView::new("reader", (), reader).show(ui);
    seen.quiet = MemoView::new("quiet", (), quiet).show(ui);
    seen.other = MemoView::new("other", (), quiet).show(ui);
}

fn step(rig: &mut Rig<RecordingRenderer>, cells: EnvCells, seen: &mut Seen) -> StepReport {
    let host = WindowEnv { cells: Some(cells), jobs: None, commands: None };
    rig.frame_with(0.0, &mut |_, cx| window_root_with(cx, host.clone(), |ui| view(ui, seen)))
}

fn fresh(shown: &Option<Shown<u64>>) -> bool {
    shown.as_ref().is_some_and(|shown| shown.fresh)
}

#[test]
fn a_density_write_reruns_only_the_scopes_that_read_the_env() {
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let main = rig.main;
    let tokens = rig.driver.runtime.env(main).tokens;
    let cells = EnvCells::in_store(rig.driver.runtime.store_mut(), tokens);
    let mut seen = Seen::default();
    let mut settled = false;
    for _ in 0..6 {
        if !step(&mut rig, cells, &mut seen).ran {
            settled = true;
            break;
        }
    }
    assert!(settled, "the rig did not settle");
    let before = seen.reader.as_ref().map(|shown| shown.value);

    cells.density.set(rig.driver.runtime.store_mut(), craft_theme::Size::Comfortable);
    let changed = step(&mut rig, cells, &mut seen);
    assert!(fresh(&seen.reader), "the scope that called Env::of ran: {changed:?}");
    assert!(!fresh(&seen.quiet) && !fresh(&seen.other), "scopes that read no env value kept their value: {changed:?}");
    assert_ne!(seen.reader.as_ref().map(|shown| shown.value), before, "the reader saw the tokens of the new density");
    assert_eq!(changed.frame.runs, 0, "no component reads the env here: {changed:?}");

    let idle = step(&mut rig, cells, &mut seen);
    assert_eq!((idle.frame.runs, idle.frame.scopes), (0, 0), "nothing reruns once settled: {idle:?}");
}

/// `EnvCells::set_root` writes only the cells whose value moved: the same root values again rerun nothing, and a new
/// direction reruns the scopes that read the env and no others.
#[test]
fn set_root_reruns_only_the_readers_of_cells_that_changed() {
    use craft_ui_contract::runtime::env::RootEnv;
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let main = rig.main;
    let tokens = rig.driver.runtime.env(main).tokens;
    let cells = EnvCells::in_store(rig.driver.runtime.store_mut(), tokens);
    let mut root = RootEnv::default();
    cells.set_root(rig.driver.runtime.store_mut(), &root);
    let mut seen = Seen::default();
    let settled = (0..6).any(|_| !step(&mut rig, cells, &mut seen).ran);
    assert!(settled, "the rig did not settle");

    cells.set_root(rig.driver.runtime.store_mut(), &root);
    let same = step(&mut rig, cells, &mut seen);
    assert!(!fresh(&seen.reader) && !fresh(&seen.quiet), "the same root values write no cell: {same:?}");
    assert_eq!((same.frame.runs, same.frame.scopes), (0, 0), "nothing reruns for an unchanged root: {same:?}");

    root.direction = craft_i18n::Direction::Rtl;
    cells.set_root(rig.driver.runtime.store_mut(), &root);
    let moved = step(&mut rig, cells, &mut seen);
    assert!(fresh(&seen.reader), "the scope that called Env::of ran for the new direction: {moved:?}");
    assert!(!fresh(&seen.quiet) && !fresh(&seen.other), "scopes that read no env value kept their value: {moved:?}");
}
