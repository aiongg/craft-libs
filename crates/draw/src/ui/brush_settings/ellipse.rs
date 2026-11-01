//! Photoshop's tip angle and roundness widget ([`TipEllipse`]) and its geometry.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditResponse;
use egui::{AsIdSalt, IdSalt, Ui, Vec2};

use super::labels::BrushSettingsLabels;

/// A tip's angle and roundness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TipAngle {
    /// The major axis's angle in whole degrees, counter-clockwise from the right, −180 to 180.
    pub degrees: f64,
    /// The minor axis over the major, 0.01 to 1.
    pub roundness: f64,
}

impl Default for TipAngle {
    fn default() -> Self {
        Self { degrees: 0.0, roundness: 1.0 }
    }
}

/// What a press on the widget grabbed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TipEllipseGrab {
    /// The arrow: a drag turns the angle.
    Angle,
    /// A roundness dot: a drag squashes the ellipse.
    Roundness,
}

/// The two roundness dots, relative to the widget's centre in points (screen y down), for an
/// ellipse of major radius `radius` (`brush.ellipse_radius`, 34): `±minor × radius ×
/// clamp(roundness, 0, 1)`, where the minor axis is the major turned 90° counter-clockwise,
/// `(−sin a, −cos a)`.
pub fn ellipse_handles(angle: TipAngle, radius: f32) -> [Vec2; 2] {
    // pc: crates/ui-egui/src/panels/brush_sections.rs:163-168 `roundness_handles`
    todo!()
}

/// What a press at `at` (relative to the centre) grabs: a roundness dot within `reach`
/// (`brush.ellipse_grab`, 7) of either, else the arrow.
pub fn ellipse_grab(at: Vec2, angle: TipAngle, radius: f32, reach: f32) -> TipEllipseGrab {
    // pc: crates/ui-egui/src/panels/brush_sections.rs:171-173 `ellipse_grab` (`<= 7.0`)
    todo!()
}

/// The value after a drag to `at` (relative to the centre) holding `grab`. Angle: the direction
/// of `at` in whole degrees, counter-clockwise, `atan2(−y, x)` rounded; unchanged within 1 pt of
/// the centre. Roundness: `|at · minor| / radius` clamped to 0.01 to 1; the angle stays.
pub fn ellipse_drag(grab: TipEllipseGrab, at: Vec2, angle: TipAngle, radius: f32) -> TipAngle {
    // pc: crates/ui-egui/src/panels/brush_sections.rs:177-187 `ellipse_drag`
    todo!()
}

/// Photoshop's angle and roundness widget: a disc with cross lines, the tip's ellipse, an arrow
/// along the major axis and two dots on the minor axis. Drag the arrow (anywhere off the dots) to
/// turn the angle, a dot to set the roundness. Reports [`EditEvent<TipAngle>`](craft_ui::edit::EditEvent):
/// a drag is one gesture (`Begin`, `Update`, `Commit`; Esc cancels and the value goes back); a
/// click on the disc is a `Set`.
///
/// - **Look.** `brush.ellipse_size` (84) square; the disc radius `brush.ellipse_disc` (40) on
///   `field.fill` with a 1 pt `field.border` and `line.divider` cross lines; the ellipse
///   `brush.ellipse_radius` (34) filled `text.default` at 18 % with a `brush.ellipse_line` (1.5)
///   `text.default` outline (48 segments); an `accent` arrow of the same width to
///   `brush.ellipse_arrow_reach` (5) past the ellipse, its head `brush.ellipse_head` (7 long, 8
///   wide); the dots `brush.ellipse_dot` (4) in `overlay.handle_fill` with a 1 pt `accent` ring.
///   Focus ring around the disc.
/// - **Keys.** One Tab stop. Up and Right turn the angle 1° counter-clockwise, Down and Left 1°
///   clockwise (Shift: 15°), wrapping at ±180; Alt with the arrows changes the roundness by 1 %
///   (Up and Right rounder). Each press is a `Set`.
/// - **AccessKit.** A group "Tip angle and roundness" with the value "{angle}°, {roundness} %";
///   the arrow keys as Increment and Decrement of the angle.
/// - **RTL.** Not mirrored: it shows document geometry.
#[must_use = "show the widget with `show`"]
pub struct TipEllipse {
    id_salt: IdSalt,
    value: TipAngle,
    labels: Option<BrushSettingsLabels>,
}

impl TipEllipse {
    /// The widget for `value`.
    pub fn new(id_salt: impl AsIdSalt, value: TipAngle) -> Self {
        Self { id_salt: IdSalt::new(id_salt), value, labels: None }
    }

    /// The words; the installed language's by default.
    pub fn labels(mut self, labels: BrushSettingsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the widget.
    pub fn show(self, ui: &mut Ui) -> EditResponse<TipAngle> {
        // pc: crates/ui-egui/src/panels/brush_sections.rs:190-246 `ellipse_widget` (84 square, disc 40, ellipse 34 at 18 %, arrow +5 with a head 7 long and 8 wide, dots 4)
        todo!()
    }
}
