//! Edit gestures across the runtime: the table of running gestures, which `Ui::cancel_edits` reaches and which notes
//! a gesture whose node left the tree, and the removal watch that keeps a gesture in it.

use crate::contract::Context;
use crate::primitives::Id;
use craft_behavior::{CancelReason, GestureId};
use craft_ui_rt::{BuildCx, NodeId};
use std::sync::{Arc, Mutex};

/// The key of the gesture slot under a component's base id.
pub(crate) fn gesture_key(id: Id) -> Id {
    crate::runtime::memory::sub(id, "edit")
}

/// The running gestures of the runtime, each with the node that runs it once that node's build has named it, and the
/// gestures that ended because their node left the tree since the host last took them.
#[derive(Default)]
struct Table {
    running: Vec<(GestureId, Option<NodeId>)>,
    removed: Vec<GestureId>,
}

/// The runtime's table of running gestures, held by the runtime memory store and by each armed [`RemovalWatch`]
/// (which a node's memory keeps, so it is shared across threads like every slot).
#[derive(Clone, Default)]
pub(crate) struct LiveGestures(Arc<Mutex<Table>>);

impl LiveGestures {
    fn with<R>(&self, f: impl FnOnce(&mut Table) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    fn end(&self, gesture: GestureId, removed: bool) {
        self.with(|table| {
            table.running.retain(|(running, _)| *running != gesture);
            if removed {
                table.removed.push(gesture);
            }
        });
    }
}

/// The removal watch of a running gesture, a shown slot beside it under the same owner. The node's memory goes with the
/// node, so the watch's drop is the node's removal: an armed watch ends its gesture as removed. A clone is disarmed, so
/// a copy dropped by a read or a measuring run notes nothing.
#[derive(Default)]
#[doc(hidden)]
pub struct RemovalWatch {
    armed: Option<(LiveGestures, GestureId)>,
}

impl RemovalWatch {
    /// Arms the watch for `gesture`, entering it in the runtime's table of running gestures. Arming it again for the
    /// same gesture changes nothing.
    #[doc(hidden)]
    pub fn arm(&mut self, ctx: &Context, gesture: GestureId) {
        if self.armed.as_ref().is_some_and(|(_, armed)| *armed == gesture) {
            return;
        }
        self.disarm();
        let live = ctx.memory().with(|memory| memory.gestures.clone());
        live.with(|table| table.running.push((gesture, None)));
        self.armed = Some((live, gesture));
    }

    /// Disarms the watch: the gesture ended on its own, and its drop notes nothing.
    #[doc(hidden)]
    pub fn disarm(&mut self) {
        if let Some((live, gesture)) = self.armed.take() {
            live.end(gesture, false);
        }
    }

    /// The gesture it watches runs.
    pub(crate) fn armed(&self) -> bool {
        self.armed.is_some()
    }
}

impl Clone for RemovalWatch {
    fn clone(&self) -> Self {
        RemovalWatch::default()
    }
}

impl Drop for RemovalWatch {
    fn drop(&mut self) {
        if let Some((live, gesture)) = self.armed.take() {
            live.end(gesture, true);
        }
    }
}

/// Keeps the removal watch of the gesture slot under `slot` in step with the component's gesture after its apply step:
/// armed while `running` names a gesture, and that gesture's node entered in the table when the running build ends
/// (through the build log, the one place a hook reaches its node). A running gesture also counts for
/// `Ui::watch_gestures`.
pub(crate) fn keep_watch(memory: &mut crate::runtime::memory::MemoryStage, slot: Id, running: Option<GestureId>) {
    if memory.is_peek() {
        return;
    }
    let Some(gesture) = running else {
        // An idle component keeps no watch slot.
        if let Some(mut watch) = memory.take::<RemovalWatch>(slot) {
            watch.disarm();
        }
        return;
    };
    let ctx = memory.ctx().clone();
    memory.shown::<RemovalWatch>(slot).arm(&ctx, gesture);
    memory.note_gesture_running();
    let live = ctx.memory().with(|memory| memory.gestures.clone());
    crate::contract::context::log_build_read(move |cx| {
        let node = cx.node();
        live.with(|table| {
            if let Some(entry) = table.running.iter_mut().find(|(running, _)| *running == gesture) {
                entry.1 = Some(node);
            }
        });
    });
}

/// `Ui::cancel_edits`: every running gesture ends with `reason` at its node's next value step. The cancel serial moves,
/// and each node with a running gesture builds, so a settled node's gesture ends too.
pub(crate) fn cancel_edits(cx: &mut BuildCx<'_>, ctx: &Context, reason: CancelReason) {
    let live = ctx.memory().with(|memory| {
        memory.edits.cancel_serial += 1;
        memory.edits.cancel_reason = Some(reason);
        memory.gestures.clone()
    });
    let nodes: Vec<NodeId> = live.with(|table| table.running.iter().filter_map(|(_, node)| *node).collect());
    for node in nodes {
        cx.request_build_of(node);
    }
}

impl Context {
    /// The gestures that ended because the component running them left the tree since the last call: each ended with
    /// `CancelReason::Removed`. An edit host (`GestureHost`) takes them after each step and drops their open steps.
    pub fn take_removed_gestures(&self) -> Vec<GestureId> {
        let live = self.memory().with(|memory| memory.gestures.clone());
        live.with(|table| std::mem::take(&mut table.removed))
    }
}
