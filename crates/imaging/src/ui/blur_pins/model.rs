//! The Blur Gallery's pins as typed data (task WS-BP1).

use serde::{Deserialize, Serialize};

use craft_ui::canvas::kurbo::Point;

/// The most pins (PhotoCraft `MAX_PINS`: more than anyone places by hand; it bounds the
/// parameters and the per-pixel loop).
pub const MAX_PINS: usize = 64;

/// A new pin's amount when nothing is remembered: 15.
pub const DEFAULT_AMOUNT: f64 = 15.0;

/// The four Blur Gallery filters with pins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlurKind {
    /// Field Blur: the pins' amounts interpolate over the image.
    Field,
    /// Iris Blur: a superellipse per pin, sharp inside the feather.
    Iris,
    /// Spin Blur: an ellipse per pin; the amount is the blur angle.
    Spin,
    /// Tilt-Shift: one band.
    TiltShift,
}

impl BlurKind {
    /// The amount's top: 500 px of blur, or Spin's 360° blur angle.
    pub fn max(self) -> f64 {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:36-38, :66 `max`.
        if self == BlurKind::Spin { 360.0 } else { 500.0 }
    }
}

/// An Iris or Spin pin's shape.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrisShape {
    /// Radii as fractions of the bounds' shorter side, 0.01..=1.
    pub radius_x: f64,
    pub radius_y: f64,
    /// Degrees, clockwise on screen, −180..180.
    pub angle: f64,
    /// 0 (an ellipse) ..= 100 (a rounded rectangle); Iris only.
    pub roundness: f64,
    /// Where the blur starts, as a fraction of the shape, 0..=0.99; Iris only.
    pub feather: f64,
}

impl IrisShape {
    /// Iris Blur's default: radii 0.35 and 0.25, upright, an ellipse, feather 0.5.
    pub fn iris() -> IrisShape {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:109-114.
        IrisShape { radius_x: 0.35, radius_y: 0.25, angle: 0.0, roundness: 0.0, feather: 0.5 }
    }

    /// Spin Blur's default: a circle of radius 0.3.
    pub fn spin() -> IrisShape {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:118 `spin`.
        IrisShape { radius_x: 0.3, radius_y: 0.3, ..IrisShape::iris() }
    }

    /// The shape with every value finite and in range (non-finite values back to the defaults,
    /// the angle wrapped into −180..180).
    #[expect(clippy::todo, reason = "skeleton: task WS-BP1")]
    pub fn clean(self) -> IrisShape {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:122 `clean`.
        todo!()
    }

    /// The superellipse exponent the engine uses: 2 + roundness / 100 × 6 (2 an ellipse … 8).
    pub fn exponent(self) -> f64 {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:134 `exponent`.
        2.0 + self.roundness / 100.0 * 6.0
    }
}

/// Tilt-Shift's band.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TiltBand {
    /// Degrees, −90..=90; 0 a horizontal band, positive turns it anticlockwise on screen.
    pub angle: f64,
    /// The sharp band's half width beyond the centre line, as a fraction of the bounds' shorter
    /// side, 0..=1 (0.1).
    pub focus: f64,
    /// The transition's width beyond the sharp band, 0.01..=1 (0.15).
    pub transition: f64,
}

impl TiltBand {
    /// The band with every value finite and in range (the angle folded into −90..=90: the band
    /// turned half a turn is the same).
    #[expect(clippy::todo, reason = "skeleton: task WS-BP1")]
    pub fn clean(self) -> TiltBand {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:150 `clean`.
        todo!()
    }

    /// The band's unit normal and its direction along the band (the engine's).
    pub fn axes(self) -> ([f64; 2], [f64; 2]) {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:163 `axes`.
        let (s, c) = self.angle.to_radians().sin_cos();
        ([s, c], [c, -s])
    }
}

/// One pin: its position as fractions of the bounds, its amount (blur in pixels, Spin's blur angle
/// in degrees), and an Iris or Spin pin's shape or the Tilt-Shift band.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlurPin {
    pub x: f64,
    pub y: f64,
    pub amount: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iris: Option<IrisShape>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tilt: Option<TiltBand>,
}

/// The pins of one Blur Gallery filter (lead call 48: typed, where PhotoCraft kept a JSON map in
/// the dialog's fields): the kind, the filter's bounds (the selection's, else the canvas's) in
/// document pixels `[x0, y0, x1, y1]`, the pins, and the selected pin. The dialog's fields keep
/// the remembered amount and shape; the app maps `BlurPins` to the filter's `pins` parameters
/// (Tilt-Shift: `centerX`, `centerY`, `angle`, `focus`, `transition`, `blur`).
#[derive(Clone, Debug, PartialEq)]
pub struct BlurPins {
    pub kind: BlurKind,
    pub bounds: [f64; 4],
    pub pins: Vec<BlurPin>,
    pub selected: Option<usize>,
}

impl BlurPins {
    /// One pin in the middle of `bounds` with `start` (the remembered amount and shape), selected:
    /// what the filter opens with, and what Alt + Cancel (Reset) starts again from.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-BP1")]
    pub fn new(kind: BlurKind, bounds: [f64; 4], start: BlurPin) -> BlurPins {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:171 `init`, :182 `reset`.
        todo!()
    }

    /// The bounds' shorter side in document pixels: the unit of radii and widths.
    pub fn short_side(&self) -> f64 {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:309 `short_side`.
        (self.bounds[2] - self.bounds[0]).min(self.bounds[3] - self.bounds[1]).max(1.0)
    }

    /// A pin's position in document pixels.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-BP1")]
    pub fn to_document(&self, pin: &BlurPin) -> Point {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:313 `to_doc`.
        todo!()
    }

    /// A document point as fractions of the bounds, clamped to 0..=1 (pins stay inside).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-BP1")]
    pub fn from_document(&self, at: Point) -> (f64, f64) {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:317 `from_doc`.
        todo!()
    }

    /// Adds a pin at `at` (inside the bounds; not for Tilt-Shift, not past [`MAX_PINS`]) with the
    /// selected pin's amount and shape, and selects it. Returns whether it was added.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-BP1")]
    pub fn add(&mut self, at: Point) -> bool {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:571-583 (`Grab::New`).
        todo!()
    }

    /// Removes the selected pin unless it is the last; the one before it is selected.
    #[expect(clippy::todo, reason = "skeleton: task WS-BP1")]
    pub fn remove_selected(&mut self) -> bool {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:385 `remove_selected`.
        todo!()
    }

    /// The pins cleaned: positions in 0..=1, amounts in 0..=[`BlurKind::max`], shapes and bands
    /// [`IrisShape::clean`] and [`TiltBand::clean`], at most [`MAX_PINS`], the selection in range.
    #[expect(clippy::todo, reason = "skeleton: task WS-BP1")]
    pub fn clean(&mut self) {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:228 `pins`.
        todo!()
    }
}
