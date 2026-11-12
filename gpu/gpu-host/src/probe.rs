//! A test start of the window's graphics in a child process, before this process loads any
//! graphics driver (Windows; FilmCraft #1061, #2634, #1038).
//!
//! A driver can hang inside wgpu's instance, adapter or device creation instead of failing. The
//! process then lives on with no window and nothing reported, and the start-up record, which
//! counts starts that died, never moves on. No thread of a process stuck like that can reliably
//! report it or end it, since the driver holds the Windows loader lock. So the test runs in a child
//! process (this program started with [`CHILD_ARG`], which the app's `main` hands to
//! [`run_child_if_asked`] first). The child does what the window's start will do on the rung:
//! create the instance, list the adapters, choose one and create the device, and it writes a line
//! on its standard output as each step begins. A child that has not finished within [`TIMEOUT`] is
//! ended by this process, which started it and holds its handle, and the next rung is tried. The
//! window then starts on the first rung that passed. When none passes, the user gets a message
//! naming each rung, where it stopped and on which adapter, instead of an app that never opens.
//!
//! The test runs only when something changed since the last start (the record's `test` entry):
//! another app version or rung, other display drivers, or a last start that never got its window
//! and never came back. Other launches start directly. A start that passed its test, or skipped
//! it, but neither showed a frame nor ended within [`HUNG_AFTER`] is taken as hung in the window
//! itself, and the next launch moves on from its rung.

use std::ffi::OsString;
use std::io::{BufRead as _, Write as _};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::adapter;
use crate::limits::device_descriptor;
use crate::start::{Attempt, GraphicsStart, Rung, Test, chosen_power, ladder_known, learned_dx12_hardware, next_untried, unix_now, update_held};

/// The first argument of the test-start child: `<app> --craft-gpu-probe <rung> <preference> [<adapter key>…]`.
pub const CHILD_ARG: &str = "--craft-gpu-probe";
/// How long a test start may take. The slowest working start seen took about 9 s from launch to
/// an adapter on a cold start (FilmCraft #1808); a hung one never gets there.
pub const TIMEOUT: Duration = Duration::from_secs(25);
/// A start still without its window after this long is taken as hung, not slow.
pub const HUNG_AFTER: Duration = Duration::from_secs(60);
/// How long an ended child gets to go before it is left behind.
const END_GRACE: Duration = Duration::from_secs(3);
/// How often a running child is looked at.
const POLL: Duration = Duration::from_millis(50);
/// Lines read from a child; it writes five.
const MAX_LINES: usize = 32;
/// Adapters named in a message.
const MAX_ADAPTERS: usize = 8;

/// The step a test start was at: what each line from the child says it begins.
fn stage_text(stage: &str) -> &'static str {
    match stage {
        "instance" => "loading the graphics driver",
        "adapters" => "listing the graphics adapters",
        "adapter" => "choosing a graphics adapter",
        "device" => "creating the graphics device",
        _ => "starting",
    }
}

/// A short description of an adapter: name, backend and driver.
fn describe(info: &wgpu::AdapterInfo) -> String {
    let driver = [info.driver.trim(), info.driver_info.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
    if driver.is_empty() { format!("{} ({:?})", info.name, info.backend) } else { format!("{} ({:?}, {driver})", info.name, info.backend) }
}

// ----------------------------------------------------------------------------------- the child

/// In the test-start child (`args`, without the program name, begin with [`CHILD_ARG`]): run the
/// test and return the exit code. `None` in any other launch.
pub fn run_child_if_asked(args: &[OsString]) -> Option<i32> {
    let mut args = args.iter();
    if args.next()?.to_str()? != CHILD_ARG {
        return None;
    }
    let Some(rung) = args.next().and_then(|r| r.to_str()).and_then(Rung::parse) else {
        say("error", serde_json::json!({ "error": "no graphics rung given" }));
        return Some(2);
    };
    let chosen = args.next().and_then(|p| p.to_str()).and_then(chosen_power);
    let skip = args.filter_map(|k| k.to_str().map(str::to_string)).collect();
    Some(run_child(&Attempt { rung, skip, chosen }))
}

/// The child's arguments for a test of `attempt` under the Graphics Processor `preference`.
fn child_args(attempt: &Attempt, preference: &str) -> Vec<String> {
    let mut args = vec![CHILD_ARG.to_string(), attempt.rung.name().to_string(), preference.to_string()];
    args.extend(attempt.skip.iter().cloned());
    args
}

/// Write one line for the parent and flush it, so it arrives even if the next step hangs.
fn say(stage: &str, extra: serde_json::Value) {
    let mut line = serde_json::json!({ "stage": stage });
    if let (Some(line), serde_json::Value::Object(extra)) = (line.as_object_mut(), extra) {
        line.extend(extra);
    }
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn run_child(attempt: &Attempt) -> i32 {
    say("instance", serde_json::json!({}));
    let desc = attempt.instance_descriptor();
    let backends = desc.backends;
    let instance = wgpu::Instance::new(desc);
    say("adapters", serde_json::json!({}));
    let adapters = pollster::block_on(instance.enumerate_adapters(backends));
    let names: Vec<String> = adapters.iter().map(|a| describe(&a.get_info())).collect();
    let mut listed = serde_json::json!({ "adapters": names });
    if attempt.rung == Rung::Dx12
        && let Some(map) = listed.as_object_mut()
    {
        // What orders the ladder (`start::ladder`), learned here so the parent never lists them.
        let hardware =
            adapters.iter().map(wgpu::Adapter::get_info).any(|i| i.backend == wgpu::Backend::Dx12 && i.device_type != wgpu::DeviceType::Cpu);
        map.insert("dx12_hardware".into(), hardware.into());
    }
    say("adapter", listed);
    let select = adapter::selector(attempt.ranking(), |_, _| {}, || {});
    let adapter = match select(&adapters, None) {
        Ok(a) => a,
        Err(e) => {
            say("error", serde_json::json!({ "error": e }));
            return 1;
        }
    };
    say("device", serde_json::json!({ "chosen": describe(&adapter.get_info()) }));
    match pollster::block_on(adapter.request_device(&device_descriptor(&adapter, &attempt.device_request()))) {
        Ok(_) => {
            say("done", serde_json::json!({}));
            0
        }
        Err(e) => {
            say("error", serde_json::json!({ "error": e.to_string() }));
            1
        }
    }
}

// ---------------------------------------------------------------------------------- the parent

/// What a child reported, line by line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Report {
    /// The last step it began.
    stage: Option<String>,
    /// The adapters it listed.
    adapters: Vec<String>,
    /// Whether DirectX 12 listed a hardware adapter (a test of [`Rung::Dx12`]).
    dx12_hardware: Option<bool>,
    /// The adapter it chose.
    chosen: Option<String>,
    /// The error it reported.
    error: Option<String>,
    /// It created the device.
    done: bool,
}

impl Report {
    fn take(&mut self, line: &str) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { return };
        let text = |key: &str| v.get(key).and_then(|s| s.as_str()).map(str::to_string);
        if let Some(adapters) = v.get("adapters").and_then(|a| a.as_array()) {
            self.adapters = adapters.iter().filter_map(|a| a.as_str().map(str::to_string)).take(MAX_ADAPTERS).collect();
        }
        if let Some(hardware) = v.get("dx12_hardware").and_then(serde_json::Value::as_bool) {
            self.dx12_hardware = Some(hardware);
        }
        if let Some(chosen) = text("chosen") {
            self.chosen = Some(chosen);
        }
        match text("stage").as_deref() {
            Some("done") => self.done = true,
            Some("error") => self.error = text("error").or_else(|| Some("unknown error".to_string())),
            Some(stage) => self.stage = Some(stage.to_string()),
            None => {}
        }
    }
}

/// How a test start ended.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Passed(Report),
    /// It reported an error.
    Failed(Report),
    /// It was still running after the timeout (seconds) and was ended.
    Hung(Report, u64),
    /// It ended without a result: a crash inside the driver, typically.
    Died(Report, Option<i32>),
    /// The child could not be started; nothing is known about the rung.
    NotRun(String),
}

impl Outcome {
    fn report(&self) -> Option<&Report> {
        match self {
            Outcome::Passed(r) | Outcome::Failed(r) | Outcome::Hung(r, _) | Outcome::Died(r, _) => Some(r),
            Outcome::NotRun(_) => None,
        }
    }
}

/// Run `cmd` as a test start, reading its lines, and end it after `timeout`.
fn run_test(mut cmd: Command, timeout: Duration) -> Outcome {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // CREATE_NO_WINDOW: no console window flashes up for the child.
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Outcome::NotRun(e.to_string()),
    };
    let report = Arc::new(Mutex::new(Report::default()));
    if let Some(out) = child.stdout.take() {
        let report = Arc::clone(&report);
        let read = move || {
            for line in std::io::BufReader::new(out).lines().take(MAX_LINES) {
                let Ok(line) = line else { break };
                if let Ok(mut r) = report.lock() {
                    r.take(&line);
                }
            }
        };
        if let Err(e) = std::thread::Builder::new().name("gpu probe".into()).spawn(read) {
            log::warn!("gpu: can't read the test start: {e}");
        }
    }
    let snapshot = |report: &Arc<Mutex<Report>>| report.lock().map(|r| r.clone()).unwrap_or_default();
    let began = Instant::now();
    loop {
        let r = snapshot(&report);
        if r.done {
            // The device exists: the rung works, however long the child takes to go.
            end(&mut child);
            return Outcome::Passed(r);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                // Let the reader take the last lines.
                std::thread::sleep(POLL);
                let r = snapshot(&report);
                return if r.done {
                    Outcome::Passed(r)
                } else if r.error.is_some() {
                    Outcome::Failed(r)
                } else {
                    Outcome::Died(r, status.code())
                };
            }
            Ok(None) => {}
            Err(e) => {
                end(&mut child);
                return Outcome::NotRun(e.to_string());
            }
        }
        if began.elapsed() >= timeout {
            end(&mut child);
            return Outcome::Hung(snapshot(&report), timeout.as_secs());
        }
        std::thread::sleep(POLL);
    }
}

/// End `child` and give it [`END_GRACE`] to go; one stuck in the kernel is left behind.
fn end(child: &mut Child) {
    let _ = child.kill();
    let began = Instant::now();
    while began.elapsed() < END_GRACE {
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        std::thread::sleep(POLL);
    }
    log::warn!("gpu: the test start (process {}) did not end", child.id());
}

/// The text for a rung that did not pass: what happened, at which step, on which adapter.
fn outcome_text(outcome: &Outcome) -> String {
    let at = |r: &Report| {
        let step = stage_text(r.stage.as_deref().unwrap_or(""));
        match &r.chosen {
            Some(adapter) => format!("{step} on {adapter}"),
            None if !r.adapters.is_empty() => format!("{step} (adapters: {})", r.adapters.join("; ")),
            None => step.to_string(),
        }
    };
    match outcome {
        Outcome::Passed(_) => "passed".to_string(),
        Outcome::Failed(r) => {
            let error = r.error.as_deref().unwrap_or("unknown error");
            if r.adapters.is_empty() { error.to_string() } else { format!("{error} (adapters: {})", r.adapters.join("; ")) }
        }
        Outcome::Hung(r, secs) => format!("stopped responding while {}, and was ended after {secs} s", at(r)),
        Outcome::Died(r, code) => {
            let code = code.map_or_else(|| "no exit code".to_string(), |c| format!("exit code {c}"));
            format!("ended unexpectedly while {} ({code})", at(r))
        }
        Outcome::NotRun(e) => format!("the test start could not run: {e}"),
    }
}

// ---------------------------------------------------------------------------- when to test

/// Whether this launch tests its rung first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Need {
    /// Nothing changed since a start that worked: start directly.
    No,
    /// Something changed, or nothing is known: test.
    Test,
    /// The last start on this rung never got its window and never came back: move on from it.
    LastHung,
}

/// What the record's `test` entry says about a start on `rung` with the display `drivers` at
/// `now`. Another app version forgets the whole record, so it never gets here.
fn need(test: Option<&Test>, rung: Rung, drivers: &str, now: u64) -> Need {
    let Some(t) = test else { return Need::Test };
    if t.rung != rung || t.drivers != drivers {
        return Need::Test;
    }
    if t.open {
        // Another copy of the app may still be starting: only a start this old counts as hung.
        return if now.saturating_sub(t.started) >= HUNG_AFTER.as_secs() { Need::LastHung } else { Need::Test };
    }
    Need::No
}

/// The rungs to test, in order: `current` first (unless `skip_current`), then the others as the
/// ladder's fallback goes ([`next_untried`]). `forced` (`--graphics`) tests that rung only.
fn candidates(ladder: &[Rung], current: Rung, skip_current: bool, forced: bool) -> Vec<Rung> {
    if forced {
        return vec![current];
    }
    // `current` counts as tried either way, so a skipped one doesn't come back at the end.
    let mut tried = vec![current];
    let mut order = if skip_current { Vec::new() } else { vec![current] };
    let mut at = current;
    while let Some(next) = next_untried(ladder, at, &tried) {
        tried.push(next);
        order.push(next);
        at = next;
    }
    order
}

/// The installed display drivers, as one text that changes when any of them does: names and
/// versions from the registry's display adapter class. Empty where unknown.
#[cfg(windows)]
fn display_drivers() -> String {
    use winsafe::{HKEY, RegistryValue, co};
    const CLASS: &str = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
    let Ok(class) = HKEY::LOCAL_MACHINE.RegOpenKeyEx(Some(CLASS), co::REG_OPTION::default(), co::KEY::READ) else { return String::new() };
    let Ok(keys) = class.RegEnumKeyEx() else { return String::new() };
    let mut drivers: Vec<String> = keys
        .take(64)
        .filter_map(Result::ok)
        .filter(|k| k.len() == 4 && k.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|k| {
            let key = class.RegOpenKeyEx(Some(&k), co::REG_OPTION::default(), co::KEY::READ).ok()?;
            let text = |name: &str| match key.RegQueryValueEx(Some(name)) {
                Ok(RegistryValue::Sz(s)) => Some(s),
                _ => None,
            };
            Some(format!("{} {}", text("DriverDesc")?, text("DriverVersion").unwrap_or_default()))
        })
        .collect();
    drivers.sort();
    drivers.join("; ")
}

#[cfg(not(windows))]
fn display_drivers() -> String {
    String::new()
}

/// Record a start on `rung` with `drivers` that begins now and is open until it shows a frame or
/// ends ([`GraphicsStart::presented`], [`GraphicsStart::ended`]).
fn open_test(dir: &Path, rung: Rung, drivers: &str) {
    let test = Test { rung, drivers: drivers.to_string(), started: unix_now(), open: true };
    update_held(dir, |r| r.test = Some(test));
}

// ------------------------------------------------------------------------------- the choice

/// Before this process loads a graphics driver (Windows), test `start`'s rung in a child process
/// when the record says so, and move `start` to the first rung that passes. A rung picked on the
/// command line is the only one tested; with `WGPU_BACKEND` or `WGPU_ADAPTER_NAME` set, or without
/// the record, nothing is tested. `Err` is the error for [`crate::start::failure_message`] when no
/// rung passed, each rung's failure being in [`GraphicsStart::failures`]; the start is then over.
pub fn choose(start: &GraphicsStart) -> Result<(), String> {
    if !cfg!(windows) || start.explicit() {
        return Ok(());
    }
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            log::warn!("gpu: no test start: {e}");
            return Ok(());
        }
    };
    let preference = start.preference().to_string();
    choose_with(start, &display_drivers(), |attempt| {
        let mut cmd = Command::new(&exe);
        cmd.args(child_args(attempt, &preference));
        run_test(cmd, TIMEOUT)
    })
}

/// [`choose`] with the display drivers `drivers`, testing an attempt with `test`, on any platform.
fn choose_with(start: &GraphicsStart, drivers: &str, test: impl Fn(&Attempt) -> Outcome) -> Result<(), String> {
    let Some(dir) = start.held_folder().map(Path::to_path_buf) else { return Ok(()) };
    let mut last = None;
    update_held(&dir, |r| last = r.test.clone());
    let rung = start.attempt().rung;
    let needed = need(last.as_ref(), rung, drivers, unix_now());
    if needed == Need::No {
        open_test(&dir, rung, drivers);
        return Ok(());
    }
    let forced = start.forced();
    let skip_current = needed == Need::LastHung && !forced;
    if skip_current {
        let hung = "the last start passed its graphics test but never showed its window";
        log::warn!("gpu: {}: {hung}", rung.label());
        start.add_failure(rung, hung);
    }
    match test_rungs(start, rung, skip_current, forced, &dir, drivers, test) {
        Ok(()) => Ok(()),
        Err(()) => {
            start.abandon();
            Err(if forced { "the chosen graphics did not start in a test" } else { "no graphics start passed its test" }.to_string())
        }
    }
}

/// Test the rungs from `current` in order with `test` and start on the first that passes; `Err`
/// when none did. The order is taken again after each test: the test of [`Rung::Dx12`] tells
/// whether DirectX 12 lists a hardware adapter, which orders the rest of the ladder.
fn test_rungs(
    start: &GraphicsStart,
    current: Rung,
    skip_current: bool,
    forced: bool,
    dir: &Path,
    drivers: &str,
    test: impl Fn(&Attempt) -> Outcome,
) -> Result<(), ()> {
    let mut tested: Vec<Rung> = Vec::new();
    while let Some(rung) = candidates(ladder_known(), current, skip_current, forced).into_iter().find(|r| !tested.contains(r)) {
        tested.push(rung);
        log::info!("gpu: test start on {}", rung.label());
        let attempt = Attempt { rung, ..start.attempt() };
        let outcome = test(&attempt);
        if let Some(hardware) = outcome.report().and_then(|r| r.dx12_hardware) {
            learned_dx12_hardware(hardware);
        }
        match &outcome {
            Outcome::Passed(r) => {
                log::info!("gpu: {} passed its test start{}", rung.label(), r.chosen.as_deref().map(|a| format!(" on {a}")).unwrap_or_default())
            }
            // The test itself could not run: start as before rather than not at all.
            Outcome::NotRun(e) => log::warn!("gpu: the test start on {} could not run: {e}; starting without it", rung.label()),
            other => {
                let text = outcome_text(other);
                log::warn!("gpu: {}: {text}", rung.label());
                start.add_failure(rung, &text);
                continue;
            }
        }
        if rung != start.attempt().rung {
            log::warn!("gpu: starting on {} after its test start", rung.label());
            start.switch_to(rung);
        }
        open_test(dir, rung, drivers);
        return Ok(());
    }
    Err(())
}

#[cfg(test)]
mod tests;
