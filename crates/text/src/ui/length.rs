//! One owner for a length kept in points and shown in the document's unit: the unit type the text controls take,
//! and the number field over it.

use craft_behavior::{EditEvent, Mixed, NumberCodec};
use craft_ui::contract::Bind;
use craft_ui::runtime::env::{Label, UnitKind, Units};
use craft_ui::text::NumberField;
use std::ops::RangeInclusive;

/// The unit a text control shows its lengths in. Lengths are stored in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DocUnit {
    /// What the shown number counts: a length kind or `Pixels`.
    pub kind: UnitKind,
    /// The document's resolution, which sizes `Pixels`.
    pub pixels_per_inch: f64,
}

impl DocUnit {
    /// `kind` (a length kind or `Pixels`), with the document's resolution for pixels.
    pub const fn new(kind: UnitKind, pixels_per_inch: f64) -> Self {
        DocUnit { kind, pixels_per_inch }
    }

    /// Points at 72 pixels per inch: the default of every control.
    pub const fn points() -> Self {
        DocUnit { kind: UnitKind::Points, pixels_per_inch: UnitKind::POINTS_PER_INCH }
    }

    /// How many of this unit make one point; 1 for a kind that is not a length.
    fn per_point(self, points_per_inch: f64) -> f64 {
        if !(points_per_inch.is_finite() && points_per_inch > 0.0) {
            return 1.0;
        }
        let per_inch = match self.kind {
            UnitKind::Pixels => Some(self.pixels_per_inch),
            kind => kind.per_inch(points_per_inch),
        };
        per_inch.filter(|per| per.is_finite() && *per > 0.0).map_or(1.0, |per| per / points_per_inch)
    }

    /// `points` in this unit; `points_per_inch` comes from `Units::points_per_inch()`.
    pub fn shown(self, points: f64, points_per_inch: f64) -> f64 {
        points * self.per_point(points_per_inch)
    }

    /// `shown` in this unit back to points.
    pub fn to_points(self, shown: f64, points_per_inch: f64) -> f64 {
        shown / self.per_point(points_per_inch)
    }
}

impl Default for DocUnit {
    fn default() -> Self {
        DocUnit::points()
    }
}

/// A `NumberField` over a length kept in points: it shows `points` in `unit`, reads typed units ("2 cm", "12 pt",
/// expressions) through `Units::reader_with_points`, and keeps `range` (points) as the shown range.
pub(crate) fn length_field<'a>(
    units: &Units,
    label: impl Into<Label>,
    unit: DocUnit,
    points: Mixed<Option<f64>>,
    range: RangeInclusive<f64>,
) -> NumberField<'a> {
    let ppi = units.points_per_inch();
    let shown = match points {
        Mixed::Same(value) => Mixed::Same(value.map(|value| unit.shown(value, ppi))),
        Mixed::Mixed => Mixed::Mixed,
    };
    let (a, b) = (unit.shown(*range.start(), ppi), unit.shown(*range.end(), ppi));
    let codec = NumberCodec::new(units.reader_with_points(Some(unit.kind), ppi), *unit.kind.decimals().end());
    NumberField::new(label, Bind::given(shown)).codec(codec).range(a.min(b)..=a.max(b)).step(unit.kind.step()).unit(units.symbol(unit.kind))
}

/// The field's edit back in points.
pub(crate) fn length_edit(units: &Units, unit: DocUnit, edit: EditEvent<Option<f64>>) -> EditEvent<Option<f64>> {
    let ppi = units.points_per_inch();
    edit.map(|value| value.map(|shown| unit.to_points(shown, ppi)))
}
