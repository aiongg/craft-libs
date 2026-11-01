#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Dates and times written for a formatting [`Region`] (N11.02), and fixed UTC stamps for logs
//! and file names.
//!
//! - [`DateFormat`]: dates, times, month and weekday names and the first day of the week in the
//!   region's time locale ([`Region::time`]) and clock ([`Region::hour_cycle`]), from CLDR data
//!   (ICU4X `icu_datetime`, data in `data/dates.postcard` from `data/generate.sh`). Build it once
//!   per region and keep it; [`I18n::dates`] holds one, as [`I18n::format`] does for numbers.
//! - Relative times ("3 minutes ago") are craft-ui's `localized::relative_age`: their words are
//!   craft-ui word rows, so they follow the interface language and the catalog extraction finds
//!   them. A list that shows the date after a week writes it with [`DateFormat`], which follows
//!   the region.
//! - [`utc_stamp`]: `2026-10-08 14:05` (UTC), the same in every language: render-queue logs,
//!   export file names, report headers.
//!
//! Dates are proleptic Gregorian. Years outside ±9999 are kept, written with as many digits as
//! they need (EffectCraft's and PdfCraft's 400-year-cycle rule), never collapsed to the epoch.

use std::sync::Arc;
use std::time::SystemTime;

use crate::{I18n, Region};

/// A day of the Gregorian calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate {
    pub year: i64,
    /// 1 to 12.
    pub month: u8,
    /// 1 to the month's length.
    pub day: u8,
}

impl CalendarDate {
    /// `None` when `month` or `day` is out of range for that year (`2026-02-29`).
    pub fn new(year: i64, month: u8, day: u8) -> Option<CalendarDate> {
        todo!()
    }

    /// The date `days` after 1970-01-01 (negative before it).
    // pdfcraft: crates/ui-egui/src/workflows/combine_ui/parts.rs:279 civil_from_days
    pub fn from_days(days: i64) -> CalendarDate {
        todo!()
    }

    /// Days since 1970-01-01; the inverse of [`from_days`](Self::from_days).
    pub fn days(self) -> i64 {
        todo!()
    }

    pub fn weekday(self) -> Weekday {
        todo!()
    }

    /// 28 to 31.
    pub fn days_in_month(year: i64, month: u8) -> u8 {
        todo!()
    }

    /// `n` days later (earlier when negative).
    pub fn add_days(self, n: i64) -> CalendarDate {
        CalendarDate::from_days(self.days() + n)
    }

    /// `n` months later, the day clamped to the new month's length (Jan 31 + 1 month is Feb 28
    /// or 29), as calendar pickers step months.
    pub fn add_months(self, n: i64) -> CalendarDate {
        todo!()
    }
}

/// A day of the week.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    /// Monday first (ISO 8601).
    pub const ALL: [Weekday; 7] =
        [Weekday::Monday, Weekday::Tuesday, Weekday::Wednesday, Weekday::Thursday, Weekday::Friday, Weekday::Saturday, Weekday::Sunday];
}

/// Which clock a moment is read on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimeZone {
    /// The system's offset at that moment (daylight saving included).
    #[default]
    Local,
    Utc,
}

/// A date with a time of day, on one clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime {
    pub date: CalendarDate,
    /// 0 to 23.
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateTime {
    /// The moment `unix` seconds after 1970-01-01T00:00Z, on `zone`'s clock. The local offset
    /// is the system's (chrono's `Local`; the browser's `Date` on wasm).
    // effectcraft: crates/ui-egui/src/panels/home.rs:74 utc_parts
    pub fn from_unix(unix: i64, zone: TimeZone) -> DateTime {
        todo!()
    }

    /// [`from_unix`](Self::from_unix) of a `SystemTime`; a time before 1970 counts back.
    pub fn from_system(time: SystemTime, zone: TimeZone) -> DateTime {
        todo!()
    }
}

/// How much of a date [`DateFormat::format`] writes, in the region's order and words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateLength {
    /// `10/31/26`, `31.10.26`, `2026/10/31`.
    Short,
    /// `Oct 31, 2026`, `31.10.2026`, `31 oct. 2026`.
    Medium,
    /// `October 31, 2026`, `31. Oktober 2026`.
    Long,
    /// `Saturday, October 31, 2026`.
    Full,
}

/// How much of a time [`DateFormat::format`] writes, on the region's clock (`2:05 PM`, `14:05`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimePrecision {
    Minutes,
    Seconds,
}

/// What [`DateFormat::format`] writes: a date, a time, or both joined as the locale joins them
/// (`Oct 31, 2026, 2:05 PM`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DateStyle {
    pub date: Option<DateLength>,
    pub time: Option<TimePrecision>,
}

impl DateStyle {
    /// `Oct 31, 2026`: lists and file details.
    pub const DATE: DateStyle = DateStyle { date: Some(DateLength::Medium), time: None };
    /// `Oct 31, 2026, 2:05 PM`: "Modified" columns, history rows (LightCraft's short time).
    pub const DATE_TIME: DateStyle = DateStyle { date: Some(DateLength::Medium), time: Some(TimePrecision::Minutes) };
    /// `October 31, 2026 at 2:05:11 PM`: a photo's capture time (LightCraft's display time).
    pub const LONG_DATE_TIME: DateStyle = DateStyle { date: Some(DateLength::Long), time: Some(TimePrecision::Seconds) };
    /// `2:05 PM`.
    pub const TIME: DateStyle = DateStyle { date: None, time: Some(TimePrecision::Minutes) };
}

/// How long a month or weekday name is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameLength {
    /// `S`, `O`: calendar column heads where room is short.
    Narrow,
    /// `Sat`, `Oct`.
    Abbreviated,
    /// `Saturday`, `October`.
    Wide,
}

/// How dates and times are written in a formatting region.
///
/// ```ignore
/// let dates = i18n.dates();
/// dates.format(DateTime::from_system(modified, TimeZone::Local), DateStyle::DATE_TIME); // "Oct 31, 2026, 2:05 PM"
/// dates.month_name(10, NameLength::Wide);                                               // "October", "Oktober"
/// dates.first_weekday();                                                                // Sunday in en-US, Monday in de-DE
/// ```
#[derive(Clone, Debug)]
pub struct DateFormat {
    /// The time locale, BCP 47.
    locale: String,
    /// The region's clock where the system sets one apart from the locale.
    hour_cycle: Option<crate::HourCycle>,
}

impl DateFormat {
    /// The format for `region`'s time locale and clock.
    pub fn new(region: &Region) -> DateFormat {
        Self { locale: region.time.clone(), hour_cycle: region.hour_cycle }
    }

    /// The format for a locale tag (`de-CH`), with the locale's clock.
    pub fn from_tag(tag: &str) -> DateFormat {
        Self::new(&Region::from_tag(tag))
    }

    /// `at` written as `style` says. A style with neither part is empty.
    // new: ICU4X DateTimeFormatter with the region's hour cycle; the apps write English only (lightcraft: crates/ui-egui/src/i18n.rs:445 display_time)
    pub fn format(&self, at: DateTime, style: DateStyle) -> String {
        todo!()
    }

    /// The name of `month` (1 to 12) as it stands alone (a calendar's title): `October`,
    /// `październik`.
    pub fn month_name(&self, month: u8, length: NameLength) -> String {
        todo!()
    }

    /// The name of `day` as it stands alone: `Saturday`, `Sa`.
    pub fn weekday_name(&self, day: Weekday, length: NameLength) -> String {
        todo!()
    }

    /// The day a week starts on in the region: Sunday in the United States, Monday in most of
    /// Europe, Saturday in parts of the Middle East.
    pub fn first_weekday(&self) -> Weekday {
        todo!()
    }

    /// A date typed in the region's short or medium form (`10/31/2026`, `31.10.2026`,
    /// `2026-10-31` always), for a date field. Two-digit years are 2000 to 2099. `None` when the
    /// text isn't a valid date.
    pub fn parse_date(&self, text: &str) -> Option<CalendarDate> {
        todo!()
    }
}

impl I18n {
    /// How dates are written: [`Region::system`]'s until [`I18n::set_dates`], and again after
    /// [`I18n::reset_dates`].
    // new: the counterpart of I18n::format for numbers
    pub fn dates(&self) -> Arc<DateFormat> {
        todo!()
    }

    /// Write dates as `dates` does from now on: the user's region setting.
    pub fn set_dates(&self, dates: DateFormat) {
        todo!()
    }

    /// Write dates as the system region does again.
    pub fn reset_dates(&self) {
        todo!()
    }
}

/// `unix` seconds as `YYYY-MM-DD hh:mm` (or `hh:mm:ss` with [`TimePrecision::Seconds`]), UTC,
/// with no zone suffix: the same in every language. Years past 9999 keep all their digits
/// (`{y:04}`, never `%Y`'s `+`).
// effectcraft: crates/ui-egui/src/panels/home.rs:92 format_utc, crates/ui-egui/src/panels/render_queue.rs:62 utc_stamp
pub fn utc_stamp(unix: i64, precision: TimePrecision) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    #![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

    // 1970-01-01 is day 0 and a Thursday; 2000-02-29 exists and 2100-02-29 doesn't; from_days
    // and days round-trip across ±400-year cycles and years past 9999; Jan 31 + 1 month is Feb 29
    // in 2028.
    #[test]
    #[ignore = "skeleton"]
    fn calendar_dates_round_trip_and_step() {
        todo!()
    }

    // en-US writes "Oct 31, 2026, 2:05 PM", de-DE "31.10.2026, 14:05", en-US with HourCycle::H23
    // "14:05"; month and weekday names stand alone; en-US weeks start on Sunday and de-DE on Monday.
    #[test]
    #[ignore = "skeleton"]
    fn dates_follow_the_region() {
        todo!()
    }

    // 0 is "1970-01-01 00:00", seconds add ":00", a year past 9999 keeps its digits.
    #[test]
    #[ignore = "skeleton"]
    fn utc_stamps_are_fixed_width_and_language_free() {
        todo!()
    }
}
