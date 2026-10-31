//! The join field: enter a session code or an invite link (N5.11).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use crate::collab::CollabTransport;

craft_ui::words! {
    /// The field's words, English by default (FigCraft's).
    #[non_exhaustive]
    pub struct JoinLabels {
        hint: "join.hint" = "Code or invite link";
        name: "join.name" = "Join code or invite link";
        join: "join.join" = "Join";
        joining: "join.joining" = "Joining…";
        retry: "join.retry" = "Try Again";
        cancel: "join.cancel" = "Cancel";
    }
}

/// FigCraft's join field (one component for the share popover, the home screen and the Join
/// dialog): a text field (its text is not read out by AccessKit: it may hold a link's secret)
/// and Join, enabled when the field isn't blank and no join is pending; Enter joins. On join the
/// field clears; while joining it shows Joining… with Cancel, and a failure's reason with
/// Try Again.
pub struct JoinField<'a> {
    id_salt: IdSalt,
    transport: &'a mut dyn CollabTransport,
    typed: &'a mut String,
    width: f32,
    focus: bool,
    labels: Option<JoinLabels>,
}

impl<'a> JoinField<'a> {
    pub fn new(id_salt: impl AsIdSalt, transport: &'a mut dyn CollabTransport, typed: &'a mut String) -> Self {
        Self { id_salt: IdSalt::new(id_salt), transport, typed, width: 196.0, focus: false, labels: None }
    }

    /// Width of the field (FigCraft's `JOIN_WIDTH`, 196 points).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Take the keyboard focus this pass.
    pub fn focus(mut self, on: bool) -> Self {
        self.focus = on;
        self
    }

    pub fn labels(mut self, labels: JoinLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Calls the transport's `join` and `cancel_join`; `true` when a join started this pass.
    // figcraft: crates/ui-egui/src/live.rs:718 `join_field`, `:738` `start_join`, `:759`
    // `join_progress`, `:808` `retry_join`.
    pub fn show(self, ui: &mut Ui) -> (Response, bool) {
        todo!()
    }
}
