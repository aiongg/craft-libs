//! [`PreviewStatus`] and [`PreviewTicket`]: the bookkeeping of a preview the app computes off the
//! UI thread for a full-window workspace (Select and Mask, Filter Gallery, Content-Aware Fill,
//! Wide Angle).

use egui::{Response, Ui};

use super::labels::FullWindowLabels;

/// One request for a preview: the workspace hands it to the app with the session it should
/// compute, and the app hands it back with the result. Tickets grow with every request, so a
/// result for an older ticket is recognised and dropped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PreviewTicket(pub u64);

/// The state every proxy-preview workspace keeps: the latest ticket asked for, the one shown, a
/// progress while the latest computes, and one error text.
///
/// - [`request`](Self::request) after every change of the session gives a new ticket and clears
///   the error; the workspace reports it as `preview_wanted` in its output, once.
/// - [`ready`](Self::ready) with the ticket of a finished result: true when it is the latest
///   ticket (show it), false for an older one (drop it).
/// - [`pending`](Self::pending) while the latest ticket has no result: the zoom bar's progress
///   bar and the polite "Updating preview" label.
/// - [`failed`](Self::failed) records the error of the latest ticket; an older ticket's failure
///   is dropped like its result.
///
/// ```
/// # use craft_ui::dialog::full_window::PreviewStatus;
/// let mut status = PreviewStatus::new();
/// let first = status.request();
/// let second = status.request();
/// assert!(status.pending());
/// // The app's job for `first` finishes after `second` was asked for: dropped.
/// assert!(!status.ready(first));
/// assert!(status.ready(second));
/// assert!(!status.pending());
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreviewStatus {
    /// The latest ticket asked for; 0 before the first request.
    want: u64,
    /// The latest ticket whose result was shown or failed.
    shown: u64,
    /// The latest ticket's progress, 0 to 1, as the app reported it.
    progress: Option<f32>,
    /// The latest ticket's error.
    error: Option<String>,
}

impl PreviewStatus {
    /// No request yet, nothing pending.
    pub fn new() -> Self {
        Self::default()
    }

    /// A new ticket for the session as it is now; clears the error and the progress.
    #[expect(clippy::todo, reason = "skeleton: K17")]
    pub fn request(&mut self) -> PreviewTicket {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:376-377 (`want` "bumped whenever the
        // state changes; a result for an older generation is dropped").
        todo!()
    }

    /// The app's result for `ticket` is in: true when it is the latest ticket (the workspace shows
    /// it), false when a newer one was asked for since (the result is dropped).
    #[expect(clippy::todo, reason = "skeleton: K17")]
    pub fn ready(&mut self, ticket: PreviewTicket) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:380 (`shown`), :391 (`error`).
        let _ = ticket;
        todo!()
    }

    /// The app's job for `ticket` failed with `message` (translated by the app): kept as the
    /// error when `ticket` is the latest; true then.
    #[expect(clippy::todo, reason = "skeleton: K17")]
    pub fn failed(&mut self, ticket: PreviewTicket, message: impl Into<String>) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:391 (`error`, one text under the panel).
        let _ = (ticket, message.into());
        todo!()
    }

    /// The app's progress for `ticket`, 0 to 1; ignored for an older ticket.
    #[expect(clippy::todo, reason = "skeleton: K17")]
    pub fn report_progress(&mut self, ticket: PreviewTicket, fraction: f32) {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:694-697 (the progress bar "while the
        // preview renders").
        let _ = (ticket, fraction);
        todo!()
    }

    /// The latest ticket has no result yet.
    #[expect(clippy::todo, reason = "skeleton: K17")]
    pub fn pending(&self) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:376-380 (spinner while `shown != want`).
        todo!()
    }

    /// The latest ticket's progress while [`pending`](Self::pending): `Some(None)` while pending
    /// with no progress reported (an indeterminate bar), `None` when nothing is pending.
    #[expect(clippy::todo, reason = "skeleton: K17")]
    pub fn progress(&self) -> Option<Option<f32>> {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:694-697 (`(p > 0.0).then_some(p)`).
        todo!()
    }

    /// The latest ticket's error.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Draws the status under a panel: nothing while fine; the error as "Preview failed: …" in
    /// `status.bad.fg` (`caption` role, wrapping); and while [`pending`](Self::pending) a polite
    /// AccessKit live label "Updating preview" with no visible text (the zoom bar's progress bar,
    /// where the workspace has one, is the visible cue).
    #[expect(clippy::todo, reason = "skeleton: K17")]
    pub fn show(&self, ui: &mut Ui, labels: &FullWindowLabels) -> Response {
        // new: PhotoCraft shows the error as faint danger text (select_mask_ui.rs:391 `error`);
        // the live label is the step's AccessKit rule (step7-workspaces.md "AccessKit").
        let _ = (ui, labels);
        todo!()
    }
}
