//! [`RulerLine`]: the Ruler tool's line with its optional protractor arm, and its
//! [`RulerReading`]: the numbers the options bar and AccessKit show. No egui.

#![expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TO-R1)")]

use kurbo::Point;

/// A new line shorter than this, in document units, is cleared on release (PhotoCraft's 0.5).
pub const MIN_LINE: f64 = 0.5;

/// The Ruler line in document units: from `start` to `end`, and with a protractor a second arm
/// from `start` (the vertex) to `protractor`.
/// Serialises camelCase, points as kurbo's: the app stores the line in its document.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulerLine {
    pub start: Point,
    pub end: Point,
    pub protractor: Option<Point>,
}

/// A part of the line a press grabs and the keys move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RulerPart {
    Start,
    End,
    /// The protractor arm's free end.
    Protractor,
}

impl RulerPart {
    /// The part's key for [`Part::key`](craft_ui::canvas::Part::key): 0, 1, 2 in keyboard order.
    pub fn key(self) -> u64 {
        match self {
            RulerPart::Start => 0,
            RulerPart::End => 1,
            RulerPart::Protractor => 2,
        }
    }

    /// The part keyed `key`.
    pub fn of_key(key: u64) -> Option<RulerPart> {
        match key {
            0 => Some(RulerPart::Start),
            1 => Some(RulerPart::End),
            2 => Some(RulerPart::Protractor),
            _ => None,
        }
    }
}

/// What the Ruler measures, as the engine's `ruler_info` gives it: X and Y of the start, W and H
/// (end minus start, signed), the angle A and the lengths L1 and L2.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RulerReading {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Degrees. A line: counter-clockwise from +x with y up, -180 to 180. With a protractor: the
    /// angle between the two arms, 0 to 180.
    pub angle: f64,
    /// The first arm's length.
    pub l1: f64,
    /// The protractor arm's length; `None` without a protractor.
    pub l2: Option<f64>,
}

impl RulerLine {
    /// A line from `start` to `end` without a protractor.
    pub fn new(start: Point, end: Point) -> Self {
        Self { start, end, protractor: None }
    }

    /// Whether every point is finite.
    pub fn is_finite(&self) -> bool {
        // new: `Point::is_finite` of the three points.
        todo!()
    }

    /// The point of `part`; `None` for the protractor of a line without one.
    pub fn point(&self, part: RulerPart) -> Option<Point> {
        // new: field access.
        todo!()
    }

    /// The part a press at `at` grabs within `reach` document units: the protractor's end first,
    /// then the end, then the start (PhotoCraft's order); `None` elsewhere.
    pub fn part_at(&self, at: Point, reach: f64) -> Option<RulerPart> {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:250 (the Down branch's match order).
        todo!()
    }

    /// The numbers of the line. The engine's `ruler_info` duplicates them on PhotoCraft's side
    /// (lead call 19); a PhotoCraft test compares the two.
    pub fn reading(&self) -> RulerReading {
        // pc: crates/engine/src/cmd/analysis_cmds.rs:614 `ruler_info` (without the rounding to 4
        // decimals and the scale, which `RulerReading::scaled` is).
        todo!()
    }
}

impl RulerReading {
    /// Use Measurement Scale: X, Y, W, H, L1 and L2 multiplied by `factor`; the angle unchanged.
    pub fn scaled(self, factor: f64) -> RulerReading {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:497 `options_bar` (the Ruler branch's `f`).
        todo!()
    }
}
