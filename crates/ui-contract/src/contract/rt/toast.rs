//! Toasts on craft-ui-rt: a window's queue shown as one stack on the toast layer, owned by the node
//! whose build shows it (the shell's body). A card's timer is a build wake of that node at the
//! earliest deadline, so a waiting stack runs no frame; the pointer over the stack holds every
//! timer, and each starts over with its whole timeout when the hold ends.

use craft_ui_rt::{Align, Layer, LayerSpec, Placement, Rect, Side, Vec2, hash_key};

use super::key::ComponentKey;
use super::overlay::{LayerShow, Surface, show_layer};
use super::run::{Component, Cx, MeasureCx, show_leaf};
use super::ui::Ui;
use crate::contract::{MemoKey, MemoProps, POPUP_PAD};
use crate::runtime::toast::{MAX_CARDS, Toast, ToastEnd, ToastId};

/// Ends kept until read, oldest dropped first.
const ENDS_KEPT: usize = 32;

/// The stack's key among its owner's layer children.
const STACK_KEY: u64 = 0x746f_6173_74;

struct Card {
    id: ToastId,
    toast: Toast,
    /// When the timer runs out; `None` while held or for a card with no timeout.
    deadline: Option<f64>,
}

/// One window's toasts.
#[derive(Default)]
pub struct ToastQueue {
    cards: Vec<Card>,
    next: u64,
    ends: Vec<(ToastId, ToastEnd)>,
    /// The card ids the stack last showed, hashed: the stack's body runs when they change.
    shown: u64,
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
        self.cards.push(Card { id, toast, deadline: None });
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

    /// Runs the timers to `now`: a held stack clears every deadline, an unheld one starts the
    /// missing ones and ends the cards whose deadline passed. Returns the earliest deadline left.
    fn tick(&mut self, now: f64, held: bool) -> Option<f64> {
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

/// Shows a window's toast queue.
pub struct ToastHost;

impl ToastHost {
    /// Shows `queue` at the end-bottom corner of `body` (a window rect) on the toast layer, as a
    /// layer child of the node `ui` builds. That node builds again at the earliest deadline.
    pub fn show(ui: &mut Ui<'_, '_>, queue: &mut ToastQueue, body: Rect) {
        let cx = &mut *ui.cx;
        let held = cx.peek_layer::<super::overlay::LayerScope>(Layer::Toast, STACK_KEY).is_some_and(|p| p.local.hovered);
        let next = queue.tick(cx.now(), held);
        cx.request_build_at(next);
        if queue.cards.is_empty() {
            return;
        }
        let ids = hash_key(&queue.cards.iter().map(|c| c.id.value()).collect::<Vec<_>>());
        let rerun = ids != queue.shown;
        queue.shown = ids;
        let tokens = ui.flow.tokens.clone();
        let corner = Rect::from_min_size(body.right_bottom(), Vec2::ZERO);
        let spec = LayerSpec::new(Placement::Beside { rect: corner, side: Side::Above, align: Align::Max, gap: tokens.space.pad.regular });
        let show = LayerShow {
            layer: Layer::Toast,
            spec,
            surface: Surface::Popup,
            pad: POPUP_PAD.points(&tokens),
            share: Vec2::new(0.4, 0.5),
            anchor: Some(corner),
            role: accesskit::Role::Group,
            hover: true,
        };
        let cards: Vec<String> = queue.cards.iter().map(|c| c.toast.message.as_str().to_owned()).collect();
        let id = ui.flow.id.with("craft-toasts");
        show_layer(ui.cx, id, STACK_KEY, show, rerun, &mut |ui| {
            for message in &cards {
                ToastCard { message }.show(ui);
            }
        });
    }
}

/// One card. Its size is estimated from its message until craft text runs on ui-rt.
struct ToastCard<'a> {
    message: &'a str,
}

impl ToastCard<'_> {
    fn show(self, ui: &mut Ui<'_, '_>) {
        show_leaf(self, ui);
    }
}

impl ComponentKey for ToastCard<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.message.key(key);
    }
}

impl Component for ToastCard<'_> {
    const ROLE: accesskit::Role = accesskit::Role::Alert;
    type Output = ();

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        let size = cx.tokens.size.icon.md;
        Vec2::new((self.message.chars().count() as f32 * size * 0.55).min(cx.room.x), cx.tokens.size.button)
    }

    fn access(&self, node: &mut accesskit::Node, _: &Cx<'_>) {
        node.set_label(self.message);
    }

    fn finish(self, _: &Cx<'_>) {}
}
