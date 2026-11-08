//! The outer gesture of a container whose `ContainerProps::outer_edit` is on: it opens with the container's body and
//! ends with the container's close, so a `GestureHost` keeps one undo step for the edits of the body's fields.

use crate::contract::BodyCx;
use crate::runtime::edit::{RemovalWatch, next_gesture_id};
use crate::runtime::frame::{OuterEdit, note_outer_edit};
use crate::runtime::memory::{MemoryStage, sub};
use craft_behavior::GestureId;
use egui::{Context, Id};
use std::sync::Arc;

/// How a container's close ends its outer gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OuterEnd {
    /// An OK: the edits stay, as one undo step.
    Commit,
    /// Any other close: the edits are taken back.
    Cancel,
}

/// Where the outer gesture of one open of the container stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    /// No open of the container has begun a gesture: the next pass that shows the body begins one.
    #[default]
    Idle,
    /// The gesture runs.
    Open(GestureId),
    /// The close ended the gesture; the body may still be on screen while the layer fades out.
    Ended,
}

/// The outer gesture under a container's body, in the body's shown memory. The removal watch ends it as removed when
/// the container is evicted while it runs.
#[derive(Clone, Default)]
struct Outer {
    stage: Stage,
    watch: RemovalWatch,
}

/// The slot of the outer gesture under the body `body`.
fn slot(body: Id) -> Id {
    sub(body, "outer_edit")
}

/// Opens the outer gesture of a container whose body runs in this pass, once for each open. The container calls it
/// from a region hook of its body (the header), so the gesture begins before any field of the body edits.
pub(crate) fn keep(ctx: &Context, cx: &mut BodyCx<'_>) {
    let outer = cx.memory.shown::<Outer>(slot(cx.id));
    if outer.stage != Stage::Idle {
        return;
    }
    let id = next_gesture_id(ctx);
    outer.stage = Stage::Open(id);
    outer.watch.arm(ctx, id);
    note_outer_edit(ctx, OuterEdit::Begin { id, label: Arc::from(cx.resolved.label), doc_key: cx.resolved.doc_key });
}

/// Settles the outer gesture of the container `id` at its finish. `shown`: its body ran in this pass. `closed`: it
/// closed in this pass, and how that ends the gesture. A body that no longer runs without a close (the app closed the
/// container through its open binding) cancels the gesture, and the next open begins a new one.
pub(crate) fn settle(ctx: &Context, memory: &mut MemoryStage, id: Id, shown: bool, closed: Option<OuterEnd>) {
    if memory.is_peek() {
        return;
    }
    let outer = memory.shown::<Outer>(slot(sub(id, "body")));
    let end = match (closed, shown) {
        (Some(end), _) => end,
        (None, true) => return,
        (None, false) => {
            if outer.stage == Stage::Ended {
                outer.stage = Stage::Idle;
            }
            OuterEnd::Cancel
        }
    };
    let Stage::Open(gesture) = outer.stage else { return };
    outer.stage = if shown { Stage::Ended } else { Stage::Idle };
    outer.watch.disarm();
    note_outer_edit(
        ctx,
        match end {
            OuterEnd::Commit => OuterEdit::Commit { id: gesture },
            OuterEnd::Cancel => OuterEdit::Cancel { id: gesture },
        },
    );
}
