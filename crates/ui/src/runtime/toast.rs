//! The toast queue: one per viewport, filled from any code with a `Context`.
//!
//! The shell's `ToastHost` shows the queue and tells it which cards the pointer or focus holds. A card's timer runs
//! only while nothing holds it. Each new toast is announced once, when it is first shown.

use crate::runtime::access::{self, Politeness};
use crate::runtime::egui::{pass, repaint};
use crate::runtime::env::Label;
use crate::runtime::frame::{self, PerViewport};
use egui::{Context, ViewportId};

/// How long a toast stays when it names no timeout, in seconds.
pub const TIMEOUT: f32 = 5.0;

/// A message the user sees for a while.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Toast {
    /// The text of the card.
    pub message: Label,
    /// Seconds the card stays; `None` keeps it until it is dismissed.
    pub timeout: Option<f32>,
    /// How the announcement is made.
    pub politeness: Politeness,
}

impl Toast {
    /// A toast of `message` that stays [`TIMEOUT`] seconds and is announced politely.
    pub fn new(message: impl Into<Label>) -> Self {
        Toast { message: message.into(), timeout: Some(TIMEOUT), politeness: Politeness::Polite }
    }

    /// Sets how long the card stays.
    pub fn timeout(mut self, timeout: Option<f32>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the announcement's politeness.
    pub fn politeness(mut self, politeness: Politeness) -> Self {
        self.politeness = politeness;
        self
    }
}

/// A posted toast's handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(u64);

/// A toast in the queue, as the host reads it.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Card {
    /// The handle.
    pub id: ToastId,
    /// The data.
    pub toast: Toast,
}

struct Live {
    card: Card,
    shown: bool,
    remaining: Option<f32>,
    held: bool,
}

#[derive(Default)]
struct Queue {
    next: u64,
    live: Vec<Live>,
    /// The pass time of the last end hook, to measure the timers' elapsed time.
    last_tick: Option<f64>,
}

/// Posts `toast` to the running viewport.
pub fn post(ctx: &Context, toast: Toast) -> ToastId {
    post_to(ctx, pass::current_viewport(ctx), toast)
}

/// Posts `toast` to `viewport`.
pub fn post_to(ctx: &Context, viewport: ViewportId, toast: Toast) -> ToastId {
    let id = PerViewport::<Queue>::with(ctx, viewport, |queue| {
        queue.next += 1;
        let id = ToastId(queue.next);
        let remaining = toast.timeout;
        queue.live.push(Live { card: Card { id, toast }, shown: false, remaining, held: false });
        id
    });
    repaint::request_of(ctx, viewport);
    id
}

/// Removes a toast before its timer ends.
pub fn dismiss(ctx: &Context, id: ToastId) {
    PerViewport::<Queue>::with(ctx, pass::current_viewport(ctx), |queue| queue.live.retain(|live| live.card.id != id));
    repaint::request(ctx);
}

/// The cards of the running viewport, oldest first.
pub fn cards(ctx: &Context) -> Vec<Card> {
    PerViewport::<Queue>::with(ctx, pass::current_viewport(ctx), |queue| queue.live.iter().map(|live| live.card.clone()).collect())
}

/// The host shows `id` in this pass; `held` says the pointer or focus is on it. The first show announces it.
pub fn shown(ctx: &Context, id: ToastId, text: &str, held: bool) {
    let announce = PerViewport::<Queue>::with(ctx, pass::current_viewport(ctx), |queue| {
        let live = queue.live.iter_mut().find(|live| live.card.id == id)?;
        live.held = held;
        let first = !live.shown;
        live.shown = true;
        first.then_some(live.card.toast.politeness)
    });
    if let Some(politeness) = announce {
        access::announce(ctx, text, politeness);
    }
}

/// The end hook: runs the timers of the shown, unheld cards, drops the expired ones and asks for a pass when the
/// next one expires.
pub(crate) fn end_pass(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    let now = frame::clock(ctx);
    let wake = PerViewport::<Queue>::with(ctx, viewport, |queue| {
        let elapsed = queue.last_tick.map_or(0.0, |last| (now - last).max(0.0)) as f32;
        queue.last_tick = Some(now);
        for live in &mut queue.live {
            if live.shown && !live.held && let Some(remaining) = &mut live.remaining {
                *remaining -= elapsed;
            }
        }
        queue.live.retain(|live| live.remaining.is_none_or(|remaining| remaining > 0.0));
        queue.live.iter().filter(|live| live.shown && !live.held).filter_map(|live| live.remaining).reduce(f32::min)
    });
    if let Some(seconds) = wake {
        repaint::request_after(ctx, viewport, f64::from(seconds));
    }
}
