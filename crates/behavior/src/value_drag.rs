//! `ValueDrag`: the drag maths of sliders, dials, scrubbing number fields and rulers, written once (8.7).

use crate::flow::Axis;
use crate::press::{PressPhase, PressState};
use emath::Pos2;
use std::ops::{Range, RangeInclusive};

/// How the pointer's position along the axis becomes a value.
#[derive(Clone, Debug, PartialEq)]
enum Mapping {
    /// The pointer position on a track from `track.start` to `track.end` along the axis, in layer points.
    Absolute { track: Range<f32> },
    /// Value units per point of travel from where the press began.
    Relative { rate: f64 },
}

/// Turns a press into a value. Components set it as data and call [`ValueDrag::value`] in `update`.
#[derive(Clone, Debug, PartialEq)]
pub struct ValueDrag {
    mapping: Mapping,
    axis: Axis,
    range: RangeInclusive<f64>,
    wrap: bool,
    fine: f64,
}

/// The factor Shift applies to travel when a drag sets none.
const FINE: f64 = 0.1;

impl ValueDrag {
    /// The value follows the pointer on `track`: `range.start()` at `track.start`, `range.end()` at `track.end`.
    /// A track whose `start` lies after its `end` runs backwards, as a vertical slider's does.
    pub fn absolute(track: Range<f32>, range: RangeInclusive<f64>) -> Self {
        ValueDrag { mapping: Mapping::Absolute { track }, axis: Axis::X, range, wrap: false, fine: FINE }
    }

    /// The value changes by `rate` units per point of travel from the press. Vertical travel upwards increases it.
    pub fn relative(rate: f64, range: RangeInclusive<f64>) -> Self {
        ValueDrag { mapping: Mapping::Relative { rate }, axis: Axis::X, range, wrap: false, fine: FINE }
    }

    /// The axis the pointer travels on; the default is horizontal.
    pub const fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// The factor Shift applies to travel.
    pub const fn fine(mut self, factor: f64) -> Self {
        self.fine = factor;
        self
    }

    /// Wraps past either end of the range instead of stopping there, as a hue dial does.
    pub const fn wrap(mut self, on: bool) -> Self {
        self.wrap = on;
        self
    }

    /// The value the press gives, `None` while it gives none: before a drag starts, and after a cancel. `start` is the
    /// value when the drag began. Horizontal travel mirrors when `rtl`.
    pub fn value(&self, start: f64, press: &PressState, rtl: bool) -> Option<f64> {
        let (origin, pos) = match press.phase {
            PressPhase::DragStart { start, pos } | PressPhase::Drag { start, pos, .. } | PressPhase::Release { start, pos, .. } => {
                (start.origin, pos)
            }
            _ => return None,
        };
        let fine = if press.mods.shift { self.fine } else { 1.0 };
        let raw = match &self.mapping {
            Mapping::Absolute { track } => {
                let at = |point: Pos2| self.fraction(point, track, rtl) * (self.range.end() - self.range.start()) + self.range.start();
                if fine == 1.0 { at(pos) } else { start + (at(pos) - at(origin)) * fine }
            }
            Mapping::Relative { rate } => {
                let (from, to) = (self.along(origin), self.along(pos));
                let travel = match self.axis {
                    Axis::X if rtl => from - to,
                    Axis::X => to - from,
                    Axis::Y => from - to,
                };
                let travel = f64::from(travel);
                start + travel * rate * fine
            }
        };
        Some(self.fit(raw))
    }

    fn along(&self, point: Pos2) -> f32 {
        match self.axis {
            Axis::X => point.x,
            Axis::Y => point.y,
        }
    }

    /// Where along the track `point` lies, 0 at its start and 1 at its end, mirrored under `rtl` on the horizontal axis.
    fn fraction(&self, point: Pos2, track: &Range<f32>, rtl: bool) -> f64 {
        let span = track.end - track.start;
        let x = if self.axis == Axis::X && rtl { track.start + track.end - self.along(point) } else { self.along(point) };
        if span == 0.0 { 0.0 } else { f64::from((x - track.start) / span) }
    }

    fn fit(&self, value: f64) -> f64 {
        let (low, high) = (*self.range.start(), *self.range.end());
        if self.wrap && high > low { low + (value - low).rem_euclid(high - low) } else { value.clamp(low, high) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::press::{PointerButton, PressStart};
    use craft_keymap::Mods;
    use emath::{Vec2, pos2};

    fn drag(from: Pos2, to: Pos2, mods: Mods) -> PressState {
        let start = PressStart::new(from, PointerButton::Primary, 0.0, 1);
        PressState::new(PressPhase::Drag { start, pos: to, delta: Vec2::ZERO }).mods(mods)
    }

    #[test]
    fn an_absolute_drag_follows_the_pointer_on_the_track_and_stops_at_its_ends() {
        let slider = ValueDrag::absolute(100.0..300.0, 0.0..=10.0);
        let value = |x: f32, rtl| slider.value(0.0, &drag(pos2(150.0, 0.0), pos2(x, 0.0), Mods::NONE), rtl);
        assert_eq!(value(200.0, false), Some(5.0));
        assert_eq!(value(500.0, false), Some(10.0));
        assert_eq!(value(0.0, false), Some(0.0));
        assert_eq!(value(250.0, true), Some(2.5), "the track mirrors under RTL");
    }

    #[test]
    fn a_vertical_track_that_runs_backwards_gives_the_minimum_at_the_bottom() {
        let slider = ValueDrag::absolute(300.0..100.0, 0.0..=10.0).axis(Axis::Y);
        assert_eq!(slider.value(0.0, &drag(pos2(0.0, 300.0), pos2(0.0, 200.0), Mods::NONE), false), Some(5.0));
    }

    #[test]
    fn shift_makes_the_travel_fine_from_the_value_at_the_start() {
        let slider = ValueDrag::absolute(100.0..300.0, 0.0..=10.0);
        let fine = slider.value(4.0, &drag(pos2(150.0, 0.0), pos2(250.0, 0.0), Mods::SHIFT), false);
        assert_eq!(fine, Some(4.0 + 5.0 * 0.1));
    }

    #[test]
    fn a_relative_drag_scales_travel_and_vertical_travel_upwards_increases() {
        let scrub = ValueDrag::relative(0.5, 0.0..=100.0);
        assert_eq!(scrub.value(10.0, &drag(pos2(0.0, 0.0), pos2(20.0, 0.0), Mods::NONE), false), Some(20.0));
        assert_eq!(scrub.value(10.0, &drag(pos2(0.0, 0.0), pos2(20.0, 0.0), Mods::NONE), true), Some(0.0), "horizontal travel mirrors under RTL");
        let vertical = ValueDrag::relative(1.0, 0.0..=100.0).axis(Axis::Y);
        assert_eq!(vertical.value(10.0, &drag(pos2(0.0, 50.0), pos2(0.0, 30.0), Mods::NONE), false), Some(30.0));
    }

    #[test]
    fn a_wrapping_drag_comes_round_the_other_end_and_a_press_that_has_not_dragged_gives_nothing() {
        let hue = ValueDrag::relative(1.0, 0.0..=360.0).wrap(true);
        assert_eq!(hue.value(350.0, &drag(pos2(0.0, 0.0), pos2(20.0, 0.0), Mods::NONE), false), Some(10.0));
        assert_eq!(hue.value(10.0, &drag(pos2(0.0, 0.0), pos2(-20.0, 0.0), Mods::NONE), false), Some(350.0));
        let start = PressStart::new(pos2(0.0, 0.0), PointerButton::Primary, 0.0, 1);
        assert_eq!(hue.value(0.0, &PressState::new(PressPhase::Down { start }), false), None);
    }
}
