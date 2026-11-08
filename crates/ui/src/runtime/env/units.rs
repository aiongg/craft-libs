//! `Units`: what typed number input and unit readouts need from the env (core 2.1): the locale's separators and the
//! unit spellings of a number's kind in the user's language, and the file sizes and durations written with them.

use super::EnvValue;
use craft_behavior::quantity::{Reader, is_unit_spelling};
use craft_i18n::{FileSize, Format, WordSet as _, Words};
use serde::{Deserialize, Serialize};
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

/// The size of a point, the Point/Pica Size of the Units preference. An app sets it once on its root env with
/// `RootEnv::values`; `Units` reads it for typed Points and Picas and for their conversion to other lengths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum PointSize {
    /// 72 points to the inch.
    #[default]
    PostScript,
    /// 72.27 points to the inch.
    Traditional,
}

impl PointSize {
    /// How many points make an inch.
    pub const fn points_per_inch(self) -> f64 {
        match self {
            PointSize::PostScript => UnitKind::POINTS_PER_INCH,
            PointSize::Traditional => 72.27,
        }
    }
}

impl EnvValue for PointSize {
    fn root() -> Self {
        PointSize::default()
    }
}

/// What a number counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    /// The points in an inch of PostScript points, the default of Points and Picas.
    pub const POINTS_PER_INCH: f64 = 72.0;

    /// How many units of this kind make an inch, for the kinds that measure length; `points_per_inch` sets the size
    /// of Points and Picas (72, or 72.27 for traditional points). `None` for a kind that is not a length.
    pub fn per_inch(self, points_per_inch: f64) -> Option<f64> {
        match self {
            UnitKind::Inches => Some(1.0),
            UnitKind::Centimetres => Some(2.54),
            UnitKind::Millimetres => Some(25.4),
            UnitKind::Points => Some(points_per_inch),
            UnitKind::Picas => Some(points_per_inch / 12.0),
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
    /// The scope's point size: what `reader` and `points_per_inch` use for Points and Picas.
    point_size: PointSize,
}

impl Units {
    pub(crate) fn new(format: Arc<Format>, words: Arc<UnitWords>, scope_words: Arc<Words>, point_size: PointSize) -> Units {
        Units { format, words, scope_words, readers: Arc::default(), point_size }
    }

    /// How many points make an inch in the scope: the app's point-size preference.
    pub fn points_per_inch(&self) -> f64 {
        self.point_size.points_per_inch()
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

    /// A zoom of `n` percent: `100%`, `12.5%`, `66.67%`. The fewest digits that show it to two decimals, so the
    /// ladder's thirds read as Photoshop shows them.
    pub fn zoom_percent(&self, n: f64) -> String {
        self.text(UnitKind::Percent, n, 0..=2)
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
        self.readers[Self::reader_slot(kind)].get_or_init(|| self.build_reader(kind, self.points_per_inch())).clone()
    }

    /// Like [`reader`](Units::reader), with Points and Picas of `points_per_inch` to the inch, so a typed `12pt` is the
    /// length a dialog's own point size converts.
    pub fn reader_with_points(&self, kind: Option<UnitKind>, points_per_inch: f64) -> Reader {
        if points_per_inch == self.points_per_inch() { self.reader(kind) } else { self.build_reader(kind, points_per_inch) }
    }

    /// The index of the reader for `kind` in `readers`: 0 for plain numbers, then one per unit kind.
    fn reader_slot(kind: Option<UnitKind>) -> usize {
        kind.map_or(0, |kind| 1 + UnitKind::ALL.iter().position(|other| *other == kind).unwrap_or(0))
    }

    fn build_reader(&self, kind: Option<UnitKind>, points_per_inch: f64) -> Reader {
        let (decimal, group) = self.format.separators();
        let mut reader = Reader::new().decimal(decimal);
        if let Some(group) = group {
            reader = reader.group(group);
        }
        let Some(kind) = kind else { return reader };
        let english = Units::new(self.format.clone(), english_words(), Arc::new(Words::english()), self.point_size);
        for other in UnitKind::ALL {
            let factor = match (kind.per_inch(points_per_inch), other.per_inch(points_per_inch)) {
                (Some(of), Some(per)) => of / per,
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
