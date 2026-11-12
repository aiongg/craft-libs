//! The first-frame watch: a window whose adapter hangs instead of failing.
//!
//! On a hybrid laptop the discrete GPU never put the window's first frame on the screen, with no
//! error and no panic, and the app stayed in the background without a window (DesignCraft,
//! VectorCraft #964). Giving up once the first frame is late also fired on healthy starts: a window
//! on a locked screen or another workspace whose compositor holds its frames back, or a large
//! document composing on a slow machine. So a late frame is only reported; the adapter counts as
//! hanging only when the UI thread is stuck outside the app's code (in eframe, wgpu or the driver),
//! makes no progress at all for [`HANG_TIMEOUT`], and the window isn't known to be hidden. A busy
//! UI thread, however long, never counts (#5160).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use super::{Failure, GraphicsStart, Next};

/// How long the window's first frame may take before the watch logs it and leaves a notice
/// ([`GraphicsStart::notice`]). The first frame draws the whole interface and compiles its shaders,
/// a second at most on a slow machine.
pub const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(20);

/// How long the UI thread may be stuck in the graphics stack before the first frame, the window
/// not known to be hidden, before its adapter counts as hanging.
pub const HANG_TIMEOUT: Duration = Duration::from_secs(60);

/// How often the watch looks.
const TICK: Duration = Duration::from_millis(250);

/// What the watch reads from the window (craft-ui-gpu implements it on the egui context).
pub trait FrameSignals: Send + Sync {
    /// UI passes run; from 2 on, the first frame was presented.
    fn frames(&self) -> u64;
    /// The window is known to be minimized, occluded or without focus: a compositor may hold its
    /// frames back (a Wayland surface that isn't shown blocks the next frame).
    fn hidden(&self) -> bool;
    /// Ask for a frame, so a UI thread that is only idle draws its second frame.
    fn request_repaint(&self);
}

/// Where the UI thread is, for the watch: in the app's code ([`Busy`]) or not, and how often it
/// left the app's code, which moves while the thread is alive.
#[derive(Debug, Default)]
pub(crate) struct UiThread {
    busy: AtomicBool,
    leaves: AtomicU64,
}

/// The UI thread runs the app's code while this lives: a long wait now is the app being busy,
/// never a graphics hang. craft-ui-gpu wraps the app's update in one ([`GraphicsStart::busy`]).
#[derive(Debug)]
pub struct Busy(Arc<UiThread>);

impl UiThread {
    pub(crate) fn busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }

    pub(crate) fn leaves(&self) -> u64 {
        self.leaves.load(Ordering::Relaxed)
    }
}

impl Busy {
    pub(crate) fn enter(ui: &Arc<UiThread>) -> Self {
        ui.busy.store(true, Ordering::Relaxed);
        Self(Arc::clone(ui))
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::Relaxed);
        self.0.leaves.fetch_add(1, Ordering::Relaxed);
    }
}

/// One look at the window's start.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample {
    pub frames: u64,
    pub busy: bool,
    pub leaves: u64,
    pub hidden: bool,
}

/// What [`Watch::step`] makes of the window's start.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Wait,
    /// The first frame was presented: nothing more to watch.
    Shown,
    /// The first frame is late: log it and say so, once.
    Late,
    /// The UI thread has been stuck in the graphics stack for [`HANG_TIMEOUT`].
    Hung,
}

#[derive(Debug, Default)]
pub(crate) struct Watch {
    /// Time waited for the first frame, the window not known to be hidden.
    waited: Duration,
    /// Time the UI thread has been stuck outside the app's code without progress.
    stuck: Duration,
    leaves: u64,
    late: bool,
}

impl Watch {
    pub(crate) fn step(&mut self, s: Sample, dt: Duration) -> Verdict {
        if s.frames >= 2 {
            return Verdict::Shown;
        }
        if s.hidden {
            self.stuck = Duration::ZERO;
            return Verdict::Wait;
        }
        self.waited = self.waited.saturating_add(dt);
        if s.busy || s.leaves != self.leaves {
            self.leaves = s.leaves;
            self.stuck = Duration::ZERO;
        } else {
            self.stuck = self.stuck.saturating_add(dt);
        }
        if self.stuck >= HANG_TIMEOUT {
            Verdict::Hung
        } else if self.waited >= FIRST_FRAME_TIMEOUT && !self.late {
            self.late = true;
            Verdict::Late
        } else {
            Verdict::Wait
        }
    }
}

/// Watch the window's first frame from another thread, from the app's creation. Each look asks
/// for a repaint. Once the first frame is shown, the start is [`GraphicsStart::presented`]. When
/// the adapter hangs, the app starts again without it ([`GraphicsStart::failed`] with
/// [`Failure::Hung`]) and this process, whose UI thread is stuck in the driver, ends.
pub fn watch_first_frame(start: GraphicsStart, signals: impl FrameSignals + 'static) {
    let watch = move || {
        let mut watch = Watch::default();
        loop {
            std::thread::sleep(TICK);
            let (busy, leaves) = start.ui_thread();
            let sample = Sample { frames: signals.frames(), busy, leaves, hidden: signals.hidden() };
            signals.request_repaint();
            match watch.step(sample, TICK) {
                Verdict::Wait => continue,
                Verdict::Shown => {
                    start.presented();
                    return;
                }
                Verdict::Late => {
                    start.late(FIRST_FRAME_TIMEOUT);
                    continue;
                }
                Verdict::Hung => {}
            }
            match start.failed(Failure::Hung, sample.frames) {
                Next::Restart(state) => match state.exec() {
                    Ok(()) => end_hung_process(),
                    Err(e) => log::error!("gpu: starting again failed: {e}"),
                },
                Next::Retry(_) | Next::GiveUp(_) => log::error!("gpu: the window hangs in the graphics stack and nothing is left to try"),
            }
            return;
        }
    };
    if let Err(e) = std::thread::Builder::new().name("first frame".into()).spawn(watch) {
        log::warn!("gpu: no watch on the window's first frame: {e}");
    }
}

/// End this process from the watch thread while its UI thread is stuck in the graphics driver.
/// On Windows `exit` runs the DLLs' detach code, which can wait on that thread forever and leave an
/// invisible process holding the log and the app's single-instance port, so the process is
/// terminated.
fn end_hung_process() {
    log::logger().flush();
    #[cfg(windows)]
    if let Err(e) = winsafe::HPROCESS::GetCurrentProcess().TerminateProcess(0) {
        log::error!("gpu: ending the process failed: {e}");
    }
    std::process::exit(0);
}
