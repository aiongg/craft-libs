//! Liquify's canvas: the canvas tool, the backdrop layering, the mesh (task WS-LQ2).

use egui::{Color32, ColorImage, Modifiers, Painter, PointerButton, Pos2, Shape, Stroke};

use craft_ui::canvas::brush::{
    BrushCursor, BrushDimensions, BrushLevel, BrushResize, BrushResizeLabels, BrushResizeOptions, BrushStyle, BrushTip, TipFalloff,
};
use craft_ui::canvas::draft::{DraftEvent, DraftKind, DraftPoint, DraftReach, OutlineDraft};
use craft_ui::canvas::kurbo::{Point, Size};
use craft_ui::canvas::{
    CanvasTool, CanvasView, CursorHost, CursorOptions, HandTool, NavigationOptions, PixelRect, RasterSource, ToolAction, ToolContext, ToolCursor,
    ToolKey, ToolPointer, ZoomTool,
};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::{Labels as _, Units};

use super::LiquifyEvent;
use super::session::{BackdropMode, LiquifySettings, LiquifyStroke};
use super::style::LiquifyStyle;
use super::tools::{LiquifyBrush, LiquifyTool, StrokeRecorder};

/// The seconds between the dabs of a tool that acts while the brush rests (PhotoCraft: 30 a
/// second).
const HOLD_INTERVAL: f64 = 1.0 / 30.0;

/// The displaced mesh Show Mesh draws, from [`LiquifySource::mesh`](super::LiquifySource::mesh):
/// `columns × rows` nodes in row order, in document pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LiquifyMesh {
    pub columns: usize,
    pub rows: usize,
    pub points: Vec<Point>,
}

impl LiquifyMesh {
    /// The mesh's lines on screen (each horizontal and vertical edge between neighbouring nodes
    /// once), for `warp::paint_mesh`. A mesh whose `points` don't match `columns × rows` has no
    /// lines.
    pub fn edges(&self, view: &CanvasView) -> Vec<[Pos2; 2]> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:1248 `draw_mesh` (PhotoCraft traces the
        // field's iso-lines itself; the app supplies the displaced nodes, so the lines are the
        // grid's edges).
        if self.columns == 0 || self.rows == 0 || self.columns.checked_mul(self.rows) != Some(self.points.len()) {
            return Vec::new();
        }
        let at = |row: usize, column: usize| view.to_screen(self.points[row * self.columns + column]);
        let mut edges = Vec::with_capacity(2 * self.points.len());
        for row in 0..self.rows {
            for column in 0..self.columns {
                if column + 1 < self.columns {
                    edges.push([at(row, column), at(row, column + 1)]);
                }
                if row + 1 < self.rows {
                    edges.push([at(row, column), at(row + 1, column)]);
                }
            }
        }
        edges
    }
}

/// How the image and the backdrop are drawn for Show Backdrop: the tint of the image and of the
/// backdrop drawn behind or in front of it (white with the alpha of the opacity).
///
/// | Mode | Image | Backdrop |
/// |---|---|---|
/// | none shown | white | — |
/// | In Front | white | in front, at the opacity |
/// | Behind | at the opacity | behind, white |
/// | Blend | at 1 − opacity | in front, at the opacity |
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropLayering {
    pub image: Color32,
    pub behind: Option<Color32>,
    pub front: Option<Color32>,
}

impl BackdropLayering {
    /// The layering for a backdrop that is shown (`shown`) in `mode` at `opacity` (0..=100).
    pub fn of(shown: bool, mode: BackdropMode, opacity: f64) -> BackdropLayering {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:876 `backdrop_layering` (Photoshop's modes,
        // measured on its Liquify, #7609).
        let white = Color32::WHITE; // design: allow(white is the tint that leaves a layer as it is)
        let fraction = if opacity.is_finite() { (opacity / 100.0).clamp(0.0, 1.0) } else { 0.0 };
        let alpha = |a: f64| Color32::from_white_alpha((a.clamp(0.0, 1.0) * 255.0).round() as u8);
        if !shown {
            return BackdropLayering { image: white, behind: None, front: None };
        }
        match mode {
            BackdropMode::InFront => BackdropLayering { image: white, behind: None, front: Some(alpha(fraction)) },
            BackdropMode::Behind => BackdropLayering { image: alpha(fraction), behind: Some(white), front: None },
            BackdropMode::Blend => BackdropLayering { image: alpha(1.0 - fraction), behind: None, front: Some(alpha(fraction)) },
        }
    }

    /// Whether the image is drawn as it is, with nothing behind or in front of it.
    pub(super) fn is_plain(&self) -> bool {
        self.behind.is_none() && self.front.is_none() && self.image == Color32::WHITE // design: allow(the untinted image)
    }
}

/// The image with the backdrop laid behind and in front of it as a [`BackdropLayering`] says,
/// composited into one picture so the canvas draws it through its one image.
pub(super) struct LayeredImage<'a> {
    pub(super) image: &'a dyn RasterSource,
    pub(super) backdrop: &'a dyn RasterSource,
    pub(super) layering: BackdropLayering,
}

/// `texel` (sRGBA, unmultiplied) with its alpha scaled by the alpha of `tint`.
fn faded(texel: Color32, tint: Color32) -> Color32 {
    let alpha = (u16::from(texel.a()) * u16::from(tint.a()) + 127) / 255;
    Color32::from_rgba_premultiplied(texel.r(), texel.g(), texel.b(), alpha as u8)
}

/// `top` over `bottom`, both sRGBA unmultiplied.
fn over(bottom: Color32, top: Color32) -> Color32 {
    let (ta, ba) = (f32::from(top.a()) / 255.0, f32::from(bottom.a()) / 255.0);
    let alpha = ta + ba * (1.0 - ta);
    if alpha <= 0.0 {
        return Color32::TRANSPARENT;
    }
    let mix = |t: u8, b: u8| ((f32::from(t) * ta + f32::from(b) * ba * (1.0 - ta)) / alpha).round() as u8;
    Color32::from_rgba_premultiplied(mix(top.r(), bottom.r()), mix(top.g(), bottom.g()), mix(top.b(), bottom.b()), (alpha * 255.0).round() as u8)
}

fn packed(tint: Option<Color32>) -> u64 {
    tint.map_or(0, |c| (1 << 32) | u64::from(u32::from_le_bytes(c.to_array())))
}

impl RasterSource for LayeredImage<'_> {
    fn size(&self) -> [u32; 2] {
        self.image.size()
    }

    fn revision(&self) -> u64 {
        let layering = [packed(Some(self.layering.image)), packed(self.layering.behind), packed(self.layering.front)];
        let mixed = layering.iter().fold(0_u64, |acc, part| acc.rotate_left(17) ^ part.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        self.image.revision() ^ self.backdrop.revision().rotate_left(29) ^ mixed
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::TRANSPARENT);
        let backdrop_fits = self.backdrop.size() == self.image.size();
        let mut layer = ColorImage::filled(out.size, Color32::TRANSPARENT);
        let layers = [
            self.layering.behind.filter(|_| backdrop_fits).map(|tint| (self.backdrop, tint)),
            Some((self.image, self.layering.image)),
            self.layering.front.filter(|_| backdrop_fits).map(|tint| (self.backdrop, tint)),
        ];
        for (source, tint) in layers.into_iter().flatten() {
            layer.pixels.fill(Color32::TRANSPARENT);
            source.read(region, factor, &mut layer);
            for (below, above) in out.pixels.iter_mut().zip(&layer.pixels) {
                *below = over(*below, faded(*above, tint));
            }
        }
    }
}

/// How long the latest brush stroke has run and how many dabs it has made, measured in input
/// time (the diagnostics PhotoCraft's stats line showed; the app adds its own field and preview
/// times). It stays after the stroke ends, until the next press starts a new one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct LiquifyTiming {
    /// The dabs so far: the press, each new point and each rest-time dab.
    pub dabs: usize,
    /// Seconds from the press to the latest dab.
    pub seconds: f64,
}

/// The state of Liquify's canvas between passes: the stroke being recorded, the Freeze Lasso
/// being drawn, the brush resize, and the events not yet taken. The workspace hands
/// [`LiquifyCanvasTool`] to its `FullWindowCanvas` while a Liquify tool is current, and the
/// library's `HandTool` or `ZoomTool` for H and Z (kept as private fields here).
#[derive(Debug)]
pub struct LiquifyCanvas {
    recorder: StrokeRecorder,
    lasso: Option<(OutlineDraft, bool)>,
    resize: BrushResize,
    events: Vec<LiquifyEvent>,
    /// The brush when the resize began: Esc puts it back.
    resize_from: Option<LiquifyBrush>,
    /// The latest point of the stroke in progress, to end it without a pointer.
    at: [f64; 2],
    /// The input time of the stroke's latest dab, for the tools that act while the brush rests.
    last_dab: f64,
    /// The input time of the press of the latest stroke, and the stroke's dabs.
    began: f64,
    timing: Option<LiquifyTiming>,
    /// The user's cursor options, for the brush outline.
    cursor_options: CursorOptions,
    hand: HandTool,
    zoom: ZoomTool,
}

impl Default for LiquifyCanvas {
    fn default() -> Self {
        Self {
            recorder: StrokeRecorder::default(),
            lasso: None,
            resize: BrushResize::default(),
            events: Vec::new(),
            resize_from: None,
            at: [0.0; 2],
            last_dab: 0.0,
            began: 0.0,
            timing: None,
            cursor_options: CursorOptions::default(),
            hand: HandTool::default(),
            zoom: ZoomTool::new(Size::ZERO, NavigationOptions::default()),
        }
    }
}

impl LiquifyCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// The canvas tool for this pass over `settings` (the tool, the brush; the resize writes the
    /// brush's size and density).
    pub fn tool<'a>(&'a mut self, settings: &'a mut LiquifySettings, style: &'a LiquifyStyle) -> LiquifyCanvasTool<'a> {
        LiquifyCanvasTool { canvas: self, settings, style }
    }

    /// The user's cursor options (the brush outline's look); Photoshop's by default.
    pub fn cursor_options(&mut self, options: CursorOptions) {
        self.cursor_options = options;
    }

    /// The timing of the latest brush stroke; `None` before the first.
    pub fn timing(&self) -> Option<LiquifyTiming> {
        self.timing
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<LiquifyEvent> {
        std::mem::take(&mut self.events)
    }

    /// A stroke, a lasso or a resize is in progress: the history waits (`settle(.., busy)`).
    pub fn busy(&self) -> bool {
        self.recorder.is_active() || self.lasso.is_some() || self.resize.active()
    }

    /// Sets the document size and the user's navigation options the Hand and Zoom tools run with.
    pub(super) fn navigate(&mut self, size: Size, prefs: NavigationOptions) {
        self.hand.size = size;
        self.hand.prefs = prefs;
        self.zoom.size = size;
        self.zoom.prefs = prefs;
    }

    /// The Hand tool.
    pub(super) fn hand(&mut self) -> &mut HandTool {
        &mut self.hand
    }

    /// The Zoom tool.
    pub(super) fn zoom(&mut self) -> &mut ZoomTool {
        &mut self.zoom
    }

    /// Ends the stroke in progress where it was last drawn: its `Commit`, for the caller to
    /// record. `None` with no stroke.
    pub(super) fn end_stroke(&mut self) -> Option<LiquifyEvent> {
        self.recorder.end(self.at, Modifiers::NONE).map(LiquifyEvent::Stroke)
    }

    /// A rest-time dab of the tool that acts while the brush is held still.
    fn hold(&mut self, at: Point, modifiers: Modifiers, time: f64) {
        if let Some(event) = self.recorder.hold([at.x, at.y, 1.0], modifiers) {
            self.events.push(LiquifyEvent::Stroke(event));
            self.dabbed([at.x, at.y], time);
        }
    }

    /// A dab at `at` at input time `time`.
    fn dabbed(&mut self, at: [f64; 2], time: f64) {
        self.at = at;
        self.last_dab = time;
        if let Some(timing) = self.timing.as_mut() {
            timing.dabs += 1;
            timing.seconds = (time - self.began).max(0.0);
        }
    }
}

/// Whether the tool keeps working while the brush is held still.
fn acts_at_rest(tool: LiquifyTool) -> bool {
    // pc: crates/algo/src/liquify.rs:92 `is_stationary`.
    !matches!(tool, LiquifyTool::ForwardWarp | LiquifyTool::PushLeft | LiquifyTool::FreezeLasso | LiquifyTool::Hand | LiquifyTool::Zoom)
}

/// [`LiquifyCanvas`] as the `CanvasTool` of one pass.
///
/// - Press, drag, release with a brush tool: one stroke through [`StrokeRecorder`] (Alt at the
///   press paints with [`LiquifyTool::alternate`](super::LiquifyTool::alternate)), reported as
///   [`LiquifyEvent::Stroke`] Begin, Update with each new point (the new points only), Commit
///   with the whole stroke. Esc or a lost pointer
///   reports Cancel, then a Replay (the workspace adds it from the session).
/// - The Freeze Lasso: an [`OutlineDraft`] of `DraftKind::Freehand` (`DraftReach::of` the
///   tokens); the release closes it into a stroke with
///   `amount` 1, or 0 when Alt was held at the press; drawn in `liquify.freeze` (thaw:
///   `liquify.thaw`).
/// - Alt + right-drag, or Control + Alt + drag ([`BrushResize::chord`]; [`BrushResize::takes`]
///   accepts the secondary button while Alt is held): a
///   [`BrushResize`] with `BrushLevel::Density`, `max_diameter` 15000; horizontal travel sets the
///   size, vertical the density over `brush.resize_travel`; the outline stays at the anchor; no
///   stroke. Each update writes `settings.brush`.
/// - Hover and paint: the brush outline under the pointer (`BrushCursor` over `BrushTip::round`
///   of the size), at the resize's anchor while it runs with the `TipFalloff::from_hardness` of
///   the density, none while the pointer is off the canvas; the lasso path while it is drawn.
/// - The cursor: the brush outline's (`BrushCursor::cursor`), the crosshair for the lasso.
pub struct LiquifyCanvasTool<'a> {
    canvas: &'a mut LiquifyCanvas,
    settings: &'a mut LiquifySettings,
    style: &'a LiquifyStyle,
}

impl LiquifyCanvasTool<'_> {
    /// Ends the stroke, the lasso and the resize in progress without recording them (the host's
    /// `cancel`, and the workspace when the host has no gesture to cancel).
    pub(super) fn abandon(&mut self, reason: CancelReason) {
        // new: Esc, a lost pointer, a tool switch or ⌘Z ends the stroke without recording it
        // (PhotoCraft has no cancel).
        if let Some(event) = self.canvas.recorder.cancel(reason) {
            self.canvas.events.push(LiquifyEvent::Stroke(event));
        }
        if let Some((mut draft, _)) = self.canvas.lasso.take() {
            draft.cancel();
        }
        if self.canvas.resize.cancel().is_some()
            && let Some(brush) = self.canvas.resize_from.take()
        {
            self.settings.brush = brush;
        }
    }

    /// Writes a resize's values to the brush.
    fn apply_resize(&mut self, event: Option<EditEvent<BrushDimensions>>) {
        if let Some(BrushDimensions { diameter, level }) = event.as_ref().and_then(EditEvent::value).copied() {
            self.settings.brush.size = diameter;
            self.settings.brush.density = level * 100.0;
        }
    }

    fn show_readout(&self, cx: &mut ToolContext<'_>) {
        let labels = BrushResizeLabels::of_ctx(&cx.canvas.ctx);
        if let Some(readout) = self.canvas.resize.readout(&Units::of_ctx(&cx.canvas.ctx), &labels) {
            cx.readout(readout);
        }
    }

    fn begin_resize(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        let brush = self.settings.brush;
        let options = BrushResizeOptions::new(&BrushStyle::of(cx.tokens), LiquifyBrush::SIZE_MAX).level(BrushLevel::Density);
        let start = BrushDimensions { diameter: brush.size, level: brush.density / 100.0 };
        if self.canvas.resize.begin(cx.view, pointer.screen, start, &options).is_none() {
            return ToolAction::Ignored;
        }
        self.canvas.resize_from = Some(brush);
        self.show_readout(cx);
        ToolAction::Claimed
    }

    /// The lasso's stroke for the closed polygon `points`.
    fn lasso_stroke(&self, points: &[Point], thaw: bool) -> LiquifyStroke {
        let brush = self.settings.brush;
        LiquifyStroke {
            tool: LiquifyTool::FreezeLasso,
            size: brush.size,
            density: brush.density,
            pressure: brush.pressure,
            rate: brush.rate,
            points: points.iter().map(|p| [p.x, p.y, 1.0]).collect(),
            amount: Some(if thaw { 0.0 } else { 1.0 }),
        }
    }
}

impl CanvasTool for LiquifyCanvasTool<'_> {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:1162-1216 (the preview's pointer input;
        // :1203-1207 the tools that act at rest keep dabbing at 30 dabs a second while the brush
        // is held still: the host calls `drag` every pass and `drag` dabs, so this asks for the
        // pass).
        if self.canvas.recorder.is_active() && acts_at_rest(self.settings.tool) {
            cx.request_repaint();
        }
    }

    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:671 `brush_circle_at` (the outline stays at
        // the anchor while resizing: the pointer hides and `paint` draws it there).
        if self.canvas.resize.active() {
            return ToolCursor::Hidden;
        }
        if self.settings.tool == LiquifyTool::FreezeLasso {
            let style = CursorHost::style(cx.tokens);
            return ToolCursor::Crosshair { length: style.crosshair.0, gap: style.crosshair.1 };
        }
        let size = self.settings.brush.size;
        BrushCursor::new(&BrushTip::round(size, size / 2.0)).painting(self.canvas.recorder.is_active()).cursor(
            &BrushStyle::of(cx.tokens),
            &self.canvas.cursor_options,
            cx.view,
            cx.caps_lock,
        )
    }

    fn takes(&self, button: PointerButton, modifiers: Modifiers) -> bool {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:676 `brush_resize_gesture` (Alt with the
        // secondary button, or Ctrl and Alt). The primary button always paints (Alt reverses to
        // the alternate tool); the secondary starts the resize chord while Alt is held.
        BrushResize::takes(button, modifiers)
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:693 `pointer` (Down), :682 `resized_brush`.
        if BrushResize::chord(pointer.button, pointer.modifiers) {
            return self.begin_resize(pointer, cx);
        }
        if pointer.button != PointerButton::Primary {
            return ToolAction::Ignored;
        }
        let tool = self.settings.tool;
        if tool == LiquifyTool::FreezeLasso {
            let mut draft = OutlineDraft::new(DraftKind::Freehand, DraftReach::of(cx.tokens));
            draft.press(DraftPoint::new(pointer.doc, pointer.modifiers, cx.view.zoom), false);
            self.canvas.lasso = Some((draft, pointer.modifiers.alt));
            return ToolAction::Claimed;
        }
        if !tool.is_brush() {
            return ToolAction::Ignored;
        }
        // A stroke whose release was lost ends first: the recorder would replace it silently.
        if let Some(event) = self.canvas.end_stroke() {
            self.canvas.events.push(event);
        }
        let at = [pointer.doc.x, pointer.doc.y, 1.0];
        let event = self.canvas.recorder.begin(tool, &self.settings.brush, at, pointer.modifiers);
        self.canvas.events.push(LiquifyEvent::Stroke(event));
        self.canvas.at = [at[0], at[1]];
        self.canvas.last_dab = cx.time;
        self.canvas.began = cx.time;
        self.canvas.timing = Some(LiquifyTiming { dabs: 1, seconds: 0.0 });
        cx.request_repaint();
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:704 (Move), :1196-1209 (a point that moved,
        // or the held dab).
        if self.canvas.resize.active() {
            let event = self.canvas.resize.track(cx.view, pointer.screen, pointer.modifiers);
            self.apply_resize(event);
            self.show_readout(cx);
            return;
        }
        if let Some((draft, _)) = self.canvas.lasso.as_mut() {
            draft.drag(DraftPoint::new(pointer.doc, pointer.modifiers, cx.view.zoom));
            return;
        }
        if !self.canvas.recorder.is_active() {
            return;
        }
        // PhotoCraft passes pressure 1 whatever the pen reports.
        let at = [pointer.doc.x, pointer.doc.y, 1.0];
        if let Some(event) = self.canvas.recorder.extend(at, pointer.modifiers) {
            self.canvas.events.push(LiquifyEvent::Stroke(event));
            self.canvas.dabbed([at[0], at[1]], cx.time);
        } else if acts_at_rest(self.settings.tool) && cx.time - self.canvas.last_dab > HOLD_INTERVAL {
            self.canvas.hold(pointer.doc, pointer.modifiers, cx.time);
        }
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:714 (Up), :299 `close_lasso`.
        if self.canvas.resize.active() {
            let event = self.canvas.resize.finish(cx.view, pointer.screen, pointer.modifiers);
            self.apply_resize(event);
            self.canvas.resize_from = None;
            return;
        }
        if let Some((mut draft, thaw)) = self.canvas.lasso.take() {
            let point = DraftPoint::new(pointer.doc, pointer.modifiers, cx.view.zoom);
            if let Some(DraftEvent::Closed { points, .. }) = draft.release(point) {
                let stroke = self.lasso_stroke(&points, thaw);
                let event = EditEvent::commit(GestureId::next(), stroke, true, pointer.modifiers);
                self.canvas.events.push(LiquifyEvent::Stroke(event));
            }
            return;
        }
        if let Some(event) = self.canvas.recorder.end([pointer.doc.x, pointer.doc.y], pointer.modifiers) {
            self.canvas.events.push(LiquifyEvent::Stroke(event));
        }
    }

    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        self.abandon(reason);
    }

    fn key(&mut self, _key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // new: Esc during a stroke, lasso or resize reaches `cancel` through the host and never
        // this method; outside a gesture Esc is the dialog's Cancel. Other keys are the
        // workspace's.
        false
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:1142-1149 (the lasso: danger freezes,
        // accent thaws), :1151-1159 (the brush circle, at the anchor while resizing; the cursor
        // draws it under the pointer).
        if let Some((draft, thaw)) = &self.canvas.lasso
            && draft.points().len() > 1
        {
            let color = if *thaw { self.style.thaw } else { self.style.freeze };
            let line: Vec<Pos2> = draft.points().iter().map(|p| cx.view.to_screen(*p)).collect();
            painter.add(Shape::line(line, Stroke::new(self.style.lasso_width, color)));
        }
        if let Some(BrushDimensions { diameter, level }) = self.canvas.resize.dimensions() {
            let falloff = TipFalloff::from_hardness(diameter / 2.0, level);
            self.canvas.resize.paint(painter, cx.view, &BrushStyle::of(cx.tokens), &falloff);
        }
    }
}
