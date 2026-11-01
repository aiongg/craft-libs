//! The render (export) queue list (N1.15).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::time::Duration;

use craft_ui::behavior::{Move, Selection, SelectionChange};
use craft_ui::edit::EditEvent;
use egui::{AsIdSalt, IdSalt, Response, Ui};

craft_ui::words! {
    /// The queue's words, English by default (FilmCraft's).
    #[non_exhaustive]
    pub struct RenderQueueLabels {
        title: "renderQueue.title" = "Destinations";
        empty: "renderQueue.empty" = "Send exports here with Send to Queue, then start the queue.";
        ready: "renderQueue.ready" = "Ready";
        encoding: "renderQueue.encoding" = "Encoding {n}%";
        done: "renderQueue.done" = "Done";
        failed: "renderQueue.failed" = "Failed: {e}";
        cancelled: "renderQueue.cancelled" = "Cancelled";
        time_left: "renderQueue.timeLeft" = "{time} left";
        cancel: "renderQueue.cancel" = "Cancel";
        retry: "renderQueue.retry" = "Retry";
        remove: "renderQueue.remove" = "Remove";
        start: "renderQueue.start" = "Start Queue";
        stop: "renderQueue.stop" = "Stop";
        clear_finished: "renderQueue.clearFinished" = "Clear Finished";
    }
}

/// An item's state.
#[derive(Clone, Debug, PartialEq)]
pub enum RenderStatus {
    Ready,
    /// Progress 0…1 and the time left when the encoder knows it.
    Encoding {
        progress: f32,
        remaining: Option<Duration>,
    },
    Done,
    /// The encoder's message, in the app's words.
    Failed(String),
    Cancelled,
}

/// A queue item's id, the app's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderItemId(pub u64);

/// A queue row: "Sequence → file.mp4", the preset, the status.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderQueueItem {
    pub id: RenderItemId,
    pub source: String,
    pub file: String,
    pub preset: String,
    pub status: RenderStatus,
}

/// What the queue asks of the app, in the order the user did it this pass.
#[derive(Clone, Debug, PartialEq)]
pub enum RenderQueueEvent {
    /// A drag of rows, or Cmd/Ctrl+] and [ on the selected rows (the list's reorder; Premiere's
    /// queue moves items by drag), as one gesture.
    Reorder(EditEvent<Move<RenderItemId>>),
    Selection(SelectionChange<RenderItemId>),
    Cancel(RenderItemId),
    Retry(RenderItemId),
    /// The row's Remove button, or Delete on the selection (one event per item, in order).
    Remove(RenderItemId),
    Start,
    Stop,
    ClearFinished,
}

/// FilmCraft's export queue (Premiere's queue in Export mode), drawn through the kernel's
/// [`List`](craft_ui::list::List) of [`ListRow`](craft_ui::list::ListRow)s: each row's label
/// "source → file", its detail line "preset · status"; an Encoding row shows its progress with a
/// [`Progress`](craft_ui::controls::Progress) bar and the time left on its own line
/// ([`RenderQueueLabels::time_left`] with the time written as FilmCraft's `format_eta`: seconds
/// under a minute, `m:ss` under an hour, then `h:mm:ss`, digits through `LocaleNumbers`).
/// Trailing row buttons: Cancel (Ready or Encoding) or Retry (Failed or Cancelled), and Remove
/// (not while encoding). Under the list a row of buttons: Start Queue (when something is Ready
/// and the queue is stopped), Stop (while running), Clear Finished (when an item is Done, Failed
/// or Cancelled). The empty queue shows [`RenderQueueLabels::empty`].
///
/// Keyboard and AccessKit are the list's: one Tab stop, arrows move, Shift and Cmd/Ctrl select,
/// Cmd/Ctrl+] and [ move the selected rows (decisions §4.2, lists), Delete removes them; the
/// trailing buttons are row actions; the footer buttons are Tab stops after the list.
pub struct RenderQueueList<'a> {
    id_salt: IdSalt,
    items: &'a [RenderQueueItem],
    selection: &'a Selection<RenderItemId>,
    running: bool,
    labels: Option<RenderQueueLabels>,
}

impl<'a> RenderQueueList<'a> {
    pub fn new(id_salt: impl AsIdSalt, items: &'a [RenderQueueItem], selection: &'a Selection<RenderItemId>, running: bool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), items, selection, running, labels: None }
    }

    pub fn labels(mut self, labels: RenderQueueLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/export_mode/queue.rs:7 `queue_column`;
    // crates/export/src/settings.rs:769 `format_eta`.
    pub fn show(self, ui: &mut Ui) -> RenderQueueListOutput {
        todo!()
    }
}

/// What the queue did this pass.
#[must_use]
pub struct RenderQueueListOutput {
    pub response: Response,
    pub events: Vec<RenderQueueEvent>,
}
