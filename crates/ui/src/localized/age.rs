//! [`relative_age`]: how long ago something was, in the words and numbers of the user's locale.

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
    }
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
