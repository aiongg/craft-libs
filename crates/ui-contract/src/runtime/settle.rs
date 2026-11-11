//! Whether the app is settled, from the framework's own signals: motion, a rerun its frame did not run, background
//! results, on-demand fonts, immediate repaint requests and injected input. Read between frames, after a frame's
//! passes ran.

use std::fmt;

use egui::Context;

use crate::runtime::egui::{injected, pass, repaint, sizing};
pub use crate::runtime::motion::Looping;

/// What keeps the app busy, signal by signal. Motion, reruns and repaints are joined over every live viewport.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SettleState {
    /// A tween is not at its end value.
    pub animating: bool,
    /// Motion that never ends while it shows.
    pub looping: Option<Looping>,
    /// Seconds, on the pass clock, from the last pass to the earliest one-shot wake a component asked for (a hold, a
    /// delay).
    pub wake_in: Option<f64>,
    /// A frame's last pass asked to be run again and the frame did not.
    pub rerun: bool,
    /// Background results not yet shown.
    pub jobs: usize,
    /// On-demand fonts still loading.
    pub fonts: bool,
    /// A pass was asked for now and has not run.
    pub repaint: bool,
    /// Injected input not yet consumed.
    pub input: bool,
}

impl SettleState {
    /// Nothing will run now apart from looping motion: what shows stays until a wake, a loop's next step or input.
    /// An unstepped loop asks for a pass every pass, so while one runs a repaint request is not told apart from it.
    pub fn at_rest(&self) -> bool {
        let repaint = self.repaint && self.looping != Some(Looping::Loop);
        !(self.animating || self.rerun || self.jobs > 0 || self.fonts || repaint || self.input)
    }

    /// At rest, with no one-shot wake pending and no looping motion.
    pub fn is_settled(&self) -> bool {
        self.at_rest() && self.wake_in.is_none() && self.looping.is_none()
    }

    /// What is busy, in a fixed order.
    pub fn busy(&self) -> Vec<Busy> {
        let mut busy = Vec::new();
        if self.animating {
            busy.push(Busy::Animating);
        }
        if let Some(looping) = self.looping {
            busy.push(Busy::Looping(looping));
        }
        if let Some(wake_in) = self.wake_in {
            busy.push(Busy::Wake(wake_in));
        }
        if self.rerun {
            busy.push(Busy::Rerun);
        }
        if self.jobs > 0 {
            busy.push(Busy::Jobs(self.jobs));
        }
        if self.fonts {
            busy.push(Busy::Fonts);
        }
        if self.repaint {
            busy.push(Busy::Repaint);
        }
        if self.input {
            busy.push(Busy::Input);
        }
        busy
    }
}

/// One signal that keeps the app from being settled.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Busy {
    /// [`SettleState::animating`].
    Animating,
    /// [`SettleState::looping`].
    Looping(Looping),
    /// [`SettleState::wake_in`]: a one-shot wake in this many seconds.
    Wake(f64),
    /// [`SettleState::rerun`].
    Rerun,
    /// [`SettleState::jobs`]: this many background results not yet shown.
    Jobs(usize),
    /// [`SettleState::fonts`].
    Fonts,
    /// [`SettleState::repaint`].
    Repaint,
    /// [`SettleState::input`].
    Input,
}

impl fmt::Display for Busy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Busy::Animating => f.write_str("animation"),
            Busy::Looping(Looping::Loop) => f.write_str("loop"),
            Busy::Looping(Looping::Periodic) => f.write_str("periodic motion"),
            Busy::Wake(seconds) => write!(f, "wake in {seconds:.2} s"),
            Busy::Rerun => f.write_str("rerun"),
            Busy::Jobs(count) => write!(f, "jobs: {count}"),
            Busy::Fonts => f.write_str("fonts"),
            Busy::Repaint => f.write_str("repaint"),
            Busy::Input => f.write_str("input"),
        }
    }
}

/// The settle state of `ctx` after its last frame.
pub fn state(ctx: &Context) -> SettleState {
    let mut state = SettleState {
        jobs: crate::runtime::pending::in_flight(ctx),
        fonts: crate::fonts::on_demand::loading(ctx),
        input: !injected::idle(ctx),
        ..SettleState::default()
    };
    let mut viewports = pass::live_viewports(ctx);
    if !viewports.contains(&egui::ViewportId::ROOT) {
        viewports.push(egui::ViewportId::ROOT);
    }
    for viewport in viewports {
        let motion = crate::runtime::motion::outcome(ctx, viewport);
        state.animating |= motion.animating;
        state.looping = state.looping.max(motion.looping);
        state.wake_in = match (state.wake_in, motion.wake_in) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        state.rerun |= sizing::rerun_left(ctx, viewport);
        state.repaint |= repaint::pending_now(ctx, viewport);
    }
    state
}
