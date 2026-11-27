//! Steps 11 and 15, apply: the events written to the binding.

use crate::contract::cx::{CxCore, gesture_key};
use crate::contract::values::Checked;
use crate::contract::{BindMut, Component, Cx, Validity};
use craft_behavior::{Applied, CancelReason, EditEvent, Gesture, Mixed, Value};
use std::sync::Arc;

/// What an apply step did to the component's value: a write moves the node's component key, so the
/// binding's readers rerun through ui-rt's reads, not through a rerun of the pass.
#[derive(Default)]
pub(crate) struct ValueEdit {
    /// A given binding's value changed: the app applies it, and the framework cannot see who read it.
    given: bool,
    /// The value in the binding's place or kept slot changed.
    written: bool,
    /// The component's shown value changed.
    shown: bool,
}

impl ValueEdit {
    /// The binding or the shown value changed.
    pub(crate) fn wrote(&self) -> bool {
        self.given || self.written || self.shown
    }
}

/// Steps 11 and 15, apply: the events not yet applied are checked and written to the binding. A read-only component refuses
/// every one.
pub(crate) fn apply_values<C: Component>(comp: &mut C, cx: &mut Cx<'_, C>) -> ValueEdit {
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
        return ValueEdit::default();
    }
    let shown_before = cx.shown.clone();
    let Some(bind) = comp.bind() else { return ValueEdit::default() };
    let state =
        BindingState { events: &mut cx.events, applied: &mut cx.applied, shown: &mut cx.shown, mixed: &mut cx.mixed, validity: &mut cx.validity };
    apply_binding(bind, cx.core, state, from, shown_before)
}

/// Mutable value state for the apply step, shared by components that bind the same value type.
struct BindingState<'a, V> {
    events: &'a mut Vec<EditEvent<V>>,
    applied: &'a mut usize,
    shown: &'a mut Option<Arc<V>>,
    mixed: &'a mut bool,
    validity: &'a mut Validity<V>,
}

fn apply_binding<V: Value>(
    mut bind: BindMut<'_, V>,
    core: &mut CxCore<'_>,
    state: BindingState<'_, V>,
    from: usize,
    shown_before: Option<Arc<V>>,
) -> ValueEdit {
    let slot = gesture_key(core.id);
    let (mut emitted, mut began, mut landing, mut last_update) = (None, false, None, None);
    let (mut given, mut written) = (false, false);
    let mut at = from;
    while at < state.events.len() {
        match bind.checked(&mut state.events[at]) {
            Checked::Keep => {}
            Checked::Cancel => {
                if let EditEvent::Commit { id, before, .. } = state.events[at].clone() {
                    state.events[at] = EditEvent::cancel(id, CancelReason::Refused, Some(before));
                    let dead = core.press_held();
                    let gesture = core.memory.shown::<Gesture<V>>(slot);
                    *gesture = gesture.ended(dead);
                    if matches!(state.events[at], EditEvent::Cancel { before: Some(Mixed::Mixed), .. }) {
                        *state.shown = None;
                        *state.mixed = true;
                    }
                }
            }
            Checked::Drop => {
                state.events.remove(at);
                *state.applied = state.events.len();
                continue;
            }
        }
        let event = &state.events[at];
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
        // Every event that changes the value counts: a proposal, commit or set, and a cancel that restores.
        if bind.is_given()
            && let Some(value) = event.landed()
        {
            given |= bind.get() != Some(Mixed::Same(value.clone()));
        }
        written |= bind.apply(event) != Applied::Unchanged;
        if let Some(value) = event.landed() {
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
    let gesture = core.memory.shown::<Gesture<V>>(slot);
    let running = gesture.is_running().then_some(gesture.id);
    if gesture.is_running() {
        gesture.emitted = emitted.or(gesture.emitted.take());
        gesture.last_given = given_now.or(gesture.last_given.take());
        // A value the check replaced is the gesture's proposal, so the next pass shows it.
        if let Some(value) = last_update {
            gesture.proposal = Some(value);
        }
    }
    gesture.watched = running.is_some();
    if let Some(value) = landing {
        if bind.is_kept() {
            let id = core.id;
            written |= state.shown.as_deref() != Some(&value);
            bind.keep(&mut *core.memory, id, value.clone());
        }
        *state.shown = Some(Arc::new(value));
        *state.mixed = false;
    }
    if let Some(value) = state.shown.as_deref() {
        *state.validity = bind.validity(value);
    }
    let shown = state.shown.as_deref() != shown_before.as_deref();
    ValueEdit { given, written, shown }
}
