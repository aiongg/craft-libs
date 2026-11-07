//! `Units`: what typed number input and unit readouts need from the env (core 2.1): the locale's separators and the
//! unit spellings of a number's kind in the user's language.

use craft_behavior::quantity::{Reader, is_unit_spelling};
use craft_i18n::{Format, WordSet as _};
use std::ops::RangeInclusive;
use std::sync::Arc;

craft_ui::words! {
    /// The unit words: `{n}` is where the number goes.
    pub struct UnitWords {
        pixels: unit "units.pixels" = "{n} px";
        percent: unit "units.percent" = "{n}%";
        degrees: unit "units.degrees" = "{n}°";
        points: unit "units.points" = "{n} pt";
        times: unit "units.times" = "{n}×";
        inches: unit "units.inches" = "{n} in";
        centimetres: unit "units.centimetres" = "{n} cm";
        millimetres: unit "units.millimetres" = "{n} mm";
        picas: unit "units.picas" = "{n} pica";
    }
}

/// What a number counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UnitKind {
    /// `{n} px`.
    Pixels,
    /// `{n}%`.
    Percent,
    /// `{n}°`.
    Degrees,
    /// `{n} pt`.
    Points,
    /// `{n}×`.
    Times,
    /// `{n} in`.
    Inches,
    /// `{n} cm`.
    Centimetres,
    /// `{n} mm`.
    Millimetres,
    /// `{n} pica`.
    Picas,
}

impl UnitKind {
    const ALL: [UnitKind; 9] = [
        UnitKind::Pixels,
        UnitKind::Percent,
        UnitKind::Degrees,
        UnitKind::Points,
        UnitKind::Times,
        UnitKind::Inches,
        UnitKind::Centimetres,
        UnitKind::Millimetres,
        UnitKind::Picas,
    ];

    /// The digits after the decimal mark a number of this kind shows by default, at most.
    pub const fn decimals(self) -> RangeInclusive<usize> {
        match self {
            UnitKind::Pixels => 0..=0,
            UnitKind::Percent | UnitKind::Degrees | UnitKind::Points => 0..=1,
            UnitKind::Times | UnitKind::Centimetres | UnitKind::Millimetres | UnitKind::Picas => 0..=2,
            UnitKind::Inches => 0..=3,
        }
    }

    /// The length of one unit of this kind in millimetres, for the kinds that measure length.
    fn millimetres(self) -> Option<f64> {
        match self {
            UnitKind::Inches => Some(25.4),
            UnitKind::Centimetres => Some(10.0),
            UnitKind::Millimetres => Some(1.0),
            UnitKind::Points => Some(25.4 / 72.0),
            UnitKind::Picas => Some(25.4 / 6.0),
            _ => None,
        }
    }
}

/// Reads and writes numbers with their unit in the user's language and digits. Build it with `Env::units` or
/// `EnvView::units`.
#[derive(Clone, Debug)]
pub struct Units {
    format: Arc<Format>,
    words: Arc<UnitWords>,
}

impl Units {
    pub(crate) fn new(format: Arc<Format>, words: Arc<UnitWords>) -> Units {
        Units { format, words }
    }

    fn template(&self, kind: UnitKind) -> &str {
        let words = &*self.words;
        match kind {
            UnitKind::Pixels => words.pixels.as_str(),
            UnitKind::Percent => words.percent.as_str(),
            UnitKind::Degrees => words.degrees.as_str(),
            UnitKind::Points => words.points.as_str(),
            UnitKind::Times => words.times.as_str(),
            UnitKind::Inches => words.inches.as_str(),
            UnitKind::Centimetres => words.centimetres.as_str(),
            UnitKind::Millimetres => words.millimetres.as_str(),
            UnitKind::Picas => words.picas.as_str(),
        }
    }

    /// The unit's spelling in the user's language without the number: `mm`, `px`.
    pub fn symbol(&self, kind: UnitKind) -> String {
        self.template(kind).replace("{n}", "").trim().to_owned()
    }

    /// The digits after the decimal mark `n` needs within `decimals`: the fewest that show it as it rounds at the most.
    fn digits(n: f64, decimals: &RangeInclusive<usize>) -> usize {
        let (least, most) = (*decimals.start(), *decimals.end());
        let scale = 10f64.powi(i32::try_from(most).unwrap_or(0));
        let target = (n * scale).round();
        (least..=most)
            .find(|d| {
                let at = 10f64.powi(i32::try_from(*d).unwrap_or(0));
                ((n * at).round() / at * scale).round() == target
            })
            .unwrap_or(most)
    }

    /// `n` of `kind` with between the least and most digits of `decimals` after the decimal mark, in the user's
    /// language and digits.
    pub fn text(&self, kind: UnitKind, n: f64, decimals: RangeInclusive<usize>) -> String {
        let number = self.format.number_ungrouped(n, Self::digits(n, &decimals));
        craft_i18n::fmt(self.template(kind), &[("n", &number)])
    }

    /// `n` of `kind` with its default digits ([`UnitKind::decimals`]).
    pub fn show(&self, kind: UnitKind, n: f64) -> String {
        self.text(kind, n, kind.decimals())
    }

    /// `n` pixels: `12 px`.
    pub fn pixels(&self, n: f64) -> String {
        self.show(UnitKind::Pixels, n)
    }

    /// `n` percent: `12%`.
    pub fn percent(&self, n: f64) -> String {
        self.show(UnitKind::Percent, n)
    }

    /// `n` degrees: `12°`.
    pub fn degrees(&self, n: f64) -> String {
        self.show(UnitKind::Degrees, n)
    }

    /// `n` with digit groups and between the least and most digits of `decimals` after the decimal mark, in the
    /// user's language and digits, with no unit.
    pub fn number(&self, n: f64, decimals: RangeInclusive<usize>) -> String {
        self.format.number(n, Self::digits(n, &decimals))
    }

    /// A reader for numbers of `kind` (plain numbers when `None`): the locale's decimal and group separators and
    /// the unit spellings that convert to `kind`, in the user's language and in English.
    pub fn reader(&self, kind: Option<UnitKind>) -> Reader {
        let (decimal, group) = self.format.separators();
        let mut reader = Reader::new().decimal(decimal);
        if let Some(group) = group {
            reader = reader.group(group);
        }
        let Some(kind) = kind else { return reader };
        let english = UnitWords::english();
        let english = Units { format: self.format.clone(), words: Arc::new(english) };
        for other in UnitKind::ALL {
            let factor = match (kind.millimetres(), other.millimetres()) {
                (Some(of), Some(per)) => per / of,
                _ if other == kind => 1.0,
                _ => continue,
            };
            for spelling in [self.symbol(other), english.symbol(other)] {
                if is_unit_spelling(&spelling) {
                    reader = reader.unit(&spelling, factor);
                }
            }
        }
        reader
    }
}
