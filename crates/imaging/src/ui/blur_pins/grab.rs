//! What a press on the pins grabs, and the drag that follows (task WS-BP1).

use egui::CursorIcon;

use craft_ui::canvas::CanvasView;
use craft_ui::canvas::kurbo::Point;

use super::model::BlurPins;
use super::style::BlurPinsStyle;

/// What a press grabs, by pin index. Handles belong to the selected pin only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlurGrab {
    /// An Iris or Spin outline handle: axis 0..=3 (+u, +v, −u, −v) resizes that radius.
    Axis { pin: usize, axis: usize },
    /// Iris's rounding square.
    Round { pin: usize },
    /// One of Iris's four feather dots.
    Feather { pin: usize, dot: usize },
    /// A Tilt-Shift turn dot, on one side of the band (−1 or 1).
    Turn { pin: usize, side: i8 },
    /// A Tilt-Shift solid line: the sharp band's width.
    Focus { pin: usize },
    /// A Tilt-Shift dashed line: the transition's width.
    Transition { pin: usize },
    /// A pin's ring: the amount (one turn is the full range).
    Ring { pin: usize },
    /// A pin's dot: moves it.
    Pin { pin: usize },
    /// Nothing: a click adds a pin (not Tilt-Shift).
    New,
}

impl BlurGrab {
    /// The cursor over this grab: Alias for the ring, rounding and turn dots; Move for the axis and
    /// feather handles; ResizeVertical for the Tilt-Shift lines; Grab for a pin; Crosshair over
    /// nothing (Default for Tilt-Shift, which adds no pin).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-BP1")]
    pub fn cursor(self, tilt_shift: bool) -> CursorIcon {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:539-546.
        todo!()
    }
}

/// What a press at screen point `at` grabs: the selected pin's handles first (within
/// `blur_pins.handle` + `canvas.hit`, the Tilt-Shift lines within `.line_grab` 5), then any pin's
/// dot (`.grab` 9), then any ring (`.ring` 24 ± `.ring_band` 5 / 2), else [`BlurGrab::New`].
#[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-BP1")]
pub fn grab(pins: &BlurPins, view: &CanvasView, at: egui::Pos2, style: &BlurPinsStyle) -> BlurGrab {
    // pc: crates/ui-egui/src/dialogs/blur_pins.rs:474 `grab`, :468 `line_distance`.
    todo!()
}

/// A drag's effect on `pins`: `start` is the pins at the press, `from` and `to` the press and
/// the pointer in document pixels. A pin stays inside the bounds; the ring turns the amount by the
/// angle swept (a whole turn is [`BlurKind::max`](super::BlurKind::max)); an axis handle sets its
/// radius and the angle; the rounding square sets the roundness along the diagonal; a feather dot
/// sets the feather; a turn dot the band's angle; the Tilt-Shift lines their widths. No modifier
/// changes a drag (PhotoCraft's).
#[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-BP1")]
pub fn drag(pins: &mut BlurPins, grab: BlurGrab, start: &BlurPins, from: Point, to: Point) {
    // pc: crates/ui-egui/src/dialogs/blur_pins.rs:585-672 (the drags), :322 `iris_point`, :331
    // `iris_local`, :340 `diagonal`, :346-360 (handles), :362 `tilt_point`, :369 `tilt_offset`.
    todo!()
}
