//! Steps 11 and 15, apply: the events written to the binding.

use crate::contract::cx::gesture_key;
use crate::contract::values::{Checked, landed};
use crate::contract::{Component, Cx, ValueOf};
use crate::runtime::egui::repaint;
use craft_behavior::{CancelReason, EditEvent, Gesture, Mixed};
use std::sync::Arc;

/// Steps 11 and 15, apply: the events not yet applied are checked and written to the binding. A read-only component refuses
/// every one. A given caller gets `Begin` in the pass a gesture began and its first `Update` in the next, because it has
/// no source to hold the value now.
pub(crate) fn apply_values<C: Component>(comp: &mut C, cx: &mut Cx<'_, C>) {
    let from = cx.applied;
    cx.applied = cx.events.len();
    if cx.core.resolved.read_only {
        // A gesture's end still reaches the app; only events that carry a value are refused.
        let mut at = 0;
        cx.events.retain(|event| {
            let keep = at < from || matches!(event, EditEvent::Cancel { .. });
            at += 1;
            keep
        });
        cx.applied = cx.events.len();
        return;
    }
    let Some(mut bind) = comp.bind() else { return };
    // The folded `Begin`: the updates after a `Begin` of this pass wait for the next pass of a given binding.
    let mut held = None;
    if bind.is_given()
        && let Some(begin) = cx.events[from..].iter().rposition(|event| matches!(event, EditEvent::Begin { .. }))
    {
        let after = &cx.events[from + begin + 1..];
        if !after.is_empty() && after.iter().all(|event| matches!(event, EditEvent::Update { .. })) {
            held = after.last().and_then(|event| event.value().cloned());
            cx.events.truncate(from + begin + 1);
            cx.applied = cx.events.len();
        }
    }
    let slot = gesture_key(cx.core.id);
    let (mut emitted, mut began, mut landing, mut last_update) = (None, false, None, None);
    let mut changed = false;
    let mut at = from;
    while at < cx.events.len() {
        match bind.checked(&mut cx.events[at]) {
            Checked::Keep => {}
            Checked::Cancel => {
                if let EditEvent::Commit { id, before, .. } = cx.events[at].clone() {
                    cx.events[at] = EditEvent::cancel(id, CancelReason::Refused, Some(before));
                    let dead = cx.core.press_held();
                    let gesture = cx.core.memory.shown::<Gesture<ValueOf<C>>>(slot);
                    *gesture = gesture.ended(dead);
                    if matches!(cx.events[at], EditEvent::Cancel { before: Some(Mixed::Mixed), .. }) {
                        cx.shown = None;
                        cx.mixed = true;
                    }
                }
            }
            Checked::Drop => {
                cx.events.remove(at);
                cx.applied = cx.events.len();
                continue;
            }
        }
        let event = &cx.events[at];
        match event {
            EditEvent::Begin { before: Mixed::Same(value), .. } => {
                emitted = Some(value.clone());
                began = true;
            }
            EditEvent::Update { value, .. } => {
                last_update = Some(value.clone());
                if bind.writes(event) || bind.is_given() {
                    emitted = Some(value.clone());
                }
            }
            _ => {}
        }
        if let EditEvent::Update { value, .. } | EditEvent::Commit { after: value, .. } = event {
            changed |= bind.get() != Some(Mixed::Same(value.clone()));
        }
        bind.apply(event);
        if let Some(value) = landed(event) {
            landing = Some(value.clone());
        }
        at += 1;
    }
    // The given value of the pass a gesture began in, for step 4's host-change comparison next pass.
    let given_now = if began && bind.is_given() {
        match bind.get() {
            Some(Mixed::Same(value)) => Some(value),
            _ => None,
        }
    } else {
        None
    };
    // A pass that changes the value, or holds an update for the next pass, runs again before painting, so a control drawn
    // above the field shows the edit in the frame it happens. egui gives the rerun no input events and caps a frame at
    // two passes, so nothing is delivered twice.
    if held.is_some() {
        crate::runtime::egui::sizing::request_discard(&cx.core.ctx, crate::runtime::egui::sizing::Rerun::HeldUpdate);
    } else if changed {
        crate::runtime::egui::sizing::request_discard(&cx.core.ctx, crate::runtime::egui::sizing::Rerun::Edit);
    }
    let repaint_next = held.is_some();
    let gesture = cx.core.memory.shown::<Gesture<ValueOf<C>>>(slot);
    let (running, watched) = (gesture.is_running().then_some(gesture.id), gesture.watched);
    if gesture.is_running() {
        // The value held for the next pass is already readable from the response's `shown` in this one, so a host write of
        // it is the gesture's own.
        gesture.emitted = held.clone().or(emitted).or(gesture.emitted.take());
        gesture.last_given = given_now.or(gesture.last_given.take());
        gesture.held = held.or(gesture.held.take());
        // A value the check replaced is the gesture's proposal, so the next pass shows it.
        if let Some(value) = last_update {
            gesture.proposal = Some(value);
        }
    }
    // The owner's eviction while the gesture runs ends it as removed (7.2).
    if running.is_some() || watched {
        gesture.watched = running.is_some();
        let ctx = cx.core.ctx.clone();
        let watch = cx.core.memory.shown::<crate::runtime::edit::RemovalWatch>(slot);
        match running {
            Some(id) => watch.arm(&ctx, id),
            None => watch.disarm(),
        }
    }
    if repaint_next {
        repaint::request(&cx.core.ctx);
    }
    if let Some(value) = landing {
        if bind.is_kept() {
            let id = cx.core.id;
            bind.keep(&mut *cx.core.memory, id, value.clone());
        }
        cx.shown = Some(Arc::new(value));
        cx.mixed = false;
    }
    if let Some(value) = cx.shown.as_deref() {
        cx.validity = bind.validity(value);
    }
}
