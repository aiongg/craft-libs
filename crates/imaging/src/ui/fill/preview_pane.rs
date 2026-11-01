//! [`FillPreviewPane`]: Content-Aware Fill's Preview panel (task WS-CA2).

use egui::{AsIdSalt, IdSalt, Response, Ui};

use craft_ui::canvas::RasterSource;
use craft_ui::canvas::kurbo::Rect;
use craft_ui::dialog::full_window::{FullWindowView, PreviewStatus};

/// The Preview panel: a Standard side column headed "Preview", between the canvas and the
/// settings column, with the fill result in its own view and a `FullWindowZoomBar` under it
/// (lead call 19; PhotoCraft's percent field and log slider are replaced).
///
/// - The view opens framed on the sampling window (`frame`), and keeps its own zoom and pan:
///   drag pans, the wheel zooms by `NavigationOptions`, the zoom bar steps.
/// - While [`PreviewStatus::pending`] the zoom bar shows its progress; a failure shows the error
///   ("nothing to sample from: the sampling area has no pixels outside the fill area") under the
///   view, in `status.bad.fg`; `PreviewStatus::show` carries the polite live label.
/// - The canvas in the pane is one Tab stop named by the header; it takes no tool.
#[expect(dead_code, reason = "skeleton: `show` (task WS-CA2) reads the fields")]
pub struct FillPreviewPane<'a> {
    id_salt: IdSalt,
    view: &'a mut FullWindowView,
    status: &'a PreviewStatus,
    source: Option<&'a dyn RasterSource>,
    frame: Option<Rect>,
}

impl<'a> FillPreviewPane<'a> {
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut FullWindowView, status: &'a PreviewStatus) -> Self {
        Self { id_salt: IdSalt::new(id_salt), view, status, source: None, frame: None }
    }

    /// The fill result to show (the app's last ready preview).
    pub fn source(mut self, source: Option<&'a dyn RasterSource>) -> Self {
        self.source = source;
        self
    }

    /// The sampling window in document pixels: the view fits it until the user zooms.
    pub fn frame(mut self, frame: Option<Rect>) -> Self {
        self.frame = frame;
        self
    }

    /// Draws the pane in the column's `ui`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-CA2")]
    pub fn show(self, ui: &mut Ui) -> Response {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:211 `caf_preview_panel`; caf_ui/preview.rs:62
        // `NOTHING_TO_SAMPLE`.
        todo!()
    }
}
