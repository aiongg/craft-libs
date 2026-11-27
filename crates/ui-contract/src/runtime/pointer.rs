//! The pointer snapshot a tool reads (`Ui::pointer`, 15.9) and the pass pointer the sensing core still names.

use crate::primitives::Id;
use craft_behavior::{Buttons, MultiTap, MultiTouch, PartRef, PointerButton, PointerEvent, PointerKind, PointerSample, PressStart, Shape, Winner};
use craft_keymap::Mods;
use craft_ui_rt::{Pos2, Rect, Vec2};

/// The pointer input of one pass of one viewport, egui-converted. Input enters a frame's first pass only (egui hands
/// the frame's events to it, and injected steps and held-back presses follow the frame): a rerun pass of the
/// frame carries no events, so its `delta`, `press`, `release` and `scroll` are empty while the held facts (`pos`,
/// `buttons`) persist. A consumer applies what its pass hands it and never turns a held fact into an event.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PointerFrame {
    /// The latest pointer position, `None` while the pointer is not in the window.
    pub pos: Option<Pos2>,
    /// The pointer movement of the pass.
    pub delta: Vec2,
    /// The buttons held.
    pub buttons: Buttons,
    /// The primary press of the pass.
    pub press: Option<PressStart>,
    /// Modifiers carried by `press`, including the app's latched modifiers. A pass has at most one press.
    pub press_mods: Option<Mods>,
    /// The button released in the pass.
    pub release: Option<PointerButton>,
    /// The modifiers held, with the ones the app latches for the user (a Modifier Keys panel) added: the app holds
    /// them down for every press and drag. Key chords read the keyboard's own modifiers instead.
    pub mods: Mods,
    /// The modifiers of the pass's last button release, with the latched ones added; the held `mods` in a pass with no
    /// release and in a rerun pass. The one source of a pick's and a drop's modifiers.
    pub pick_mods: Mods,
    /// Where the pass's last button release went up; the latest `pos` in a pass with no release and in a rerun pass. The
    /// one source of a drop's position: a pointer that moves on or leaves the window after the release in the same
    /// frame does not move the drop.
    pub pick_pos: Option<Pos2>,
    /// The tap a per-viewport recognizer reports when the last finger lifts, in the frame's first pass only.
    pub multi_tap: Option<craft_behavior::MultiTap>,
    /// The pass's time in seconds.
    pub time: f64,
    /// The device.
    pub kind: PointerKind,
    /// Pressure and tilt of the current contact.
    pub sample: Option<PointerSample>,
    /// The two-finger gesture.
    pub multi_touch: Option<MultiTouch>,
    /// Wheel and trackpad scroll, in points.
    pub scroll: Vec2,
    /// Pinch and Ctrl or Cmd with the wheel, folded into one factor.
    pub zoom: f32,
    /// The pass's summed `Event::Rotate` radians.
    pub rotation: f32,
    /// The movement that turns a press into a drag.
    pub slop: f32,
    /// The least hit size across for this pointer kind.
    pub target: f32,
}

/// egui's click and drag widgets for the latest press.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[doc(hidden)]
pub struct PressTargets {
    pub click: Option<Id>,
    pub drag: Option<Id>,
}

/// What the begin hook read of the pass, for the sensing core and the services. Built from the pass's own events,
/// so a rerun pass of a frame (see [`PointerFrame`]) reads none.
#[derive(Clone, Debug, Default)]
#[doc(hidden)]
pub struct PassPointer {
    pub frame: Option<PointerFrame>,
    /// The primary events, presses carrying their click counts.
    pub events: Vec<PointerEvent>,
    pub hover: Option<Pos2>,
    pub primary_down: bool,
    pub press_targets: PressTargets,
    pub focus_lost: bool,
    /// Whether the real pointer moved or entered, or a held modifier or key changed, in the pass.
    pub stirred: bool,
    /// A finger landed in the pass while another was down: a press open now is a gesture's.
    pub gesture: bool,
}
