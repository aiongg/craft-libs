//! The window's graphics start: which rung and adapter it starts with, and what follows when a
//! start fails (native only).
//!
//! A driver can crash inside wgpu's instance or device creation, where no Rust error handling
//! runs (Intel's `igvk64.dll`, `igd12umd64.dll`; AMD's `atio6axx.dll`), hang instead of failing,
//! or claim the window's surface and then fail to draw to it. Every launch then died the same way
//! until the user set `WGPU_BACKEND` by hand. So a start is an [`Attempt`]: a [`Rung`] of this
//! platform's [`ladder`] and the adapters it leaves out, and [`GraphicsStart`] moves on from one
//! that fails:
//!
//! | Event | Next |
//! |---|---|
//! | A launch finds the record's start in progress, with its lock free | that adapter is left out until it shows a frame; with no adapter picked yet (it crashed while listing), the next rung |
//! | A start alive past [`DEATH_WINDOW`], or a second launch while the lock is held | not a crash; the second launch records nothing and leaves nothing out |
//! | wgpu reports an error before any frame (no adapter, device creation failed) | in this process, the next rung down not tried, then any not tried from the top (FilmCraft #2581) |
//! | A graphics panic, or a wgpu error before [`STARTUP_FRAMES`] frames | a restart without the picked adapter (VectorCraft #502, FilmCraft #2553) |
//! | The first-frame watch finds the UI thread stuck in the graphics stack | a restart without the adapter ([`watch_first_frame`]) |
//! | No adapter is left to leave out | the next rung, nothing left out |
//! | Every rung failed | [`failure_message`]; the record starts over at the next launch |
//! | The first frame is shown | the rung settles (saved, with the notice and its reset); the adapter's crash is cleared |
//! | `WGPU_BACKEND` or `WGPU_ADAPTER_NAME` set | one attempt, the record left alone |
//! | Another app version or Graphics Processor preference | the record is forgotten |
//! | `--graphics <rung>`, `--safe-gpu` | [`Keep::Saved`], [`Keep::ThisLaunch`] |
//!
//! One crash moves on: the lock and the death window already tell a crash from a second copy of
//! the app starting or a start that hangs. On Windows, [`crate::probe::choose`] first tests the
//! rung in a child process, for drivers that hang before the window exists.

mod ladder;
mod message;
mod record;
mod restart;
mod watch;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

pub use ladder::{Attempt, Rung, ladder};
pub use message::failure_message;
pub use record::{DEATH_WINDOW, FILE, Record};
pub use restart::{RESTART_ARG, RestartState, graphics_panic, watch_panics};
pub use watch::{Busy, FIRST_FRAME_TIMEOUT, FrameSignals, HANG_TIMEOUT, watch_first_frame};

pub(crate) use ladder::{ladder_known, learned_dx12_hardware, next_untried};
#[cfg(test)]
pub(crate) use record::read as read_record;
pub(crate) use record::{Test, unix_now, update_held};

use record::{Opened, Recorded, Starting};
use watch::UiThread;

use crate::env;

/// Frames the UI ran before a failure below which the window was still starting up. A failing
/// adapter fails on the first or second frame; later the adapter has proved itself, and a lost
/// window is something else (such as the compositor restarting).
pub const STARTUP_FRAMES: u64 = 10;

/// How long a rung picked by hand applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// From now on (`--graphics <rung>`); `--graphics` with the top rung undoes a fallback.
    Saved,
    /// This launch only (`--safe-gpu`).
    ThisLaunch,
}

/// What the app tells the start about the user's choices.
#[derive(Clone, Debug)]
pub struct Choice {
    /// A rung picked on the command line.
    pub forced: Option<(Rung, Keep)>,
    /// The app's Graphics Processor preference: `highPerformance`, `lowPower`, or anything else
    /// for automatic. The record is forgotten when it changes.
    pub preference: String,
    /// The app's version. The record is forgotten when it changes.
    pub version: &'static str,
}

/// How a start failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// wgpu returned an error (eframe's `Error::Wgpu`).
    Reported(String),
    /// The window's run panicked, with this message; [`graphics_panic`] says whether the panic
    /// came from the graphics stack.
    Panicked(String),
    /// The first-frame watch found the UI thread stuck in the graphics stack.
    Hung,
}

/// What follows a failed start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    /// Start the window again in this process with this attempt.
    Retry(Attempt),
    /// Start the app again ([`RestartState::exec`]).
    Restart(RestartState),
    /// Nothing is left to try, or the failure isn't one of the graphics start: report this error.
    /// After a graphics failure, [`failure_message`] with [`GraphicsStart::failures`] tells the
    /// user what was tried.
    GiveUp(String),
}

/// The window's graphics start. Cheap to clone; clones share the start (the app, the adapter
/// selector and the first-frame watch each hold one).
#[derive(Clone)]
pub struct GraphicsStart(Arc<Inner>);

struct Inner {
    /// The record folder, unless `WGPU_BACKEND` or `WGPU_ADAPTER_NAME` choose.
    folder: Option<PathBuf>,
    /// This start holds the record in `folder` until it is over.
    holds: bool,
    explicit: bool,
    forced: Option<(Rung, Keep)>,
    preference: String,
    chosen: Option<wgpu::PowerPreference>,
    state: Mutex<State>,
    picked: Mutex<Option<Recorded>>,
    /// Counts the attempts of this process, so the death-window timer of an earlier one leaves a
    /// later one alone.
    generation: AtomicU64,
    reset: AtomicBool,
    shown: AtomicBool,
    /// Shown, ended or abandoned: the record is let go.
    over: AtomicBool,
    ui: Arc<UiThread>,
    late: Mutex<Option<String>>,
    documents: Mutex<Vec<OsString>>,
}

struct State {
    rung: Rung,
    /// Adapters a restart failed on.
    restart_skip: Vec<String>,
    /// Adapters earlier starts crashed on.
    crashed: Vec<String>,
    tried: Vec<Rung>,
    failures: Vec<(Rung, String)>,
}

impl State {
    fn attempt(&self, chosen: Option<wgpu::PowerPreference>) -> Attempt {
        let mut skip = self.restart_skip.clone();
        skip.extend(self.crashed.iter().filter(|k| !self.restart_skip.contains(k)).cloned());
        Attempt { rung: self.rung, skip, chosen }
    }
}

/// The power preference the app's Graphics Processor preference names; `None` is automatic.
pub(crate) fn chosen_power(preference: &str) -> Option<wgpu::PowerPreference> {
    match preference {
        "highPerformance" => Some(wgpu::PowerPreference::HighPerformance),
        "lowPower" => Some(wgpu::PowerPreference::LowPower),
        _ => None,
    }
}

impl GraphicsStart {
    /// This launch's start: read the record in `folder` (the app's data folder; `None` records
    /// nothing), note what the start before died on, pick the rung, and record the start until
    /// its first frame or its end. `restart` is [`RestartState::take_from_args`]. The folder also
    /// becomes the record folder of driver work outside the window ([`Record::set_folder`]).
    pub fn begin(folder: Option<&Path>, choice: Choice, restart: Option<RestartState>) -> Self {
        let start = Self::begin_at(folder, choice, restart, env::explicit_choice());
        start.expire_later();
        start
    }

    /// [`Self::begin`] with `explicit` for [`env::explicit_choice`], without the death-window timer.
    pub(crate) fn begin_at(folder: Option<&Path>, choice: Choice, restart: Option<RestartState>, explicit: bool) -> Self {
        let top = ladder::top();
        let forced = choice.forced.map(|(r, k)| (if ladder::on_ladder(r) { r } else { top }, k));
        if folder.is_some() {
            Record::set_folder(folder);
        }
        let folder = folder.filter(|_| !explicit).map(Path::to_path_buf);
        let mut held: Option<Record> = None;
        if let Some(dir) = &folder {
            match record::hold(dir) {
                Opened::Held(previous) => held = Some(Self::take_over(previous, &choice, forced, top)),
                Opened::Busy => {
                    log::info!("gpu: another start holds {}: this start records nothing and leaves nothing out", dir.join(FILE).display())
                }
                Opened::Failed(e) => log::warn!("gpu: {}: {e}", dir.join(FILE).display()),
            }
        } else if explicit {
            log::info!("gpu: {} or {} is set: one start as they say, the start-up record left alone", env::BACKEND, env::ADAPTER_NAME);
        }
        let restart_rung = restart.as_ref().map(|r| r.rung).filter(|r| ladder::on_ladder(*r));
        let saved = held.as_ref().and_then(|r| r.rung).filter(|r| ladder::on_ladder(*r));
        let rung = restart_rung.or(forced.map(|f| f.0)).or(saved).unwrap_or(top);
        let crashed: Vec<String> = held.as_ref().map_or_else(Vec::new, |r| r.crashed.iter().map(|c| c.key.clone()).collect());
        if !crashed.is_empty() {
            log::warn!("gpu: leaving out the adapters earlier starts crashed on: {}", crashed.join(", "));
        }
        let (restart_skip, tried) = restart.map_or_else(Default::default, |r| (r.skip, r.tried));
        let holds = held.is_some();
        if let (Some(dir), Some(mut r)) = (&folder, held) {
            r.starting = Some(Starting { rung, adapter: None, native: false });
            record::update_held(dir, |x| *x = r);
        }
        log::info!("gpu: starting with {} ({})", rung.label(), rung.name());
        Self(Arc::new(Inner {
            folder,
            holds,
            explicit,
            forced,
            chosen: chosen_power(&choice.preference),
            preference: choice.preference,
            state: Mutex::new(State { rung, restart_skip, crashed, tried, failures: Vec::new() }),
            picked: Mutex::new(None),
            generation: AtomicU64::new(0),
            reset: AtomicBool::new(false),
            shown: AtomicBool::new(false),
            over: AtomicBool::new(!holds),
            ui: Arc::default(),
            late: Mutex::new(None),
            documents: Mutex::new(Vec::new()),
        }))
    }

    /// The record this launch starts from: the one the last start left, forgotten for another
    /// version or preference (a scope entry stays: it names a process that died), with that
    /// start's crash taken in, and a rung picked to keep saved.
    fn take_over(previous: Option<Record>, choice: &Choice, forced: Option<(Rung, Keep)>, top: Rung) -> Record {
        let mut r = match previous {
            Some(p) if p.version == choice.version && p.preference == choice.preference => p,
            Some(p) => Record { scope: p.scope, ..Record::default() },
            None => Record::default(),
        };
        if let Some(s) = r.starting.take() {
            match s.adapter {
                Some(adapter) => {
                    log::warn!("gpu: the last start ended on {} ({}) before its first frame: leaving it out", adapter.name, adapter.key);
                    r.add_crashed(adapter);
                }
                None => {
                    // Listing the adapters is what crashed, so it isn't asked how DirectX 12 lists
                    // them now: the order for a DirectX 12 GPU decides.
                    let below = ladder::ladder_known().iter().skip_while(|x| **x != s.rung).nth(1).copied();
                    log::warn!(
                        "gpu: the last start ended while it listed the adapters of {}: starting {}",
                        s.rung.label(),
                        below.map_or("from the top", Rung::label)
                    );
                    r.rung = below;
                }
            }
        }
        r.version = choice.version.to_string();
        r.preference = choice.preference.clone();
        if let Some((rung, Keep::Saved)) = forced {
            log::info!("gpu: {} chosen on the command line", rung.label());
            r.rung = (rung != top).then_some(rung);
        }
        r
    }

    /// Clear this attempt's start from the record [`DEATH_WINDOW`] after it began, painted or not:
    /// a start still alive then is not a crash.
    fn expire_later(&self) {
        let Some(dir) = self.0.folder.clone().filter(|_| self.0.holds) else { return };
        let start = self.clone();
        let generation = self.0.generation.load(Ordering::SeqCst);
        let spawned = std::thread::Builder::new().name("gpu start".into()).spawn(move || {
            std::thread::sleep(DEATH_WINDOW);
            if !start.0.over.load(Ordering::SeqCst) && start.0.generation.load(Ordering::SeqCst) == generation {
                update_held(&dir, |r| r.starting = None);
            }
        });
        if let Err(e) = spawned {
            log::warn!("gpu: no death-window timer: {e}");
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The attempt to start the window with now.
    pub fn attempt(&self) -> Attempt {
        self.state().attempt(self.0.chosen)
    }

    /// The selector picked an adapter (`native`: the listing found one of a backend other than
    /// OpenGL): a crash from now on is blamed on it.
    pub(crate) fn picked(&self, key: &str, name: &str, native: bool) {
        let adapter = Recorded { key: key.to_string(), name: name.trim().to_string() };
        *self.0.picked.lock().unwrap_or_else(PoisonError::into_inner) = Some(adapter.clone());
        self.change_held(|r| {
            if let Some(s) = &mut r.starting {
                s.adapter = Some(adapter);
                s.native = native;
            }
        });
    }

    fn change_held(&self, change: impl FnOnce(&mut Record)) {
        if let Some(dir) = self.0.folder.as_deref().filter(|_| self.0.holds && !self.0.over.load(Ordering::SeqCst)) {
            update_held(dir, change);
        }
    }

    /// The start is over: change the record a last time and let it go. Once only.
    fn finish(&self, change: impl FnOnce(&mut Record)) {
        if self.0.over.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Some(dir) = self.0.folder.as_deref() {
            update_held(dir, change);
            record::release(dir);
        }
    }

    /// The first frame was shown: the rung settles, unless the notice's reset was used or the
    /// rung applies to this launch only, and the adapter no longer counts as crashed.
    pub fn presented(&self) {
        if self.0.shown.swap(true, Ordering::SeqCst) {
            return;
        }
        let rung = self.state().rung;
        let picked = self.0.picked.lock().unwrap_or_else(PoisonError::into_inner).clone();
        let keep = !self.0.reset.load(Ordering::SeqCst) && !matches!(self.0.forced, Some((_, Keep::ThisLaunch)));
        let top = ladder::top();
        self.finish(|r| {
            r.starting = None;
            if let Some(p) = picked {
                r.crashed.retain(|c| c.key != p.key);
            }
            if keep {
                r.rung = (rung != top).then_some(rung);
            }
            close_test(r, rung);
        });
    }

    /// The start ended without crashing: a clean exit, an error, or a restart. It was not a hang
    /// either. Calling it more than once, or after [`Self::presented`], does nothing.
    pub fn ended(&self) {
        let rung = self.state().rung;
        self.finish(|r| {
            r.starting = None;
            close_test(r, rung);
        });
    }

    /// The start ends before any window because no rung passed its test start: its start is over,
    /// but the record of an earlier start that hung stays open, so the next launch moves on from
    /// that rung instead of starting it untested.
    pub(crate) fn abandon(&self) {
        self.finish(|r| r.starting = None);
    }

    /// What follows the failure `failure` after `frames` frames (see the module docs). A
    /// [`Next::Restart`] or [`Next::GiveUp`] ends this start ([`Self::ended`]).
    pub fn failed(&self, failure: Failure, frames: u64) -> Next {
        let next = self.next_after(failure, frames, graphics_panic());
        if !matches!(next, Next::Retry(_)) {
            self.ended();
        }
        next
    }

    pub(crate) fn next_after(&self, failure: Failure, frames: u64, graphics_panic: bool) -> Next {
        let error = match &failure {
            Failure::Reported(e) | Failure::Panicked(e) => e.clone(),
            Failure::Hung => format!(
                "the window's first frame didn't reach the screen and the UI thread was stuck in the graphics stack for {} s",
                HANG_TIMEOUT.as_secs()
            ),
        };
        let graphics = !matches!(failure, Failure::Panicked(_)) || graphics_panic;
        if !graphics || frames >= STARTUP_FRAMES {
            return Next::GiveUp(error);
        }
        let mut st = self.state();
        let rung = st.rung;
        st.failures.push((rung, error.clone()));
        if self.0.explicit {
            return Next::GiveUp(error);
        }
        let in_process = matches!(failure, Failure::Reported(_)) && frames == 0;
        let picked = self.0.picked.lock().unwrap_or_else(PoisonError::into_inner).take();
        if !in_process && let Some(p) = picked.filter(|p| !st.restart_skip.contains(&p.key) && !st.crashed.contains(&p.key)) {
            log::error!("gpu: the window failed on {} ({error}): starting again without it", p.name);
            let mut skip = st.restart_skip.clone();
            skip.push(p.key);
            return Next::Restart(self.restart_state(rung, skip, st.tried.clone()));
        }
        if !st.tried.contains(&rung) {
            st.tried.push(rung);
        }
        let Some(next) = next_untried(ladder::ladder(), rung, &st.tried) else {
            log::error!("gpu: every rung failed; the next launch starts from the top");
            self.change_held(|r| {
                r.rung = None;
                r.crashed.clear();
            });
            return Next::GiveUp(error);
        };
        log::warn!("gpu: {} failed ({error}): trying {}", rung.label(), next.label());
        if !in_process {
            return Next::Restart(self.restart_state(next, Vec::new(), st.tried.clone()));
        }
        st.rung = next;
        st.restart_skip.clear();
        self.0.generation.fetch_add(1, Ordering::SeqCst);
        // The error is certain: the next launch starts on the next rung too.
        let save = !self.0.reset.load(Ordering::SeqCst) && !matches!(self.0.forced, Some((_, Keep::ThisLaunch)));
        let top = ladder::top();
        self.change_held(|r| {
            if save {
                r.rung = (next != top).then_some(next);
            }
            r.starting = Some(Starting { rung: next, adapter: None, native: false });
        });
        let attempt = st.attempt(self.0.chosen);
        drop(st);
        self.expire_later();
        Next::Retry(attempt)
    }

    fn restart_state(&self, rung: Rung, skip: Vec<String>, tried: Vec<Rung>) -> RestartState {
        let documents = self.0.documents.lock().unwrap_or_else(PoisonError::into_inner).clone();
        RestartState { rung, skip, tried, documents }
    }

    /// Start on `rung` instead, from the next attempt on (a test start passed on it, [`crate::probe`]).
    pub(crate) fn switch_to(&self, rung: Rung) {
        let mut st = self.state();
        st.rung = rung;
        st.restart_skip.clear();
        drop(st);
        self.0.generation.fetch_add(1, Ordering::SeqCst);
        self.change_held(|r| r.starting = Some(Starting { rung, adapter: None, native: false }));
        self.expire_later();
    }

    /// Keep `error` for `rung` among this start's failures (a rung that failed its test start).
    pub(crate) fn add_failure(&self, rung: Rung, error: &str) {
        self.state().failures.push((rung, error.to_string()));
    }

    /// The rungs this start tried that failed, each with its error, for [`failure_message`].
    pub fn failures(&self) -> Vec<(Rung, String)> {
        self.state().failures.clone()
    }

    /// The record folder, while this start holds the record.
    pub(crate) fn held_folder(&self) -> Option<&Path> {
        self.0.folder.as_deref().filter(|_| self.0.holds && !self.0.over.load(Ordering::SeqCst))
    }

    /// Whether `WGPU_BACKEND` or `WGPU_ADAPTER_NAME` choose.
    pub(crate) fn explicit(&self) -> bool {
        self.0.explicit
    }

    /// Whether the rung was picked on the command line.
    pub(crate) fn forced(&self) -> bool {
        self.0.forced.is_some()
    }

    /// The app's Graphics Processor preference.
    pub(crate) fn preference(&self) -> &str {
        &self.0.preference
    }

    /// The window starts on a fallback, not the top of the ladder.
    pub fn degraded(&self) -> bool {
        self.state().rung != ladder::top()
    }

    /// A note for the window, in English (the app translates it): a first frame that was late, or
    /// the fallback the window draws with, which [`Self::reset_to_default`] undoes.
    pub fn notice(&self) -> Option<String> {
        if let Some(late) = self.0.late.lock().unwrap_or_else(PoisonError::into_inner).clone() {
            return Some(late);
        }
        let rung = self.state().rung;
        self.degraded()
            .then(|| format!("Drawing with {} after {} failed to start. Reset to go back at the next start.", rung.label(), ladder::top().label()))
    }

    /// The first frame is `after` late (the first-frame watch): log it and leave a notice.
    pub(crate) fn late(&self, after: Duration) {
        let adapter = self.0.picked.lock().unwrap_or_else(PoisonError::into_inner).as_ref().map_or_else(|| "(none)".to_string(), |p| p.name.clone());
        log::warn!("gpu: the window's first frame hasn't reached the screen in {} s on {adapter}", after.as_secs());
        *self.0.late.lock().unwrap_or_else(PoisonError::into_inner) = Some(format!(
            "The window took over {} s to show on {adapter}. If it stays black, see the log; {} or {} chooses another graphics processor.",
            after.as_secs(),
            env::ADAPTER_NAME,
            env::POWER_PREF
        ));
    }

    /// Start from the top of the ladder next time (the notice's reset). This process keeps its
    /// rung: the device can't be swapped under a running window.
    pub fn reset_to_default(&self) -> Result<(), String> {
        self.0.reset.store(true, Ordering::SeqCst);
        let Some(dir) = &self.0.folder else { return Ok(()) };
        let path = dir.join(FILE);
        match record::update(dir, |r| r.rung = None) {
            Ok(true) => {
                log::info!("gpu: the next start begins from the top of the ladder");
                Ok(())
            }
            Ok(false) => Err(format!("{} is held by another start", path.display())),
            Err(e) => Err(format!("can't write {}: {e}", path.display())),
        }
    }

    /// The UI thread runs the app's code until the guard drops ([`Busy`]).
    pub fn busy(&self) -> Busy {
        Busy::enter(&self.0.ui)
    }

    fn ui_thread(&self) -> (bool, u64) {
        (self.0.ui.busy(), self.0.ui.leaves())
    }

    /// Documents the OS handed this run outside its arguments (macOS Apple events), for a restart
    /// to pass on: the OS doesn't send them twice.
    pub fn opening(&self, paths: impl IntoIterator<Item = OsString>) {
        self.0.documents.lock().unwrap_or_else(PoisonError::into_inner).extend(paths);
    }
}

/// The test record of a start on `rung` is closed: that start showed a frame or ended.
fn close_test(r: &mut Record, rung: Rung) {
    if let Some(t) = &mut r.test
        && t.open
        && t.rung == rung
    {
        t.open = false;
    }
}

#[cfg(test)]
mod tests;
