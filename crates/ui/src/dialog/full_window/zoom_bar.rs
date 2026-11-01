//! [`FullWindowZoomBar`]: Filter Gallery's zoom bar under a full-window canvas.

use egui::{IdSalt, Response, Ui};

use super::labels::FullWindowLabels;
use super::view::FullWindowView;
use egui::AsIdSalt;

/// What [`FullWindowZoomBar::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FullWindowZoomBarResponse {
    /// The bar's strip.
    pub response: Response,
    /// The view's zoom or fit changed this pass (a button, the select).
    pub changed: bool,
}

/// The strip under a full-window canvas: Zoom Out, Zoom In, a zoom select, then a progress bar
/// while the preview renders or a note. Filter Gallery's preview and Content-Aware Fill's Preview
/// panel show it; the other canvases show the zoom in the title strip only.
///
/// ```ignore
/// FullWindowZoomBar::new("gallery-zoom", &mut view)
///     .progress(preview.progress())
///     .note(format!("Preview {ms} ms"))
///     .show(ui);
/// let canvas = FullWindowCanvas::new("gallery-view", &mut view).source(&preview_image).show(ui);
/// ```
///
/// - **Place.** An `egui::Panel` at the bottom of what is left of the dialog's body (or of a side
///   column's body), `full_window.zoom_bar.height` tall on the side fill (`full_window.side.fill`), padded by
///   `full_window.zoom_bar.pad`. Call it after the side columns and before the canvas it serves.
/// - **Buttons.** Zoom Out and Zoom In [`IconButton`](crate::controls::IconButton)s step the
///   view's ladder ([`FullWindowView::step`]); each is disabled at its end of the range.
/// - **Select.** A [`Select`](crate::controls::Select) "Zoom" of Fit in View, 12.5 %, 25 %, 50 %,
///   100 %, 200 % and 400 %; a zoom off the menu (stepped to 300 %) is listed as itself in its
///   place among them, as PhotoCraft lists it. The fitted view shows Fit in View.
/// - **Progress or note.** While [`progress`](Self::progress) is set, a
///   [`Progress`](crate::controls::Progress) bar `full_window.progress.width` ×
///   `full_window.progress.height` (indeterminate for `Some(None)`); otherwise the
///   [`note`](Self::note) in `text.faint`, `caption` role, cut with an ellipsis.
/// - **Keyboard and AccessKit.** The buttons and the select are Tab stops in reading order; the
///   strip mirrors in a right-to-left interface.
#[must_use = "show the bar with `show`"]
pub struct FullWindowZoomBar<'a> {
    id_salt: IdSalt,
    view: &'a mut FullWindowView,
    progress: Option<Option<f32>>,
    note: Option<String>,
    labels: Option<FullWindowLabels>,
}

impl<'a> FullWindowZoomBar<'a> {
    /// A zoom bar for `view`, the same view the canvas under it shows.
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut FullWindowView) -> Self {
        Self { id_salt: IdSalt::new(id_salt), view, progress: None, note: None, labels: None }
    }

    /// The preview's progress: `None` shows the note, `Some(None)` an indeterminate bar,
    /// `Some(Some(f))` a bar at `f` (0 to 1). [`PreviewStatus::progress`](super::PreviewStatus::progress)
    /// gives it.
    pub fn progress(mut self, progress: Option<Option<f32>>) -> Self {
        self.progress = progress;
        self
    }

    /// Text after the select while no progress shows (translated by the app): Filter Gallery's
    /// "Preview 120 ms".
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// The words (default: the installed words).
    pub fn labels(mut self, labels: FullWindowLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    #[expect(clippy::todo, reason = "skeleton: K17")]
    pub fn show(self, ui: &mut Ui) -> FullWindowZoomBarResponse {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:673-700 (the strip 30 high on `t.dock`,
        // padding 10 × 3, buttons "minus" and "plus" at 22, the select of Fit in View and six
        // zooms with the off-menu value inserted in order, the 90 × 6 progress bar, else
        // "Preview N ms" in `t.text_faint` at 11 pt).
        let _ = (self.id_salt, self.view, self.progress, self.note, self.labels, ui);
        todo!()
    }
}
