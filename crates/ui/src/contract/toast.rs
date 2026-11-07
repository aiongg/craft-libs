//! The toast host (15.5): one layer per viewport that shows each queued toast as a `Notice` in `NoticeLayout::Card`.

use super::overlay::{self, Anchor, Layer, Overlay, Place};
use super::{Props as _, Show as _};
use crate::notices::{Notice, NoticeAction, NoticeLayout};
use crate::runtime::egui::memory as store;
use crate::runtime::env::Env;
use crate::runtime::memory::sub;
use crate::runtime::motion;
use crate::runtime::toast::{self, Toast, ToastEnd};
use craft_behavior::overlay::{Align, Side};
use egui::{Align2, Rect, Ui};

/// The toasts of the cards shown last, so the layer's exit fade still shows them after the queue emptied.
#[derive(Clone, Default)]
struct Shown(Vec<Toast>);

/// The layer that shows the toast queue of a viewport. The shell calls `show` once per pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct ToastHost;

impl ToastHost {
    /// Shows every queued toast of the running viewport, oldest at the top, in the viewport's end corner. A card the
    /// pointer rests on keeps its timer paused; the first show of a toast announces it. When the queue empties the
    /// layer fades out with the last cards.
    pub fn show(ui: &Ui) {
        let ctx = ui.ctx();
        let cards = toast::cards(ctx);
        let env = Env::of(ui);
        let id = sub(store::unique("craft_ui.toast_host"), crate::runtime::egui::pass::current_viewport(ctx));
        let open = !cards.is_empty();
        let presence = motion::layer_presence(ctx, id, id, env.motion, open, motion::spec::OPEN_FADE);
        if !presence.visible {
            overlay::forget_layer(ctx, id, id);
            let _ = store::take::<Shown>(ctx, sub(id, "shown"));
            return;
        }
        let toasts: Vec<Toast> = if open {
            cards.iter().map(|card| card.toast.clone()).collect()
        } else {
            store::peek_clone::<Shown>(ctx, sub(id, "shown")).unwrap_or_default().0
        };
        if open {
            store::put(ctx, sub(id, "shown"), Shown(toasts.clone()));
        }
        let overlay = Overlay::toast().place(Place::anchored(Anchor::Viewport(corner(&env)), Side::Above, Align::End));
        let layer = Layer { id, owner: id, overlay: &overlay, anchor: Rect::NOTHING, env: &env, exiting: !open, fade: presence.t };
        overlay::host(ctx, &layer, &mut |ui| {
            for (at, toast) in toasts.iter().enumerate() {
                let card = cards.get(at);
                let salt = ("toast", card.map(|card| card.id), card.is_none().then_some(at));
                let mut notice = Notice::new(toast.tone, toast.message.clone()).layout(NoticeLayout::Card).dismiss().id_salt(salt);
                if let Some(title) = &toast.title {
                    notice = notice.title(title.clone());
                }
                for action in &toast.actions {
                    notice = notice.action(NoticeAction::new(action.label.clone()));
                }
                let out = notice.show(ui);
                if let Some(card) = card {
                    toast::shown(ctx, card.id, &announcement(&env, toast), out.response.hovered());
                    if let Some(taken) = out.action {
                        toast::close(ctx, card.id, ToastEnd::Action(taken));
                    } else if out.dismissed {
                        toast::close(ctx, card.id, ToastEnd::Dismissed);
                    }
                }
            }
        });
    }
}

/// The viewport's end-bottom corner: right in left-to-right, left in right-to-left.
fn corner(env: &Env) -> Align2 {
    if env.direction.is_rtl() { Align2::LEFT_BOTTOM } else { Align2::RIGHT_BOTTOM }
}

/// What the card announces when it first shows: the title and the message.
fn announcement(env: &Env, toast: &Toast) -> String {
    let view = env.view();
    let message = view.resolve(&toast.message);
    match &toast.title {
        Some(title) => format!("{}, {}", view.resolve(title).as_str(), message.as_str()),
        None => message.as_str().to_owned(),
    }
}
