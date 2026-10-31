//! The render (export) queue list (N1.15).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

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
        up: "renderQueue.up" = "Up";
        down: "renderQueue.down" = "Down";
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
    /// Progress 0…1 and the seconds left when known.
    Encoding {
        progress: f32,
        remaining: Option<f64>,
    },
    Done,
    Failed(String),
    Cancelled,
}

/// A queue row: "Sequence → file.mp4", the preset, the status.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderQueueItem {
    pub id: u64,
    pub source: String,
    pub file: String,
    pub preset: String,
    pub status: RenderStatus,
}

/// What a queue control asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderQueueAction {
    MoveUp(u64),
    MoveDown(u64),
    Cancel(u64),
    Retry(u64),
    Remove(u64),
    Start,
    Stop,
    ClearFinished,
}

/// FilmCraft's export queue (Premiere's queue in Export mode): alternating rows with
/// "source → file", "preset · status" (Encoding shows its progress and the time left on its own
/// line with a 6-point progress bar), and Up, Down, Cancel (Ready or Encoding) or Retry, Remove
/// (not while encoding); a footer with Start Queue (when something is Ready and the queue is
/// stopped), Stop (while running), Clear Finished (when an item is Done, Failed or Cancelled).
/// The empty queue shows [`RenderQueueLabels::empty`]. Every button is a Tab stop in row order.
/// The time left uses `// needs: step5 craft_i18n::format` (durations).
pub struct RenderQueueList<'a> {
    id_salt: IdSalt,
    items: &'a [RenderQueueItem],
    running: bool,
    labels: Option<RenderQueueLabels>,
}

impl<'a> RenderQueueList<'a> {
    pub fn new(id_salt: impl AsIdSalt, items: &'a [RenderQueueItem], running: bool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), items, running, labels: None }
    }

    pub fn labels(mut self, labels: RenderQueueLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/export_mode/queue.rs:7 `queue_column`.
    pub fn show(self, ui: &mut Ui) -> (Response, Option<RenderQueueAction>) {
        todo!()
    }
}
