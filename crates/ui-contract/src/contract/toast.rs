//! The toast host (15.5): a window's queued toasts as one stack on the toast layer, a layer child of the node whose
//! build shows it (the window root's body), at the end-bottom corner of its body. The queue (`ToastQueue`) keeps the
//! cards and their timers; a timer is a build wake of the showing node at the earliest deadline, so a waiting stack
//! runs no frame. A component of a higher layer draws each card (`notices::ToastStack`). Each card is announced once,
//! when the stack first shows it.

pub use crate::contract::rt::toast::ToastQueue;

use crate::contract::rt::overlay::{LayerScope, LayerShow, Surface, show_layer};
use crate::contract::{IdSalt, POPUP_PAD, Ui};
use crate::runtime::access::Politeness;
use crate::runtime::env::Env;
use crate::runtime::focus::{FocusPolicy, FocusTarget};
use crate::runtime::toast::{Toast, ToastEnd, ToastId};
use craft_ui_rt::{Align, AnnouncePoliteness, Layer, LayerSpec, Placement, Rect, Side, Vec2};

/// The stack's key among its owner's layer children.
const STACK_KEY: u64 = 0x746f_6173_74;

/// What a drawn card reports back to the host.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct CardOutcome {
    /// The pointer rests on the card; the timers stay paused.
    pub hovered: bool,
    /// How the user ended the card in this run.
    pub end: Option<ToastEnd>,
}

impl CardOutcome {
    /// A card the pointer rests on when `hovered`, which the user ended as `end`.
    pub fn new(hovered: bool, end: Option<ToastEnd>) -> Self {
        CardOutcome { hovered, end }
    }
}

/// The layer that shows a window's toast queue. The window root's body calls `show` in its build with its queue and
/// its body's rect, so each window shows its own toasts in its own body.
#[derive(Clone, Copy, Debug, Default)]
pub struct ToastHost;

impl ToastHost {
    /// Shows every card of `queue`, oldest at the top, at the end-bottom corner of `body` (a window rect), drawing each
    /// with `card(ui, toast, salt)`, where `salt` is stable for the card. The pointer on the stack or a card keeps every
    /// timer paused. A card the user ends while focus is in it hands focus to the card after it, else the card before.
    /// The node `ui` builds builds again at the earliest deadline. A measuring run shows nothing: the layer takes no
    /// room in the body.
    pub fn show(ui: &mut Ui, queue: &mut ToastQueue, body: Rect, card: &mut dyn FnMut(&mut Ui, &Toast, IdSalt) -> CardOutcome) {
        Self::show_at(ui, queue, body, None, card);
    }

    /// Shows the queue along the bottom of its body at `alignment` (`Min` the left, `Max` the right), or at the
    /// locale's trailing bottom corner when `None`.
    pub fn show_at(
        ui: &mut Ui,
        queue: &mut ToastQueue,
        body: Rect,
        alignment: Option<Align>,
        card: &mut dyn FnMut(&mut Ui, &Toast, IdSalt) -> CardOutcome,
    ) {
        if ui.is_measuring() {
            return;
        }
        ui.ctx().set_toast_host(ui.id());
        let env = Env::of(ui);
        let rtl = env.direction.is_rtl();
        let cx = &mut *ui.cx;
        let stack = cx.peek_layer::<LayerScope>(Layer::Toast, STACK_KEY);
        // The pointer over the stack's frame, or on a card as the card reported it last, holds the timers.
        let held = stack.is_some_and(|p| p.local.hovered) || queue.cards_hovered;
        let next = queue.tick(cx.now(), held);
        cx.request_build_at(next);
        if queue.is_empty() {
            queue.cards_hovered = false;
            return;
        }
        // Each new card is read out once, as the stack first shows it, by the node that shows the stack.
        let fresh = queue.take_unannounced();
        if !fresh.is_empty() {
            for toast in fresh {
                let politeness = match toast.politeness {
                    Politeness::Polite => AnnouncePoliteness::Polite,
                    Politeness::Assertive => AnnouncePoliteness::Assertive,
                };
                ui.cx.announce(toast.message.resolved(&env.words).as_str(), politeness);
            }
        }
        let shown = queue.shown_key();
        let rerun = shown != queue.shown;
        queue.shown = shown;
        let tokens = ui.flow.tokens.clone();
        let align = alignment.unwrap_or(if rtl { Align::Min } else { Align::Max });
        let corner = match align {
            Align::Min => body.left_bottom(),
            Align::Center => body.center_bottom(),
            Align::Max => body.right_bottom(),
        };
        let corner = Rect::from_min_size(corner, Vec2::ZERO);
        let spec = LayerSpec::new(Placement::Beside { rect: corner, side: Side::Above, align, gap: tokens.space.pad.regular });
        let show = LayerShow {
            layer: Layer::Toast,
            spec,
            surface: Surface::Popup,
            pad: POPUP_PAD.points(&tokens),
            share: Vec2::new(0.4, 0.5),
            anchor: Some(corner),
            role: accesskit::Role::Group,
            hover: true,
            sweep: ui.flow.sweep,
            focus: FocusPolicy::TAB_LAYER,
            entry: None,
            host: None,
            on_activate: false,
            enter: false,
        };
        let cards: Vec<(ToastId, Toast)> = queue.cards().map(|(id, toast)| (id, toast.clone())).collect();
        let id = ui.flow.id.with("craft-toasts");
        // A kept stack runs no card and reports nothing new.
        let mut ran = None;
        show_layer(ui.cx, id, STACK_KEY, show, rerun, &mut |ui| ran = Some(run_cards(ui, id, &cards, card)));
        let Some(StackRun { hovered, ends, focus }) = ran else { return };
        queue.cards_hovered = hovered;
        for (toast, end) in ends {
            queue.close(toast, end);
        }
        if let Some(target) = focus {
            ui.request_focus(FocusTarget::Id(target));
        }
    }
}

/// What one run of the stack's body found: whether the pointer rests on a card, the cards the user ended, and the
/// card focus goes on to.
#[derive(Debug, Default)]
struct StackRun {
    hovered: bool,
    ends: Vec<(ToastId, ToastEnd)>,
    focus: Option<crate::contract::Id>,
}

/// Runs the cards of the stack whose content is keyed under `stack`. Each card's id is `stack` with its salt, so the
/// host knows which card holds focus.
fn run_cards(
    ui: &mut Ui<'_, '_>,
    stack: crate::contract::Id,
    cards: &[(ToastId, Toast)],
    card: &mut dyn FnMut(&mut Ui, &Toast, IdSalt) -> CardOutcome,
) -> StackRun {
    let mut run = StackRun::default();
    // The cards that stay, and the place among them of the card the user ended while it held focus.
    let mut staying = Vec::new();
    let mut ended_focused = None;
    for (toast_id, toast) in cards {
        let salt = IdSalt::new(("toast", toast_id.value()));
        let card_id = stack.with_salt(salt);
        let out = card(ui, toast, salt);
        run.hovered |= out.hovered;
        match out.end {
            Some(end) => {
                if ui.focus_in_scope(card_id) == Some(true) {
                    ended_focused = Some(staying.len());
                }
                run.ends.push((*toast_id, end));
            }
            None => staying.push(card_id),
        }
    }
    // The focus goes on to the card after the ended one, else the one before; the last card's end returns it.
    run.focus = ended_focused.and_then(|at| staying.get(at).or_else(|| at.checked_sub(1).and_then(|before| staying.get(before)))).copied();
    run
}
