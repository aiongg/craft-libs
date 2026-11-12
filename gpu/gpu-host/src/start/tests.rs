use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use super::ladder::{Os, backends_for, gl_beside, ladder_for, top};
use super::record::{self, enter_on};
use super::restart::in_graphics_stack;
use super::watch::{Sample, Verdict, Watch};
use super::*;
use crate::adapter::SoftwareRule;
use crate::limits::LimitsTier;

const V: &str = "1.0.0";

fn choice() -> Choice {
    Choice { forced: None, preference: "automatic".to_string(), version: V }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("craft-gpu-host-start-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn launch(dir: &std::path::Path) -> GraphicsStart {
    GraphicsStart::begin_at(Some(dir), choice(), None, false)
}

/// The process dies: its lock goes, its record stays as it was.
fn die(start: GraphicsStart) {
    if let Some(dir) = start.held_folder() {
        record::release(dir);
    }
}

fn saved_rung(dir: &std::path::Path) -> Option<Rung> {
    record::read(dir).and_then(|r| r.rung)
}

/// Each platform's rungs and what each one starts with: one table instead of a test per platform
/// and combination (FilmCraft, FigCraft, PhotoCraft, LightCraft ladders). Windows starts on DirectX
/// 12 alone (Vulkan loads every driver: VectorCraft #806), reaches OpenGL after Vulkan (AMD's
/// `atio6axx.dll`: FilmCraft #477), and without a DirectX 12 GPU tries WARP before OpenGL
/// (PhotoCraft #5191). The software rung takes only software adapters, the others none; only the
/// downlevel rung asks for less; no rung validates indirect calls (PhotoCraft #3107).
#[test]
fn each_platform_ladder() {
    use wgpu::Backends as B;
    let hardware = || true;
    assert_eq!(ladder_for(Os::Windows, hardware), [Rung::Dx12, Rung::Vulkan, Rung::Gl, Rung::Downlevel, Rung::Software]);
    assert_eq!(ladder_for(Os::Windows, || false), [Rung::Dx12, Rung::Vulkan, Rung::Software, Rung::Gl, Rung::Downlevel]);
    assert_eq!(ladder_for(Os::Mac, hardware), [Rung::Metal, Rung::Downlevel]);
    assert_eq!(ladder_for(Os::Other, hardware), [Rung::Native, Rung::Gl, Rung::Downlevel, Rung::Software]);
    let table: &[(Os, Rung, B)] = &[
        (Os::Windows, Rung::Dx12, B::DX12),
        (Os::Windows, Rung::Vulkan, B::VULKAN),
        (Os::Windows, Rung::Gl, B::GL),
        (Os::Windows, Rung::Downlevel, B::DX12),
        (Os::Windows, Rung::Software, B::DX12),
        (Os::Mac, Rung::Metal, B::METAL),
        (Os::Mac, Rung::Downlevel, B::METAL),
        (Os::Other, Rung::Native, B::VULKAN.union(B::GL)),
        (Os::Other, Rung::Gl, B::GL),
        (Os::Other, Rung::Downlevel, B::VULKAN.union(B::GL)),
        (Os::Other, Rung::Software, B::VULKAN.union(B::GL)),
    ];
    for &(os, rung, backends) in table {
        assert_eq!(backends_for(rung, os, false), backends, "{os:?} {rung:?}");
        let attempt = Attempt { rung, skip: Vec::new(), chosen: None };
        let software = if rung == Rung::Software { SoftwareRule::Only } else { SoftwareRule::Exclude };
        assert_eq!(rung.software_rule(), software, "{rung:?}");
        let tier = if rung == Rung::Downlevel { LimitsTier::Downlevel } else { LimitsTier::Adapter };
        assert_eq!(attempt.device_request().tier, tier, "{rung:?}");
        assert_eq!(Rung::parse(rung.name()), Some(rung));
    }
    if std::env::var_os(crate::env::VALIDATION_INDIRECT_CALL).is_none() {
        let flags = crate::access::instance_descriptor(B::GL).flags;
        assert!(!flags.contains(wgpu::InstanceFlags::VALIDATION_INDIRECT_CALL));
    }
}

/// VectorCraft #4150: wgpu's OpenGL backend hung the app on a keyboard-layout switch, so Windows'
/// DirectX 12 rung lists OpenGL only once it leaves an adapter out or one is named; a machine
/// without a DirectX 12 GPU reaches OpenGL on its own rung, after WARP.
#[test]
fn windows_lists_opengl_once_a_start_leaves_an_adapter_out() {
    assert!(!gl_beside(false, false));
    assert!(gl_beside(true, false));
    assert!(gl_beside(false, true));
    assert_eq!(backends_for(Rung::Dx12, Os::Windows, true), wgpu::Backends::DX12 | wgpu::Backends::GL);
    let warp_only = ladder_for(Os::Windows, || false);
    let at = |r: Rung| warp_only.iter().position(|x| *x == r);
    assert!(at(Rung::Software) < at(Rung::Gl), "WARP before OpenGL without a DirectX 12 GPU");
}

/// FilmCraft #2581: a saved software rung where no software adapter exists goes back up to the
/// top instead of ending every later start; down the ladder first.
#[test]
fn a_failed_fallback_goes_back_up_to_a_rung_not_tried() {
    let win = ladder_for(Os::Windows, || true);
    assert_eq!(next_untried(win, Rung::Software, &[Rung::Software]), Some(Rung::Dx12));
    assert_eq!(next_untried(win, Rung::Dx12, &[Rung::Dx12]), Some(Rung::Vulkan));
    assert_eq!(next_untried(win, Rung::Software, &[Rung::Vulkan, Rung::Software]), Some(Rung::Dx12));
    assert_eq!(next_untried(win, Rung::Downlevel, win), None, "every rung tried");
    assert_eq!(next_untried(ladder_for(Os::Mac, || true), Rung::Downlevel, &[Rung::Metal, Rung::Downlevel]), None);
}

/// VectorCraft #2243: a driver crash before the first frame leaves that adapter out of the next
/// start, and of every start after, until a start on it shows a frame. A start that showed its
/// first frame or ended leaves nothing behind.
#[test]
fn a_start_that_never_showed_a_frame_leaves_its_adapter_out_of_the_next() {
    let dir = temp_dir("crash");
    let first = launch(&dir);
    assert!(first.attempt().skip.is_empty());
    first.picked("Dx12:8086:46a6", "Intel(R) UHD Graphics", true);
    die(first);
    let second = launch(&dir);
    assert_eq!(second.attempt().skip, vec!["Dx12:8086:46a6".to_string()]);
    second.picked("Dx12:10de:2204", "NVIDIA GeForce RTX 3090", true);
    second.ended();
    let third = launch(&dir);
    assert_eq!(third.attempt().skip, vec!["Dx12:8086:46a6".to_string()], "still left out; the clean start cleared only itself");
    // The user names the crashed adapter, and it shows a frame: no longer left out.
    third.picked("Dx12:8086:46a6", "Intel(R) UHD Graphics", true);
    third.presented();
    let fourth = launch(&dir);
    assert!(fourth.attempt().skip.is_empty());
    fourth.ended();
    let _ = std::fs::remove_dir_all(&dir);
}

/// A start that died while listing the adapters, before one was picked, moves the next launch to
/// the next rung; when the last rung dies that way, the ladder starts over (DesignCraft).
#[test]
fn a_crash_while_listing_moves_on_and_the_ladder_starts_over_at_its_end() {
    let dir = temp_dir("listing");
    let mut start = launch(&dir);
    for expected in ladder().iter().skip(1) {
        die(start);
        start = launch(&dir);
        assert_eq!(start.attempt().rung, *expected);
        assert!(start.degraded());
    }
    die(start);
    let again = launch(&dir);
    assert_eq!(again.attempt().rung, top(), "after the last rung, the top again");
    again.ended();
    let _ = std::fs::remove_dir_all(&dir);
}

/// A second copy of the app starting while the first holds the record neither counts the first
/// as crashed nor disturbs it; a lock held only briefly (a child between `fork` and `exec`) is
/// waited for (PhotoCraft).
#[test]
fn a_second_launch_records_nothing_and_a_briefly_held_lock_is_waited_for() {
    let dir = temp_dir("lock");
    let first = launch(&dir);
    first.picked("Vulkan:1002:164e", "AMD Radeon", true);
    let held = std::fs::File::open(dir.join(FILE)).expect("record");
    die(first);
    held.lock().expect("lock");
    let second = launch(&dir);
    assert!(second.attempt().skip.is_empty() && second.held_folder().is_none(), "records nothing, leaves nothing out");
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        held.unlock().expect("unlock");
    });
    let third = launch(&dir);
    release.join().expect("release");
    assert_eq!(third.attempt().skip, vec!["Vulkan:1002:164e".to_string()], "the first start's crash, once the lock went");
    third.ended();
    let _ = std::fs::remove_dir_all(&dir);
}

/// A damaged, huge or foreign record starts from the top; a data folder that can't be written
/// still starts, and its reset says why it failed.
#[test]
fn corrupt_huge_or_foreign_files_start_from_the_top() {
    let dir = temp_dir("corrupt");
    std::fs::create_dir_all(&dir).expect("dir");
    let huge = format!(r#"{{"version":"{V}","preference":"automatic","rung":"software","pad":"{}"}}"#, "x".repeat(100_000));
    let foreign = format!(r#"{{"version":"{V}","preference":"automatic","rung":"no-such-rung","crashed":[7]}}"#);
    for text in ["garbage", huge.as_str(), foreign.as_str()] {
        std::fs::write(dir.join(FILE), text).expect("write");
        let start = launch(&dir);
        assert_eq!(start.attempt().rung, top(), "{}", &text[..text.len().min(40)]);
        assert!(start.attempt().skip.is_empty());
        start.ended();
        assert!(record::read(&dir).is_some(), "replaced with a well-formed record");
    }
    let file = dir.join("not-a-folder");
    std::fs::write(&file, "x").expect("write");
    let start = GraphicsStart::begin_at(Some(&file), choice(), None, false);
    assert_eq!(start.attempt().rung, top());
    start.presented();
    assert!(start.reset_to_default().is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The crashes and the saved rung count only for the app version and Graphics Processor
/// preference they were recorded under (VectorCraft, DesignCraft); `WGPU_BACKEND` leaves the
/// record alone.
#[test]
fn crashes_are_forgotten_in_another_version_or_under_another_preference() {
    let dir = temp_dir("forget");
    let first = launch(&dir);
    first.picked("Metal:Apple M1", "Apple M1", true);
    die(first);
    let explicit = GraphicsStart::begin_at(Some(&dir), choice(), None, true);
    assert!(explicit.attempt().skip.is_empty() && explicit.held_folder().is_none());
    assert!(record::read(&dir).and_then(|r| r.starting).is_some(), "the record is left alone");
    let other = GraphicsStart::begin_at(Some(&dir), Choice { preference: "highPerformance".to_string(), ..choice() }, None, false);
    assert!(other.attempt().skip.is_empty());
    other.ended();
    let newer = GraphicsStart::begin_at(Some(&dir), Choice { version: "1.0.1", ..choice() }, None, false);
    assert!(newer.attempt().skip.is_empty());
    newer.ended();
    let _ = std::fs::remove_dir_all(&dir);
}

/// The notice's reset, from a clone of the start, wins over the first frame that follows: the
/// next launch starts from the top (FilmCraft).
#[test]
fn reset_to_default_wins_over_a_later_first_frame() {
    let dir = temp_dir("reset");
    let last = *ladder().last().unwrap_or(&top());
    let forced = GraphicsStart::begin_at(Some(&dir), Choice { forced: Some((last, Keep::Saved)), ..choice() }, None, false);
    forced.presented();
    let start = launch(&dir);
    assert_eq!(start.attempt().rung, last, "a rung picked by hand is kept");
    assert_eq!(start.clone().reset_to_default(), Ok(()));
    start.presented();
    assert_eq!(saved_rung(&dir), None);
    assert_eq!(launch(&dir).attempt().rung, top());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Errors wgpu reports before any frame walk down the ladder in this process, saving each rung at
/// once (FilmCraft), until every rung failed: then the record starts over.
#[test]
fn reported_errors_walk_the_ladder_and_save_each_rung_at_once() {
    let dir = temp_dir("step");
    let start = launch(&dir);
    for expected in ladder().iter().skip(1) {
        match start.next_after(Failure::Reported("no adapter".to_string()), 0, false) {
            Next::Retry(a) => assert_eq!(a.rung, *expected),
            other => panic!("expected a retry on {expected:?}, got {other:?}"),
        }
        assert_eq!(saved_rung(&dir), Some(*expected));
    }
    assert!(matches!(start.next_after(Failure::Reported("no adapter".to_string()), 0, false), Next::GiveUp(_)));
    assert_eq!(start.failures().len(), ladder().len());
    assert_eq!(saved_rung(&dir), None, "the next launch starts over");
    start.ended();
    let _ = std::fs::remove_dir_all(&dir);
}

/// The merged restart model, one row per event: what follows each failure (VectorCraft #502,
/// DesignCraft, FigCraft, FilmCraft #2553).
#[test]
fn restart_sequences() {
    let dir = temp_dir("restart");
    let top = top();
    let second = ladder().get(1).copied();
    let adapter = "Vulkan:10de:2204";

    // A graphics panic on the first frames: the same rung without the adapter.
    let start = launch(&dir);
    start.picked(adapter, "NVIDIA", true);
    let next = start.next_after(Failure::Panicked("surface".to_string()), 1, true);
    assert_eq!(next, Next::Restart(RestartState { rung: top, skip: vec![adapter.to_string()], tried: Vec::new(), documents: Vec::new() }));
    start.ended();

    // A panic elsewhere is a bug, and a failure after the start-up frames isn't the adapter's.
    let start = launch(&dir);
    start.picked(adapter, "NVIDIA", true);
    assert_eq!(start.next_after(Failure::Panicked("index out of bounds".to_string()), 1, false), Next::GiveUp("index out of bounds".to_string()));
    assert!(matches!(start.next_after(Failure::Reported("lost".to_string()), STARTUP_FRAMES, false), Next::GiveUp(_)));
    start.ended();

    // The restarted start hangs on the next adapter... and then on none left: the next rung, with
    // nothing left out. Its rung is not saved: a later launch may try it again (DesignCraft).
    let restart = RestartState { rung: top, skip: vec![adapter.to_string()], tried: Vec::new(), documents: Vec::new() };
    let start = GraphicsStart::begin_at(Some(&dir), choice(), Some(restart.clone()), false);
    assert_eq!(start.attempt().skip, restart.skip);
    start.picked(adapter, "NVIDIA", true);
    match (start.next_after(Failure::Hung, 0, false), second) {
        (Next::Restart(s), Some(second)) => assert_eq!((s.rung, s.skip.is_empty(), s.tried), (second, true, vec![top])),
        (Next::GiveUp(_), None) => {}
        (other, _) => panic!("{other:?}"),
    }
    start.ended();
    if let Some(second) = second {
        let moved = RestartState { rung: second, skip: Vec::new(), tried: vec![top], documents: Vec::new() };
        let start = GraphicsStart::begin_at(Some(&dir), choice(), Some(moved), false);
        assert_eq!(start.attempt().rung, second);
        assert_eq!(saved_rung(&dir), None, "a restart's rung is not recorded");
        start.ended();
    }

    // `WGPU_BACKEND` or `WGPU_ADAPTER_NAME`: one attempt.
    let explicit = GraphicsStart::begin_at(Some(&dir), choice(), None, true);
    assert!(matches!(explicit.next_after(Failure::Reported("no adapter".to_string()), 0, false), Next::GiveUp(_)));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The restart state travels as one argument, taken out before the app parses its own.
#[test]
fn a_restart_state_travels_as_one_argument() {
    let state =
        RestartState { rung: Rung::Gl, skip: vec!["Metal:Intel Iris Pro, Graphics".to_string()], tried: vec![Rung::Dx12], documents: Vec::new() };
    let mut args: Vec<std::ffi::OsString> = vec!["app".into(), "a.svg".into(), state.to_arg().into(), format!("{RESTART_ARG}garbage").into()];
    assert_eq!(RestartState::take_from_args(&mut args), Some(state));
    assert_eq!(args, vec![std::ffi::OsString::from("app"), "a.svg".into()]);
}

/// The panics that restart without the adapter come from wgpu, egui-wgpu or naga's sources.
#[test]
fn panics_in_wgpu_egui_wgpu_and_naga_are_graphics_failures() {
    assert!(in_graphics_stack("/home/u/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-core-30.0.1/src/device/global.rs"));
    assert!(in_graphics_stack(r"C:\Users\u\.cargo\git\checkouts\egui-3f56db2\egui-wgpu-0.33.0\src\winit.rs"));
    assert!(in_graphics_stack("/cargo/naga-30.0.0/src/back/msl/writer.rs"));
    assert!(!in_graphics_stack("apps/vectorcraft/src/gpu.rs"));
    assert!(!in_graphics_stack("/cargo/registry/src/x/egui-0.33.0/src/context.rs"));
}

fn sample(frames: u64, busy: bool, leaves: u64, hidden: bool) -> Sample {
    Sample { frames, busy, leaves, hidden }
}

/// DesignCraft: a UI thread stuck outside the app's code with no progress for the hang timeout
/// hangs its adapter; a late first frame is only reported, once.
#[test]
fn a_ui_thread_stuck_in_the_graphics_stack_hangs_its_adapter() {
    let mut w = Watch::default();
    let tick = Duration::from_secs(1);
    let mut verdicts = Vec::new();
    for _ in 0..=HANG_TIMEOUT.as_secs() {
        verdicts.push(w.step(sample(1, false, 3, false), tick));
    }
    assert_eq!(verdicts.iter().filter(|v| **v == Verdict::Late).count(), 1);
    assert_eq!(verdicts.last(), Some(&Verdict::Hung));
    assert_eq!(Watch::default().step(sample(2, false, 0, false), tick), Verdict::Shown);
}

/// DesignCraft #5160: a UI thread busy in the app's code, or one that keeps leaving it, never
/// hangs its adapter, however long the first frame takes.
#[test]
fn a_busy_ui_thread_never_hangs_its_adapter() {
    let mut w = Watch::default();
    let tick = Duration::from_secs(1);
    for i in 0..HANG_TIMEOUT.as_secs() * 3 {
        assert_ne!(w.step(sample(0, true, 0, false), tick), Verdict::Hung);
        assert_ne!(w.step(sample(1, false, i, false), tick), Verdict::Hung);
    }
}

/// A window that is minimized, occluded or unfocused may have its frames held back by the
/// compositor: that time doesn't count towards a hang.
#[test]
fn time_while_the_window_is_hidden_does_not_count() {
    let mut w = Watch::default();
    let tick = Duration::from_secs(1);
    for _ in 0..HANG_TIMEOUT.as_secs() - 1 {
        assert_ne!(w.step(sample(1, false, 0, false), tick), Verdict::Hung);
    }
    assert_eq!(w.step(sample(1, false, 0, true), tick), Verdict::Wait);
    assert_ne!(w.step(sample(1, false, 0, false), tick), Verdict::Hung, "the stuck time started over");
}

/// LightCraft #136: overlapping record scopes, on any thread, share one entry until the last
/// ends, and the next launch hears of a process that died inside one, once.
#[test]
fn overlapping_scopes_share_one_entry_until_the_last_ends() {
    static ACTIVE: Mutex<u32> = Mutex::new(0);
    let dir = temp_dir("scope");
    let outer = enter_on(Some(dir.clone()), &ACTIVE, "device creation");
    let inner = enter_on(Some(dir.clone()), &ACTIVE, "kernel compilation");
    let entry = record::read(&dir).and_then(|r| r.scope).map(|s| s.what);
    assert_eq!(entry.as_deref(), Some("device creation"));
    drop(outer);
    assert!(record::read(&dir).and_then(|r| r.scope).is_some(), "one scope still runs");
    // The process dies inside the inner scope: its entry stays.
    std::mem::forget(inner);
    let crashed = Record::take_crashed_scope(&dir);
    assert!(crashed.is_some_and(|c| c.starts_with("device creation")));
    assert_eq!(Record::take_crashed_scope(&dir), None, "reported once");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A panic inside a scope still ends it: the process survived.
#[test]
fn a_panic_inside_the_scope_still_removes_the_entry() {
    static ACTIVE: Mutex<u32> = Mutex::new(0);
    let dir = temp_dir("scope-panic");
    let caught = std::panic::catch_unwind(|| {
        let _scope = enter_on(Some(dir.clone()), &ACTIVE, "pipeline compilation");
        panic!("driver work failed");
    });
    assert!(caught.is_err());
    assert!(record::read(&dir).and_then(|r| r.scope).is_none());
    assert_eq!(*ACTIVE.lock().unwrap_or_else(std::sync::PoisonError::into_inner), 0);
    let _ = std::fs::remove_dir_all(&dir);
}
