//! `CanvasTool` (8.3): the tool a canvas hands its background presses to, with what a call reads.
//!
//! A tool holds its own state between passes and reads nothing from `egui`. [`ToolHost`](super::ToolHost) maps the
//! canvas's press, hover and keys to the trait's calls; pointer positions are screen points, and the tool maps
//! them to document units with the canvas's view ([`ToolContext::space`]), so it holds no copy of the view.

use super::{CanvasSpace, ToolPath};
use crate::primitives::{ContentPainterCx, Draw};
use crate::runtime::env::Label;
use craft_behavior::kurbo::Point;
use craft_behavior::{CancelReason, CanvasView, Cursor, CursorIcon, PointerButton, PointerSample};
use craft_keymap::{Chord, Key, Mods};
use egui::{Pos2, Rect, Vec2};

/// A pointer sample delivered to a tool, in screen points, with the pen's reading. A mouse reports pressure 1, no
/// tilt and no eraser.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToolPointer {
    /// The pointer on screen.
    pub pos: Pos2,
    /// The modifiers held when the sample was taken.
    pub mods: Mods,
    /// Pressure, tilt, rotation and the eraser end.
    pub sample: PointerSample,
    /// The button the gesture runs with; the primary for a hover.
    pub button: PointerButton,
    /// How many presses make up the click run the gesture began with: 1 for a lone press, 2 for a double press.
    /// 0 for a sample with no press (a hover).
    pub clicks: u32,
    /// The time of the sample, in seconds: the pass that took it. A press carries the time the button went down,
    /// and a drag or release sample the time of the pass it arrived in.
    pub time: f64,
}

impl ToolPointer {
    /// A mouse sample of `button` at `pos` and `time`, with no modifiers and no press.
    pub const fn new(pos: Pos2, button: PointerButton, time: f64) -> Self {
        ToolPointer { pos, mods: Mods::NONE, sample: PointerSample::new(1.0), button, clicks: 0, time }
    }

    /// The modifiers held.
    #[must_use]
    pub const fn mods(mut self, mods: Mods) -> Self {
        self.mods = mods;
        self
    }

    /// The pen's reading.
    #[must_use]
    pub const fn sample(mut self, sample: PointerSample) -> Self {
        self.sample = sample;
        self
    }

    /// The length of the click run.
    #[must_use]
    pub const fn clicks(mut self, clicks: u32) -> Self {
        self.clicks = clicks;
        self
    }
}

/// What a tool does with a press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolAction {
    /// The tool owns the gesture: it gets the drags and the release.
    Claimed,
    /// A click the tool handled whole (a Zoom step, an eyedropper sample): no drag follows.
    Done,
    /// The press is not the tool's: the host drops the whole press.
    Ignored,
}

/// A key the host passes to the tool while the canvas has the keyboard and no text part keeps it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ToolKey {
    /// An arrow key: a unit direction on screen (not mirrored in a right-to-left interface) and the modifiers.
    #[non_exhaustive]
    Nudge {
        /// The direction.
        direction: Vec2,
        /// The modifiers held.
        mods: Mods,
    },
    /// Delete (`forward`) or Backspace.
    #[non_exhaustive]
    Delete {
        /// Delete toward the reading end.
        forward: bool,
    },
    /// Enter.
    Submit,
    /// Escape with no gesture running.
    Cancel,
    /// A key press the tool kept with [`ToolContext::keep_keys`]: every key without Ctrl or Cmd, so the app's chords
    /// still run. It carries no release; [`ToolContext::held`] says whether the key is still down.
    Kept(Chord),
}

/// How a view change eases to its target instead of jumping, as a zoom click does under Animated Zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ViewEase {
    /// A document point and the screen point it stays at while the change eases, for a zoom about the pointer.
    pub anchor: Option<(Point, Pos2)>,
}

/// What a tool asked the host for in a call; the host applies it after the call.
#[derive(Clone, Debug, Default)]
pub(crate) struct ToolRequests {
    pub(crate) readout: Option<Vec<(Label, String)>>,
    pub(crate) repaint: bool,
    pub(crate) keep_keys: bool,
    pub(crate) view: Option<CanvasView>,
    pub(crate) ease: Option<ViewEase>,
}

/// What a tool sees and may ask for during a call.
#[non_exhaustive]
pub struct ToolContext<'a> {
    /// The pointer's last sample this pass; `None` while the pointer is off the canvas and no gesture runs.
    pub pointer: Option<ToolPointer>,
    /// The modifiers held now.
    pub mods: Mods,
    /// The canvas on screen.
    pub rect: Rect,
    /// The pass time, in seconds.
    pub time: f64,
    /// The press arena's slop, in screen points: the least movement that makes a press a drag, and the least size a
    /// drawn rectangle has to count. 0 for a context built without one.
    pub slop: f32,
    space: Option<CanvasSpace>,
    down: &'a [Key],
    painter: Option<&'a ContentPainterCx<'a>>,
    requests: ToolRequests,
}

impl<'a> ToolContext<'a> {
    /// A context over the canvas `rect`, which shows its document through `space` (`None` for a canvas without a
    /// view), with no key held: the host builds one per call; a test of a tool builds its own.
    pub fn new(rect: Rect, space: Option<CanvasSpace>) -> Self {
        ToolContext {
            pointer: None,
            mods: Mods::NONE,
            rect,
            time: 0.0,
            slop: 0.0,
            space,
            down: &[],
            painter: None,
            requests: ToolRequests::default(),
        }
    }

    /// The pointer's last sample.
    #[must_use]
    pub fn pointer(mut self, pointer: Option<ToolPointer>) -> Self {
        self.pointer = pointer;
        self
    }

    /// The modifiers held now.
    #[must_use]
    pub fn mods(mut self, mods: Mods) -> Self {
        self.mods = mods;
        self
    }

    /// The pass time.
    #[must_use]
    pub fn time(mut self, time: f64) -> Self {
        self.time = time;
        self
    }

    /// The keys held now.
    #[must_use]
    pub fn down(mut self, down: &'a [Key]) -> Self {
        self.down = down;
        self
    }

    /// The press arena's slop.
    #[must_use]
    pub fn slop(mut self, slop: f32) -> Self {
        self.slop = slop;
        self
    }

    /// The painter of a paint call; the host sets it.
    #[must_use]
    pub(crate) fn painter(mut self, painter: &'a ContentPainterCx<'a>) -> Self {
        self.painter = Some(painter);
        self
    }

    /// The canvas's view and interface scale this pass, to map screen points to document units and back at run
    /// time; `None` for a canvas without a view. After [`set_view`](Self::set_view) it shows the view the tool
    /// asked for, so the calls after it in the pass start from there.
    pub fn space(&self) -> Option<CanvasSpace> {
        self.space
    }

    /// Sets the canvas's view: the host reports it in [`ToolHostResponse::view`](super::ToolHostResponse::view) and
    /// the app copies it into its bound view. The last view set in a pass wins.
    pub fn set_view(&mut self, view: CanvasView) {
        self.requests.view = Some(view);
        self.requests.ease = None;
        if let Some(space) = self.space.as_mut() {
            space.view = view;
        }
    }

    /// [`set_view`](Self::set_view) for a change that eases to `view` ([`ZoomAnimation`](super::ZoomAnimation)),
    /// keeping `anchor` (a document point and the screen point it stays at) fixed on the way.
    pub fn ease_view(&mut self, view: CanvasView, anchor: Option<(Point, Pos2)>) {
        self.set_view(view);
        self.requests.ease = Some(ViewEase { anchor });
    }

    /// The view the tool set in this call, for a test of a tool.
    pub fn requested_view(&self) -> Option<CanvasView> {
        self.requests.view
    }

    /// The ease the tool asked for in this call, for a test of a tool.
    pub fn requested_ease(&self) -> Option<ViewEase> {
        self.requests.ease
    }

    /// Paints marching ants along `points` (closed when `closed`), shifted `phase` points, in a paint call; it paints
    /// nothing in any other.
    pub fn marching_ants(&self, points: &[Pos2], closed: bool, phase: f32) {
        if let Some(painter) = self.painter {
            painter.marching_ants(points, closed, phase);
        }
    }

    /// Whether `key` is down now, whatever the tool kept: a marquee that sizes from its centre while Space is held
    /// reads `held(Key::Space)` on every drag. The host withholds Space from the app's shortcuts while a gesture
    /// runs, so a momentary Hand does not take over mid-drag.
    pub fn held(&self, key: Key) -> bool {
        self.down.contains(&key)
    }

    /// Labelled values to show beside the pointer this pass (a marquee's W and H). The last request of a pass wins.
    pub fn readout(&mut self, rows: Vec<(Label, String)>) {
        self.requests.readout = Some(rows);
    }

    /// Asks for another pass (an animation, a held key that repeats).
    pub fn request_repaint(&mut self) {
        self.requests.repaint = true;
    }

    /// Keeps the canvas's plain keys for the tool this pass and the next: they arrive as [`ToolKey::Kept`] and the
    /// app's single-key shortcuts leave them.
    pub fn keep_keys(&mut self) {
        self.requests.keep_keys = true;
    }

    pub(crate) fn take_requests(&mut self) -> ToolRequests {
        std::mem::take(&mut self.requests)
    }
}

/// A tool on the canvas. The app owns the value and hands it to [`ToolHost::run`](super::ToolHost::run) each
/// pass; a tool with a gesture in progress keeps its state in itself. One `Begin`...`Commit` makes a drag: `press`
/// returning [`ToolAction::Claimed`] begins it, `drag` updates it, `release` commits it, `cancel` ends it.
pub trait CanvasTool {
    /// Called first in every pass, whatever the pointer does: the tool takes this pass's app state and advances
    /// what runs without the pointer (an animation). Default: nothing.
    fn each_pass(&mut self, _cx: &mut ToolContext<'_>) {}

    /// A button the tool takes went down on the canvas away from every canvas control.
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction;

    /// The pointer moved while the tool owns the gesture.
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>);

    /// The button came up: commit the gesture.
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>);

    /// The pointer moved over the canvas with no gesture running (hover outlines, a pen's rubber band). Default:
    /// nothing.
    fn hover(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    /// The gesture ends without a commit; `reason` says why ([`CancelReason::Escape`] for Escape). The tool drops
    /// its drag state and reports `reason` on the gesture's events.
    fn cancel(&mut self, reason: CancelReason, cx: &mut ToolContext<'_>);

    /// A key while the canvas has the keyboard and no text part keeps it. Return `true` to take it from the app's
    /// shortcuts. Escape during a gesture never reaches here: it cancels. Default: `false`.
    fn key(&mut self, _key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        false
    }

    /// Draws the tool's feedback over the document (a marquee in progress, pen segments). Runs after the canvas
    /// controls.
    fn paint(&self, draw: &Draw<'_>, cx: &ToolContext<'_>);

    /// The cursor over the canvas. Default: the system arrow.
    fn cursor(&self, _cx: &ToolContext<'_>) -> Cursor {
        Cursor::System(CursorIcon::Default)
    }

    /// Whether a press of `button` with `mods` held starts a gesture with this tool. A gesture keeps its button
    /// until that button comes up. A brush takes the primary button, and the secondary while Alt is held (the
    /// resize chord). The middle button never reaches `takes`: the canvas turns every middle press into a pan before
    /// the host asks. Default: the primary button.
    fn takes(&self, button: PointerButton, _mods: Mods) -> bool {
        button == PointerButton::Primary
    }

    /// Whether a drag past the canvas edge asks the app to scroll the view (marquees, lassos, brushes, crop: yes;
    /// the Hand and Zoom tools: no). Default: `true`.
    fn autoscrolls(&self) -> bool {
        true
    }

    /// How the tool is reached without a pointer: a tool that creates or selects geometry names the command whose
    /// form takes the same numbers; a freehand tool is `ToolPath::Freehand`, a named exception.
    fn command_form(&self) -> ToolPath;
}
