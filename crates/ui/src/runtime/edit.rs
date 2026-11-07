//! The pass's edit events and edit-command routing: the context-free calls an app makes around edits.

use super::egui::{memory::Service, repaint};
use craft_behavior::{CancelReason, GestureId};
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

/// The newest gesture id handed out in this context.
#[derive(Clone, Copy, Default)]
struct Gestures(GestureId);

/// A gesture id no gesture of this context has had: ids grow in the order gestures begin, across components and
/// across a component's eviction, as `GestureHost` assumes.
pub(crate) fn next_gesture_id(ctx: &Context) -> GestureId {
    Service::<Gestures>::of(ctx).with(|gestures| {
        gestures.0 = gestures.0.next();
        gestures.0
    })
}
