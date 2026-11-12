use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::*;
use crate::start::{Choice, failure_message, ladder, read_record};

const WINDOWS: &[Rung] = &[Rung::Dx12, Rung::Vulkan, Rung::Gl, Rung::Downlevel, Rung::Software];
const LINUX: &[Rung] = &[Rung::Native, Rung::Gl, Rung::Downlevel, Rung::Software];
const NOW: u64 = 1_000_000;

fn test_record(rung: Rung, open: bool, started: u64) -> Test {
    Test { rung, drivers: "GPU 1.0".to_string(), started, open }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("craft-gpu-host-probe-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn launch(dir: &Path, forced: Option<Rung>) -> GraphicsStart {
    let choice = Choice { forced: forced.map(|r| (r, crate::start::Keep::Saved)), preference: "automatic".to_string(), version: "1.0.0" };
    GraphicsStart::begin_at(Some(dir), choice, None, false)
}

fn test_of(dir: &Path) -> Option<Test> {
    read_record(dir).and_then(|r| r.test)
}

/// A start that hung long ago in `rung`, and is still open.
fn stuck(dir: &Path, rung: Rung) {
    let t = test_record(rung, true, unix_now().saturating_sub(HUNG_AFTER.as_secs() * 2));
    update_held(dir, |r| r.test = Some(t));
}

/// A fake test start that counts the rungs it was asked to test.
fn counting(outcome: impl Fn(Rung) -> Outcome) -> (Rc<RefCell<Vec<Rung>>>, impl Fn(&Attempt) -> Outcome) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&seen);
    (seen, move |a: &Attempt| {
        log.borrow_mut().push(a.rung);
        outcome(a.rung)
    })
}

fn hung_at(stage: &str) -> Outcome {
    Outcome::Hung(Report { stage: Some(stage.to_string()), ..Report::default() }, 25)
}

/// FilmCraft #1061: the rung is tested when anything changed, or the last start on it never got
/// its window; a start still open but young may be another copy of the app starting.
#[test]
fn a_start_tests_its_rung_when_anything_changed_or_the_last_start_hung() {
    let ok = test_record(Rung::Dx12, false, NOW - 3600);
    assert_eq!(need(None, Rung::Dx12, "GPU 1.0", NOW), Need::Test, "nothing known yet");
    assert_eq!(need(Some(&ok), Rung::Dx12, "GPU 1.0", NOW), Need::No, "nothing changed");
    assert_eq!(need(Some(&ok), Rung::Vulkan, "GPU 1.0", NOW), Need::Test, "another rung");
    assert_eq!(need(Some(&ok), Rung::Dx12, "GPU 1.1", NOW), Need::Test, "a driver update");
    let open = test_record(Rung::Dx12, true, NOW - HUNG_AFTER.as_secs());
    assert_eq!(need(Some(&open), Rung::Dx12, "GPU 1.0", NOW), Need::LastHung);
    let young = test_record(Rung::Dx12, true, NOW - 5);
    assert_eq!(need(Some(&young), Rung::Dx12, "GPU 1.0", NOW), Need::Test);
}

/// The rungs tested follow the ladder's fallback, a rung that hung doesn't come back, and a rung
/// picked by hand is the only one tested.
#[test]
fn the_rungs_tested_follow_the_fallback_and_skip_one_that_hung() {
    assert_eq!(candidates(WINDOWS, Rung::Dx12, false, false), WINDOWS);
    assert_eq!(candidates(WINDOWS, Rung::Dx12, true, false), &WINDOWS[1..], "the hung rung does not come back");
    assert_eq!(candidates(WINDOWS, Rung::Software, false, false), vec![Rung::Software, Rung::Dx12, Rung::Vulkan, Rung::Gl, Rung::Downlevel]);
    assert_eq!(candidates(WINDOWS, Rung::Gl, true, false), vec![Rung::Downlevel, Rung::Software, Rung::Dx12, Rung::Vulkan]);
    assert_eq!(candidates(LINUX, Rung::Native, false, false), LINUX);
    assert_eq!(candidates(WINDOWS, Rung::Vulkan, false, true), vec![Rung::Vulkan]);
}

/// The parent follows the child's lines, and says where a rung stopped and on which adapter.
#[test]
fn the_report_follows_the_childs_lines() {
    let mut r = Report::default();
    for line in [
        r#"{"stage":"instance"}"#,
        "not json",
        r#"{"stage":"adapters"}"#,
        r#"{"stage":"adapter","adapters":["NVIDIA GeForce RTX 3060 (Dx12, NVIDIA 617.14)","Microsoft Basic Render Driver (Dx12)"],"dx12_hardware":true}"#,
        r#"{"stage":"device","chosen":"NVIDIA GeForce RTX 3060 (Dx12, NVIDIA 617.14)"}"#,
    ] {
        r.take(line);
    }
    assert_eq!(r.stage.as_deref(), Some("device"));
    assert_eq!((r.adapters.len(), r.dx12_hardware), (2, Some(true)));
    assert!(!r.done && r.error.is_none());
    let text = outcome_text(&Outcome::Hung(r.clone(), 25));
    assert_eq!(
        text,
        "stopped responding while creating the graphics device on NVIDIA GeForce RTX 3060 (Dx12, NVIDIA 617.14), and was ended after 25 s"
    );
    r.take(r#"{"stage":"done"}"#);
    assert!(r.done);
    let mut failed = Report::default();
    failed.take(r#"{"stage":"adapter","adapters":["llvmpipe (Vulkan)"]}"#);
    failed.take(r#"{"stage":"error","error":"no software graphics adapter"}"#);
    assert_eq!(outcome_text(&Outcome::Failed(failed)), "no software graphics adapter (adapters: llvmpipe (Vulkan))");
    let died = Report { stage: Some("instance".to_string()), ..Report::default() };
    assert_eq!(outcome_text(&Outcome::Died(died, Some(-1073741819))), "ended unexpectedly while loading the graphics driver (exit code -1073741819)");
}

/// A command that writes `lines` and exits with `code`.
#[cfg(windows)]
fn lines_then_exit(lines: &[&str], code: i32) -> Command {
    use std::os::windows::process::CommandExt as _;
    // cmd's `echo` prints the rest of its command verbatim, quotes included.
    let script: String = lines.iter().map(|l| format!("echo {l}&")).collect();
    let mut cmd = Command::new("cmd");
    cmd.raw_arg(format!("/C {script}exit {code}"));
    cmd
}

/// A command that writes `lines` and exits with `code`.
#[cfg(not(windows))]
fn lines_then_exit(lines: &[&str], code: i32) -> Command {
    let script: String = lines.iter().map(|l| format!("printf '%s\\n' '{l}'; ")).collect();
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(format!("{script}exit {code}"));
    cmd
}

/// A command that runs for `secs` seconds without writing anything.
fn hangs_for(secs: u32) -> Command {
    let mut cmd = if cfg!(windows) { Command::new("ping") } else { Command::new("sleep") };
    if cfg!(windows) {
        cmd.args(["-n", &(secs + 1).to_string(), "127.0.0.1"]);
    } else {
        cmd.arg(secs.to_string());
    }
    cmd
}

/// A child that hangs is ended at the timeout, not waited for.
#[test]
fn a_test_start_that_hangs_is_ended_after_the_timeout() {
    let began = Instant::now();
    let outcome = run_test(hangs_for(30), Duration::from_secs(1));
    assert!(matches!(outcome, Outcome::Hung(_, 1)), "{outcome:?}");
    assert!(began.elapsed() < Duration::from_secs(15), "ended, not waited for: {:?}", began.elapsed());
}

/// A child passes, fails or dies as its lines and exit code say; one that can't start tells
/// nothing about the rung.
#[test]
fn a_test_start_passes_fails_or_dies_as_its_lines_and_exit_say() {
    let passed = run_test(lines_then_exit(&[r#"{"stage":"instance"}"#, r#"{"stage":"done"}"#], 0), Duration::from_secs(20));
    assert!(matches!(passed, Outcome::Passed(_)), "{passed:?}");
    let failed = run_test(lines_then_exit(&[r#"{"stage":"error","error":"no adapter"}"#], 1), Duration::from_secs(20));
    assert!(matches!(&failed, Outcome::Failed(r) if r.error.as_deref() == Some("no adapter")), "{failed:?}");
    let died = run_test(lines_then_exit(&[r#"{"stage":"device"}"#], 3), Duration::from_secs(20));
    assert!(matches!(&died, Outcome::Died(r, Some(3)) if r.stage.as_deref() == Some("device")), "{died:?}");
    let missing = run_test(Command::new("craft-gpu-host-no-such-program-for-the-test"), Duration::from_secs(20));
    assert!(matches!(missing, Outcome::NotRun(_)), "{missing:?}");
}

/// A rung that hangs in its test moves on to one that passes; that start's test record stays open
/// until it shows a frame or ends.
#[test]
fn a_hung_rung_moves_on_to_one_that_passes_and_the_record_closes_when_the_start_ends() {
    let dir = temp_dir("move-on");
    let start = launch(&dir, None);
    let first = start.attempt().rung;
    let (_, test) = counting(|r| if r == first { hung_at("device") } else { Outcome::Passed(Report::default()) });
    let passed = choose_with(&start, "GPU 1.0", test);
    if ladder().len() > 1 {
        assert_eq!(passed, Ok(()));
        assert_ne!(start.attempt().rung, first, "moved on from the rung that hung");
        let failures = start.failures();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].1.starts_with("stopped responding while creating the graphics device"), "{failures:?}");
        assert_eq!(test_of(&dir).map(|t| (t.rung, t.open)), Some((start.attempt().rung, true)), "open until the start shows a frame or ends");
        start.ended();
        assert_eq!(test_of(&dir).map(|t| t.open), Some(false));
    } else {
        assert!(passed.is_err(), "nothing to move on to");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// When no rung passes, the message names each rung and where it stopped.
#[test]
fn no_rung_passing_ends_in_a_message_naming_each_rung() {
    let dir = temp_dir("none");
    let start = launch(&dir, None);
    let (seen, test) = counting(|_| hung_at("instance"));
    let error = choose_with(&start, "", test).err().unwrap_or_default();
    assert_eq!(seen.borrow().len(), ladder().len(), "every rung tested");
    assert_eq!(start.failures().len(), ladder().len());
    let log = PathBuf::from("/data/Logs/app.log");
    let msg = failure_message("App", "1.0.0", &error, &start.failures(), Some(&log));
    for rung in ladder() {
        assert!(msg.contains(rung.label()), "{msg}");
    }
    assert!(msg.contains("stopped responding while loading the graphics driver") && msg.contains(&log.display().to_string()), "{msg}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A start tests once, then starts directly until something changes (a driver update).
#[test]
fn a_start_tests_once_then_starts_directly_until_something_changes() {
    let dir = temp_dir("once");
    for (drivers, tests) in [("GPU 1.0", 1), ("GPU 1.0", 0), ("GPU 2.0", 1)] {
        let start = launch(&dir, None);
        let (seen, test) = counting(|_| Outcome::Passed(Report::default()));
        assert_eq!(choose_with(&start, drivers, test), Ok(()));
        assert_eq!(seen.borrow().len(), tests, "drivers {drivers}");
        start.ended();
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A start that passed its test but never showed its window, and never came back, moves the next
/// launch on from its rung without testing it again.
#[test]
fn a_start_that_hung_in_its_window_moves_on_from_its_rung() {
    let dir = temp_dir("hung");
    let start = launch(&dir, None);
    let first = start.attempt().rung;
    stuck(&dir, first);
    let (seen, test) = counting(|_| Outcome::Passed(Report::default()));
    let result = choose_with(&start, "GPU 1.0", test);
    if ladder().len() > 1 {
        assert_eq!(result, Ok(()));
        assert!(!seen.borrow().contains(&first), "the hung rung is not tested again: {:?}", seen.borrow());
        assert_ne!(start.attempt().rung, first);
        assert!(start.failures().iter().any(|(r, e)| *r == first && e.contains("never showed its window")));
    } else {
        assert!(result.is_err(), "nothing to move on to");
    }
    start.ended();
    let _ = std::fs::remove_dir_all(&dir);
}

/// A start that ends in the message, because no rung passed, keeps an earlier hang on record:
/// closing it would let the next launch start the hung rung untested.
#[test]
fn a_start_that_ends_in_the_message_keeps_an_earlier_hang_on_record() {
    let dir = temp_dir("abandon");
    let start = launch(&dir, None);
    let first = start.attempt().rung;
    stuck(&dir, first);
    let (_, test) = counting(|_| Outcome::Hung(Report::default(), 25));
    assert!(choose_with(&start, "GPU 1.0", test).is_err());
    start.ended();
    let kept = test_of(&dir);
    assert_eq!(kept.as_ref().map(|t| (t.rung, t.open)), Some((first, true)), "still open");
    assert_eq!(need(kept.as_ref(), first, "GPU 1.0", unix_now()), Need::LastHung, "the next launch moves on again");
    let _ = std::fs::remove_dir_all(&dir);
}
