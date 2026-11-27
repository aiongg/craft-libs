//! The toast queues: one per window (`WindowKey`), filled from any code with a `Context`.
//!
//! The window's shell shows its queue through `ToastHost`, at the end-bottom corner of its body, and tells it which
//! cards the pointer or focus holds; an embedded shell shows its own queue inside its own body. A card's timer runs only
//! while nothing holds it, and starts over with its whole timeout when the hold ends. Each new toast is announced once,
//! when it is first shown. At most `MAX_CARDS` cards are queued per window unless progress cards fill it; a toast's end
//! (an action taken, a dismissal, the timer, a drop by the cap) is read once with `end`.

use crate::runtime::access::{self, Politeness};
use crate::runtime::env::Label;

use craft_theme::variant::Tone;

/// How long a toast stays when it names no timeout, in seconds: the source apps' notification duration.
pub const TIMEOUT: f32 = 6.0;

/// Most cards queued at once per window. Posting past it keeps the new card and drops the oldest earlier card that
/// would leave by itself (no action, a timeout), else the oldest earlier card without progress; `end` reports the drop
/// as `Dropped`. A progress card is never dropped: when only progress cards are earlier, the new card is queued past
/// the cap.
pub const MAX_CARDS: usize = 3;

/// An action of a toast card: a button-like link that ends the toast and is reported by `end`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToastAction {
    /// The action's text.
    pub label: Label,
}

impl ToastAction {
    /// An action named `label`.
    pub fn new(label: impl Into<Label>) -> Self {
        ToastAction { label: label.into() }
    }
}

/// How a toast left the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastEnd {
    /// The action at this index was taken.
    Action(usize),
    /// The user closed the card.
    Dismissed,
    /// The timer ran out.
    Expired,
    /// The cap removed the card to make room for a newer one.
    Dropped,
}

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
    /// The tone the card reads in.
    pub tone: Tone,
    /// A title above the message.
    pub title: Option<Label>,
    /// Bullet lines under the message.
    pub lines: Vec<Label>,
    /// The actions after the message, in reading order.
    pub actions: Vec<ToastAction>,
    /// The fraction done, `0..=1`; a card with progress has no timer and is never the one the cap drops.
    pub progress: Option<f32>,
}

impl Toast {
    /// A neutral toast of `message` that stays [`TIMEOUT`] seconds and is announced politely.
    pub fn new(message: impl Into<Label>) -> Self {
        Toast {
            message: message.into(),
            timeout: Some(TIMEOUT),
            politeness: Politeness::Polite,
            tone: Tone::Neutral,
            title: None,
            lines: Vec::new(),
            actions: Vec::new(),
            progress: None,
        }
    }

    /// Sets the title above the message.
    pub fn title(mut self, title: impl Into<Label>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the bullet lines under the message.
    pub fn lines<L: Into<Label>>(mut self, lines: impl IntoIterator<Item = L>) -> Self {
        self.lines = lines.into_iter().map(Into::into).collect();
        self
    }

    /// Adds an action after the message.
    pub fn action(mut self, action: ToastAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Sets how long the card stays.
    pub fn timeout(mut self, timeout: Option<f32>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the fraction done, clamped to `0..=1`; the card then has no timer.
    pub fn progress(mut self, fraction: f32) -> Self {
        self.progress = Some(clamp_fraction(fraction));
        self.timeout = None;
        self
    }
}

/// `fraction` clamped to `0..=1`, and 0 when it is not a number.
fn clamp_fraction(fraction: f32) -> f32 {
    if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) }
}

/// A posted toast's handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(u64);

impl ToastId {
    /// The handle numbered `value` (the queue on craft-ui-rt numbers its own).
    pub(crate) fn from_value(value: u64) -> Self {
        ToastId(value)
    }

    /// The handle as a number, unique among the toasts of one queue: what a host reports to automation.
    pub fn value(self) -> u64 {
        self.0
    }
}
