//! The pass's edit events and edit-command routing: the context-free calls an app makes around edits.

use super::egui::repaint;
use craft_behavior::GestureId;
use egui::Context;

/// Notes that the component running gesture `id` was evicted: the gesture ends with `CancelReason::Removed`, which the
/// edit hosts read from `FrameOutput::removed_gestures`.
pub(crate) fn note_removed(ctx: &Context, id: GestureId) {
    super::frame::note_removed_gesture(ctx, id);
    repaint::request(ctx);
}

/// The removal watch of a running gesture, a shown slot beside it under the same owner. Its drop is the dispose hook of
/// the owner's eviction: only the slot the runtime moves through the stage is armed, so a clone dropped by a read
/// notes nothing.
#[derive(Default)]
#[doc(hidden)]
pub struct RemovalWatch {
    armed: Option<(Context, GestureId)>,
}

impl RemovalWatch {
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
        if let Some((ctx, id)) = self.armed.take() {
            note_removed(&ctx, id);
        }
    }
}
