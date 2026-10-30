//! [`BlockingOverlay`]: an input block with no chrome, for work the window must not interrupt.

use egui::{Color32, Context, CursorIcon, Frame, Modal};

use super::pass::{self, Entry};
use crate::theme::{ToEgui, current};

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
/// - Nothing is in the AccessKit tree unless the host gives it a [`name`](BlockingOverlay::name).
/// - [`any_modal`](super::any_modal) reports it, and no canvas pass-through reaches under it.
#[must_use = "show the overlay with `show`"]
pub struct BlockingOverlay {
    id: egui::Id,
    cursor: CursorIcon,
    scrim: bool,
    name: Option<String>,
}

impl BlockingOverlay {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self { id: egui::Id::unique(("craft_ui::dialog::blocking", id)), cursor: CursorIcon::Wait, scrim: false, name: None }
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

    pub fn show(self, ctx: &Context) {
        let Self { id, cursor, scrim, name } = self;
        let color = if scrim { current(ctx).color.surface.scrim.to_egui() } else { Color32::TRANSPARENT };
        let mut area = Modal::default_area(id);
        if let Some(name) = name {
            area = area.accessible_name(name);
        }
        let shown = Modal::new(id).area(area).backdrop_color(color).frame(Frame::NONE).show(ctx, |_| {});
        pass::record(ctx, Entry { id, layer: shown.response.layer_id, rect: ctx.content_rect(), modal: true });
        ctx.set_cursor_icon(cursor);
    }
}
