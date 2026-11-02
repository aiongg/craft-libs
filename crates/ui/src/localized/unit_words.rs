//! The unit words of numbers shown to the user: pixels, percent, degrees, points and times
//! ([`UnitKind`]), written in the user's language and digits by [`Units`]. Every readout, menu
//! and field that shows one of these quantities takes its text from here, so a translation of
//! `{n} px` reaches all of them. The text is a catalog row of the `unit` context keyed by the
//! English template, the same row [`Unit::translated`] reads for a field.

use std::ops::RangeInclusive;
use std::sync::Arc;

use egui::{Context, Ui};

use super::{LocaleNumbers, Unit, Word, Words};

/// What a number counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
}

/// The unit words, for the family `WORDS` lists.
pub const UNIT_WORDS: &[Word] = &[
    Word::in_context("unit", "{n} px"),
    Word::in_context("unit", "{n}%"),
    Word::in_context("unit", "{n}°"),
    Word::in_context("unit", "{n} pt"),
    Word::in_context("unit", "{n}×"),
];

impl UnitKind {
    /// The word with `{n}` where the number goes.
    pub const fn word(self) -> Word {
        match self {
            UnitKind::Pixels => UNIT_WORDS[0],
            UnitKind::Percent => UNIT_WORDS[1],
            UnitKind::Degrees => UNIT_WORDS[2],
            UnitKind::Points => UNIT_WORDS[3],
            UnitKind::Times => UNIT_WORDS[4],
        }
    }

    /// The digits after the decimal mark a number of this kind shows by default, at most:
    /// whole pixels, a tenth of a percent, degree or point, a hundredth of a factor.
    pub const fn decimals(self) -> RangeInclusive<usize> {
        match self {
            UnitKind::Pixels => 0..=0,
            UnitKind::Percent | UnitKind::Degrees | UnitKind::Points => 0..=1,
            UnitKind::Times => 0..=2,
        }
    }
}

/// Writes numbers with their unit in the user's language and digits. Build it with
/// [`Units::of`] once per pass.
#[derive(Clone, Debug)]
pub struct Units {
    numbers: Arc<LocaleNumbers>,
    words: Arc<Words>,
}

impl Units {
    /// The units of `numbers` and `words`.
    pub fn new(numbers: Arc<LocaleNumbers>, words: Arc<Words>) -> Units {
        Units { numbers, words }
    }

    /// The units of the installed Host: its language and digits, English without one.
    pub fn of(ui: &Ui) -> Units {
        Units::of_ctx(ui.ctx())
    }

    /// [`Units::of`] for code that has a context and no `Ui`.
    pub fn of_ctx(ctx: &Context) -> Units {
        Units::new(LocaleNumbers::of_ctx(ctx), Words::of_ctx(ctx))
    }

    /// The numbers these units write.
    pub fn numbers(&self) -> &LocaleNumbers {
        &self.numbers
    }

    /// `n` of `kind` with between the least and most digits of `decimals` after the decimal mark.
    pub fn text(&self, kind: UnitKind, n: f64, decimals: RangeInclusive<usize>) -> String {
        craft_i18n::fmt(&self.words.get(kind.word()), &[("n", &self.numbers.format(n, decimals))])
    }

    /// `n` of `kind` with its default digits ([`UnitKind::decimals`]).
    pub fn show(&self, kind: UnitKind, n: f64) -> String {
        self.text(kind, n, kind.decimals())
    }

    /// Whole pixels: `37 px`.
    pub fn pixels(&self, n: f64) -> String {
        self.show(UnitKind::Pixels, n)
    }

    /// A percentage, with a tenth when it has one: `12.5%`.
    pub fn percent(&self, n: f64) -> String {
        self.show(UnitKind::Percent, n)
    }

    /// Degrees: `45°`.
    pub fn degrees(&self, n: f64) -> String {
        self.show(UnitKind::Degrees, n)
    }

    /// Points: `12 pt`.
    pub fn points(&self, n: f64) -> String {
        self.show(UnitKind::Points, n)
    }

    /// A factor: `2×`.
    pub fn times(&self, n: f64) -> String {
        self.show(UnitKind::Times, n)
    }

    /// The unit of `kind` for a number field (`NumberField::unit`), in the user's language.
    pub fn unit(&self, kind: UnitKind) -> Unit {
        Unit::new(&self.words.get(kind.word()))
    }
}
