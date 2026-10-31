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
//!    ([`Autoscroll`](super::Autoscroll)) and reports the move in the response, together with
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

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Context, CursorIcon, Id, IdSalt, Key, Modifiers, Painter, PointerButton, Pos2, Response, Ui};
use kurbo::{Point, Vec2};

use super::Readout;
use super::cursor::ToolCursor;
use super::snap::{SnapGesture, SnapTargets};
use super::view::CanvasView;
use crate::localized::LocaleNumbers;
use crate::theme::UiTokens;

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
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
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
    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {}

    /// The gesture ends without a commit: Esc, the pointer lost, the tool switched, or the
    /// app's [`ToolHost::cancel`]. The tool drops its drag state.
    fn cancel(&mut self, cx: &mut ToolContext<'_>);

    /// A key while the canvas has the keyboard and no text field claims it. Return `true` to
    /// take it from the app's shortcuts (the pen's Delete, a marquee's arrows). Esc during a
    /// gesture never reaches here (it cancels). Default: `false`.
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
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

/// The host's state between passes: the gesture in progress and the queued input.
#[derive(Clone, Debug, Default)]
struct HostState {
    /// The tool owns a gesture; its button and the pointer's last sample.
    gesture: Option<(PointerButton, ToolPointer)>,
    queued: Vec<ToolInput>,
    /// Autoscroll's accumulated sub-point remainder.
    scroll_carry: egui::Vec2,
}

/// The on-canvas tool host. Built each pass, shown once with the canvas's response and the tool.
pub struct ToolHost<'a> {
    id_salt: IdSalt,
    snap: Option<&'a SnapTargets>,
    autoscroll: bool,
    pen: Option<&'a super::pointer::PenInput>,
    enabled: bool,
    keys: bool,
}

impl<'a> ToolHost<'a> {
    /// A host for one canvas, keyed by `id_salt` under the `Ui` it is shown in (one per document view);
    /// [`ToolHostResponse::id`] is the resulting id.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), snap: None, autoscroll: true, pen: None, enabled: true, keys: true }
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

    /// The pen reader whose samples fill pressure, tilt, rotation and the eraser end. Without
    /// one every sample is a mouse.
    pub fn pen(mut self, pen: &'a super::pointer::PenInput) -> Self {
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
        todo!()
    }

    /// Queue an automation pointer event for the host `id` (from [`ToolHostResponse::id`]); it
    /// is delivered on the next pass in order, after the OS pointer's events. A `Down` while the
    /// OS pointer holds a gesture is dropped.
    // pc: crates/ui-egui/src/control/dispatch.rs `ui.pointer` → tool_input::tool_event.
    pub fn inject(ctx: &Context, id: Id, input: ToolInput) {
        todo!()
    }

    /// Cancel the gesture in progress (the app switched tools or documents, a modal opened):
    /// the tool's [`cancel`](CanvasTool::cancel) runs on the next `show`.
    // pc: crates/ui-egui/src/state.rs drag = None on tool switch; tool_memory.rs.
    pub fn cancel(ctx: &Context, id: Id) {
        todo!()
    }

    /// Whether the host `id` has a gesture in progress (for the app's tool switch and the
    /// momentary keys: a drag finishes with the tool it started with).
    pub fn in_gesture(ctx: &Context, id: Id) -> bool {
        todo!()
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
        todo!()
    }

    /// `8 / zoom`: the snap tolerance in document units, or `None` with Ctrl held (snapping off
    /// while dragging, Photoshop) unless `ctrl_is_tool_modifier` (the corner-distortion key).
    // pc: crates/ui-egui/src/canvas/snap_ui.rs:76 `tolerance`.
    pub fn snap_tolerance(view: &CanvasView, modifiers: Modifiers, ctrl_is_tool_modifier: bool) -> Option<f64> {
        todo!()
    }

    fn state(ctx: &Context, id: Id) -> HostState {
        todo!()
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
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Ignored
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {}

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {}

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {}

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {}

    fn autoscrolls(&self) -> bool {
        false
    }
}

/// Lets the host's [`Claim`](super::Claim) name the tool as an overlay: the tool's claim id under the canvas.
pub(super) fn tool_claim_id(host: Id) -> Id {
    host.with("tool")
}
