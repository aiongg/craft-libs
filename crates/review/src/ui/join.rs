//! The join field: enter a session code or an invite link (N5.11).

use craft_theme::ButtonVariant;
use craft_ui::controls::{Button, TextField};
use craft_ui::localized::Labels as _;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::comments::text;
use super::style::ReviewStyle;
use crate::collab::{CollabRequest, CollabTransport, JoinState};

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
/// dialog): a secret text field (masked, its text is not read out by AccessKit: it may hold a link's secret)
/// and Join, enabled when the field isn't blank and no join is pending; Enter joins. On join the
/// field clears; while joining it shows Joining… with Cancel, and a failure's reason with
/// Try Again, which reports [`CollabRequest::RetryJoin`] (the app kept what the join started with).
pub struct JoinField<'a> {
    id_salt: IdSalt,
    transport: &'a dyn CollabTransport,
    typed: &'a mut String,
    width: Option<f32>,
    focus: bool,
    labels: Option<JoinLabels>,
}

impl<'a> JoinField<'a> {
    /// A field that edits `typed` and joins through `transport`.
    pub fn new(id_salt: impl AsIdSalt, transport: &'a dyn CollabTransport, typed: &'a mut String) -> Self {
        Self { id_salt: IdSalt::new(id_salt), transport, typed, width: None, focus: false, labels: None }
    }

    /// Width of the field; [`ReviewStyle::join_width`] by default.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Take the keyboard focus this pass.
    pub fn focus(mut self, on: bool) -> Self {
        self.focus = on;
        self
    }

    /// Uses these label texts instead of the ones read from the active locale's catalog.
    pub fn labels(mut self, labels: JoinLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Join, Cancel and Try Again come back as [`CollabRequest::Join`] and
    /// [`CollabRequest::CancelJoin`]; the field reads the join's progress from
    /// [`CollabTransport::join_state`].
    // figcraft: crates/ui-egui/src/live.rs:718 `join_field`, `:738` `start_join`, `:759`
    // `join_progress`, `:808` `retry_join`.
    pub fn show(self, ui: &mut Ui) -> JoinFieldOutput {
        let Self { id_salt, transport, typed, width, focus, labels } = self;
        let labels = labels.unwrap_or_else(|| JoinLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let field_id = id.with("field");
        let state = transport.join_state();
        let mut requests = Vec::new();
        let response = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(tokens.space.gap.inline, tokens.space.gap.row);
                let mut join = false;
                ui.horizontal(|ui| {
                    let field = TextField::new(id.with("text"), &labels.name, typed.as_str())
                        .id(field_id)
                        .placeholder(&labels.hint)
                        .secret(true)
                        .width(width.unwrap_or_else(|| style.join_width(ui.painter(), &tokens)))
                        .show(ui);
                    if let Some(value) = field.event.as_ref().and_then(|event| event.value()) {
                        value.clone_into(typed);
                    }
                    if focus {
                        ui.memory_mut(|memory| memory.request_focus(field_id));
                    }
                    let filled = !typed.trim().is_empty() && !state.pending();
                    join = ui.add_enabled(filled, Button::new(&labels.join).variant(ButtonVariant::Primary).id_salt(id.with("join"))).clicked();
                    join |= field.submitted && filled;
                });
                if join {
                    let code = typed.trim().to_owned();
                    typed.clear();
                    requests.push(CollabRequest::Join(code));
                }
                match &state {
                    JoinState::Joining | JoinState::Reconnecting { .. } => {
                        ui.horizontal(|ui| {
                            text(ui, &tokens, style.body_text, tokens.color.text.muted.to_egui(), &labels.joining);
                            if ui.add(Button::new(&labels.cancel).id_salt(id.with("cancel"))).clicked() {
                                requests.push(CollabRequest::CancelJoin);
                            }
                        });
                    }
                    JoinState::Failed(reason) => {
                        text(ui, &tokens, style.body_text, tokens.color.status.bad.fg.to_egui(), reason);
                        if ui.add(Button::new(&labels.retry).id_salt(id.with("retry"))).clicked() {
                            requests.push(CollabRequest::RetryJoin);
                        }
                    }
                    JoinState::Idle | JoinState::Joined => {}
                }
            })
            .response;
        JoinFieldOutput { response, requests }
    }
}

/// What the join field did this pass.
#[must_use]
pub struct JoinFieldOutput {
    /// The field's response.
    pub response: Response,
    /// The requests the user made: join, cancel or retry.
    pub requests: Vec<CollabRequest>,
}
