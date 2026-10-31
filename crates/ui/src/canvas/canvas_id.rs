//! [`CanvasId`]: the one id a canvas is known by.

use egui::{AsIdSalt, Id, IdSalt, Rect, Response, Sense, Ui};

/// The id of one canvas, made before the canvas is: the [`ToolHost`](super::ToolHost), the
/// per-canvas state of the navigation pieces ([`ZoomAnimation`](super::ZoomAnimation),
/// [`FlickPan`](super::FlickPan), [`MomentaryKeys`](super::MomentaryKeys),
/// [`PixelGrid`](super::PixelGrid)) and [`AppShell::canvas`](crate::shell::AppShell::canvas)
/// all key their state by it. It is the id of the canvas widget: interact with
/// [`interact`](Self::interact) (or [`id`](Self::id)), so the response's id is this one.
///
/// ```no_run
/// use craft_ui::canvas::{CanvasId, ToolHost};
/// # fn demo(ui: &mut egui::Ui, rect: egui::Rect) {
/// let canvas_id = CanvasId::new(ui, "doc-1");
/// // ... MomentaryKeys::update(ctx, canvas_id, ..), ZoomAnimation::shown(ctx, canvas_id, ..) ...
/// let canvas = canvas_id.interact(ui, rect, egui::Sense::click_and_drag() | egui::Sense::FOCUSABLE);
/// let host = ToolHost::new(canvas_id);
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CanvasId(Id);

impl CanvasId {
    /// The id for the canvas keyed `salt` under `ui` (one per document view).
    pub fn new(ui: &Ui, salt: impl AsIdSalt) -> Self {
        Self(ui.make_persistent_id(IdSalt::new(salt)))
    }

    /// The canvas known by the egui id `id` (an app that already keys its views by one).
    pub fn from_id(id: Id) -> Self {
        Self(id)
    }

    /// The id of the canvas widget `response` came from.
    pub fn of(response: &Response) -> Self {
        Self(response.id)
    }

    /// The egui id: the canvas widget's.
    pub fn id(self) -> Id {
        self.0
    }

    /// A sub-id for state kept beside the canvas.
    pub fn with(self, salt: impl AsIdSalt) -> Id {
        self.0.with(salt)
    }

    /// Interact with the canvas rect under this id.
    pub fn interact(self, ui: &Ui, rect: Rect, sense: Sense) -> Response {
        ui.interact(rect, self.0, sense)
    }
}
