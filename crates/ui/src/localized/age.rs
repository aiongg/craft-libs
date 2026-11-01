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
        /// [`AgeStyle::Short`]'s forms under a day: "just now", "3 min ago", "2 h ago".
        now_short: "age.nowShort" = "just now";
        minutes_short: "age.minutesShort" = "{n} min ago";
        hours_short: "age.hoursShort" = "{n} h ago";
    }
}

/// How [`relative_age`] words its steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AgeStyle {
    /// "less than a minute ago", "3 minutes ago", "2 hours ago", "4 days ago", "2 months ago",
    /// "1 year ago" (the recovery dialog, the start screen, EffectCraft's Home).
    #[default]
    Long,
    /// "just now", "3 min ago", "2 h ago", then as [`Long`](Self::Long) from a day on, for
    /// narrow columns (PdfCraft's file list).
    Short,
}

/// From this age on a list column shows the date instead (EffectCraft's and PdfCraft's rule):
/// seven days. The caller writes the date with craft-i18n's `DateFormat`:
///
/// ```ignore
/// let text = if age < AGE_SHOWN_AS_DATE { relative_age(ctx, age, AgeStyle::Short) }
///            else { i18n.dates().format(DateTime::from_unix(then, TimeZone::Local), DateStyle::DATE) };
/// ```
pub const AGE_SHOWN_AS_DATE: u64 = 7 * DAY;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const MONTH: u64 = 30 * DAY;
const YEAR: u64 = 365 * DAY;

/// How long ago `seconds` was, in `style`. [`AgeStyle::Long`]: "less than a minute ago" under
/// a minute, then whole minutes, hours, days, months (30 days) and years ("1 minute ago",
/// "3 hours ago", "2 days ago"). [`AgeStyle::Short`] writes "just now", "{n} min ago" and
/// "{n} h ago" under a day and the long forms after. Both use the locale's plural forms and
/// digits. A list that shows the date after a week compares with [`AGE_SHOWN_AS_DATE`].
// effectcraft: crates/ui-egui/src/panels/home.rs:100 format_ago
// pdfcraft: crates/ui-egui/src/workflows/combine_ui/parts.rs:248 ago
pub fn relative_age(ctx: &Context, seconds: u64, style: AgeStyle) -> String {
    if style == AgeStyle::Short && seconds < DAY {
        return short_age(ctx, seconds);
    }
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

/// [`AgeStyle::Short`] under a day: `now_short` under a minute, then `minutes_short`
/// (`counts::MINUTES_AGO_SHORT`) and `hours_short` (`counts::HOURS_AGO_SHORT`) with the locale's
/// plural forms.
// pdfcraft: crates/ui-egui/src/workflows/combine_ui/parts.rs:248 ago
#[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands")]
fn short_age(ctx: &Context, seconds: u64) -> String {
    todo!()
}
