//! [`ToolHost`]: the on-canvas tool host. One tool at a time gets the canvas's presses, drags,
//! releases, hovers and keys as document-space events, snapped and pen-aware, and paints its
//! feedback over the document. Tools (step 7), the colour droppers and the canvas text editor
//! (step 4) implement [`CanvasTool`]; the app owns the tool values and switches between them.
//!
//! Order within a pass: the overlays run first on the host's canvas `Response` (handles, guides,
//! then the tool, see [`Claim`](super::Claim)); the host takes the press only when no overlay did. The host:
//!
//! 1. reads the pointer from the canvas response (or a queued automation event,
//!    [`ToolHost::inject`]), maps it through the [`CanvasView`] and merges the pen sample
//!    ([`PenInput`](super::PenInput)) into a [`ToolPointer`];
//! 2. snaps it when the tool asks ([`CanvasTool::snap_gesture`]) with the [`SnapTargets`] the host
//!    was given, Ctrl held turns snapping off (Photoshop);
//! 3. calls the tool's [`press`](CanvasTool::press), [`drag`](CanvasTool::drag),
//!    [`release`](CanvasTool::release) or [`hover`](CanvasTool::hover); Esc during a drag calls
//!    [`cancel`](CanvasTool::cancel), other keys go to [`key`](CanvasTool::key) while the canvas
//!    has keyboard focus and no text field has the keys;
//! 4. autoscrolls the view toward a pointer dragged past the canvas edge
//!    ([`Autoscroll`](super::Autoscroll), within the [`document`](ToolHost::document)'s edges) and reports the move in the response, together with
//!    any view the tool asked for ([`ToolContext::set_view`]: the Hand, Zoom and Rotate View
//!    tools) as [`ToolHostResponse::view`];
//! 5. lets the tool [`paint`](CanvasTool::paint) and sets the cursor the tool returned through
//!    the [`CursorHost`](super::CursorHost) (the OS pointer, a bitmap, or the painted tool icon
//!    with its badge).
//!
//! A momentary tool (Space for the Hand, see [`MomentaryKeys`](super::MomentaryKeys)) is the
//! app's: it passes the temporary tool to `show` instead of the selected one; a drag in progress
//! keeps the tool it started with until it ends.
//!
//! ```no_run
//! use craft_ui::canvas::{CanvasTool, ToolHost, ToolHostResponse};
//! # fn demo(ui: &mut egui::Ui, view: &craft_ui::canvas::CanvasView, canvas: &egui::Response, tool: &mut dyn CanvasTool) {
//! let ToolHostResponse { gesture, scrolled_by, .. } = ToolHost::new("doc-1").autoscroll(true).show(ui, view, canvas, tool);
//! # }
//! ```

use egui::{AsIdSalt, Context, CursorIcon, Event, Id, IdSalt, Key, Modifiers, Painter, PointerButton, Pos2, Response, Ui};
use kurbo::{Point, Size, Vec2};

use super::claim::Claim;
use super::cursor::{CursorHost, ToolCursor};
use super::navigate::Autoscroll;
use super::pointer::{PenInput, PenSample};
use super::snap::{SNAP_PX, SnapGesture, SnapTargets};
use super::view::CanvasView;
use super::{Readout, Turn};
use crate::localized::LocaleNumbers;
use crate::theme::{Tokens, TokensOf as _, UiTokens};

/// A pointer sample delivered to a tool, in document and screen space, with the pen's reading.
/// A mouse reports `pressure` 1, no tilt, `eraser` false.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolPointer {
    /// The pointer in document units, after snapping when the tool asked for it.
    pub doc: Point,
    /// The pointer in document units before snapping.
    pub raw: Point,
    /// The pointer on screen.
    pub screen: Pos2,
    /// Pen pressure 0..=1; 1 for a mouse.
    pub pressure: f32,
    /// Pen tilt in degrees (-90..90, x and y, W3C Pointer Events convention); 0 for a mouse.
    pub tilt: [f32; 2],
    /// Pen barrel rotation in degrees 0..360.
    pub rotation: f32,
    /// The pen's eraser end is in use.
    pub eraser: bool,
    /// The button the gesture runs with (hover: the primary).
    pub button: PointerButton,
    pub modifiers: Modifiers,
    /// `egui` input time of the sample.
    pub time: f64,
    /// The sample was moved by snapping (`doc != raw`).
    pub snapped: bool,
}

/// What a tool does with a press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolAction {
    /// The tool owns the gesture: it gets the drags and the release.
    Claimed,
    /// A click the tool handled whole (a Zoom tool step, an eyedropper sample): no drag follows.
    Done,
    /// The press is not the tool's (over the pasteboard with a tool that does nothing there): the
    /// host leaves it to the canvas's fallback (the app's pasteboard click, nothing).
    Ignored,
}

/// A key the host passes to the tool while the canvas has the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolKey {
    pub key: Key,
    pub modifiers: Modifiers,
    /// `true` on the press (and each repeat), `false` on the release.
    pub pressed: bool,
    /// The key is a repeat of a held press.
    pub repeat: bool,
}

/// The phase of the tool's pointer gesture this pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolGesture {
    /// The press was claimed this pass.
    Began,
    /// A drag moved this pass.
    Moved,
    /// The button came up: the tool committed.
    Ended,
    /// Esc, a lost pointer or [`ToolHost::cancel`] ended the gesture without a commit.
    Cancelled,
}

/// What the host did in a pass.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "apply the scroll to the app's view and read the gesture"]
pub struct ToolHostResponse {
    /// The host's id, for [`ToolHost::inject`] and [`ToolHost::cancel`].
    pub id: Id,
    /// The gesture's phase, when one began, moved or ended this pass.
    pub gesture: Option<ToolGesture>,
    /// How far autoscroll moved the view, in document units (already in [`view`](Self::view)).
    pub scrolled_by: Option<Vec2>,
    /// The view after this pass's autoscroll and the tool's [`ToolContext::set_view`], when
    /// either changed it: the app copies its centre, zoom and rotation back into its own view
    /// (the host has only a copy).
    pub view: Option<CanvasView>,
    /// The cursor the tool asked for, after the host set it: for a status bar or test.
    pub cursor: CursorIcon,
    /// The press this pass was [`ToolAction::Ignored`] by the tool: the app may use it.
    pub ignored_press: Option<ToolPointer>,
    /// A key the tool took this pass, so the app's shortcut dispatch leaves it.
    pub took_key: Option<ToolKey>,
}

/// What a tool sees and may ask for during a call. Requests (a cursor, a readout, a repaint)
/// are applied by the host after the call.
pub struct ToolContext<'a> {
    /// The document-to-screen mapping of this pass.
    pub view: &'a CanvasView,
    /// The canvas's response: hover, focus, rect.
    pub canvas: &'a Response,
    /// The pointer's last sample this pass, snapped as the tool asked.
    pub pointer: Option<ToolPointer>,
    pub modifiers: Modifiers,
    /// `egui` input time and the seconds since the last pass.
    pub time: f64,
    pub dt: f32,
    /// The snap targets the host was given, for a tool that snaps its own geometry (a transform
    /// box's edges) rather than the pointer.
    pub snap: Option<&'a SnapTargets>,
    /// The snap tolerance in document units at this zoom (8 screen points over the zoom), or
    /// `None` while Ctrl holds snapping off.
    pub snap_tolerance: Option<f64>,
    pub tokens: &'a UiTokens,
    pub numbers: &'a LocaleNumbers,
    /// Caps Lock is on ([`CursorHost::caps_lock`]): the precise cursors of
    /// [`ToolCursor::brush`] and [`ToolCursor::tool`] take it.
    pub caps_lock: bool,
    requests: ToolRequests,
}

/// What a tool asked the host for in a call.
#[derive(Clone, Debug, Default)]
struct ToolRequests {
    cursor: Option<ToolCursor>,
    view: Option<CanvasView>,
    readout: Option<Readout>,
    repaint: bool,
    keep_keys: bool,
}

impl<'a> ToolContext<'a> {
    /// The cursor to show after this call, instead of [`CanvasTool::cursor`]'s: the Move tool's
    /// grab hand during a drag.
    pub fn set_cursor(&mut self, cursor: ToolCursor) {
        self.requests.cursor = Some(cursor);
    }

    /// Labelled values drawn beside the pointer this pass (a marquee's W and H).
    pub fn readout(&mut self, readout: Readout) {
        self.requests.readout = Some(readout);
    }

    /// Change the view (the Hand tool's pan, a Zoom tool click, Rotate View): the host reports it
    /// in [`ToolHostResponse::view`] and later calls this pass see it in
    /// [`view`](Self::view). The last request of a pass wins.
    pub fn set_view(&mut self, view: CanvasView) {
        self.requests.view = Some(view);
    }

    /// Ask for another pass (an animation, a held key that repeats).
    pub fn request_repaint(&mut self) {
        self.requests.repaint = true;
    }

    /// Claim the canvas's keys for the tool this pass and the next: the app's shortcuts leave
    /// them (a text editor on the canvas, the pen's Delete).
    pub fn keep_keys(&mut self) {
        self.requests.keep_keys = true;
    }
}

/// A tool on the canvas. The app owns the value and hands it to [`ToolHost::show`] each pass;
/// a tool with a gesture in progress keeps its state between passes in itself.
///
/// Every method gets the same [`ToolContext`]; the pointer in it is already in document units
/// and snapped as [`snap_gesture`](Self::snap_gesture) asked. Tools never read `ui.input()`
/// for the pointer: automation feeds the host the same events.
pub trait CanvasTool {
    /// The cursor over the canvas when no gesture runs, for `pointer` (document units; `None`
    /// off the canvas). The default is the system arrow.
    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        ToolCursor::System(CursorIcon::Default)
    }

    /// The primary (or the tool's own) button went down on the canvas and no overlay took it.
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction;

    /// The pointer moved while the tool owns the gesture (and on every pass the host feeds
    /// recovered samples: a pen may report several per pass, oldest first).
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>);

    /// The button came up: commit the gesture.
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>);

    /// The pointer moved over the canvas with no gesture running (pen rubber band, hover
    /// outlines). Default: nothing.
    fn hover(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    /// The gesture ends without a commit: Esc, the pointer lost, the tool switched, or the
    /// app's [`ToolHost::cancel`]. The tool drops its drag state.
    fn cancel(&mut self, cx: &mut ToolContext<'_>);

    /// A key while the canvas has the keyboard and no text field claims it. Return `true` to
    /// take it from the app's shortcuts (the pen's Delete, a marquee's arrows). Esc during a
    /// gesture never reaches here (it cancels). Default: `false`.
    fn key(&mut self, _key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        false
    }

    /// Draw the tool's feedback over the document (a marquee in progress, a brush outline that
    /// is not the cursor, pen segments). Runs after the overlays.
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>);

    /// How the host snaps the pointer for this tool: a point (marquee corners, pen anchors), a
    /// moving rectangle (the Move tool), or not at all (`None`, the default: brushes, zoom, hand).
    fn snap_gesture(&self) -> Option<SnapGesture> {
        None
    }

    /// Whether a drag past the canvas edge scrolls the view (marquees, lassos, brushes, crop:
    /// yes; the Hand and Zoom tools: no). Default: `true`.
    fn autoscrolls(&self) -> bool {
        true
    }

    /// The button the tool works with. Default: the primary. The right button never reaches a
    /// tool's gesture unless it says so here (PhotoCraft's Brush erases with it when the
    /// preference asks).
    fn button(&self) -> PointerButton {
        PointerButton::Primary
    }
}

/// A pointer event from automation (PhotoCraft's `ui.pointer`), queued with
/// [`ToolHost::inject`] and delivered on the next pass as if the OS pointer did it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToolInput {
    Down { doc: Point, pressure: f32, tilt: [f32; 2], rotation: f32, modifiers: Modifiers },
    Move { doc: Point, pressure: f32, tilt: [f32; 2], rotation: f32, modifiers: Modifiers },
    Up { doc: Point, modifiers: Modifiers },
}

/// The gesture a tool owns: its button, the last sample it got and the raw document point of
/// the press (a moving rectangle snaps relative to it).
#[derive(Clone, Copy, Debug)]
struct Active {
    button: PointerButton,
    last: ToolPointer,
    origin: Point,
    /// Started by [`ToolHost::inject`]: the OS pointer neither moves nor ends it.
    injected: bool,
}

/// The host's state between passes: the gesture in progress and the queued input.
#[derive(Clone, Debug, Default)]
struct HostState {
    gesture: Option<Active>,
    /// A primary press that waits for another control's typing run to end ([`Claim::turn`]).
    waiting: Option<(PointerButton, ToolPointer)>,
    queued: Vec<ToolInput>,
    /// [`ToolHost::cancel`] was called.
    cancel: bool,
    /// Autoscroll's accumulated screen points while the view cannot move yet.
    scroll_carry: egui::Vec2,
    /// The tool asked to keep the keys in the last pass ([`ToolContext::keep_keys`]).
    keep_keys: bool,
}

/// The on-canvas tool host. Built each pass, shown once with the canvas's response and the tool.
pub struct ToolHost<'a> {
    id_salt: IdSalt,
    snap: Option<&'a SnapTargets>,
    autoscroll: bool,
    pen: Option<&'a PenInput>,
    enabled: bool,
    keys: bool,
    document: Size,
}

impl<'a> ToolHost<'a> {
    /// A host for one canvas, keyed by `id_salt` under the `Ui` it is shown in (one per document view);
    /// [`ToolHostResponse::id`] is the resulting id.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), snap: None, autoscroll: true, pen: None, enabled: true, keys: true, document: Size::ZERO }
    }

    /// The snap targets of this pass (built by the app from its document, see
    /// [`SnapTargets`]); the host snaps the pointer with them for tools whose
    /// [`snap_gesture`](CanvasTool::snap_gesture) asks.
    pub fn snap(mut self, targets: &'a SnapTargets) -> Self {
        self.snap = Some(targets);
        self
    }

    /// Whether a drag past the canvas edge scrolls the view (with the tool's consent). On by
    /// default.
    pub fn autoscroll(mut self, on: bool) -> Self {
        self.autoscroll = on;
        self
    }

    /// The document's size in document units: autoscroll stops at the document's edges. Without
    /// it (the default, an empty size) the view never autoscrolls.
    pub fn document(mut self, size: Size) -> Self {
        self.document = size;
        self
    }

    /// The pen reader whose samples fill pressure, tilt, rotation and the eraser end. Without
    /// one every sample is a mouse.
    pub fn pen(mut self, pen: &'a PenInput) -> Self {
        self.pen = Some(pen);
        self
    }

    /// Off, the host delivers no presses or keys (a modal workspace over the canvas) but still
    /// paints and sets the cursor.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether keys reach the tool. Off while the app routes keys itself (a text field has
    /// them). On by default.
    pub fn keys(mut self, keys: bool) -> Self {
        self.keys = keys;
        self
    }

    /// Run the tool for this pass. `canvas` is the host's canvas response (one `interact` over the
    /// canvas rect, after the overlays ran); the tool gets the press only when [`Claim`](super::Claim) has no
    /// owner. The gesture's state lives in egui memory under the host's id, so the tool value
    /// may be rebuilt between passes as long as it keeps its own drag state.
    // pc: crates/ui-egui/src/canvas/tool_input.rs:294 `tool_event`, canvas/paint_mouse.rs:90 `Buttons`,
    // canvas.rs:1315 `draw_canvas_tool_cursor`, canvas/canvas_autoscroll.rs:21.
    pub fn show(self, ui: &mut Ui, view: &CanvasView, canvas: &Response, tool: &mut dyn CanvasTool) -> ToolHostResponse {
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.id_salt);
        CursorHost::install(&ctx);
        let tokens = Tokens::of(ui);
        let numbers = LocaleNumbers::of(ui);
        let mut run = Run {
            pass: Pass {
                canvas,
                snap: self.snap,
                tokens: &tokens,
                numbers: &numbers,
                view: *view,
                view_changed: false,
                modifiers: Modifiers::NONE,
                time: 0.0,
                dt: 0.0,
                caps_lock: CursorHost::caps_lock(&ctx),
                cursor: None,
                readout: None,
                keep_keys: false,
            },
            state: {
                let state = Self::state(&ctx, id);
                // Input queued from here on belongs to the next pass.
                ctx.data_mut(|d| {
                    let stored = d.get_temp_mut_or_default::<HostState>(id);
                    stored.queued.clear();
                    stored.cancel = false;
                });
                state
            },
            resp: ToolHostResponse {
                id,
                gesture: None,
                scrolled_by: None,
                view: None,
                cursor: CursorIcon::Default,
                ignored_press: None,
                took_key: None,
            },
            tool,
            pen: self.pen.map(|pen| pen.sample().sanitized()),
            document: self.document,
            autoscroll: self.autoscroll,
            ctx: ctx.clone(),
        };
        run.pass.time = ctx.input(|i| i.time);
        run.pass.dt = ctx.input(|i| i.unstable_dt);
        run.pass.modifiers = ctx.input(|i| i.modifiers);
        run.step(self.enabled, self.keys);
        run.finish(ui)
    }

    /// Queue an automation pointer event for the host `id` (from [`ToolHostResponse::id`]); it
    /// is delivered on the next pass in order, after the OS pointer's events. A `Down` while the
    /// OS pointer holds a gesture is dropped.
    // pc: crates/ui-egui/src/control/dispatch.rs `ui.pointer` → tool_input::tool_event.
    pub fn inject(ctx: &Context, id: Id, input: ToolInput) {
        ctx.data_mut(|d| d.get_temp_mut_or_default::<HostState>(id).queued.push(input));
        ctx.request_repaint();
    }

    /// Cancel the gesture in progress (the app switched tools or documents, a modal opened):
    /// the tool's [`cancel`](CanvasTool::cancel) runs on the next `show`.
    // pc: crates/ui-egui/src/state.rs drag = None on tool switch; tool_memory.rs.
    pub fn cancel(ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.get_temp_mut_or_default::<HostState>(id).cancel = true);
        ctx.request_repaint();
    }

    /// Whether the host `id` has a gesture in progress (for the app's tool switch and the
    /// momentary keys: a drag finishes with the tool it started with).
    pub fn in_gesture(ctx: &Context, id: Id) -> bool {
        let state = Self::state(ctx, id);
        state.gesture.is_some() || state.waiting.is_some()
    }

    /// The pointer sample the host would deliver for `screen` with `modifiers`: `view` mapping,
    /// snapping per `snap`, `pen` merged (`None` is a mouse). Pure, for tests and for tools that
    /// sample the pointer outside a gesture.
    // pc: crates/ui-egui/src/canvas/snap_ui.rs:493 `filter_event`, input/stylus.rs:263 `sample`.
    pub fn sample(
        view: &CanvasView,
        screen: Pos2,
        modifiers: Modifiers,
        snap: Option<SnapAt<'_>>,
        pen: Option<super::pointer::PenSample>,
        time: f64,
    ) -> ToolPointer {
        make_pointer(screen, view.to_doc(screen), modifiers, snap, None, pen, time)
    }

    /// `8 / zoom`: the snap tolerance in document units, or `None` with Ctrl held (snapping off
    /// while dragging, Photoshop) unless `ctrl_is_tool_modifier` (the corner-distortion key).
    // pc: crates/ui-egui/src/canvas/snap_ui.rs:76 `tolerance`.
    pub fn snap_tolerance(view: &CanvasView, modifiers: Modifiers, ctrl_is_tool_modifier: bool) -> Option<f64> {
        let off = modifiers.ctrl && !modifiers.mac_cmd && !ctrl_is_tool_modifier;
        (!off).then(|| SNAP_PX / view.zoom.max(0.01))
    }

    fn state(ctx: &Context, id: Id) -> HostState {
        ctx.data(|d| d.get_temp::<HostState>(id)).unwrap_or_default()
    }
}

/// The pointer sample for `screen` / `raw` (the same point in document units, exact when it came
/// from automation). `origin` is the raw press point of a moving-rectangle gesture; without it the
/// rectangle is snapped where it is.
fn make_pointer(
    screen: Pos2,
    raw: Point,
    modifiers: Modifiers,
    snap: Option<SnapAt<'_>>,
    origin: Option<Point>,
    pen: Option<PenSample>,
    time: f64,
) -> ToolPointer {
    let doc = match snap {
        None => raw,
        Some(SnapAt { gesture, targets, tolerance }) => match gesture {
            SnapGesture::Point => targets.snap_point(raw, tolerance).0,
            SnapGesture::Rect { at_press } => {
                let moved = at_press + (raw - origin.unwrap_or(raw));
                raw + targets.snap_rect(moved, tolerance).0
            }
            SnapGesture::Axis { vertical } => {
                let value = if vertical { raw.x } else { raw.y };
                let snapped = targets.snap_axis(vertical, value, tolerance).0;
                if vertical { Point::new(snapped, raw.y) } else { Point::new(raw.x, snapped) }
            }
        },
    };
    let pen = pen.unwrap_or(PenSample::MOUSE).sanitized();
    ToolPointer {
        doc,
        raw,
        screen,
        pressure: pen.pressure,
        tilt: pen.tilt,
        rotation: pen.rotation,
        eraser: pen.eraser,
        button: PointerButton::Primary,
        modifiers,
        time,
        snapped: doc != raw,
    }
}

/// What the tool and the host share during one pass: the working view (tool calls later in the
/// pass see the earlier ones' [`ToolContext::set_view`]) and what the calls asked for.
struct Pass<'p> {
    canvas: &'p Response,
    snap: Option<&'p SnapTargets>,
    tokens: &'p UiTokens,
    numbers: &'p LocaleNumbers,
    view: CanvasView,
    view_changed: bool,
    modifiers: Modifiers,
    time: f64,
    dt: f32,
    caps_lock: bool,
    cursor: Option<ToolCursor>,
    readout: Option<Readout>,
    keep_keys: bool,
}

impl Pass<'_> {
    fn context(&self, pointer: Option<ToolPointer>) -> ToolContext<'_> {
        let modifiers = pointer.map_or(self.modifiers, |p| p.modifiers);
        ToolContext {
            view: &self.view,
            canvas: self.canvas,
            pointer,
            modifiers,
            time: self.time,
            dt: self.dt,
            snap: self.snap,
            snap_tolerance: ToolHost::snap_tolerance(&self.view, modifiers, false),
            tokens: self.tokens,
            numbers: self.numbers,
            caps_lock: self.caps_lock,
            requests: ToolRequests::default(),
        }
    }

    /// Runs `run` with a context for `pointer` and applies what it asked for.
    fn call<R>(&mut self, pointer: Option<ToolPointer>, run: impl FnOnce(&mut ToolContext<'_>) -> R) -> R {
        let mut cx = self.context(pointer);
        let result = run(&mut cx);
        let requests = std::mem::take(&mut cx.requests);
        if let Some(view) = requests.view {
            self.view = view;
            self.view_changed = true;
        }
        if requests.cursor.is_some() {
            self.cursor = requests.cursor;
        }
        if requests.readout.is_some() {
            self.readout = requests.readout;
        }
        if requests.repaint {
            self.canvas.ctx.request_repaint();
        }
        self.keep_keys |= requests.keep_keys;
        result
    }
}

/// One pass of the host: the working state, the tool and the response being built.
struct Run<'r, 'p> {
    pass: Pass<'p>,
    state: HostState,
    resp: ToolHostResponse,
    tool: &'r mut dyn CanvasTool,
    pen: Option<PenSample>,
    document: Size,
    autoscroll: bool,
    ctx: Context,
}

impl Run<'_, '_> {
    /// The sample for a pointer at `screen` (or at `raw` document units, from automation).
    /// At the press Ctrl belongs to the tool and a moving rectangle has not moved yet.
    fn sample(
        &self,
        screen: Pos2,
        raw: Option<Point>,
        modifiers: Modifiers,
        pen: Option<PenSample>,
        button: PointerButton,
        press: bool,
    ) -> ToolPointer {
        let view = &self.pass.view;
        let raw_doc = raw.unwrap_or_else(|| view.to_doc(screen));
        let screen = if raw.is_some() { view.to_screen(raw_doc) } else { screen };
        let snap =
            self.pass.snap.zip(self.tool.snap_gesture()).filter(|(_, gesture)| !(press && matches!(gesture, SnapGesture::Rect { .. }))).and_then(
                |(targets, gesture)| ToolHost::snap_tolerance(view, modifiers, press).map(|tolerance| SnapAt { gesture, targets, tolerance }),
            );
        let origin = self.state.gesture.map(|g| g.origin);
        ToolPointer { button, ..make_pointer(screen, raw_doc, modifiers, snap, origin, pen, self.pass.time) }
    }

    fn call<R>(&mut self, pointer: Option<ToolPointer>, run: impl FnOnce(&mut dyn CanvasTool, &mut ToolContext<'_>) -> R) -> R {
        let tool = &mut *self.tool;
        self.pass.call(pointer, |cx| run(tool, cx))
    }

    fn report(&mut self, gesture: ToolGesture) {
        let over = |old: Option<ToolGesture>| matches!((old, gesture), (_, ToolGesture::Ended | ToolGesture::Cancelled) | (None, _));
        if over(self.resp.gesture) {
            self.resp.gesture = Some(gesture);
        }
    }

    fn begin(&mut self, p: ToolPointer, injected: bool) {
        match self.call(Some(p), |tool, cx| tool.press(&p, cx)) {
            ToolAction::Claimed => {
                self.state.gesture = Some(Active { button: p.button, last: p, origin: p.raw, injected });
                self.report(ToolGesture::Began);
            }
            ToolAction::Done => self.report(ToolGesture::Ended),
            ToolAction::Ignored => self.resp.ignored_press = Some(p),
        }
    }

    fn drag(&mut self, p: ToolPointer) {
        let Some(active) = self.state.gesture.as_mut() else { return };
        let moved = active.last.screen != p.screen || active.last.doc != p.doc;
        active.last = p;
        self.call(Some(p), |tool, cx| tool.drag(&p, cx));
        if moved || self.resp.scrolled_by.is_some() {
            self.report(ToolGesture::Moved);
        }
    }

    fn end(&mut self, p: ToolPointer) {
        if self.state.gesture.take().is_some() {
            self.call(Some(p), |tool, cx| tool.release(&p, cx));
            self.report(ToolGesture::Ended);
        }
    }

    fn cancel(&mut self) {
        self.state.waiting = None;
        if let Some(active) = self.state.gesture.take() {
            self.call(Some(active.last), |tool, cx| tool.cancel(cx));
            self.report(ToolGesture::Cancelled);
        }
    }

    fn hover(&mut self, p: ToolPointer) {
        self.call(Some(p), |tool, cx| tool.hover(&p, cx));
    }

    /// Scroll the view toward a pointer dragged past the canvas edge.
    fn autoscroll(&mut self, screen: Pos2) {
        if !self.autoscroll || !self.tool.autoscrolls() {
            return;
        }
        let step = Autoscroll::edge_step(screen, self.pass.canvas.rect, self.pass.dt);
        if step == egui::Vec2::ZERO {
            self.state.scroll_carry = egui::Vec2::ZERO;
            return;
        }
        let carry = (self.state.scroll_carry + step).clamp(egui::Vec2::splat(-8.0), egui::Vec2::splat(8.0));
        match Autoscroll::scroll(&mut self.pass.view, carry, self.document) {
            Some(by) => {
                self.state.scroll_carry = egui::Vec2::ZERO;
                self.pass.view_changed = true;
                self.resp.scrolled_by = Some(self.resp.scrolled_by.unwrap_or(Vec2::ZERO) + by);
            }
            None => self.state.scroll_carry = carry,
        }
    }

    fn step(&mut self, enabled: bool, keys: bool) {
        let ctx = self.ctx.clone();
        let canvas = self.pass.canvas;
        let claim = tool_claim_id(self.resp.id);
        let button = self.tool.button();
        let (pos, origin, pressed, down, released, key_events) = ctx.input(|i| {
            let key_events: Vec<ToolKey> = i
                .events
                .iter()
                .filter_map(|e| match e {
                    Event::Key { key, pressed, repeat, modifiers, .. } => {
                        Some(ToolKey { key: *key, modifiers: *modifiers, pressed: *pressed, repeat: *repeat })
                    }
                    _ => None,
                })
                .collect();
            (
                i.pointer.latest_pos(),
                i.pointer.press_origin(),
                i.pointer.button_pressed(button),
                i.pointer.button_down(button),
                i.pointer.button_released(button),
                key_events,
            )
        });
        let modifiers = self.pass.modifiers;
        let pen = self.pen;
        let kept_keys = self.state.keep_keys;

        if std::mem::take(&mut self.state.cancel) || !enabled {
            self.cancel();
        }

        // A press that waited for another control's typing run.
        let waited = self.state.waiting.is_some();
        if let Some((button, press)) = self.state.waiting
            && enabled
        {
            match Claim::turn(&ctx, canvas.id, claim) {
                Turn::Waits => {}
                Turn::Runs => {
                    self.state.waiting = None;
                    self.begin(ToolPointer { button, ..press }, false);
                }
                Turn::Now => self.state.waiting = None,
            }
        }

        let mut began = false;
        let mut press_at = None;
        if enabled && self.state.gesture.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, claim, waited) {
            let at = origin.or(pos).filter(|at| pressed && canvas.is_pointer_button_down_on() && canvas.rect.contains(*at));
            // Only the primary button is claimed; the others yield to an overlay's claim.
            let free = |owner: Option<Id>| match button {
                PointerButton::Primary => Claim::take(&ctx, canvas.id, claim),
                _ => owner.is_none(),
            };
            if let Some(at) = at.filter(|_| free(Claim::owner(&ctx, canvas.id))) {
                let p = self.sample(at, None, modifiers, pen, button, true);
                if button == PointerButton::Primary && Claim::turn(&ctx, canvas.id, claim) == Turn::Waits {
                    self.state.waiting = Some((button, p));
                } else {
                    self.begin(p, false);
                    began = self.state.gesture.is_some();
                    press_at = Some(at);
                }
            }
        }

        // The OS pointer's gesture.
        if let Some(active) = self.state.gesture.filter(|g| !g.injected) {
            if !down && !released {
                self.cancel();
            } else {
                let at = pos.unwrap_or(active.last.screen);
                let finished = released || !down;
                if !(began && !finished && press_at == Some(at)) {
                    if !finished {
                        self.autoscroll(at);
                    }
                    let p = self.sample(at, None, modifiers, pen, active.button, false);
                    if finished {
                        self.end(p);
                    } else {
                        self.drag(p);
                    }
                }
            }
        }

        // Automation's events, in order, after the pointer's.
        for input in std::mem::take(&mut self.state.queued) {
            self.injected(input, enabled);
        }

        // Keys. Esc ends a gesture whoever has the keyboard; other keys need the canvas to have it.
        let mut took = None;
        let keyboard = enabled && keys && canvas.has_focus() && !ctx.text_edit_focused();
        if keyboard {
            // egui drops the focus on Esc and moves it on the arrows unless the widget asks for them.
            let filter = egui::EventFilter { escape: true, horizontal_arrows: true, vertical_arrows: true, ..egui::EventFilter::default() };
            ctx.memory_mut(|m| m.set_focus_lock_filter(canvas.id, filter));
        }
        for key in key_events {
            if enabled && keys && key.pressed && key.key == Key::Escape && self.state.gesture.is_some() {
                self.cancel();
                took = Some(key);
            } else if keyboard {
                let pointer = self.state.gesture.map(|g| g.last);
                if self.call(pointer, |tool, cx| tool.key(&key, cx)) || kept_keys || self.pass.keep_keys {
                    took = Some(key);
                }
            }
        }
        self.resp.took_key = took;

        // A pointer over the canvas with no gesture hovers.
        if enabled
            && self.state.gesture.is_none()
            && canvas.contains_pointer()
            && let Some(at) = pos
        {
            let p = self.sample(at, None, modifiers, pen, PointerButton::Primary, false);
            self.hover(p);
        }
    }

    fn injected(&mut self, input: ToolInput, enabled: bool) {
        let button = self.tool.button();
        let own = self.state.gesture.is_none_or(|g| g.injected);
        match input {
            ToolInput::Down { doc, pressure, tilt, rotation, modifiers } if enabled && self.state.gesture.is_none() => {
                let pen = PenSample { pressure, tilt, rotation, eraser: false, time_ms: 0.0 };
                let p = self.sample(Pos2::ZERO, Some(doc), modifiers, Some(pen), button, true);
                self.begin(p, true);
            }
            ToolInput::Move { doc, pressure, tilt, rotation, modifiers } if enabled && own => {
                let pen = PenSample { pressure, tilt, rotation, eraser: false, time_ms: 0.0 };
                let gesture = self.state.gesture.is_some();
                let p = self.sample(Pos2::ZERO, Some(doc), modifiers, Some(pen), button, false);
                if gesture {
                    self.drag(p);
                } else {
                    self.hover(p);
                }
            }
            ToolInput::Up { doc, modifiers } if own && self.state.gesture.is_some() => {
                let p = self.sample(Pos2::ZERO, Some(doc), modifiers, None, button, false);
                self.end(p);
            }
            _ => {}
        }
    }

    /// Paint the tool, set the cursor, store the state and return the response.
    fn finish(mut self, ui: &mut Ui) -> ToolHostResponse {
        let ctx = self.ctx.clone();
        let canvas = self.pass.canvas;
        let last = self.state.gesture.map(|g| g.last);
        let pos = ctx.input(|i| i.pointer.latest_pos());
        let hover = canvas.contains_pointer().then_some(pos).flatten();
        let pointer = last.or_else(|| hover.map(|at| self.sample(at, None, self.pass.modifiers, self.pen, PointerButton::Primary, false)));

        let painter = ui.painter_at(canvas.rect);
        self.tool.paint(&painter, &self.pass.context(pointer));
        if let Some(readout) = self.pass.readout.take()
            && let Some(p) = pointer
        {
            readout.show(&ctx, p.screen);
        }
        if let Some(p) = pointer {
            let cursor = match self.pass.cursor.take() {
                Some(cursor) => cursor,
                None => self.tool.cursor(&self.pass.context(Some(p))),
            };
            let style = CursorHost::style(self.pass.tokens);
            self.resp.cursor = CursorHost::set(&ctx, ui.painter(), p.screen, &cursor, &style);
        }

        self.state.keep_keys = self.pass.keep_keys;
        self.resp.view = self.pass.view_changed.then_some(self.pass.view);
        // Automation may have queued input while the tool ran.
        let queued = ctx.data(|d| d.get_temp::<HostState>(self.resp.id)).map(|s| (s.queued, s.cancel)).unwrap_or_default();
        self.state.queued.extend(queued.0);
        self.state.cancel |= queued.1;
        ctx.data_mut(|d| d.insert_temp(self.resp.id, self.state));
        self.resp
    }
}

/// How [`ToolHost::sample`] snaps: the gesture kind, the targets and the tolerance in document
/// units (see [`ToolHost::snap_tolerance`]).
#[derive(Clone, Copy, Debug)]
pub struct SnapAt<'a> {
    pub gesture: SnapGesture,
    pub targets: &'a SnapTargets,
    pub tolerance: f64,
}

/// A tool that does nothing: the host's tool when the app has none selected (a document with
/// no tools, a viewer). The cursor is the arrow, presses are ignored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoTool;

impl CanvasTool for NoTool {
    fn press(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Ignored
    }

    fn drag(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn release(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {}

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}

    fn autoscrolls(&self) -> bool {
        false
    }
}

/// Lets the host's [`Claim`](super::Claim) name the tool as an overlay: the tool's claim id under the canvas.
pub(super) fn tool_claim_id(host: Id) -> Id {
    host.with("tool")
}
