//! `Units`: what typed number input and unit readouts need from the env (core 2.1): the locale's separators and the
//! unit spellings of a number's kind in the user's language, and the file sizes and durations written with them.

use craft_behavior::quantity::{Reader, is_unit_spelling};
use craft_i18n::{FileSize, Format, WordSet as _, Words};
use std::ops::RangeInclusive;
use std::sync::{Arc, OnceLock};

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
        seconds: unit "units.seconds" = "{n} s";
        pixels_per_inch: unit "units.pixelsPerInch" = "{n} ppi";
        pixels_per_centimetre: unit "units.pixelsPerCentimetre" = "{n} ppcm";
        minutes: unit "units.minutes" = "{n} min";
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
    /// `{n} ppi`.
    PixelsPerInch,
    /// `{n} ppcm`.
    PixelsPerCentimetre,
    /// `{n} min`.
    Minutes,
}

impl UnitKind {
    const ALL: [UnitKind; 12] = [
        UnitKind::Pixels,
        UnitKind::Percent,
        UnitKind::Degrees,
        UnitKind::Points,
        UnitKind::Times,
        UnitKind::Inches,
        UnitKind::Centimetres,
        UnitKind::Millimetres,
        UnitKind::Picas,
        UnitKind::PixelsPerInch,
        UnitKind::PixelsPerCentimetre,
        UnitKind::Minutes,
    ];

    /// The digits after the decimal mark a number of this kind shows by default, at most.
    pub const fn decimals(self) -> RangeInclusive<usize> {
        match self {
            UnitKind::Pixels | UnitKind::Minutes => 0..=0,
            UnitKind::Percent | UnitKind::Degrees | UnitKind::Points => 0..=1,
            UnitKind::Times
            | UnitKind::Centimetres
            | UnitKind::Millimetres
            | UnitKind::Picas
            | UnitKind::PixelsPerInch
            | UnitKind::PixelsPerCentimetre => 0..=2,
            UnitKind::Inches => 0..=3,
        }
    }

    /// What one arrow press, wheel notch or scrub point changes a number of this kind by, where the control that shows
    /// the kind steps by the kind's rule.
    pub const fn step(self) -> f64 {
        match self {
            UnitKind::Times | UnitKind::Inches | UnitKind::Centimetres => 0.1,
            _ => 1.0,
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

/// The English unit words, built once for the process.
fn english_words() -> Arc<UnitWords> {
    static ENGLISH: OnceLock<Arc<UnitWords>> = OnceLock::new();
    ENGLISH.get_or_init(|| Arc::new(UnitWords::english())).clone()
}

/// Reads and writes numbers with their unit in the user's language and digits. Build it with `Env::units` or
/// `EnvView::units`.
#[derive(Clone, Debug)]
pub struct Units {
    format: Arc<Format>,
    words: Arc<UnitWords>,
    /// The scope's words, for the rows craft-i18n owns (file sizes).
    scope_words: Arc<Words>,
    /// The readers built so far, one slot per `reader` argument (see `reader_slot`); shared by clones.
    readers: Arc<[OnceLock<Reader>; UnitKind::ALL.len() + 1]>,
}

impl Units {
    pub(crate) fn new(format: Arc<Format>, words: Arc<UnitWords>, scope_words: Arc<Words>) -> Units {
        Units { format, words, scope_words, readers: Arc::default() }
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
            UnitKind::PixelsPerInch => words.pixels_per_inch.as_str(),
            UnitKind::PixelsPerCentimetre => words.pixels_per_centimetre.as_str(),
            UnitKind::Minutes => words.minutes.as_str(),
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

    /// `n` times: `12×`.
    pub fn times(&self, n: f64) -> String {
        self.show(UnitKind::Times, n)
    }

    /// `seconds` as a duration: `0.35 s`, `12 s`, with at most two digits after the decimal mark.
    pub fn duration(&self, seconds: f64) -> String {
        let number = self.format.number_ungrouped(seconds, Self::digits(seconds, &(0..=2)));
        craft_i18n::fmt(self.words.seconds.as_str(), &[("n", &number)])
    }

    /// `bytes` as a file size in steps of 1024 with one decimal (`1.5 KB`, `12.0 MB`, `512 bytes`), the unit
    /// names and digits of the user's language.
    pub fn file_size(&self, bytes: u64) -> String {
        self.file_size_as(bytes, FileSize::BINARY)
    }

    /// `bytes` as a file size written the way `style` says.
    pub fn file_size_as(&self, bytes: u64, style: FileSize) -> String {
        style.text(bytes, &self.format, &self.scope_words)
    }

    /// `n` with digit groups and between the least and most digits of `decimals` after the decimal mark, in the
    /// user's language and digits, with no unit.
    pub fn number(&self, n: f64, decimals: RangeInclusive<usize>) -> String {
        self.format.number(n, Self::digits(n, &decimals))
    }

    /// `n` with between the least and most digits of `decimals` after the decimal mark, in the user's language and
    /// digits, with no digit groups and no unit.
    pub fn number_ungrouped(&self, n: f64, decimals: RangeInclusive<usize>) -> String {
        self.format.number_ungrouped(n, Self::digits(n, &decimals))
    }

    /// A reader for numbers of `kind` (plain numbers when `None`): the locale's decimal and group separators and
    /// the unit spellings that convert to `kind`, in the user's language and in English.
    pub fn reader(&self, kind: Option<UnitKind>) -> Reader {
        self.readers[Self::reader_slot(kind)].get_or_init(|| self.build_reader(kind)).clone()
    }

    /// The index of the reader for `kind` in `readers`: 0 for plain numbers, then one per unit kind.
    fn reader_slot(kind: Option<UnitKind>) -> usize {
        kind.map_or(0, |kind| 1 + UnitKind::ALL.iter().position(|other| *other == kind).unwrap_or(0))
    }

    fn build_reader(&self, kind: Option<UnitKind>) -> Reader {
        let (decimal, group) = self.format.separators();
        let mut reader = Reader::new().decimal(decimal);
        if let Some(group) = group {
            reader = reader.group(group);
        }
        let Some(kind) = kind else { return reader };
        let english = Units::new(self.format.clone(), english_words(), Arc::new(Words::english()));
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
