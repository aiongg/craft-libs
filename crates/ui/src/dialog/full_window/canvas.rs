//! [`FullWindowCanvas`]: the canvas region of a full-window dialog, and [`RasterOverlay`], a
//! further image drawn over it at a texel scale.

use egui::{Color32, IdSalt, Painter, Response, Ui};
use kurbo::Point;

use super::labels::FullWindowLabels;
use super::view::{FullWindowView, FullWindowZoomKeys};
use crate::canvas::{CanvasTool, CanvasView, Checkerboard, NavigationOptions, RasterSource, ToolGesture, ToolKey};
use egui::AsIdSalt;

type PaintMarks<'a> = Box<dyn FnOnce(&Painter, &CanvasView) + 'a>;

/// An image drawn over the canvas's image: the selection's ants, Content-Aware Fill's sampling
/// tint, Liquify's freeze mask. The app gives it at its own resolution, one texel per `texel`
/// image pixels each way (a reduced proxy's factor; 1 for full resolution), and it is drawn
/// through a [`RasterDisplay`](crate::canvas::RasterDisplay) of its own over the same view, so it
/// lines up with the image at every zoom.
#[expect(dead_code, reason = "skeleton: K16 draws the overlays")]
pub struct RasterOverlay<'a> {
    pub(super) source: &'a dyn RasterSource,
    pub(super) texel: u32,
    pub(super) tint: Color32,
}

impl<'a> RasterOverlay<'a> {
    /// `source` covers the image at one texel per `texel` pixels (at least 1).
    pub fn new(source: &'a dyn RasterSource, texel: u32) -> Self {
        Self { source, texel: texel.max(1), tint: Color32::WHITE } // design: allow(white is the tint that leaves texels as they are)
    }

    /// Multiplies the overlay's texels by `tint` (white, the default, draws them as they are):
    /// the user's sampling-area colour over a coverage mask.
    pub fn tint(mut self, tint: Color32) -> Self {
        self.tint = tint;
        self
    }
}

/// What [`FullWindowCanvas::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FullWindowCanvasOutput {
    /// The view this pass drew with (after the tool, the wheel, the keys and the pan changed it):
    /// overlays the workspace draws itself map through it.
    pub view: CanvasView,
    /// The canvas widget: hover, focus, its rect.
    pub response: Response,
    /// The tool's gesture phase this pass, from [`ToolHost`](crate::canvas::ToolHost).
    pub gesture: Option<ToolGesture>,
    /// The image point under the pointer, in image pixels, while it is over the canvas.
    pub hover: Option<Point>,
    /// A key the tool took this pass, so the workspace's own key handling leaves it.
    pub took_key: Option<ToolKey>,
}

/// The canvas region of a [`FullWindowDialog`](super::FullWindowDialog): the app's image, its
/// overlays and one tool, filling what the other regions left.
///
/// ```ignore
/// let out = FullWindowCanvas::new("sam-view", &mut self.view)
///     .source(source.view())
///     .checkerboard(checker)
///     .overlay(RasterOverlay::new(source.ants(), proxy_factor))
///     .tool(&mut tool)
///     .navigation(prefs.navigation)
///     .show(ui);
/// ```
///
/// - **Look.** The rest of the body on `full_window.canvas.fill`; the image through
///   [`RasterDisplay`](crate::canvas::RasterDisplay) over the [checkerboard](Self::checkerboard),
///   with a 1 pt `full_window.canvas.outline` outside it; then each [`RasterOverlay`] in the order
///   given; then the [`painter`](Self::painter) marks; then the tool's own painting and cursor.
/// - **View.** [`FullWindowView`] fits the image on the first pass and keeps fitting it until the
///   user zooms or pans. The wheel, pinch and two-finger scroll follow the user's
///   [`NavigationOptions`] (the canvas's `WheelNavigation`), zooming about
///   the pointer; Space held or the middle button pans with the
///   [`HandTool`](crate::canvas::HandTool) whatever the tool
///   ([`MomentaryKeys`](crate::canvas::MomentaryKeys)), and Flick Panning glides when on. The
///   [zoom keys](Self::zoom_keys) act on this view only and are consumed.
/// - **Tool.** One [`CanvasTool`] in a [`ToolHost`](crate::canvas::ToolHost) on this canvas (the
///   workspace's brush or lasso, [`HandTool`](crate::canvas::HandTool),
///   [`ZoomTool`](crate::canvas::ZoomTool)); presses, drags, keys and the cursor are the host's.
///   Without a tool the canvas only navigates.
/// - **Keyboard and AccessKit.** One Tab stop, a `Canvas` named "Image" (`fullWindow.canvas`)
///   with the zoom percent as its value; while it has the focus, its keys are the tool's and the
///   navigation's. The image, overlays and tool geometry never mirror in a right-to-left
///   interface.
#[must_use = "show the canvas with `show`"]
pub struct FullWindowCanvas<'a> {
    id_salt: IdSalt,
    view: &'a mut FullWindowView,
    source: Option<&'a dyn RasterSource>,
    checkerboard: Option<Checkerboard>,
    overlays: Vec<RasterOverlay<'a>>,
    tool: Option<&'a mut dyn CanvasTool>,
    navigation: NavigationOptions,
    zoom_keys: FullWindowZoomKeys,
    painter: Option<PaintMarks<'a>>,
    labels: Option<FullWindowLabels>,
}

impl<'a> FullWindowCanvas<'a> {
    /// A canvas showing `view`, which the canvas updates (fit, zoom, pan).
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut FullWindowView) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            view,
            source: None,
            checkerboard: None,
            overlays: Vec::new(),
            tool: None,
            navigation: NavigationOptions::default(),
            zoom_keys: FullWindowZoomKeys::default(),
            painter: None,
            labels: None,
        }
    }

    /// The image to show (the app's proxy or full-resolution preview). Without one the canvas
    /// shows its fill only and the view does not fit.
    pub fn source(mut self, source: &'a dyn RasterSource) -> Self {
        self.source = Some(source);
        self
    }

    /// The user's transparency grid under the image; without one the image draws straight on the
    /// canvas fill.
    pub fn checkerboard(mut self, checker: Checkerboard) -> Self {
        self.checkerboard = Some(checker);
        self
    }

    /// Draws `overlay` over the image, after the overlays added before it.
    pub fn overlay(mut self, overlay: RasterOverlay<'a>) -> Self {
        self.overlays.push(overlay);
        self
    }

    /// The tool the canvas runs this pass.
    pub fn tool(mut self, tool: &'a mut dyn CanvasTool) -> Self {
        self.tool = Some(tool);
        self
    }

    /// The user's navigation options (default: Photoshop's).
    pub fn navigation(mut self, options: NavigationOptions) -> Self {
        self.navigation = options;
        self
    }

    /// The zoom keys (default: ⌘+, ⌘−, ⌘0, ⌘1).
    pub fn zoom_keys(mut self, keys: FullWindowZoomKeys) -> Self {
        self.zoom_keys = keys;
        self
    }

    /// Marks the workspace draws over the image and overlays, before the tool paints: planes,
    /// constraints, pins, traced outlines. `paint` gets the canvas's painter (clipped to the
    /// canvas) and this pass's view.
    pub fn painter(mut self, paint: impl FnOnce(&Painter, &CanvasView) + 'a) -> Self {
        self.painter = Some(Box::new(paint));
        self
    }

    /// The words (default: the installed words).
    pub fn labels(mut self, labels: FullWindowLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the canvas in the rest of `ui` (the dialog's body after the other regions).
    #[expect(clippy::todo, reason = "skeleton: K16")]
    pub fn show(self, ui: &mut Ui) -> FullWindowCanvasOutput {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1912-1936 (the area on `t.canvas`, fit
        // with a 20 pt margin, the hovered canvas surrendering a button's focus so Space pans,
        // `exp(scroll / 200)` wheel zoom replaced by `WheelNavigation`), gallery_ui.rs:660-672
        // (checkerboard, the image, its 1 pt `t.separator` outline outside, drag pans).
        let _ = (
            self.id_salt,
            self.view,
            self.source,
            self.checkerboard,
            self.overlays,
            self.tool,
            self.navigation,
            self.zoom_keys,
            self.painter,
            self.labels,
            ui,
        );
        todo!()
    }
}
