//! [`FullWindowCanvas`]: the canvas region of a full-window dialog, and [`RasterOverlay`], a
//! further image drawn over it at a texel scale.

use egui::accesskit::Role;
use egui::epaint::StrokeKind;
use egui::{AsIdSalt, Color32, ColorImage, EventFilter, IdSalt, Painter, PointerButton, Rect, Response, Sense, Ui, WidgetInfo};
use kurbo::{Point, Size};

use super::labels::FullWindowLabels;
use super::style::FullWindowStyle;
use super::view::{FullWindowView, FullWindowZoomKeys};
use crate::canvas::navigate::{PageKey, WheelNavigation};
use crate::canvas::{
    CanvasId, CanvasTool, CanvasView, Checkerboard, HandTool, MomentaryKeys, MomentaryTool, NavigationOptions, NoTool, PixelRect, RasterDisplay,
    RasterSource, ToolGesture, ToolHost, ToolKey, ZoomAnimation, ZoomTool,
};
use crate::localized::{Labels as _, LocaleNumbers};
use crate::theme::current;

type PaintMarks<'a> = Box<dyn FnOnce(&Painter, &CanvasView) + 'a>;

/// An image drawn over the canvas's image: the selection's ants, Content-Aware Fill's sampling
/// tint, Liquify's freeze mask. The app gives it at its own resolution, one texel per `texel`
/// image pixels each way (a reduced proxy's factor; 1 for full resolution), and it is drawn
/// through a [`RasterDisplay`](crate::canvas::RasterDisplay) of its own over the same view, so it
/// lines up with the image at every zoom.
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

    /// The id of the canvas that [`new`](Self::new) with `id_salt` runs in `ui`: the one
    /// [`ToolHost::cancel`](crate::canvas::ToolHost::cancel), [`ToolHost::inject`] and
    /// [`ZoomAnimation`] calls from the workspace use. `ui` is the one the canvas is shown in
    /// (the dialog's body).
    pub fn canvas_id(ui: &Ui, id_salt: impl AsIdSalt) -> CanvasId {
        CanvasId::from_id(ui.make_persistent_id(IdSalt::new(id_salt)))
    }

    /// Draws the canvas in the rest of `ui` (the dialog's body after the other regions).
    pub fn show(self, ui: &mut Ui) -> FullWindowCanvasOutput {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1912-1936 (the area on `t.canvas`, fit
        // with a 20 pt margin, the hovered canvas surrendering a button's focus so Space pans,
        // `exp(scroll / 200)` wheel zoom replaced by `WheelNavigation`), gallery_ui.rs:660-672
        // (checkerboard, the image, its 1 pt `t.separator` outline outside, drag pans).
        let Self { id_salt, view: window_view, source, checkerboard, overlays, tool, navigation, zoom_keys, painter: marks, labels } = self;
        let ctx = ui.ctx().clone();
        let style = FullWindowStyle::of(&current(&ctx));
        let labels = labels.unwrap_or_else(|| FullWindowLabels::of(ui));
        let ppp = ui.pixels_per_point();
        let rect = ui.available_rect_before_wrap();
        ui.allocate_rect(rect, Sense::hover());
        let canvas = CanvasId::from_id(ui.make_persistent_id(id_salt));
        let response = canvas.interact(ui, rect, Sense::click_and_drag() | Sense::FOCUSABLE);
        // The arrow keys belong to the tool on the canvas instead of moving focus.
        let arrows = EventFilter { horizontal_arrows: true, vertical_arrows: true, ..EventFilter::default() };
        ctx.memory_mut(|m| m.set_focus_lock_filter(response.id, arrows));
        crate::focus::key_owner::register_canvas(&ctx, response.id);

        let pixels = source.map_or([0, 0], |s| s.size());
        let doc = Size::new(f64::from(pixels[0]), f64::from(pixels[1]));
        if let Some(zoom) = zoom_keys.take(ui) {
            window_view.apply(zoom);
        }
        let mut view = window_view.canvas_view(rect, pixels, style.canvas_margin, ppp);

        WheelNavigation::configure(&ctx);
        if let Some(action) = WheelNavigation::read(&ctx, &response, &navigation) {
            let at = response.hover_pos().unwrap_or(rect.center());
            WheelNavigation::apply(&mut view, action, at, doc, &navigation, ppp);
        }
        // The keys are free for this dialog's layer, which is the top modal layer.
        let free = crate::focus::key_owner::keys_free(&ctx, ui.layer_id());
        let over = response.hovered() || response.has_focus();
        // Page keys stay with a focused control that is not the canvas (a tool's parts).
        let page_keys = free && over && ctx.memory(|m| m.focused()).is_none_or(|id| id == response.id);
        PageKey::handle_where(&ctx, &mut view, doc, navigation.overscroll, page_keys);
        window_view.take(&view, ppp);

        let temporary = MomentaryKeys::default().read_where(&ctx, canvas, false, free && over);
        if temporary.is_some()
            && let Some(focused) = ctx.memory(|m| m.focused()).filter(|id| *id != response.id)
        {
            // A focused button would take Space as a click.
            ctx.memory_mut(|m| m.surrender_focus(focused));
        }
        let panning = Self::middle_pan(&ctx, &response, canvas);
        let tools_key = canvas.with("navigation-tools");
        let mut tools = ctx
            .data(|d| d.get_temp::<NavigationTools>(tools_key))
            .unwrap_or_else(|| NavigationTools { hand: HandTool::new(doc, navigation), zoom: ZoomTool::new(doc, navigation) });
        tools.hand.size = doc;
        tools.hand.prefs = navigation;
        tools.zoom.size = doc;
        tools.zoom.prefs = navigation;
        tools.zoom.out = temporary == Some(MomentaryTool::ZoomOut);

        let shown = ZoomAnimation::shown(&ctx, canvas, &view, navigation.animated_zoom);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, style.canvas_fill);
        if let Some(source) = source {
            let mut display = RasterDisplay::new((canvas.id(), "image"));
            if let Some(checker) = checkerboard {
                display = display.checkerboard(checker);
            }
            display.show(ui, &shown, source);
            let corners = shown.quad(kurbo::Rect::new(0.0, 0.0, doc.width, doc.height));
            painter.rect_stroke(Rect::from_points(&corners), 0.0, style.canvas_outline, StrokeKind::Outside);
            for (n, overlay) in overlays.iter().enumerate() {
                let scaled = overlay_view(&shown, overlay.texel);
                let tinted = Tinted { source: overlay.source, tint: overlay.tint };
                RasterDisplay::new((canvas.id(), "overlay", n)).show(ui, &scaled, &tinted);
            }
        }
        if let Some(marks) = marks {
            marks(&painter, &shown);
        }

        let mut none = NoTool;
        let active: &mut dyn CanvasTool = match (temporary, tool) {
            (Some(MomentaryTool::Hand), _) => &mut tools.hand,
            (Some(MomentaryTool::ZoomIn | MomentaryTool::ZoomOut), _) => &mut tools.zoom,
            (None, _) if panning => &mut tools.hand,
            (None, Some(tool)) => tool,
            (None, None) => &mut none,
        };
        // The host works on the target view, so a pan that begins while the zoom eases keeps the
        // target zoom; `shown` is only what is painted.
        let hosted = ToolHost::new(canvas).document(doc).navigation(navigation).keys(!ctx.text_edit_focused()).show(ui, &view, &response, active);
        ctx.data_mut(|d| d.insert_temp(tools_key, tools));
        let changed = hosted.view.is_some();
        if let Some(next) = hosted.view {
            view = CanvasView { rect: view.rect, ..next };
            window_view.take(&view, ppp);
        }
        // What the canvas drew with, unless the tool moved the view after it was drawn.
        let drawn = if changed { view } else { shown };

        let name = labels.canvas.clone();
        response.widget_info(|| WidgetInfo::labeled(Role::Canvas, true, name.as_str()));
        let percent = LocaleNumbers::of(ui).percent(window_view.zoom());
        ctx.accesskit_node_builder(response.id, |node| node.set_value(percent.as_str()));

        let hover = response.hover_pos().filter(|_| response.hovered()).map(|at| drawn.to_doc(at));
        FullWindowCanvasOutput { view: drawn, response, gesture: hosted.gesture, hover, took_key: hosted.took_key }
    }

    /// Whether the middle button pans this pass: it went down over the canvas while no gesture
    /// ran, and it stays so until it is up (the pass of its release included, for the Hand's
    /// release and glide). The Hand takes the middle button whatever the app's tool is.
    fn middle_pan(ctx: &egui::Context, response: &Response, canvas: CanvasId) -> bool {
        let (pressed, down, released) = ctx
            .input(|i| (i.pointer.button_pressed(PointerButton::Middle), i.pointer.middle_down(), i.pointer.button_released(PointerButton::Middle)));
        let latch = canvas.with("middle-pan");
        let latched = ctx.data(|d| d.get_temp::<bool>(latch)).unwrap_or(false);
        let on = if pressed { response.hovered() && !ToolHost::in_gesture(ctx, canvas) } else { latched && (down || released) };
        ctx.data_mut(|d| d.insert_temp(latch, on && down));
        on
    }
}

/// The Hand and Zoom tools a momentary key brings up, kept in egui memory between passes.
#[derive(Clone)]
struct NavigationTools {
    hand: HandTool,
    zoom: ZoomTool,
}

/// `view` for an overlay at one texel per `texel` image pixels: the same screen rect as the image.
pub(super) fn overlay_view(view: &CanvasView, texel: u32) -> CanvasView {
    let texel = f64::from(texel);
    CanvasView { zoom: view.zoom * texel, center: Point::new(view.center.x / texel, view.center.y / texel), ..*view }
}

/// A [`RasterOverlay`]'s source with its texels multiplied by the tint.
pub(super) struct Tinted<'a> {
    pub(super) source: &'a dyn RasterSource,
    pub(super) tint: Color32,
}

impl RasterSource for Tinted<'_> {
    fn size(&self) -> [u32; 2] {
        self.source.size()
    }

    fn revision(&self) -> u64 {
        self.source.revision() ^ u64::from(u32::from_le_bytes(self.tint.to_array())).rotate_left(32)
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        self.source.read(region, factor, out);
        if self.tint != Color32::WHITE {
            // design: allow(white is the tint that leaves texels as they are)
            let times = |a: u8, b: u8| ((u16::from(a) * u16::from(b) + 127) / 255) as u8;
            for texel in &mut out.pixels {
                let t = self.tint;
                *texel = Color32::from_rgba_premultiplied(
                    times(texel.r(), t.r()),
                    times(texel.g(), t.g()),
                    times(texel.b(), t.b()),
                    times(texel.a(), t.a()),
                );
            }
        }
    }
}
