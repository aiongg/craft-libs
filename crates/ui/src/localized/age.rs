//! [`relative_age`]: how long ago something was, in the words and numbers of the user's locale;
//! [`relative_date`]: the same for a list column, turning into the date after a week.

use craft_i18n::{DateFormat, DateStyle, DateTime, TimeZone};
use egui::Context;

use super::words::{Counted, counts};
use super::{Labels as _, Words};

crate::words! {
    /// The words of [`relative_age`], English by default. The counted forms ("{n} minutes ago")
    /// are the plural forms of the count words `age.minutes` … `age.years`.
    #[non_exhaustive]
    pub struct AgeLabels {
        just_now: "age.justNow" = "less than a minute ago";
        minutes: "age.minutes" = "{n} minutes ago";
        hours: "age.hours" = "{n} hours ago";
        days: "age.days" = "{n} days ago";
        months: "age.months" = "{n} months ago";
        years: "age.years" = "{n} years ago";
        /// [`relative_date`] under a minute, [`AgeStyle::Long`].
        now: "age.now" = "Just now";
        /// [`relative_date`] between one and two days, [`AgeStyle::Long`].
        yesterday: "age.yesterday" = "Yesterday";
        /// [`AgeStyle::Short`]'s forms: "just now", "3 min ago", "2 h ago", "yesterday".
        now_short: "age.nowShort" = "just now";
        minutes_short: "age.minutesShort" = "{n} min ago";
        hours_short: "age.hoursShort" = "{n} h ago";
        yesterday_short: "age.yesterdayShort" = "yesterday";
    }
}

/// How [`relative_date`] words its steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AgeStyle {
    /// "Just now", "3 minutes ago", "2 hours ago", "Yesterday", "4 days ago" (EffectCraft's Home,
    /// file details).
    #[default]
    Long,
    /// "just now", "3 min ago", "2 h ago", "yesterday", "4 days ago", for narrow columns
    /// (PdfCraft's file list).
    Short,
}

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const MONTH: u64 = 30 * DAY;
const YEAR: u64 = 365 * DAY;

/// How long ago `seconds` was: "less than a minute ago" under a minute, then whole minutes,
/// hours, days, months (30 days) and years ("1 minute ago", "3 hours ago", "2 days ago"), with
/// the locale's plural forms and digits.
pub fn relative_age(ctx: &Context, seconds: u64) -> String {
    let labels = AgeLabels::of_ctx(ctx);
    let (unit, count, text): (u64, Counted, &str) = match seconds {
        0..MINUTE => return labels.just_now,
        MINUTE..HOUR => (MINUTE, counts::MINUTES_AGO, &labels.minutes),
        HOUR..DAY => (HOUR, counts::HOURS_AGO, &labels.hours),
        DAY..MONTH => (DAY, counts::DAYS_AGO, &labels.days),
        MONTH..YEAR => (MONTH, counts::MONTHS_AGO, &labels.months),
        _ => (YEAR, counts::YEARS_AGO, &labels.years),
    };
    Words::of_ctx(ctx).counted(ctx, count, text, seconds / unit)
}

/// How long before `now` the moment `then` was (both Unix seconds), for a "Modified" column or a
/// recent-files list: under 60 s (or in the future) "Just now"; under an hour whole minutes;
/// under a day whole hours; under two days "Yesterday"; under a week whole days; from a week on
/// the date, `dates.format(DateTime::from_unix(then, TimeZone::Local), DateStyle::DATE)`
/// ("Oct 31, 2026", in the region's form). EffectCraft's and PdfCraft's steps. The words follow
/// the interface language with the locale's plural forms ([`AgeLabels`], the `age.*` count
/// words); the date follows the formatting region (`I18n::dates`).
// effectcraft: crates/ui-egui/src/panels/home.rs:100 format_ago
// pdfcraft: crates/ui-egui/src/workflows/combine_ui/parts.rs:248 ago
#[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands")]
pub fn relative_date(ctx: &Context, dates: &DateFormat, now: i64, then: i64, style: AgeStyle) -> String {
    let _ = (DateStyle::DATE, DateTime::from_unix, TimeZone::Local);
    todo!()
}
