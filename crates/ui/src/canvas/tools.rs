//! The view tools (Hand, Zoom, Rotate View) and the view commands.
//!
//! A view tool never writes the app's view. It computes the view the gesture asks for from the canvas's
//! [`CanvasSpace`](super::CanvasSpace) and leaves it as a [`ViewChange`]: after [`ToolHost::run`](super::ToolHost::run)
//! the app takes it with the tool's `take_change` and copies it into its own view (through a
//! [`ZoomAnimation`](super::ZoomAnimation) when the change is eased). Every pass whose change was not taken is
//! replaced by the next one, so the app takes it every pass.

use super::navigate::{FlickPan, NavigationOptions, pan};
use super::rotate::ViewTurn;
use super::zoom_motion::{HoldZoom, ZoomDrag};
use super::{CanvasSpace, CanvasTool, ToolAction, ToolContext, ToolKey, ToolPath, ToolPointer};
use crate::primitives::{Draw, RuleKind};
use craft_behavior::canvas::zoom;
use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{CancelReason, CanvasView, Cursor, PaintedCursor, ZoomLadder};
use craft_keymap::Mods;
use egui::Pos2;

/// The rotation step of the Rotate View commands, in degrees.
const ROTATE_STEP_DEG: f64 = 15.0;

/// The view commands: the keyboard path of the view tools and of the navigation menu. [`apply`](Self::apply) is the
/// one place that says what each does to a view; the app binds chords and runs it from the fired command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, craft_ui_macros::CommandSet)]
#[command(prefix = "canvas.view")]
#[non_exhaustive]
pub enum ViewCommand {
    /// Zoom in to the next step of the zoom ladder about the view's centre.
    #[command(label = "Zoom In")]
    ZoomIn,
    /// Zoom out to the previous step of the zoom ladder about the view's centre.
    #[command(label = "Zoom Out")]
    ZoomOut,
    /// Show the whole document.
    #[command(label = "Fit on Screen")]
    FitOnScreen,
    /// Zoom until the document covers the canvas.
    #[command(label = "Fill Screen")]
    FillScreen,
    /// Turn the view clockwise by 15 degrees.
    #[command(label = "Rotate View Clockwise")]
    RotateClockwise,
    /// Turn the view counterclockwise by 15 degrees.
    #[command(label = "Rotate View Counterclockwise")]
    RotateCounterclockwise,
    /// Turn the view back to upright.
    #[command(label = "Reset Rotation")]
    ResetRotation,
}

impl ViewCommand {
    /// Applies the command to `view` for a document of `size` at `pixels_per_point`. Returns whether the view
    /// changed.
    pub fn apply(self, view: &mut CanvasView, size: Size, pixels_per_point: f32) -> bool {
        let before = *view;
        match self {
            ViewCommand::ZoomIn => {
                zoom::step(view, 1, size, pixels_per_point);
            }
            ViewCommand::ZoomOut => {
                zoom::step(view, -1, size, pixels_per_point);
            }
            ViewCommand::FitOnScreen => zoom::fit(view, size, pixels_per_point),
            ViewCommand::FillScreen => zoom::fill(view, size, pixels_per_point),
            ViewCommand::RotateClockwise => view.rotation = ViewTurn::wrap_deg(view.rotation + ROTATE_STEP_DEG),
            ViewCommand::RotateCounterclockwise => view.rotation = ViewTurn::wrap_deg(view.rotation - ROTATE_STEP_DEG),
            ViewCommand::ResetRotation => view.rotation = 0.0,
        }
        *view != before
    }
}

/// The view a tool's gesture asks the app to show.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ViewChange {
    /// The view to show.
    pub view: CanvasView,
    /// A document point and the screen point it stays at while the change eases, for a zoom about the pointer.
    pub anchor: Option<(Point, Pos2)>,
    /// The change eases ([`ZoomAnimation`](super::ZoomAnimation)) instead of jumping.
    pub eased: bool,
}

impl ViewChange {
    /// A change that jumps to `view`.
    pub const fn to(view: CanvasView) -> Self {
        ViewChange { view, anchor: None, eased: false }
    }
}

/// What a view tool has left for the app.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Outbox {
    pending: Option<ViewChange>,
}

impl Outbox {
    fn set(&mut self, change: ViewChange) {
        self.pending = Some(change);
    }

    fn take(&mut self) -> Option<ViewChange> {
        self.pending.take()
    }

    /// The view as this pass has it: the change left already, else the canvas's.
    fn view(&self, space: &CanvasSpace) -> CanvasView {
        self.pending.map_or(space.view, |change| change.view)
    }
}

/// The Hand tool: a drag pans the view with the document under the pointer; a release while still moving glides on
/// with [`FlickPan`] when Flick Panning is on; Escape during the drag puts the view back. It does not autoscroll.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct HandTool {
    /// The user's navigation choices (Flick Panning).
    pub options: NavigationOptions,
    /// The view and pointer when the drag began.
    start: Option<(CanvasView, Pos2)>,
    /// The pointer at the previous drag pass: a drag step is the flick's input.
    last: Pos2,
    flick: FlickPan,
    outbox: Outbox,
}

impl HandTool {
    /// A Hand tool under `options`.
    pub fn new(options: NavigationOptions) -> Self {
        Self { options, start: None, last: Pos2::ZERO, flick: FlickPan::default(), outbox: Outbox::default() }
    }

    /// The change this pass asks for, once. With Overscroll off the app clamps it with the canvas's scroll limits and
    /// calls [`stop_glide`](Self::stop_glide) when the clamp moved it.
    pub fn take_change(&mut self) -> Option<ViewChange> {
        self.outbox.take()
    }

    /// Ends a glide: the view hit a scroll limit.
    pub fn stop_glide(&mut self) {
        self.flick.stop();
    }

    /// The glide of a flick released with this tool, for the pass at `now` and the app's `view`, when the tool is not
    /// the one run this pass (a flick released with the Space Hand keeps going after Space comes up).
    pub fn glide(&mut self, view: &CanvasView, now: f64) -> Option<ViewChange> {
        let step = self.flick.advance(now)?;
        let mut next = *view;
        pan(&mut next, step);
        Some(ViewChange::to(next))
    }
}

impl CanvasTool for HandTool {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        if self.start.is_some() || !self.flick.gliding() {
            return;
        }
        let Some(space) = cx.space() else { return };
        let view = self.outbox.view(&space);
        if let Some(change) = self.glide(&view, cx.time) {
            self.outbox.set(change);
        }
        if self.flick.gliding() {
            cx.request_repaint();
        }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        self.flick.stop();
        self.start = cx.space().map(|space| (space.view, pointer.pos));
        self.last = pointer.pos;
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let Some((view, from)) = self.start else { return };
        let delta = pointer.pos - self.last;
        self.last = pointer.pos;
        let mut next = view;
        pan(&mut next, pointer.pos - from);
        self.outbox.set(ViewChange::to(next));
        self.flick.drag(pointer.time, delta);
        if delta != egui::Vec2::ZERO {
            cx.request_repaint();
        }
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let Some((view, from)) = self.start.take() else { return };
        let mut next = view;
        pan(&mut next, pointer.pos - from);
        // Letting go while the pointer still moves starts the glide, whose first step is this pass's.
        self.flick.release(pointer.time, self.options.flick_panning);
        if let Some(change) = self.glide(&next, pointer.time) {
            next = change.view;
        }
        if self.flick.gliding() {
            cx.request_repaint();
        }
        self.outbox.set(ViewChange::to(next));
    }

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {
        if let Some((view, _)) = self.start.take() {
            self.outbox.set(ViewChange::to(view));
        }
        self.flick.stop();
    }

    fn key(&mut self, _key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        self.flick.stop();
        false
    }

    fn paint(&self, _draw: &Draw<'_>, _cx: &ToolContext<'_>) {}

    fn cursor(&self, _cx: &ToolContext<'_>) -> Cursor {
        PaintedCursor::Hand.into()
    }

    fn autoscrolls(&self) -> bool {
        false
    }

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the arrow keys and the page keys scroll the view")
    }
}

/// The Zoom tool: a click steps the ladder about the clicked point (Alt: out; Zoom Clicked Point to Center centres
/// it), a press held still zooms continuously ([`HoldZoom`], Animated Zoom only), a drag scrubs or draws a zoom
/// rectangle ([`ZoomDrag`], Scrubby Zoom). The cursor is the magnifier, with a minus while Alt is held or `out` is
/// set (Zoom Out held with Alt+Space). At a zoom limit the view stays put.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ZoomTool {
    /// Zoom out instead of in (Alt+Space held); Alt flips it.
    pub out: bool,
    /// The document's size in document units (the ladder's lower limit).
    pub size: Size,
    /// The navigation choices the tool reads.
    pub options: NavigationOptions,
    hold: Option<HoldZoom>,
    drag: Option<ZoomDrag>,
    /// The view, time and screen point of the press in progress.
    press: Option<(CanvasView, f64, Pos2)>,
    /// The pointer at the last drag sample.
    last: Pos2,
    outbox: Outbox,
}

impl ZoomTool {
    /// A Zoom tool for a document of `size` under `options`.
    pub fn new(size: Size, options: NavigationOptions) -> Self {
        Self { out: false, size, options, hold: None, drag: None, press: None, last: Pos2::ZERO, outbox: Outbox::default() }
    }

    /// The change this pass asks for, once.
    pub fn take_change(&mut self) -> Option<ViewChange> {
        self.outbox.take()
    }

    /// One pass of a press in progress, with the pointer at `at`: the hold zooms once past its delay (and then the
    /// drag has nothing to do), else the drag scrubs or moves the rectangle's corner.
    fn advance(&mut self, at: Pos2, mods: Mods, time: f64, cx: &mut ToolContext<'_>) {
        let (Some(drag), Some(space)) = (self.drag.as_mut(), cx.space()) else { return };
        let ppp = space.pixels_per_point;
        let out = self.out != mods.alt;
        let mut view = self.outbox.view(&space);
        let before = view;
        let mut waiting = false;
        let held = match self.hold.as_mut() {
            Some(hold) => {
                let held = hold.advance(&mut view, at, time, out, self.size, ppp);
                // The press asks for passes while it waits out the delay and while it zooms.
                waiting = held || hold.waiting(time);
                held
            }
            None => false,
        };
        if !held {
            drag.drag(&mut view, at, self.size, ppp);
        }
        if waiting {
            cx.request_repaint();
        }
        if view != before {
            self.outbox.set(ViewChange::to(view));
        }
    }
}

/// The view after a Zoom tool click at `screen`: one ladder rung in (or `out`) about the point, or centred on it with
/// Zoom Clicked Point to Center. `None` at a zoom limit.
fn click_step(view: &CanvasView, screen: Pos2, out: bool, size: Size, options: &NavigationOptions, pixels_per_point: f32) -> Option<CanvasView> {
    let current = zoom::device(view, pixels_per_point);
    let next = ZoomLadder::step(current, if out { -1 } else { 1 }, size);
    if !next.is_finite() || (next / current - 1.0).abs() <= 1e-9 {
        return None;
    }
    let mut stepped = *view;
    zoom::about(&mut stepped, screen, next, options.zoom_clicked_point_to_center, pixels_per_point);
    Some(stepped)
}

impl CanvasTool for ZoomTool {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // The hold advances without pointer movement, so a press held perfectly still keeps zooming.
        if self.press.is_some() {
            let (at, mods, time) = (self.last, cx.mods, cx.time);
            self.advance(at, mods, time, cx);
        }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        let Some(space) = cx.space() else {
            return ToolAction::Claimed;
        };
        let view = self.outbox.view(&space);
        self.press = Some((view, pointer.time, pointer.pos));
        self.last = pointer.pos;
        self.hold = self.options.animated_zoom.then(|| HoldZoom::press(pointer.pos, view.to_doc(pointer.pos), pointer.time));
        self.drag = Some(ZoomDrag::begin(&view, pointer.pos, self.options.scrubby_zoom, space.pixels_per_point));
        if self.hold.is_some() {
            cx.request_repaint();
        }
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.last = pointer.pos;
        self.advance(pointer.pos, pointer.mods, pointer.time, cx);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let (Some((start, _, anchor)), Some(drag), Some(space)) = (self.press.take(), self.drag.take(), cx.space()) else { return };
        let hold = self.hold.take();
        let ppp = space.pixels_per_point;
        let out = self.out != pointer.mods.alt;
        // A hold that zoomed has already changed the view; the release is no click.
        if hold.is_some_and(|hold| hold.zooming()) {
            return;
        }
        if drag.is_click(pointer.pos) {
            // A click steps from the view the press found, not one a few points of scrub have moved.
            match click_step(&start, anchor, out, self.size, &self.options, ppp) {
                Some(next) => {
                    let anchored = (!self.options.zoom_clicked_point_to_center).then(|| (start.to_doc(anchor), anchor));
                    self.outbox.set(ViewChange { view: next, anchor: anchored, eased: self.options.animated_zoom });
                }
                None => self.outbox.set(ViewChange::to(start)),
            }
            return;
        }
        let mut view = self.outbox.view(&space);
        let mut drag = drag;
        drag.drag(&mut view, pointer.pos, self.size, ppp);
        drag.release(&mut view, out, self.size, ppp);
        self.outbox.set(ViewChange::to(view));
    }

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {
        self.hold = None;
        self.drag = None;
        if let Some((view, _, _)) = self.press.take() {
            self.outbox.set(ViewChange::to(view));
        }
    }

    /// The zoom rectangle while drawn.
    fn paint(&self, draw: &Draw<'_>, _cx: &ToolContext<'_>) {
        let Some(rect) = self.drag.as_ref().and_then(ZoomDrag::rect) else { return };
        let corners = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()];
        for (at, from) in corners.iter().enumerate() {
            draw.rule(RuleKind::Edge, *from, corners[(at + 1) % corners.len()]);
        }
    }

    fn cursor(&self, cx: &ToolContext<'_>) -> Cursor {
        PaintedCursor::zoom(self.out != cx.mods.alt).into()
    }

    fn autoscrolls(&self) -> bool {
        false
    }

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Zoom In and Zoom Out commands; the zoom rectangle has no numeric form")
    }
}

/// The Rotate View tool: a drag turns the view about the canvas centre ([`ViewTurn`]); a double click or Escape (with
/// no drag running) resets the angle to 0.
#[derive(Clone, Debug, Default)]
pub struct RotateViewTool {
    drag: Option<(ViewTurn, CanvasView)>,
    outbox: Outbox,
}

impl RotateViewTool {
    /// The change this pass asks for, once.
    pub fn take_change(&mut self) -> Option<ViewChange> {
        self.outbox.take()
    }
}

impl CanvasTool for RotateViewTool {
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        self.drag = cx.space().map(|space| (ViewTurn::begin(&space.view, pointer.pos), space.view));
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        let Some((turn, start)) = &self.drag else { return };
        let mut view = *start;
        view.rotation = turn.drag(pointer.pos, pointer.mods.shift, start.flip_x);
        self.outbox.set(ViewChange::to(view));
    }

    fn release(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        let Some((turn, start)) = self.drag.take() else { return };
        let mut view = start;
        view.rotation = if pointer.clicks >= 2 { 0.0 } else { turn.drag(pointer.pos, pointer.mods.shift, start.flip_x) };
        self.outbox.set(ViewChange::to(view));
    }

    /// Escape puts the angle back to 0 (not during a drag: that cancels it).
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        let Some(space) = cx.space() else { return false };
        if !matches!(key, ToolKey::Cancel) || self.drag.is_some() || space.view.rotation == 0.0 {
            return false;
        }
        let mut view = self.outbox.view(&space);
        view.rotation = 0.0;
        self.outbox.set(ViewChange::to(view));
        true
    }

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {
        if let Some((_, view)) = self.drag.take() {
            self.outbox.set(ViewChange::to(view));
        }
    }

    fn paint(&self, _draw: &Draw<'_>, _cx: &ToolContext<'_>) {}

    fn cursor(&self, _cx: &ToolContext<'_>) -> Cursor {
        PaintedCursor::Hand.into()
    }

    fn autoscrolls(&self) -> bool {
        false
    }

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Rotate View commands")
    }
}
