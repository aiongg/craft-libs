//! `FullWindowCanvas`: the canvas region of a full-window dialog, and `RasterOverlay`, a further image drawn over it.

use super::super::words::DialogWords;
use super::view::FullWindowView;
use crate::canvas::{
    Canvas, CanvasTool, HandTool, Momentary, NavigationOptions, PixelRect, RasterDisplay, RasterSource, ToolHost, ToolHostResponse, ToolPath,
    WheelNavigation, pan,
};
use crate::contract::{NoValue, Outcome, Props as _, Show as _, ShowContainer as _};
use crate::primitives::{Checker, Revision};
use crate::runtime::env::{Env, Label};
use craft_behavior::kurbo::Point;
use craft_behavior::{CanvasView, PixelSize};
use egui::{Color32, ColorImage, Rect, Response, Ui};

/// An image drawn over the canvas's image: the selection's ants, Content-Aware Fill's sampling tint, Liquify's freeze
/// mask. The source covers the image's pixels as the image's own source does (one texel per pixel at factor 1); an
/// app that keeps the overlay at a reduced resolution answers `read` from its reduced copy. The overlay is drawn
/// through a `RasterDisplay` of its own over the same view, so it lines up with the image at every zoom.
pub struct RasterOverlay<'a> {
    source: &'a dyn RasterSource,
    tint: Option<Color32>,
}

impl<'a> RasterOverlay<'a> {
    /// An overlay of `source`.
    pub fn new(source: &'a dyn RasterSource) -> Self {
        RasterOverlay { source, tint: None }
    }

    /// Multiplies the overlay's texels by `tint`: the user's sampling-area colour over a coverage mask.
    #[must_use]
    pub fn tint(mut self, tint: Color32) -> Self {
        self.tint = Some(tint);
        self
    }
}

/// A source with its texels multiplied by a tint.
struct Tinted<'a> {
    source: &'a dyn RasterSource,
    tint: Color32,
}

impl RasterSource for Tinted<'_> {
    fn size(&self) -> PixelSize {
        self.source.size()
    }

    fn revision(&self) -> Revision {
        Revision::of(&(self.source.revision(), self.tint.to_array()))
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        None
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        self.source.read(region, factor, out);
        let times = |a: u8, b: u8| ((u16::from(a) * u16::from(b) + 127) / 255) as u8;
        let tint = self.tint;
        for texel in &mut out.pixels {
            *texel = Color32::from_rgba_premultiplied(
                times(texel.r(), tint.r()),
                times(texel.g(), tint.g()),
                times(texel.b(), tint.b()),
                times(texel.a(), tint.a()),
            );
        }
    }
}

/// What a full-window canvas reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct FullWindowCanvasOutput {
    /// The canvas's response.
    pub response: Response,
    /// The view after the wheel, a middle-button pan or the tool changed it this pass; `None` when the user did not
    /// change it. A caller keeps `view.unwrap_or(shown)`.
    pub view: Option<FullWindowView>,
    /// The view this pass drew with: the given view settled on the canvas's rect (fitted while it fits).
    pub shown: FullWindowView,
    /// The image point under the pointer, in image pixels, while it is over the canvas.
    pub hover: Option<Point>,
    /// What the tool host did: the gesture phase, the key the tool took and the cursor it asked for.
    pub tool: ToolHostResponse,
}

impl Outcome for FullWindowCanvasOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The canvas region of a full-window dialog: the app's image, its overlays and one tool, filling what the other
/// regions left.
///
/// - **Look.** The canvas surface; the image through `RasterDisplay` over the [checkerboard](Self::checkerboard);
///   then each [`RasterOverlay`] in the order given; then the canvas controls of [`controls`](Self::controls); then
///   the tool's own painting and cursor.
/// - **View.** [`FullWindowView`] fits the image on the first pass and keeps fitting it until the user zooms or
///   pans. The wheel, pinch and two-finger scroll follow the user's [`NavigationOptions`], zooming about the pointer;
///   the middle button pans. Space-drag and the momentary Zoom run in the tool host when the app gives it its
///   [`Momentary`] set. The zoom commands (`FullWindowZoomKeys`) are the app's to apply to the view.
/// - **Tool.** One `CanvasTool` in a `ToolHost` on this canvas; presses, drags, keys and the cursor are the host's.
///   Without a tool the canvas navigates with the Hand tool.
/// - **Keyboard and AccessKit.** The canvas is one Tab stop named by its label; while it has the focus its keys are
///   the tool's. The image, overlays and tool geometry never mirror in a right-to-left interface.
#[derive(crate::contract::Pattern)]
#[must_use = "show the canvas with `show`"]
pub struct FullWindowCanvas<'a> {
    path: ToolPath,
    view: FullWindowView,
    source: &'a dyn RasterSource,
    checkerboard: Option<Checker>,
    overlays: Vec<RasterOverlay<'a>>,
    tool: Option<&'a mut dyn CanvasTool>,
    momentary: Option<&'a mut Momentary>,
    navigation: NavigationOptions,
    controls: Option<Box<dyn FnMut(&mut Ui, Rect) + 'a>>,
}

impl<'a> FullWindowCanvas<'a> {
    /// A canvas showing the image `source` through `view`. `path` says how the tool is reached without a pointer.
    pub fn new(path: ToolPath, view: &FullWindowView, source: &'a dyn RasterSource) -> Self {
        FullWindowCanvas {
            path,
            view: *view,
            source,
            checkerboard: None,
            overlays: Vec::new(),
            tool: None,
            momentary: None,
            navigation: NavigationOptions::default(),
            controls: None,
        }
    }

    /// The user's transparency grid under the image; without one the image draws straight on the canvas.
    pub fn checkerboard(mut self, checker: Checker) -> Self {
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

    /// The app's momentary tools (Space for the Hand, Cmd+Space for Zoom In).
    pub fn momentary(mut self, momentary: &'a mut Momentary) -> Self {
        self.momentary = Some(momentary);
        self
    }

    /// The user's navigation options (default: Photoshop's).
    pub fn navigation(mut self, options: NavigationOptions) -> Self {
        self.navigation = options;
        self
    }

    /// Shows the workspace's canvas controls (guides, a crop, planes, pins) over the image and overlays. `show` gets
    /// the canvas's body and the rect to show each control at (`show_at`).
    pub fn controls(mut self, show: impl FnMut(&mut Ui, Rect) + 'a) -> Self {
        self.controls = Some(Box::new(show));
        self
    }

    /// Shows the canvas in the rest of `ui`.
    pub fn show(self, ui: &mut Ui) -> FullWindowCanvasOutput {
        let FullWindowCanvas { path, view, source, checkerboard, overlays, tool, momentary, navigation, mut controls } = self;
        let ppp = ui.pixels_per_point();
        let label: Label = Env::of(ui).words::<DialogWords>().full_window_canvas.clone();
        let size = source.size();
        let shown = view.settled(ui.available_rect_before_wrap(), size, ppp);
        let shown_label = label.clone();
        let canvas = Canvas::new(label, path).view(shown.view()).show(ui, |ui, body| {
            let mut image = RasterDisplay::new(shown_label.clone(), source);
            if let Some(checker) = checkerboard {
                image = image.checkerboard(checker);
            }
            image.show_at(ui, body.rect);
            for (n, overlay) in overlays.iter().enumerate() {
                let label = shown_label.clone();
                match overlay.tint {
                    Some(tint) => {
                        let tinted = Tinted { source: overlay.source, tint };
                        RasterDisplay::new(label, &tinted).id_salt(("overlay", n)).show_at(ui, body.rect);
                    }
                    None => {
                        RasterDisplay::new(label, overlay.source).id_salt(("overlay", n)).show_at(ui, body.rect);
                    }
                }
            }
            if let Some(controls) = controls.as_mut() {
                controls(ui, body.rect);
            }
        });
        let output = canvas.output;
        let mut next = shown;
        let mut moved: CanvasView = shown.view();
        let document = shown.size();
        if let Some(wheel) = &output.wheel
            && let Some(action) = next.wheel().read(wheel, &navigation)
        {
            let at = output.response.hover_pos().unwrap_or_else(|| moved.rect.center());
            WheelNavigation::apply(&mut moved, action, at, document, ppp);
        }
        if let Some(delta) = output.pan {
            pan(&mut moved, delta);
        }
        let mut fallback = HandTool::new(navigation);
        let active: &mut dyn CanvasTool = match tool {
            Some(tool) => tool,
            None => &mut fallback,
        };
        let mut host = ToolHost::new();
        if let Some(momentary) = momentary {
            host = host.momentary(momentary);
        }
        let hosted = host.run(ui, &output, active);
        if let Some(set) = hosted.view {
            moved = set;
        }
        next = next.took(moved);
        let hover = output.response.hover_pos().filter(|_| output.response.hovered()).map(|at| next.view().to_doc(at));
        FullWindowCanvasOutput { response: output.response, view: (next != shown).then_some(next), shown, hover, tool: hosted }
    }
}
