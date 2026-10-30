//! [`BlockingOverlay`]: an input block with no chrome, for work the window must not interrupt.

use egui::accesskit::Role;
use egui::{Color32, Context, CursorIcon, Frame, Key, Modal, Modifiers};

use super::frame::DialogFrame;
use super::pass::{self, Entry};
use crate::controls::{Button, Spinner};
use crate::primitives::{Surface, SurfaceKind};
use crate::theme::{ToEgui, Tokens, TokensOf, current};

/// A modal layer with no frame, no title, no footer and no Tab stops: the window gets no input
/// while it is shown, and the cursor shows `cursor`. Show it every pass the work lasts.
///
/// ```
/// # use craft_ui::dialog::BlockingOverlay;
/// # egui::__run_test_ctx(|ctx| {
/// BlockingOverlay::new("exporting").show(ctx);
/// # });
/// ```
///
/// - The backdrop is transparent; [`scrim`](BlockingOverlay::scrim) draws `surface.scrim`.
/// - Nothing is in the AccessKit tree unless the host gives it a [`name`](BlockingOverlay::name)
///   or a [`message`](BlockingOverlay::message).
/// - A [`message`](BlockingOverlay::message) or [`cancel`](BlockingOverlay::cancel) adds a
///   centred card on the dialog surface: a spinner, the message and a Cancel button. `show`
///   returns true the pass Cancel is pressed or, while the overlay is the top modal, Esc is.
/// - [`any_modal`](super::any_modal) reports it, and no canvas pass-through reaches under it.
#[must_use = "show the overlay with `show`"]
pub struct BlockingOverlay {
    id: egui::Id,
    cursor: CursorIcon,
    scrim: bool,
    name: Option<String>,
    message: Option<String>,
    cancel: Option<String>,
}

impl BlockingOverlay {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id: egui::Id::unique(("craft_ui::dialog::blocking", id)),
            cursor: CursorIcon::Wait,
            scrim: false,
            name: None,
            message: None,
            cancel: None,
        }
    }

    /// The cursor while the overlay is shown; `Wait` by default.
    pub fn cursor(mut self, cursor: CursorIcon) -> Self {
        self.cursor = cursor;
        self
    }

    /// Dims the window with `surface.scrim`.
    pub fn scrim(mut self, scrim: bool) -> Self {
        self.scrim = scrim;
        self
    }

    /// The accessible name, for a host that wants the block announced.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Shows the card with a spinner and `message`, which is also the block's accessible name.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Shows the card with a button labelled `label`; Esc does the same.
    pub fn cancel(mut self, label: impl Into<String>) -> Self {
        self.cancel = Some(label.into());
        self
    }

    /// Shows the overlay for this pass; true the pass the user cancels (never without a
    /// [`cancel`](BlockingOverlay::cancel) label).
    pub fn show(self, ctx: &Context) -> bool {
        let Self { id, cursor, scrim, name, message, cancel } = self;
        let color = if scrim { current(ctx).color.surface.scrim.to_egui() } else { Color32::TRANSPARENT };
        let mut area = Modal::default_area(id);
        if let Some(name) = message.clone().or(name) {
            area = area.accessible_name(name);
        }
        let card = message.is_some() || cancel.is_some();
        if card {
            area = area.role(Role::Dialog);
        }
        let mut cancelled = false;
        let shown = Modal::new(id).area(area).backdrop_color(color).frame(Frame::NONE).show(ctx, |ui| {
            if !card {
                return;
            }
            let pad = DialogFrame::style(&Tokens::of(ui), ui.pixels_per_point()).pad;
            Surface::new(SurfaceKind::Dialog).show(ui, |ui| {
                egui::Frame::NONE.inner_margin(pad).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(Spinner::new());
                        if let Some(message) = &message {
                            ui.label(message);
                        }
                    });
                    if let Some(label) = &cancel {
                        ui.add_space(pad);
                        ui.with_layout(crate::direction::trailing(ui), |ui| {
                            cancelled = ui.add(Button::new(label.as_str()).id_salt("cancel")).clicked();
                        });
                    }
                });
            });
        });
        if cancel.is_some() && shown.is_top_modal && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            cancelled = true;
        }
        pass::record(ctx, Entry { id, layer: shown.response.layer_id, rect: ctx.content_rect(), modal: true });
        ctx.set_cursor_icon(cursor);
        cancelled
    }
}
