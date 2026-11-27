//! A window's toast queue on craft-ui-rt, shown by `contract::toast::ToastHost` as one stack on the
//! toast layer, owned by the node whose build shows it (the shell's body). A card's timer is a
//! build wake of that node at the earliest deadline, so a waiting stack runs no frame; the pointer
//! over the stack or a card holds every timer, and each starts over with its whole timeout when
//! the hold ends.

use crate::runtime::toast::{MAX_CARDS, Toast, ToastEnd, ToastId};

/// Ends kept until read, oldest dropped first.
const ENDS_KEPT: usize = 32;

struct Card {
    id: ToastId,
    toast: Toast,
    /// When the timer runs out; `None` while held or for a card with no timeout.
    deadline: Option<f64>,
    /// The host announced it (each card once, when the stack first shows it).
    announced: bool,
}

/// One window's toasts.
#[derive(Default)]
pub struct ToastQueue {
    cards: Vec<Card>,
    next: u64,
    ends: Vec<(ToastId, ToastEnd)>,
    /// The card ids and revision the stack last showed, hashed: the stack's body runs when they change.
    pub(crate) shown: u64,
    /// Moves on every change to a queued card's content (its progress).
    revision: u64,
    /// The pointer rested on a card in the stack's last run: the timers stay paused.
    pub(crate) cards_hovered: bool,
}

impl ToastQueue {
    /// An empty queue.
    pub fn new() -> ToastQueue {
        ToastQueue::default()
    }

    /// Queues `toast`. Past [`MAX_CARDS`] the oldest card that would leave by itself is dropped.
    pub fn post(&mut self, toast: Toast) -> ToastId {
        self.next += 1;
        let id = ToastId::from_value(self.next);
        self.cards.push(Card { id, toast, deadline: None, announced: false });
        while self.cards.len() > MAX_CARDS {
            let Some(at) = self.cards.iter().position(|c| c.toast.progress.is_none() && c.toast.actions.is_empty()).or(Some(0)) else { break };
            let card = self.cards.remove(at);
            self.end(card.id, ToastEnd::Dropped);
        }
        id
    }

    /// Ends `id` as dismissed.
    pub fn dismiss(&mut self, id: ToastId) {
        if let Some(at) = self.cards.iter().position(|c| c.id == id) {
            self.cards.remove(at);
            self.end(id, ToastEnd::Dismissed);
        }
    }

    /// Ends `id` as `end` (the user took an action or dismissed it on its card).
    pub fn close(&mut self, id: ToastId, end: ToastEnd) {
        if let Some(at) = self.cards.iter().position(|c| c.id == id) {
            self.cards.remove(at);
            self.end(id, end);
        }
    }

    /// Sets the progress of `id`'s card (a fraction, clamped to 0..=1).
    pub fn set_progress(&mut self, id: ToastId, fraction: f32) {
        if let Some(card) = self.cards.iter_mut().find(|c| c.id == id) {
            card.toast = card.toast.clone().progress(fraction);
            self.revision += 1;
        }
    }

    /// The queued cards, oldest first.
    pub fn cards(&self) -> impl Iterator<Item = (ToastId, &Toast)> {
        self.cards.iter().map(|c| (c.id, &c.toast))
    }

    /// What the stack's body shows, hashed: the card ids and the content revision.
    pub(crate) fn shown_key(&self) -> u64 {
        craft_ui_rt::hash_key(&(self.cards.iter().map(|c| c.id.value()).collect::<Vec<_>>(), self.revision))
    }

    /// How `id` ended, read once.
    pub fn take_end(&mut self, id: ToastId) -> Option<ToastEnd> {
        let at = self.ends.iter().position(|(e, _)| *e == id)?;
        Some(self.ends.remove(at).1)
    }

    /// The cards queued.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// No card is queued.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    fn end(&mut self, id: ToastId, end: ToastEnd) {
        if self.ends.len() == ENDS_KEPT {
            self.ends.remove(0);
        }
        self.ends.push((id, end));
    }

    /// The cards not announced yet, each marked announced: the host announces each card once.
    pub(crate) fn take_unannounced(&mut self) -> Vec<Toast> {
        self.cards.iter_mut().filter(|c| !std::mem::replace(&mut c.announced, true)).map(|c| c.toast.clone()).collect()
    }

    /// Runs the timers to `now`: a held stack clears every deadline, an unheld one starts the
    /// missing ones and ends the cards whose deadline passed. Returns the earliest deadline left.
    pub(crate) fn tick(&mut self, now: f64, held: bool) -> Option<f64> {
        if held {
            for card in &mut self.cards {
                card.deadline = None;
            }
            return None;
        }
        for card in &mut self.cards {
            if card.deadline.is_none()
                && card.toast.progress.is_none()
                && let Some(timeout) = card.toast.timeout
            {
                card.deadline = Some(now + f64::from(timeout));
            }
        }
        let expired: Vec<ToastId> = self.cards.iter().filter(|c| c.deadline.is_some_and(|d| d <= now)).map(|c| c.id).collect();
        for id in expired {
            self.cards.retain(|c| c.id != id);
            self.end(id, ToastEnd::Expired);
        }
        self.cards.iter().filter_map(|c| c.deadline).reduce(f64::min)
    }
}
