//! Navigation over a canvas, as in Photoshop: the wheel, trackpad pinch and twist, the page keys,
//! a Hand drag's flick, autoscroll during a tool drag, and Rotate View with its compass. The
//! user's choices are [`NavigationPrefs`] (Photoshop's Preferences › General, Tools and Enhanced
//! Controls rows).
//!
//! Each piece reads one pass's input and changes a [`CanvasView`] the app copies back into its
//! own view; zooms go through [`zoom`](super::zoom) and clamp to [`ZoomLadder`](super::ZoomLadder).
//!
//! ```no_run
//! use craft_ui::canvas::{CanvasView, navigate::{NavigationPrefs, WheelNavigation}, kurbo::Size};
//! # fn demo(ui: &egui::Ui, canvas: &egui::Response, view: &mut CanvasView, prefs: &NavigationPrefs) {
//! if let Some(action) = WheelNavigation::read(ui.ctx(), canvas, prefs) {
//!     let pointer = canvas.hover_pos().unwrap_or(view.rect.center());
//!     WheelNavigation::apply(view, action, pointer, Size::new(2000.0, 1500.0), prefs, ui.ctx().pixels_per_point());
//! }
//! # }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Context, Event, Id, Key, Modifiers, Painter, Pos2, RawInput, Rect as ScreenRect, Response, Ui, Vec2 as ScreenVec};
use kurbo::Size;

use super::cursor::ToolCursor;
use super::tool_host::{CanvasTool, ToolAction, ToolContext, ToolKey, ToolPointer};
use super::view::CanvasView;
use crate::theme::UiTokens;

/// The navigation preferences, with Photoshop's defaults. The app stores them (craft-settings)
/// and shows them in its Preferences; every navigation piece reads them from here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationPrefs {
    /// General › Zoom with Scroll Wheel: the wheel zooms and Alt + wheel scrolls. Off.
    pub zoom_with_wheel: bool,
    /// General › Animated Zoom: wheel and command zooms ease, and a held Zoom tool press zooms
    /// continuously. On.
    pub animated_zoom: bool,
    /// General › Zoom Resizes Windows (also the Zoom tool's Resize Windows to Fit). Off.
    pub zoom_resizes_windows: bool,
    /// General › Zoom Clicked Point to Center: a Zoom tool click centres the clicked point. Off.
    pub zoom_clicked_point_to_center: bool,
    /// Tools › Enable Flick Panning: a Hand drag released while moving glides on. On.
    pub flick_panning: bool,
    /// Tools › Overscroll: the view may scroll the document past the canvas edge. Off.
    pub overscroll: bool,
    /// The Zoom tool's Scrubby Zoom option: a Zoom tool drag zooms continuously instead of
    /// drawing a rectangle. On.
    pub scrubby_zoom: bool,
    /// Enhanced Controls › Zoom with Trackpad Pinch. On.
    pub pinch_zooms: bool,
    /// Enhanced Controls › Rotate View with Trackpad. Off.
    pub trackpad_rotate: bool,
    /// Enhanced Controls › Enable Gestures: a touch screen's two-finger pinch and twist. On.
    pub touch_gestures: bool,
}

impl Default for NavigationPrefs {
    // pc: crates/engine/src/prefs.rs:383 `enable_flick_panning`, :392 `overscroll`, :922 Enhanced Controls defaults.
    fn default() -> Self {
        Self {
            zoom_with_wheel: false,
            animated_zoom: true,
            zoom_resizes_windows: false,
            zoom_clicked_point_to_center: false,
            flick_panning: true,
            overscroll: false,
            scrubby_zoom: true,
            pinch_zooms: true,
            trackpad_rotate: false,
            touch_gestures: true,
        }
    }
}

/// What the wheel or a pinch does to the view this pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelAction {
    /// Multiply the zoom by this factor about the pointer (×1.1 a notch).
    Zoom(f64),
    /// Step this many ladder rungs (positive in, negative out).
    SnapZoom(i32),
    /// Move the image by this many screen points (egui's content direction).
    Pan(ScreenVec),
}

/// One pass of wheel input over the canvas, read from egui's input by [`WheelNavigation::read`]
/// or built by a test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelInput {
    /// Smoothed scroll delta (points, content direction): what panning uses, and what an eased
    /// wheel zoom follows.
    pub scroll: ScreenVec,
    /// This pass's wheel events unsmoothed: what an instant zoom and the snap zoom use.
    pub raw: ScreenVec,
    /// egui's pinch factor (1: none).
    pub zoom_delta: f32,
    /// Alt was held for the wheel gesture in progress (kept until the next wheel event, so a
    /// release during the smoothing tail does not turn it into a pan).
    pub alt: bool,
    /// Shift was held for the gesture.
    pub shift: bool,
    /// Points per wheel notch (egui's `line_scroll_speed`).
    pub notch: f32,
}

/// Mouse wheel and trackpad navigation, as in Photoshop:
///
/// - The wheel scrolls up and down; Shift or Cmd/Ctrl + wheel scrolls sideways.
/// - Alt + wheel zooms about the pointer, ×1.1 a notch; Shift + Alt + wheel steps the ladder.
/// - Zoom with Scroll Wheel swaps them: the wheel zooms, Alt + wheel scrolls, Shift + wheel steps
///   the ladder.
/// - A trackpad pinch zooms about the pointer (Zoom with Trackpad Pinch); a touch screen's pinch
///   needs Enable Gestures too.
/// - Animated Zoom eases a notch's ×1.1 over several passes (the factors multiply to ×1.1);
///   off, a notch is one instant step.
pub struct WheelNavigation;

impl WheelNavigation {
    /// The zoom factor of one wheel notch.
    pub const NOTCH: f64 = 1.1;

    /// The view change for one pass of `input` under `prefs`, if any.
    // pc: crates/ui-egui/src/canvas/wheel_nav.rs:85 `classify`.
    pub fn classify(input: WheelInput, prefs: &NavigationPrefs) -> Option<WheelAction> {
        todo!()
    }

    /// This pass's wheel input over `canvas` (hovered, no popup above it) classified under
    /// `prefs`, or `None`. Consumes the scroll so a parent scroll area does not also move.
    // pc: crates/ui-egui/src/canvas/wheel_nav.rs:338 `read`, :350 `read_with`, :502 `wheel_events`,
    // :528 `pinch_factor`, :245 `ease_zoom_delta`.
    pub fn read(ctx: &Context, canvas: &Response, prefs: &NavigationPrefs) -> Option<WheelAction> {
        todo!()
    }

    /// Apply `action` to `view` with the pointer at `pointer`: zooms anchor at
    /// [`zoom::wheel_anchor`](super::zoom::wheel_anchor) and clamp to the ladder's range; a
    /// pan moves the view (within the scroll limits unless `prefs.overscroll`). At a zoom limit
    /// the view stays put, as in Photoshop: a notch past 12800 % neither zooms nor slides the
    /// image toward the pointer. Returns whether the view changed.
    // pc: crates/ui-egui/src/canvas.rs:588 (`Zoom`: clamp, about `wheel_anchor`), :597 (`SnapZoom`: n ladder steps).
    pub fn apply(view: &mut CanvasView, action: WheelAction, pointer: Pos2, size: Size, prefs: &NavigationPrefs, pixels_per_point: f32) -> bool {
        todo!()
    }

    /// Set egui's wheel modifiers once at startup: Cmd/Ctrl + wheel scrolls sideways like Shift +
    /// wheel instead of zooming, everywhere in the app, so only a real pinch reaches
    /// `zoom_delta`.
    // pc: crates/ui-egui/src/canvas/wheel_nav.rs:128 `configure`.
    pub fn configure(ctx: &Context) {
        todo!()
    }

    /// Turn Windows' (and some Wayland systems') Ctrl + wheel fractions of a notch back into a
    /// pinch before egui sees them, so Ctrl + a notched wheel scrolls sideways and a precision
    /// touchpad pinch still zooms. Call from the app's `raw_input_hook`.
    // pc: crates/ui-egui/src/canvas/wheel_nav.rs:170 `fold_legacy_pinch`.
    pub fn fold_legacy_pinch(ctx: &Context, raw: &mut RawInput) {
        todo!()
    }
}

/// What a page key does to the view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageKey {
    /// Scroll by this many screen points (positive: show what is below or to the right).
    Scroll(ScreenVec),
    /// Show the document's top-left corner (Home) or bottom-right corner (End).
    Corner { end: bool },
}

impl PageKey {
    /// Points a Shift + page key scrolls.
    pub const SMALL_STEP: f32 = 10.0;

    /// Photoshop's page keys: Page Up and Page Down a canvas height up or down, Cmd/Ctrl turns
    /// them sideways (Page Up left, Page Down right), Shift makes the step
    /// [`SMALL_STEP`](Self::SMALL_STEP); Home and End show the corners. `None` for other keys,
    /// with Alt, or Shift on Home and End. `canvas` is the canvas size in points.
    // pc: crates/ui-egui/src/input/page_keys.rs:24 `page_key`.
    pub fn of(key: Key, modifiers: Modifiers, canvas: ScreenVec) -> Option<PageKey> {
        todo!()
    }

    /// Consume this pass's page keys while the canvas has the keyboard (no text field focused,
    /// no binding of the app's took them) and apply them to `view` within the scroll limits.
    /// Returns whether one ran.
    // pc: crates/ui-egui/src/input/page_keys.rs:44 `handle`.
    pub fn handle(ctx: &Context, view: &mut CanvasView, size: Size, overscroll: bool) -> bool {
        todo!()
    }

    /// Apply this key to `view` for a document of `size`: a scroll within the limits, or the
    /// corner at the canvas's matching corner. The zoom stays.
    pub fn apply(self, view: &mut CanvasView, size: Size, overscroll: bool) -> bool {
        todo!()
    }
}

/// Move `view` by `delta` screen points (a Hand drag, a scroll): the document follows the
/// pointer, in any rotation or mirror.
pub fn pan(view: &mut CanvasView, delta: ScreenVec) {
    todo!()
}

/// Flick panning: releasing a Hand drag (the selected Hand, the Space Hand or a middle-button
/// drag) while the pointer still moves keeps the view gliding that way, slowing to a stop. A
/// click, a new drag or a key stops it. Kept by the canvas in egui memory per view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlickPan {
    samples: Vec<(f64, ScreenVec)>,
    velocity: ScreenVec,
}

impl FlickPan {
    /// Feed one pass of a Hand drag (`dragging`, its `delta` in points) at `time` with the
    /// preference `enabled`; returns the glide to apply to the view this pass after the
    /// release, and requests a repaint while it glides. State under `id`.
    // pc: crates/ui-egui/src/prefs/flick_pan.rs `frame`, `Flick::{track, release, step}`.
    pub fn frame(ctx: &Context, id: Id, dragging: bool, delta: ScreenVec, enabled: bool) -> Option<ScreenVec> {
        todo!()
    }

    /// Stop a glide (the view hit a limit, a click).
    // pc: crates/ui-egui/src/prefs/flick_pan.rs:99 `stop`.
    pub fn stop(ctx: &Context, id: Id) {
        todo!()
    }
}

/// Touchpad momentum where the platform leaves it to the app (Wayland, which reports a lift as
/// the end of the scroll): a two-finger swipe keeps gliding after the fingers lift. Off on
/// macOS, which sends its own momentum, and on the web.
#[derive(Clone, Debug, PartialEq)]
pub struct TouchpadGlide {
    enabled: bool,
    samples: Vec<(f64, ScreenVec)>,
    velocity: ScreenVec,
}

impl TouchpadGlide {
    /// A glide that runs where the platform needs it.
    // pdfcraft: crates/ui-egui/src/canvas/fling.rs:43 `Default`.
    pub fn new() -> Self {
        todo!()
    }

    /// Read one pass's wheel events at `now`; any wheel input stops a glide, a touchpad
    /// gesture's end may start one. `glide` is false where the view must not glide this pass
    /// (a zoom, a dialog).
    // pdfcraft: crates/ui-egui/src/canvas/fling.rs:65 `read`.
    pub fn read(&mut self, events: &[Event], now: f64, glide: bool) {
        todo!()
    }

    /// The scroll this pass at `now`, in points; zero once stopped.
    // pdfcraft: crates/ui-egui/src/canvas/fling.rs:124 `step`.
    pub fn step(&mut self, now: f64) -> ScreenVec {
        todo!()
    }

    /// Whether the view moved last pass; two passes without moving (against an edge) stop the
    /// glide.
    // pdfcraft: crates/ui-egui/src/canvas/fling.rs:145 `moved`.
    pub fn moved(&mut self, moved: bool) {
        todo!()
    }
}

impl Default for TouchpadGlide {
    fn default() -> Self {
        Self::new()
    }
}

/// Autoscroll while a tool drag runs past the canvas edge (marquee, lasso, brush, crop): the view
/// scrolls toward the pointer, faster the further past the edge, and stops at the scroll limits
/// (an image that fits is never scrolled into empty pasteboard). The tool host runs it for tools
/// whose [`autoscrolls`](super::CanvasTool::autoscrolls) is true.
pub struct Autoscroll;

impl Autoscroll {
    /// Slowest and fastest scroll, points per second.
    pub const MIN_SPEED: f32 = 120.0;
    pub const MAX_SPEED: f32 = 2400.0;
    /// Extra speed per point the pointer is past the edge.
    pub const GAIN: f32 = 12.0;
    /// A long frame scrolls at most this many seconds' worth.
    pub const MAX_DT: f32 = 0.05;

    /// The scroll this pass, in screen points, for a drag with the pointer at `pointer` over a
    /// canvas `rect`: zero inside it, each axis toward the pointer.
    // pc: crates/ui-egui/src/canvas/canvas_autoscroll.rs:21 `edge_step`.
    pub fn edge_step(pointer: Pos2, rect: ScreenRect, dt: f32) -> ScreenVec {
        todo!()
    }

    /// Move `view` by `step` screen points, never past the image's on-screen bounds in the
    /// direction of travel (a view already past a limit is not pulled back). Returns the move
    /// in document units, or `None` when the view did not move.
    // pc: crates/ui-egui/src/canvas/canvas_autoscroll.rs:42 `scroll`.
    pub fn scroll(view: &mut CanvasView, step: ScreenVec, size: Size) -> Option<kurbo::Vec2> {
        todo!()
    }
}

/// The Rotate View tool's drag: the image turns about the view centre following the pointer
/// (clockwise increases the angle; mirrored in a flipped view so it stays under the pointer);
/// Shift snaps to [`SNAP_DEG`](Self::SNAP_DEG). Esc, a double-click, Reset View or the compass
/// put the angle back to 0. The view's rotation is not document history.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotateView {
    pivot: Pos2,
    start_pointer: f64,
    start_rotation: f64,
}

impl RotateView {
    /// Degrees Shift snaps to.
    pub const SNAP_DEG: f64 = 15.0;
    /// The compass's side and its margin from the canvas's top-right corner, in points.
    pub const COMPASS: f32 = 36.0;
    pub const COMPASS_MARGIN: f32 = 10.0;

    /// A drag pressed at `screen` on `view`.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:186 `drag` (b.started).
    pub fn begin(view: &CanvasView, screen: Pos2) -> Self {
        todo!()
    }

    /// The rotation for the pointer at `pointer` (Shift: `snap`), in `(-180, 180]`.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:186 `drag`.
    pub fn drag(&self, pointer: Pos2, snap: bool, flip_x: bool) -> f64 {
        todo!()
    }

    /// `a` degrees wrapped into `(-180, 180]`; not a finite number gives 0.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:42 `wrap_deg`.
    pub fn wrap_deg(a: f64) -> f64 {
        todo!()
    }

    /// The nearest multiple of `step` degrees, wrapped; a step that is not positive is ignored.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:56 `snap_deg`.
    pub fn snap_deg(a: f64, step: f64) -> f64 {
        todo!()
    }

    /// This pass's trackpad twist in degrees, when Rotate View with Trackpad is on: the
    /// trackpad's own rotate events always, a touch screen's two-finger twist only with Enable
    /// Gestures.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:142 `trackpad_delta`, :124 `apply_trackpad`.
    pub fn trackpad_delta(ctx: &Context, prefs: &NavigationPrefs) -> Option<f64> {
        todo!()
    }

    /// The compass's rect in the canvas's top-right corner.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:217 `compass_rect`.
    pub fn compass_rect(canvas: ScreenRect) -> ScreenRect {
        todo!()
    }

    /// Draw the compass (north is the document's up) while the view is turned (more than 0.05°)
    /// and sense it: a click resets the angle. Returns whether it reset, and whether the pointer
    /// is over it (no rotate drag starts there). An AccessKit button named
    /// [`NavigationLabels::reset_rotation`] from the labels installed in the context.
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:230 `overlay`, :224 `compass_visible`.
    pub fn compass(ui: &mut Ui, view: &CanvasView, tokens: &UiTokens) -> CompassResponse {
        todo!()
    }
}

/// What the Rotate View compass did this pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompassResponse {
    /// It was clicked: the app sets the rotation to 0.
    pub reset: bool,
    /// The pointer is over it.
    pub hovered: bool,
}

crate::words! {
    /// The words of the navigation controls on the canvas.
    #[non_exhaustive]
    pub struct NavigationLabels {
        /// The Rotate View compass's AccessKit name and tooltip.
        reset_rotation: "navigate.resetRotation" = "Reset Rotation";
    }
}

/// The Hand tool: a drag pans the view with the document under the pointer (the closed hand
/// while dragging, the open hand otherwise); a release while still moving glides on with
/// [`FlickPan`] when Flick Panning is on; Esc during the drag puts the view back. It does not
/// autoscroll. The tool the Space key holds ([`MomentaryKeys`](super::MomentaryKeys)).
#[derive(Clone, Debug, Default)]
pub struct HandTool {
    /// The view and pointer when the drag began.
    start: Option<(CanvasView, Pos2)>,
    /// The user's navigation preferences (Flick Panning, Overscroll), set by the app each pass.
    pub prefs: NavigationPrefs,
}

impl CanvasTool for HandTool {
    // pc: crates/ui-egui/src/canvas/tool_cursor.rs (Hand open/closed).
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        todo!()
    }

    // pc: crates/ui-egui/src/canvas.rs:1930 (hand_pan, flick_pan::frame).
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {}

    fn autoscrolls(&self) -> bool {
        false
    }
}

/// The Zoom tool: a click steps the ladder about the clicked point (Alt: out;
/// [`zoom_clicked_point_to_center`](NavigationPrefs::zoom_clicked_point_to_center) centres it),
/// a press held still zooms continuously ([`HoldZoom`](super::HoldZoom), Animated Zoom only), a
/// drag scrubs or draws a zoom rectangle ([`ZoomDrag`](super::ZoomDrag), Scrubby Zoom). The cursor
/// is the magnifier, with a minus while Alt is held or `out` is set (Zoom Out held by
/// Alt+Space). At a zoom limit the view stays put.
#[derive(Clone, Debug)]
pub struct ZoomTool {
    /// Zoom out instead of in (Alt+Space held); Alt flips it.
    pub out: bool,
    /// The document's size in document units (the ladder's lower limit).
    pub size: Size,
    pub prefs: NavigationPrefs,
    hold: Option<super::HoldZoom>,
    drag: Option<super::ZoomDrag>,
}

impl ZoomTool {
    pub fn new(size: Size, prefs: NavigationPrefs) -> Self {
        Self { out: false, size, prefs, hold: None, drag: None }
    }
}

impl CanvasTool for ZoomTool {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        todo!()
    }

    // pc: crates/ui-egui/src/canvas.rs:697 (click step), :1138; zoom_hold.rs:65 `update`; tools/zoom_tool.rs:55 `drag`.
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    /// The hold advances on hovers too, so a press held perfectly still keeps zooming.
    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        todo!()
    }

    /// The zoom rectangle while drawn.
    // pc: crates/ui-egui/src/tools/zoom_tool.rs:105 `draw`.
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        todo!()
    }

    fn autoscrolls(&self) -> bool {
        false
    }
}

/// The Rotate View tool: a drag turns the view about the canvas centre ([`RotateView`]); Esc or a
/// double-click resets the angle to 0. The cursor is the rotate hand.
#[derive(Clone, Debug, Default)]
pub struct RotateViewTool {
    drag: Option<(RotateView, CanvasView)>,
}

impl CanvasTool for RotateViewTool {
    // pc: crates/ui-egui/src/canvas/rotate_view.rs:268 `cursor`.
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        todo!()
    }

    // pc: crates/ui-egui/src/canvas/rotate_view.rs:186 `drag`.
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        todo!()
    }

    /// Esc puts the angle back to 0 (not during a drag: that cancels it).
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {}

    fn autoscrolls(&self) -> bool {
        false
    }
}
