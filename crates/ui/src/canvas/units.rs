//! [`RulerUnits`]: how rulers, guide readouts and guide values measure the document along one
//! axis.

use std::ops::RangeInclusive;

use crate::localized::{LocaleNumbers, Unit};

/// How a ruler divides its unit at a zoom.
#[derive(Clone, Copy, Debug)]
pub enum Ticks {
    /// PhotoCraft's decimal steps: the labelled step is the smallest of 1, 2, 2.5 and 5 × 10ⁿ
    /// units that leaves `ruler.step_min` points between labels, in 10 subdivisions. `whole`
    /// keeps the step a whole number of units without 2.5 (pixels).
    Decimal { whole: bool },
    /// The host's table: screen points per unit to (labelled step in units, subdivisions), for
    /// DesignCraft's eighths of an inch and picas in sixths.
    Custom(fn(f64) -> (f64, u32)),
}

/// The ruler unit of one document axis: its size, its ticks and how its numbers are written.
/// Rulers label their ticks with it; guides use it for the readout of a drag, their AccessKit
/// values and Shift's snap to the ticks.
#[derive(Clone, Debug)]
pub struct RulerUnits {
    /// Ruler units per document unit along this axis (percent is per axis).
    pub per_doc: f64,
    pub ticks: Ticks,
    /// The host's number format.
    pub numbers: LocaleNumbers,
    /// The unit around a number (`{n} px`), translated by the host.
    pub unit: Unit,
    /// Decimals of a value in this unit (PhotoCraft: px 0, pt and % 1, mm, cm and pica 2, in 3).
    pub decimals: usize,
}

impl RulerUnits {
    /// Whole document pixels, written with `numbers`.
    pub fn pixels(numbers: LocaleNumbers) -> Self {
        Self { per_doc: 1.0, ticks: Ticks::Decimal { whole: true }, numbers, unit: Unit::new("{n} px"), decimals: 0 }
    }

    /// The labelled step in ruler units and its subdivisions when one document unit is `zoom`
    /// screen points along this axis, with labels at least `step_min` points apart.
    pub fn step(&self, zoom: f64, step_min: f32) -> (f64, u32) {
        let points_per_unit = zoom / self.per_doc;
        match self.ticks {
            Ticks::Decimal { whole } => (decimal_step(f64::from(step_min) / points_per_unit, whole), 10),
            Ticks::Custom(table) => {
                let (step, subdivisions) = table(points_per_unit);
                (step, subdivisions.max(1))
            }
        }
    }

    /// `position` (document units) moved to the nearest minor tick of the ruler counting from
    /// `zero`, at `zoom` screen points per document unit (PhotoCraft's Shift snap, #3770).
    pub fn snap_to_tick(&self, position: f64, zero: f64, zoom: f64, step_min: f32) -> f64 {
        let (step, subdivisions) = self.step(zoom, step_min);
        let minor = step / f64::from(subdivisions);
        if !(minor.is_finite() && minor > 0.0 && self.per_doc > 0.0) {
            return position;
        }
        let units = (position - zero) * self.per_doc;
        zero + (units / minor).round() * minor / self.per_doc
    }

    /// The distance of `position` (document units) from `zero` in this unit, written with the
    /// unit: `103.5 px`.
    pub fn format(&self, position: f64, zero: f64, decimals: RangeInclusive<usize>) -> String {
        let number = self.numbers.format((position - zero) * self.per_doc, decimals);
        format!("{}{number}{}", self.unit.prefix(), self.unit.suffix())
    }

    /// The number a ruler prints at the tick `value` (ruler units from the zero point): the
    /// distance, without a minus sign left of or above the zero point, as a whole number when
    /// the labelled `step` is at least 1 or the ticks are whole (pixels), else with two decimals
    /// (PhotoCraft's `ruler_label`). Written with the host's numbers, ungrouped.
    pub fn tick_label(&self, value: f64, step: f64) -> String {
        let whole = matches!(self.ticks, Ticks::Decimal { whole: true });
        let (value, decimals) = if step >= 1.0 || whole { (value.abs().round(), 0) } else { (value.abs(), 2) };
        self.numbers.clone().ungrouped().format(value, decimals..=decimals)
    }

    /// The unit's short name (`px`): the template without its number.
    pub fn unit_name(&self) -> String {
        format!("{}{}", self.unit.prefix(), self.unit.suffix()).trim().to_owned()
    }

    /// [`format`](Self::format) for a readout or a value: at least one decimal, so pixels show
    /// tenths (PhotoCraft's `X: 103.2 px`), other units at their `decimals`.
    pub fn format_value(&self, position: f64, zero: f64) -> String {
        let decimals = self.decimals.max(1);
        self.format(position, zero, decimals..=decimals)
    }
}

/// The smallest of 1, 2, 2.5 and 5 × 10ⁿ at least `min`; whole steps (no 2.5, at least 1) with
/// `whole` (PhotoCraft's `nice_step`).
fn decimal_step(min: f64, whole: bool) -> f64 {
    if !min.is_finite() || min <= 0.0 {
        return 1.0;
    }
    let mut step = if whole { 1.0 } else { 10f64.powi(min.log10().floor() as i32 - 1) };
    loop {
        for m in [1.0, 2.0, 2.5, 5.0] {
            if (m != 2.5 || !whole) && step * m >= min {
                return step * m;
            }
        }
        step *= 10.0;
    }
}
