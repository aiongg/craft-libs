//! The toast stack: draws the toast queue's cards as `Notice`s in `NoticeLayout::Card`.

use crate::contract::toast::{CardOutcome, ToastHost};
use crate::contract::{Props as _, Show as _};
use crate::notices::{Notice, NoticeAction, NoticeLayout};
use crate::runtime::toast::ToastEnd;
use egui::Ui;

/// The cards of the toast queue. Whoever owns the window root calls `show` once per pass.
#[derive(Clone, Copy, Debug, Default, crate::contract::Pattern)]
pub struct ToastStack;

impl ToastStack {
    /// Shows every queued toast of the running viewport through the toast host. The stack is a layer over the
    /// viewport and takes no room in its parent, so a measuring run of the parent's body skips it: the real run
    /// opens the host's stages once.
    pub fn show(ui: &Ui) {
        if crate::runtime::measure::active(ui.ctx()) {
            return;
        }
        ToastHost::show(ui, &mut |ui, toast, salt| {
            let mut notice = Notice::new(toast.tone, toast.message.clone()).layout(NoticeLayout::Card).dismiss().id_salt(salt);
            if let Some(title) = &toast.title {
                notice = notice.title(title.clone());
            }
            for action in &toast.actions {
                notice = notice.action(NoticeAction::new(action.label.clone()));
            }
            let out = notice.show(ui);
            let end = match (out.action, out.dismissed) {
                (Some(taken), _) => Some(ToastEnd::Action(taken)),
                (None, true) => Some(ToastEnd::Dismissed),
                (None, false) => None,
            };
            CardOutcome::new(out.response.hovered(), end)
        });
    }
}
