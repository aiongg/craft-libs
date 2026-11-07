//! The pass's edit events and edit-command routing: the context-free calls an app makes around edits.

use super::egui::{memory::Service, repaint};
use craft_behavior::CancelReason;
use egui::Context;

/// The latest `cancel_all`: a serial that grows with each call, and why.
#[derive(Clone, Copy, Default)]
struct CancelAll {
    serial: u64,
    reason: Option<CancelReason>,
}

/// Cancels every running gesture at its next step 4, which then stays cancelled under its press. The app's undo and
/// redo commands call it before they change the document, so a drag ends with `reason` instead of resuming over the
/// changed value.
pub fn cancel_all(ctx: &Context, reason: CancelReason) {
    Service::<CancelAll>::of(ctx).with(|all| {
        all.serial += 1;
        all.reason = Some(reason);
    });
    repaint::request(ctx);
}

/// The serial and reason of the latest `cancel_all`; serial 0 before the first.
pub(crate) fn latest(ctx: &Context) -> (u64, Option<CancelReason>) {
    Service::<CancelAll>::of(ctx).with(|all| (all.serial, all.reason))
}
