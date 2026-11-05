//! [`FillPreviewPane`]: Content-Aware Fill's Preview panel (task WS-CA2).

use egui::{AsIdSalt, Frame, IdSalt, Panel, Response, Ui};

use craft_ui::canvas::kurbo::{Point, Rect, Size};
use craft_ui::canvas::{CanvasView, Checkerboard, HandTool, NavigationOptions, RasterSource, ZoomLadder};
use craft_ui::dialog::full_window::{
    FullWindowCanvas, FullWindowLabels, FullWindowStyle, FullWindowView, FullWindowZoomBar, FullWindowZoomKeys, PreviewStatus,
};
use craft_ui::localized::Labels as _;
use craft_ui::theme::{Tokens, TokensOf as _};

use super::FillLabels;

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
pub struct FillPreviewPane<'a> {
    id_salt: IdSalt,
    view: &'a mut FullWindowView,
    status: &'a PreviewStatus,
    source: Option<&'a dyn RasterSource>,
    frame: Option<Rect>,
    navigation: NavigationOptions,
    checkerboard: Checkerboard,
}

impl<'a> FillPreviewPane<'a> {
    /// Starts the pane over `view`, which it pans and zooms, showing `status`.
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut FullWindowView, status: &'a PreviewStatus) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            view,
            status,
            source: None,
            frame: None,
            navigation: NavigationOptions::default(),
            checkerboard: Checkerboard::default(),
        }
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

    /// The user's navigation options for the pane's wheel and drag; Photoshop's by default.
    pub fn navigation(mut self, navigation: NavigationOptions) -> Self {
        self.navigation = navigation;
        self
    }

    /// The user's transparency grid under the preview.
    pub fn checkerboard(mut self, checkerboard: Checkerboard) -> Self {
        self.checkerboard = checkerboard;
        self
    }

    /// Draws the pane in the body of a non-scrolling side column (`FullWindowSide::scrolls(false)`),
    /// which gives it the column's padded room.
    pub fn show(self, ui: &mut Ui) -> Response {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:211 `caf_preview_panel`; caf_ui/preview.rs:62
        // `NOTHING_TO_SAMPLE`.
        let Self { id_salt, view, status, source, frame, navigation, checkerboard } = self;
        let id = ui.make_persistent_id(id_salt);
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let margin = FullWindowStyle::of(&tokens).canvas_margin;
        let labels = FullWindowLabels::of(ui);
        let mut canvas_labels = labels.clone();
        canvas_labels.canvas = FillLabels::of(ui).preview;
        let ppp = ui.pixels_per_point();
        FullWindowZoomBar::new((id, "zoom"), view).progress(status.progress()).show(ui);
        Panel::bottom(id.with("status")).resizable(false).show_separator_line(false).frame(Frame::NONE).show(ui, |ui| {
            status.show(ui, &labels);
        });

        let rect = ui.available_rect_before_wrap();
        let size = source.map_or([0, 0], RasterSource::size);
        if let Some(frame) = frame.filter(|_| size[0] > 0 && size[1] > 0) {
            // Records the image size, so the zoom below is kept inside the ladder's range.
            view.canvas_view(rect, size, margin, ppp);
            let framed = ctx.data(|d| d.get_temp::<Framed>(id));
            // Frame again when the window moved, but not over the user's zoom or pan.
            let untouched = framed.is_none_or(|f| f.zoom == view.zoom() && f.center == view.center());
            if untouched && framed.is_none_or(|f| f.frame != frame) {
                let room = |side: f32| f64::from((side - 2.0 * margin).max(1.0)) * f64::from(ppp);
                let fit = (room(rect.width()) / frame.width().max(1.0)).min(room(rect.height()) / frame.height().max(1.0));
                let doc = Size::new(f64::from(size[0]), f64::from(size[1]));
                let zoom = ZoomLadder::clamp(fit, doc);
                let center = Point::new((frame.x0 + frame.x1) / 2.0, (frame.y0 + frame.y1) / 2.0);
                view.take(&CanvasView::new(rect, center, CanvasView::points_per_unit(zoom, ppp)), ppp);
                ctx.data_mut(|d| d.insert_temp(id, Framed { frame, zoom: view.zoom(), center: view.center() }));
            }
        }

        let doc = Size::new(f64::from(size[0]), f64::from(size[1]));
        let mut hand = HandTool::new(doc, navigation);
        let none = FullWindowZoomKeys { zoom_in: Vec::new(), zoom_out: Vec::new(), fit: Vec::new(), actual_pixels: Vec::new() };
        let mut canvas = FullWindowCanvas::new((id, "view"), view)
            .checkerboard(checkerboard)
            .navigation(navigation)
            .zoom_keys(none)
            .labels(canvas_labels)
            .tool(&mut hand);
        if let Some(source) = source {
            canvas = canvas.source(source);
        }
        canvas.show(ui).response
    }
}

/// The window the view was last framed on, and the zoom and centre that framing gave.
#[derive(Clone, Copy, PartialEq)]
struct Framed {
    frame: Rect,
    zoom: f64,
    center: Point,
}
