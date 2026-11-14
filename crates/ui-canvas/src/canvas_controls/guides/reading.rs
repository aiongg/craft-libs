//! `GuideReading`: the guide a drag is moving or creating, read from the running gesture.

use craft_behavior::Axis;
use egui::{Context, Id};

/// The position and orientation of the guide a drag is moving or creating.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideReading {
    /// The axis the guide runs along.
    pub along: Axis,
    /// The guide's position across that axis, in document units.
    pub position: f64,
}

/// The guide a drag is moving or creating on the canvas `canvas`, read from the framework's running gesture of the
/// guide parts, so a panel drawn before or after the canvas reads the same value; `None` when no guide drag runs.
pub fn running(ctx: &Context, canvas: Id) -> Option<GuideReading> {
    let _ = (ctx, canvas);
    todo!("cw.C74")
}
