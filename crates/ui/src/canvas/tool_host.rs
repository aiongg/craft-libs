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
//! A tool made of parts (a crop frame's corners and edges, a ruler's ends, a symmetry centre)
//! returns them from [`CanvasTool::parts`]; the host makes them one Tab stop with
//! [`PartFocus`](super::PartFocus), passes Page Up and Down picks back through
//! [`CanvasTool::pick`], gives the tool the keys while that stop has the focus, and draws the focus
//! ring round the picked part. Tools without parts (the default) get no Tab stop of their own.
//!
//! A momentary tool (Space for the Hand, see [`MomentaryKeys`](super::MomentaryKeys)) is the
//! app's: it passes the temporary tool to `show` instead of the selected one; a drag in progress
//! keeps the tool it started with until it ends. An app that switches tools calls
//! [`ToolHost::cancel`] with the old tool first.
//!
//! ```no_run
//! use craft_ui::canvas::{CanvasId, CanvasTool, ToolHost, ToolHostResponse};
//! # fn demo(ui: &mut egui::Ui, view: &craft_ui::canvas::CanvasView, rect: egui::Rect, tool: &mut dyn CanvasTool) {
//! let canvas_id = CanvasId::new(ui, "doc-1");
//! let canvas = canvas_id.interact(ui, rect, egui::Sense::click_and_drag() | egui::Sense::FOCUSABLE);
//! let ToolHostResponse { gesture, scrolled_by, .. } = ToolHost::new(canvas_id).autoscroll(true).show(ui, view, &canvas, tool);
//! # }
//! ```

use egui::{Context, CursorIcon, Event, Id, Key, Modifiers, Painter, PointerButton, Pos2, Response, Ui};
use kurbo::{Point, Rect, Size, Vec2};

use super::canvas_id::CanvasId;
use super::claim::Claim;
use super::cursor::{CursorHost, ToolCursor};
use super::navigate::{Autoscroll, FlickPan, NavigationOptions};
use super::part_focus::{Part, PartFocus};
use super::pointer::{PenInput, PenSample};
use super::snap::{SNAP_PX, SnapGesture, SnapTargets};
use super::view::CanvasView;
use super::{Readout, Turn};
use crate::behavior::{ClaimedKey, KeyClaim};
use crate::edit::CancelReason;
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
    /// How many presses make up the click run the gesture began with: 1 for a lone press, 2 for
    /// the second press within egui's double-click delay and distance
    /// ([`InputOptions`](egui::InputOptions)), and so on. 0 for a sample with no press (a hover).
    pub clicks: u32,
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

/// A tool's keyboard parts for one pass ([`CanvasTool::parts`]): the group the host makes one Tab
/// stop, as [`PartFocus`] names it for AccessKit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToolParts {
    /// The group's accessible name, e.g. "Crop frame", "Ruler" (the tool's words).
    pub group: String,
    /// The group's value for a screen reader, e.g. "1200 × 800, 40, 25, 0°".
    pub value: Option<String>,
    /// The parts in keyboard order (Page Down goes forward), with their screen rects this pass.
    pub parts: Vec<Part>,
}

impl ToolParts {
    /// A group named `group` with `parts`, no value.
    pub fn new(group: impl Into<String>, parts: Vec<Part>) -> Self {
        Self { group: group.into(), value: None, parts }
    }

    /// The group's value for a screen reader.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
}

/// A tool's drawing as one AccessKit object for one pass ([`CanvasTool::node`]): an outline being
/// drawn, a ruler line. Tools on the host get no `Ui`, so the host adds the node for them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToolNode {
    /// The object's accessible name, e.g. "Selection outline", "Ruler" (the tool's words).
    pub name: String,
    /// Its value for a screen reader, e.g. "5 points", "120 px, 30°".
    pub value: Option<String>,
    /// The drawing's bounds in document units; the host maps them to the screen. `None` gives a
    /// node of no size at the canvas centre.
    pub bounds: Option<Rect>,
}

impl ToolNode {
    /// An object named `name`, with no value and no bounds.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), value: None, bounds: None }
    }

    /// The object's value for a screen reader.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// The drawing's bounds in document units.
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }
}

/// The phase of the tool's pointer gesture this pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolGesture {
    /// The press was claimed this pass.
    Begin,
    /// A drag moved this pass.
    Update,
    /// The button came up: the tool committed.
    Commit,
    /// Esc, a lost pointer or a disabled host ended the gesture without a commit.
    Cancel,
}

/// What the host did in a pass.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "apply the scroll to the app's view and read the gesture"]
pub struct ToolHostResponse {
    /// The host's canvas, for [`ToolHost::inject`] and [`ToolHost::cancel`].
    pub id: CanvasId,
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
    /// The canvas the host works on: the key of the per-canvas state of the navigation pieces
    /// ([`ZoomAnimation`](super::ZoomAnimation), [`FlickPan`]).
    pub canvas_id: CanvasId,
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
    /// The tool's parts group ([`CanvasTool::parts`]) has the keyboard, as [`PartFocus`] says:
    /// a tool uses its picked part for keys only while this is true, and otherwise treats them
    /// as aimed at the whole object (or ignores them). Always `false` for a tool without parts
    /// and outside a pass.
    pub parts_focused: bool,
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
    /// Called first in every pass, whatever the pointer does, with `cx.pointer` unset: the tool
    /// takes this pass's app state (its options, the foreground colour) and advances what runs
    /// without the pointer (an animation). A view it asks for ([`ToolContext::set_view`]) is
    /// reported like any other. Default: nothing.
    fn each_pass(&mut self, _cx: &mut ToolContext<'_>) {}

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

    /// The gesture ends without a commit. `reason` says why: [`Escape`](CancelReason::Escape)
    /// for Esc, [`FocusLost`](CancelReason::FocusLost) for a pointer the host lost,
    /// [`Disabled`](CancelReason::Disabled) when the host was disabled, or the app's own reason
    /// for [`ToolHost::cancel`] (a tool switch is [`Removed`](CancelReason::Removed)). The tool
    /// drops its drag state and reports `reason` on the gesture's events.
    fn cancel(&mut self, reason: CancelReason, cx: &mut ToolContext<'_>);

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

    /// Whether a press of `button` with `modifiers` held starts a gesture with this tool. The host
    /// asks at each press with no gesture running, for each button pressed in that pass (primary,
    /// secondary, middle in that order), and begins with the first the tool takes;
    /// [`ToolPointer::button`] then says which one it was. A gesture keeps its button until that
    /// button comes up, whatever happens to the keys meanwhile. A brush takes the primary button
    /// always and the secondary while Alt is held (the resize chord Alt + right-drag), so Alt +
    /// click still paints (reversing the mode). Default: `button == self.button()`, so a tool that
    /// doesn't override it never changes.
    fn takes(&self, button: PointerButton, _modifiers: Modifiers) -> bool {
        button == self.button()
    }

    /// The tool's keyboard parts this pass, or `None` (the default) for a tool without parts.
    /// With parts the host makes the group one Tab stop ([`PartFocus`]), puts each part in the
    /// AccessKit tree, and lets Page Up, Page Down, Home and End pick a part
    /// ([`pick`](Self::pick)); the keys the tool reads through [`key`](Self::key) (arrows,
    /// Delete, Enter) act on [`picked`](Self::picked). Called once per pass after
    /// [`each_pass`](Self::each_pass), with `cx.pointer` unset.
    fn parts(&self, _cx: &ToolContext<'_>) -> Option<ToolParts> {
        None
    }

    /// The part the tool's keys act on, by [`Part::key`]. Default: none.
    fn picked(&self) -> Option<u64> {
        None
    }

    /// The host picked another part: Page Up or Down, Home or End while the group has the
    /// keyboard, the first part when the group gains focus from the keyboard with none picked, or
    /// `None` when the picked part is no longer among the parts. A press the tool claims on a part
    /// also gives the group focus; the tool picks that part itself. Default: nothing.
    fn pick(&mut self, _key: Option<u64>, _cx: &mut ToolContext<'_>) {}

    /// The tool's drawing as one graphics object for screen readers this pass (an outline being
    /// drawn, a ruler line), or `None` (the default) for a tool with nothing to describe. The host
    /// adds it to the AccessKit tree as a child of the canvas node: role `GraphicsObject`,
    /// hover-only, not a Tab stop, over [`ToolNode::bounds`] on the screen, as
    /// [`Marquee`](super::Marquee) describes its ants. Called once per pass after
    /// [`paint`](Self::paint), with the same context. Keyboard parts stay [`parts`](Self::parts).
    fn node(&self, _cx: &ToolContext<'_>) -> Option<ToolNode> {
        None
    }
}

/// A pointer event from automation (PhotoCraft's `ui.pointer`), queued with
/// [`ToolHost::inject`] and delivered on the next pass as if the OS pointer did it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToolInput {
    /// A press of `button`. The host starts nothing when the tool takes no such button with
    /// `modifiers` ([`CanvasTool::takes`]), as for the OS pointer.
    Down {
        button: PointerButton,
        doc: Point,
        pressure: f32,
        tilt: [f32; 2],
        rotation: f32,
        modifiers: Modifiers,
    },
    Move {
        doc: Point,
        pressure: f32,
        tilt: [f32; 2],
        rotation: f32,
        modifiers: Modifiers,
    },
    Up {
        doc: Point,
        modifiers: Modifiers,
    },
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
    /// The view of the last pass, for [`ToolHost::cancel`] outside a pass.
    view: Option<CanvasView>,
    /// Autoscroll's accumulated screen points while the view cannot move yet.
    scroll_carry: egui::Vec2,
    /// The tool asked to keep the keys in the last pass ([`ToolContext::keep_keys`]).
    keep_keys: bool,
}

/// The on-canvas tool host. Built each pass, shown once with the canvas's response and the tool.
pub struct ToolHost<'a> {
    canvas: CanvasId,
    navigation: Option<NavigationOptions>,
    snap: Option<&'a SnapTargets>,
    autoscroll: bool,
    pen: Option<&'a PenInput>,
    enabled: bool,
    keys: bool,
    document: Size,
}

impl<'a> ToolHost<'a> {
    /// A host for the canvas `canvas` (one per document view). The canvas response passed to
    /// [`show`](Self::show) is the one interacted under [`CanvasId::id`].
    pub fn new(canvas: CanvasId) -> Self {
        Self { canvas, navigation: None, snap: None, autoscroll: true, pen: None, enabled: true, keys: true, document: Size::ZERO }
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

    /// The user's navigation options. With them the host keeps a released Hand drag gliding
    /// (Flick Panning) every pass, whichever tool is in use, stopping at the
    /// [`document`](Self::document)'s scroll limits without Overscroll. Without them a flick
    /// stops at the release.
    pub fn navigation(mut self, prefs: NavigationOptions) -> Self {
        self.navigation = Some(prefs);
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
        let id = self.canvas.id();
        debug_assert_eq!(canvas.id, id, "craft-ui: the canvas response must be interacted under CanvasId::id");
        CursorHost::install(&ctx);
        let tokens = Tokens::of(ui);
        let numbers = LocaleNumbers::of(ui);
        let mut run = Run {
            pass: Pass {
                canvas_id: self.canvas,
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
                parts_focused: false,
                cursor: None,
                readout: None,
                keep_keys: false,
            },
            state: {
                let state = Self::state(&ctx, id);
                // Input queued from here on belongs to the next pass.
                ctx.data_mut(|d| d.get_temp_mut_or_default::<HostState>(id).queued.clear());
                state
            },
            resp: ToolHostResponse {
                id: self.canvas,
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
            navigation: self.navigation,
            ctx: ctx.clone(),
            parts: None,
        };
        run.pass.time = ctx.input(|i| i.time);
        run.pass.dt = ctx.input(|i| i.unstable_dt);
        run.pass.modifiers = ctx.input(|i| i.modifiers);
        run.step(ui, self.enabled, self.keys);
        run.finish(ui)
    }

    /// Queue an automation pointer event for the host of `canvas` (from
    /// [`ToolHostResponse::id`]); it is delivered on the next pass in order, after the OS
    /// pointer's events. A `Down` while the OS pointer holds a gesture is dropped.
    // pc: crates/ui-egui/src/control/dispatch.rs `ui.pointer` → tool_input::tool_event.
    pub fn inject(ctx: &Context, canvas: CanvasId, input: ToolInput) {
        ctx.data_mut(|d| d.get_temp_mut_or_default::<HostState>(canvas.id()).queued.push(input));
        ctx.request_repaint();
    }

    /// End the gesture in progress on `canvas` now, without a commit (the app switched tools or
    /// documents, a modal opened): `tool` is the tool that holds the gesture, so call it before
    /// the app replaces it. The tool's [`cancel`](CanvasTool::cancel) runs with the canvas of
    /// the last pass, and the view it asks for (the Hand puts the view back) is returned for the
    /// app to adopt. `None` when no gesture ran, or when the canvas was not shown in the last
    /// pass (nothing to run the tool against; the gesture is dropped).
    // pc: crates/ui-egui/src/state.rs drag = None on tool switch; tool_memory.rs.
    pub fn cancel(ctx: &Context, canvas: CanvasId, tool: &mut dyn CanvasTool, reason: CancelReason) -> Option<CanvasView> {
        let id = canvas.id();
        let (active, view) = ctx.data_mut(|d| {
            let stored = d.get_temp_mut_or_default::<HostState>(id);
            stored.waiting = None;
            (stored.gesture.take(), stored.view)
        });
        let (active, view, response) = (active?, view?, ctx.read_response(id)?);
        let (tokens, numbers) = (crate::theme::current(ctx), LocaleNumbers::of_ctx(ctx));
        let mut pass = Pass {
            canvas_id: canvas,
            canvas: &response,
            snap: None,
            tokens: &tokens,
            numbers: &numbers,
            view,
            view_changed: false,
            modifiers: ctx.input(|i| i.modifiers),
            time: ctx.input(|i| i.time),
            dt: ctx.input(|i| i.unstable_dt),
            caps_lock: CursorHost::caps_lock(ctx),
            parts_focused: false,
            cursor: None,
            readout: None,
            keep_keys: false,
        };
        pass.call(Some(active.last), |cx| tool.cancel(reason, cx));
        pass.view_changed.then_some(pass.view)
    }

    /// Whether the host of `canvas` has a gesture in progress (for the app's tool switch and the
    /// momentary keys: a drag finishes with the tool it started with).
    pub fn in_gesture(ctx: &Context, canvas: CanvasId) -> bool {
        let state = Self::state(ctx, canvas.id());
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
        clicks: 0,
    }
}

/// What the tool and the host share during one pass: the working view (tool calls later in the
/// pass see the earlier ones' [`ToolContext::set_view`]) and what the calls asked for.
struct Pass<'p> {
    canvas_id: CanvasId,
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
    parts_focused: bool,
    cursor: Option<ToolCursor>,
    readout: Option<Readout>,
    keep_keys: bool,
}

impl Pass<'_> {
    fn context(&self, pointer: Option<ToolPointer>) -> ToolContext<'_> {
        let modifiers = pointer.map_or(self.modifiers, |p| p.modifiers);
        ToolContext {
            canvas_id: self.canvas_id,
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
            parts_focused: self.parts_focused,
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
    navigation: Option<NavigationOptions>,
    ctx: Context,
    /// The tool's parts group this pass, when it has parts.
    parts: Option<PartsPass>,
}

/// What the host's [`PartFocus`] group did this pass.
#[derive(Clone, Copy, Debug, Default)]
struct PartsPass {
    /// The group has keyboard focus: keys go to the tool as when the canvas has it.
    focused: bool,
    /// The picked part's screen rect, while the focus came from the keyboard: the host draws
    /// [`PartFocus::ring`] round it after the tool paints.
    ring: Option<egui::Rect>,
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
        let clicks = self.state.gesture.map_or(0, |g| g.last.clicks);
        ToolPointer { button, clicks, ..make_pointer(screen, raw_doc, modifiers, snap, origin, pen, self.pass.time) }
    }

    /// `press` with its place in the click run: it continues the last press's run when it comes
    /// soon and near enough (egui's double-click delay and distance) with the same button, else it
    /// starts a new one.
    /// The count comes from the context's one click run ([`super::claim::click_count`]), which
    /// overlays share.
    fn press_clicks(&self, press: ToolPointer) -> ToolPointer {
        ToolPointer { clicks: super::claim::click_count(&self.ctx, press.button, press.time, press.screen), ..press }
    }

    fn call<R>(&mut self, pointer: Option<ToolPointer>, run: impl FnOnce(&mut dyn CanvasTool, &mut ToolContext<'_>) -> R) -> R {
        let tool = &mut *self.tool;
        self.pass.call(pointer, |cx| run(tool, cx))
    }

    fn report(&mut self, gesture: ToolGesture) {
        let over = |old: Option<ToolGesture>| matches!((old, gesture), (_, ToolGesture::Commit | ToolGesture::Cancel) | (None, _));
        if over(self.resp.gesture) {
            self.resp.gesture = Some(gesture);
        }
    }

    fn begin(&mut self, p: ToolPointer, injected: bool) {
        match self.call(Some(p), |tool, cx| tool.press(&p, cx)) {
            ToolAction::Claimed => {
                self.state.gesture = Some(Active { button: p.button, last: p, origin: p.raw, injected });
                self.report(ToolGesture::Begin);
            }
            ToolAction::Done => self.report(ToolGesture::Commit),
            ToolAction::Ignored => self.resp.ignored_press = Some(p),
        }
    }

    fn drag(&mut self, p: ToolPointer) {
        let Some(active) = self.state.gesture.as_mut() else { return };
        let moved = active.last.screen != p.screen || active.last.doc != p.doc;
        active.last = p;
        self.call(Some(p), |tool, cx| tool.drag(&p, cx));
        if moved || self.resp.scrolled_by.is_some() {
            self.report(ToolGesture::Update);
        }
    }

    fn end(&mut self, p: ToolPointer) {
        if self.state.gesture.take().is_some() {
            self.call(Some(p), |tool, cx| tool.release(&p, cx));
            self.report(ToolGesture::Commit);
        }
    }

    fn cancel(&mut self, reason: CancelReason) {
        self.state.waiting = None;
        if let Some(active) = self.state.gesture.take() {
            self.call(Some(active.last), |tool, cx| tool.cancel(reason, cx));
            self.report(ToolGesture::Cancel);
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

    /// A released Hand drag keeps gliding while no gesture runs, whichever tool is in use.
    fn glide(&mut self) {
        let Some(prefs) = self.navigation.filter(|_| self.state.gesture.is_none()) else { return };
        if FlickPan::advance_view(&self.ctx, self.pass.canvas_id, &mut self.pass.view, &prefs, self.document) {
            self.pass.view_changed = true;
        }
    }

    /// The tool's parts this pass, made one Tab stop: `None` for a tool without parts (no group,
    /// no Tab stop, nothing else changes).
    fn parts_focus(&mut self, ui: &mut Ui, enabled: bool) -> Option<PartsPass> {
        let tool = &*self.tool;
        let parts = tool.parts(&self.pass.context(None))?;
        Some(self.show_parts(ui, parts, enabled))
    }

    /// Shows `parts` as one [`PartFocus`] group over the canvas under the tool's claim id
    /// ([`tool_claim_id`], so a claimed press released on a part gives the group focus), named
    /// `parts.group` with `parts.value`, picked [`CanvasTool::picked`], focusable while `enabled`.
    /// A `pick` in the response goes to [`CanvasTool::pick`] (a [`call`](Self::call), so its
    /// requests apply). Returns whether the group has focus and, while
    /// [`keyboard`](super::PartFocusResponse::keyboard), the rect of the part picked after the
    /// pick.
    fn show_parts(&mut self, ui: &mut Ui, parts: ToolParts, enabled: bool) -> PartsPass {
        // new: tools lead call 33; the overlays call `PartFocus` the same way.
        let id = tool_claim_id(self.resp.id.id());
        let mut focus = PartFocus::new(id, parts.group.as_str()).parts(&parts.parts).picked(self.tool.picked()).focusable(enabled);
        if let Some(value) = parts.value.as_deref() {
            focus = focus.value(value);
        }
        let shown = focus.show(ui, self.pass.canvas);
        if let Some(key) = shown.pick {
            self.call(None, |tool, cx| tool.pick(key, cx));
        }
        let ring = shown.keyboard.then(|| self.tool.picked().and_then(|key| parts.parts.iter().find(|p| p.key == key)).map(|p| p.rect)).flatten();
        PartsPass { focused: shown.focused, ring }
    }

    fn step(&mut self, ui: &mut Ui, enabled: bool, keys: bool) {
        let ctx = self.ctx.clone();
        let canvas = self.pass.canvas;
        let claim = tool_claim_id(self.resp.id.id());
        let kept_keys = self.state.keep_keys;
        if !enabled {
            self.cancel(CancelReason::Disabled);
        }
        self.call(None, |tool, cx| tool.each_pass(cx));
        self.glide();
        // Before the key events are copied: the group takes its Page Up, Page Down, Home and End
        // out of the input, so the tool does not see them a second time.
        self.parts = self.parts_focus(ui, enabled);
        self.pass.parts_focused = self.parts.is_some_and(|p| p.focused);

        let (pos, origin, button, pressed, down, released, key_events) = ctx.input(|i| {
            let button = gesture_button(self.state.gesture.as_ref(), &*self.tool, self.pass.modifiers, |b| i.pointer.button_pressed(b));
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
                button,
                i.pointer.button_pressed(button),
                i.pointer.button_down(button),
                i.pointer.button_released(button),
                key_events,
            )
        });
        let modifiers = self.pass.modifiers;
        let pen = self.pen;

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
                let p = self.press_clicks(p);
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
                self.cancel(CancelReason::FocusLost);
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
        let parts_focused = self.pass.parts_focused;
        let keyboard = enabled && keys && (canvas.has_focus() || parts_focused) && !ctx.text_edit_focused();
        if keyboard {
            // egui drops the focus on Esc and moves it on the arrows unless the widget asks for them.
            let filter = egui::EventFilter { escape: true, horizontal_arrows: true, vertical_arrows: true, ..egui::EventFilter::default() };
            ctx.memory_mut(|m| m.set_focus_lock_filter(canvas.id, filter));
        }
        for key in key_events {
            if enabled && keys && key.pressed && key.key == Key::Escape && self.state.gesture.is_some() {
                self.cancel(CancelReason::Escape);
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
        let own = self.state.gesture.is_none_or(|g| g.injected);
        match input {
            ToolInput::Down { button, doc, pressure, tilt, rotation, modifiers } if enabled && self.state.gesture.is_none() => {
                if taken_button(&*self.tool, modifiers, |b| b == button).is_none() {
                    return;
                }
                let pen = PenSample { pressure, tilt, rotation, eraser: false, time_ms: 0.0 };
                let p = self.sample(Pos2::ZERO, Some(doc), modifiers, Some(pen), button, true);
                let p = self.press_clicks(p);
                self.begin(p, true);
            }
            ToolInput::Move { doc, pressure, tilt, rotation, modifiers } if enabled && own => {
                let pen = PenSample { pressure, tilt, rotation, eraser: false, time_ms: 0.0 };
                let gesture = self.state.gesture.is_some();
                let button = gesture_button(self.state.gesture.as_ref(), &*self.tool, modifiers, |_| true);
                let p = self.sample(Pos2::ZERO, Some(doc), modifiers, Some(pen), button, false);
                if gesture {
                    self.drag(p);
                } else {
                    self.hover(p);
                }
            }
            ToolInput::Up { doc, modifiers } if own && self.state.gesture.is_some() => {
                let button = gesture_button(self.state.gesture.as_ref(), &*self.tool, modifiers, |_| true);
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
        if let Some(rect) = self.parts.and_then(|p| p.ring) {
            PartFocus::ring(&painter, rect);
        }
        if let Some(node) = self.tool.node(&self.pass.context(pointer)) {
            show_node(ui, canvas, &self.pass.view, node);
        }
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
        if self.pass.keep_keys {
            KeyClaim::claim(&ctx, canvas.id, &typed_keys());
        }
        self.resp.view = self.pass.view_changed.then_some(self.pass.view);
        self.state.view = Some(self.pass.view);
        // Automation may have queued input while the tool ran.
        let id = self.resp.id.id();
        let queued = ctx.data(|d| d.get_temp::<HostState>(id)).map(|s| s.queued).unwrap_or_default();
        self.state.queued.extend(queued);
        ctx.data_mut(|d| d.insert_temp(id, self.state));
        self.resp
    }
}

/// The buttons the host offers a press, in order.
const PRESS_BUTTONS: [PointerButton; 3] = [PointerButton::Primary, PointerButton::Secondary, PointerButton::Middle];

/// The first button of `PRESS_BUTTONS` that `tool` takes with `modifiers` and `wanted` accepts.
fn taken_button(tool: &dyn CanvasTool, modifiers: Modifiers, wanted: impl Fn(PointerButton) -> bool) -> Option<PointerButton> {
    PRESS_BUTTONS.into_iter().find(|b| wanted(*b) && tool.takes(*b, modifiers))
}

/// The button the host reads this pass: the button of `gesture` when one runs (a gesture keeps
/// the button it began with, so releasing Alt mid-resize doesn't strand it), else the first
/// button pressed this pass that the tool takes (`pressed`), else the tool's own button.
fn gesture_button(gesture: Option<&Active>, tool: &dyn CanvasTool, modifiers: Modifiers, pressed: impl Fn(PointerButton) -> bool) -> PointerButton {
    gesture.map_or_else(|| taken_button(tool, modifiers, pressed).unwrap_or_else(|| tool.button()), |g| g.button)
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

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {}

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}

    fn autoscrolls(&self) -> bool {
        false
    }
}

/// The keys a tool that kept the keyboard types with: every key without Ctrl or Cmd (Shift and
/// Alt as typed), so the app's chords (Cmd+Z, Cmd+S) still run.
fn typed_keys() -> Vec<ClaimedKey> {
    Key::ALL.iter().map(|key| ClaimedKey::logical(Modifiers::NONE, *key)).collect()
}

/// Adds a tool's [`ToolNode`] to the AccessKit tree for this pass: a hover-only widget under the
/// canvas node (`accessibility_parent(canvas.id)`), role `GraphicsObject`, labelled `node.name`
/// with `node.value` as its current value, over `node.bounds` mapped through `view` and clipped to
/// the canvas (no size at the canvas centre without bounds or off the canvas). Its id comes from
/// the canvas id, so it stays the same from pass to pass.
fn show_node(ui: &mut Ui, canvas: &Response, view: &CanvasView, node: ToolNode) {
    // new: as `Marquee::describe` (`marquee.rs`, `fn describe`).
    let id = canvas.id.with("craft_ui::canvas::ToolNode");
    let corners = node
        .bounds
        .map(|b| [Point::new(b.x0, b.y0), Point::new(b.x1, b.y0), Point::new(b.x1, b.y1), Point::new(b.x0, b.y1)].map(|c| view.to_screen(c)));
    let bounds = corners.map(|c| egui::Rect::from_points(&c).intersect(view.rect));
    let rect = match bounds {
        Some(r) if r.is_positive() => r,
        _ => egui::Rect::from_center_size(view.rect.center(), egui::Vec2::ZERO),
    };
    let nodes = ui.new_child(egui::UiBuilder::new().max_rect(view.rect).accessibility_parent(canvas.id));
    let response = nodes.interact(rect, id, egui::Sense::hover());
    response.widget_info(|| {
        let mut info = egui::WidgetInfo::labeled(egui::accesskit::Role::GraphicsObject, nodes.is_enabled(), &node.name);
        info.current_text_value.clone_from(&node.value);
        info
    });
}

/// Lets the host's [`Claim`](super::Claim) name the tool as an overlay: the tool's claim id under the canvas.
pub(super) fn tool_claim_id(host: Id) -> Id {
    host.with("tool")
}
