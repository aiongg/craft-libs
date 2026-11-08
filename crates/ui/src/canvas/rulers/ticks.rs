//! The tick rule of a ruler: which values carry a tick and which of them are labelled.

use craft_behavior::RulerUnits;
use std::ops::RangeInclusive;

/// Word's inch ruler: sixteenths, down to halves.
pub const INCH_SIXTEENTHS: [u32; 4] = [16, 8, 4, 2];
/// DesignCraft's inch ruler: eighths, down to halves.
pub const INCH_EIGHTHS: [u32; 3] = [8, 4, 2];

/// The least distance between labelled ticks, in points.
pub(crate) const LABEL_STEP_MIN: f32 = 60.0;

/// The metric rule's subdivisions of a labelled step.
const METRIC_SUBDIVISIONS: f32 = 10.0;
/// Labelled inch steps in inches: 1, 2, 5, 10 times the base.
const INCH_STEPS: [f64; 9] = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0];
/// The coarsest inch step, when even 500 inches are closer than the least label step.
const INCH_STEP_LAST: f64 = 1000.0;
/// Ticks beyond this many in one ruler are not produced: a custom table that asks for them has no useful picture to
/// give.
const MAX_TICKS: i64 = 20_000;

/// Which tick table [`ticks`] follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TickRule<'a> {
    /// The units' own table ([`RulerUnits::step`]): decimal steps and ten subdivisions, or the host's custom table.
    Metric,
    /// Inches: a label every inch (every 2, 5, 10 and so on when smaller), subdivided by the first of these counts,
    /// finest first, that keeps the ticks a tenth of the least label step apart ([`INCH_SIXTEENTHS`],
    /// [`INCH_EIGHTHS`]).
    Inches(&'a [u32]),
}

/// One tick of [`ticks`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RulerTick {
    /// Ruler units from the zero point.
    pub value: f64,
    /// A labelled tick: a multiple of the labelled step.
    pub major: bool,
    /// The tick halfway between two labelled ticks: not a labelled one, in a subdivision into an even count.
    pub mid: bool,
    /// The labelled step in ruler units, which the label of a major tick is written for.
    pub step: f64,
}

/// The labelled inch step and its subdivisions at `points_per_inch` screen points per inch.
fn inch_step(points_per_inch: f64, finest: &[u32]) -> (f64, u32) {
    let step_min = f64::from(LABEL_STEP_MIN);
    let step = INCH_STEPS.into_iter().find(|step| step * points_per_inch >= step_min).unwrap_or(INCH_STEP_LAST);
    if step > 1.0 {
        return (step, 2);
    }
    let gap = f64::from(LABEL_STEP_MIN / METRIC_SUBDIVISIONS);
    let subdivisions = finest.iter().copied().find(|n| points_per_inch / f64::from(*n) >= gap).unwrap_or(1);
    (step, subdivisions)
}

/// The ticks of one ruler over `values` (ruler units from the zero point): the one tick rule of every ruler, so a
/// text ruler or a page view's ruler shows the same ticks as the canvas's. One labelled step beyond each end is
/// included, since a label starts right of its tick. `zoom` is screen points per document unit along the axis, as
/// [`RulerUnits::step`] takes it. Empty when the units are unusable or the span would need more than 20,000 ticks.
pub fn ticks(units: &RulerUnits, rule: TickRule<'_>, zoom: f64, values: RangeInclusive<f64>) -> Vec<RulerTick> {
    let (step, subdivisions) = match rule {
        TickRule::Metric => units.step(zoom, LABEL_STEP_MIN),
        TickRule::Inches(finest) => inch_step(zoom / units.per_doc, finest),
    };
    let minor = step / f64::from(subdivisions);
    if !(minor.is_finite() && minor > 0.0 && units.per_doc.is_finite() && units.per_doc > 0.0) {
        return Vec::new();
    }
    let (lo, hi) = (*values.start(), *values.end());
    let subdivisions = i64::from(subdivisions);
    let (first, last) = ((lo / minor).floor() as i64 - subdivisions, (hi / minor).ceil() as i64 + subdivisions);
    if last - first > MAX_TICKS {
        return Vec::new();
    }
    (first..=last)
        .map(|m| {
            let rem = m.rem_euclid(subdivisions);
            RulerTick { value: m as f64 * minor, major: rem == 0, mid: subdivisions % 2 == 0 && rem == subdivisions / 2, step }
        })
        .collect()
}
