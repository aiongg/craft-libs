//! How long ago something was, in the words and numbers of the interface language
//! ("Just now", "3 minutes ago", "Yesterday", "2 months ago").
//!
//! The words are catalog rows: the ids `age.justNow`, `age.yesterday` and `age.nowShort`, and the
//! `@plural` rows of the counted forms (`{n} minute ago|{n} minutes ago`, hours, days, months,
//! years, and the short minute and hour forms). [`I18n::relative_age`] reads the catalogs;
//! [`relative_age`] reads a UI scope's [`Words`] and [`Format`].

use crate::{Counted, Format, I18n, Lang, Word, Words, ltr};

const JUST_NOW: Word = Word::keyed("age.justNow", "Just now");
const YESTERDAY: Word = Word::keyed("age.yesterday", "Yesterday");
const NOW_SHORT: Word = Word::keyed("age.nowShort", "Just now");
const MINUTES: Counted = Counted::new("age.minutes", "{n} minute ago", "{n} minutes ago");
const HOURS: Counted = Counted::new("age.hours", "{n} hour ago", "{n} hours ago");
const DAYS: Counted = Counted::new("age.days", "{n} day ago", "{n} days ago");
const MONTHS: Counted = Counted::new("age.months", "{n} month ago", "{n} months ago");
const YEARS: Counted = Counted::new("age.years", "{n} year ago", "{n} years ago");
const MINUTES_SHORT: Counted = Counted::new("age.minutesShort", "{n} min ago", "{n} min ago");
const HOURS_SHORT: Counted = Counted::new("age.hoursShort", "{n} h ago", "{n} h ago");

/// How [`relative_age`] words its steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AgeStyle {
    /// "Just now", "3 minutes ago", "2 hours ago", "Yesterday", "4 days ago", "2 months ago",
    /// "1 year ago" (the recovery dialog, the start screen, a home page's recent files).
    #[default]
    Long,
    /// "Just now", "3 min ago", "2 h ago", then as [`Long`](Self::Long) from a day on, for
    /// narrow columns (a file list).
    Short,
}

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const TWO_DAYS: u64 = 2 * DAY;
const MONTH: u64 = 30 * DAY;
const YEAR: u64 = 365 * DAY;

/// From this age on a list column shows the date instead: seven days. The caller writes the date
/// with [`DateFormat`](crate::DateFormat):
///
/// ```ignore
/// let text = if age < AGE_SHOWN_AS_DATE { relative_age(&words, &format, age, AgeStyle::Short) }
///            else { i18n.dates().format(DateTime::from_unix(then, TimeZone::Local), DateStyle::DATE) };
/// ```
pub const AGE_SHOWN_AS_DATE: u64 = 7 * DAY;

/// The step `seconds` falls in: a word alone, or a count of whole units.
enum Step {
    Word(Word),
    Count(Counted, u64),
}

fn step(seconds: u64, style: AgeStyle) -> Step {
    if style == AgeStyle::Short && seconds < DAY {
        return match seconds {
            0..MINUTE => Step::Word(NOW_SHORT),
            MINUTE..HOUR => Step::Count(MINUTES_SHORT, seconds / MINUTE),
            _ => Step::Count(HOURS_SHORT, seconds / HOUR),
        };
    }
    match seconds {
        0..MINUTE => Step::Word(JUST_NOW),
        MINUTE..HOUR => Step::Count(MINUTES, seconds / MINUTE),
        HOUR..DAY => Step::Count(HOURS, seconds / HOUR),
        DAY..TWO_DAYS => Step::Word(YESTERDAY),
        TWO_DAYS..MONTH => Step::Count(DAYS, seconds / DAY),
        MONTH..YEAR => Step::Count(MONTHS, seconds / MONTH),
        _ => Step::Count(YEARS, seconds / YEAR),
    }
}

/// How long ago `seconds` was, in `style`, in the language of `words` and the digits of `format`.
/// [`AgeStyle::Long`]: "Just now" under a minute, then whole minutes, hours, "Yesterday" for one to
/// two days, then whole days, months (30 days) and years. [`AgeStyle::Short`] writes "Just now",
/// "{n} min ago" and "{n} h ago" under a day and the long forms after. Counts use the locale's
/// plural forms. A list that shows the date after a week compares with [`AGE_SHOWN_AS_DATE`].
pub fn relative_age(words: &Words, format: &Format, seconds: u64, style: AgeStyle) -> String {
    match step(seconds, style) {
        Step::Word(word) => words.get(word).into_owned(),
        Step::Count(count, n) => words.counted(count, n, &ltr(&format.integer(n))),
    }
}

impl I18n {
    /// [`relative_age`] in `lang` with the catalogs and number format of this `I18n`.
    pub fn relative_age(&self, lang: Lang, seconds: u64, style: AgeStyle) -> String {
        match step(seconds, style) {
            Step::Word(word) => self.tid(lang, word.id, word.english).to_owned(),
            Step::Count(count, n) => self.trn(lang, n, count.one, count.word.english),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Embedded;

    static CATALOGS: &Embedded = &[("ru", "@plural\t{n} year ago|{n} years ago\tпрошёл {n} год|прошло {n} года|прошло {n} лет\n", "")];

    // 30 s is "Just now", then minutes, hours, days, months and years.
    #[test]
    fn relative_age_steps() {
        let (i18n, _) = I18n::new(&[], &[]);
        let age = |seconds| i18n.relative_age(Lang::EN, seconds, AgeStyle::Long);
        assert_eq!(age(30), "Just now");
        assert_eq!(age(61), "1 minute ago");
        assert_eq!(age(3_600), "1 hour ago");
        assert_eq!(age(7200), "2 hours ago");
        assert_eq!(age(86_400), "Yesterday");
        assert_eq!(age(86_400 * 2 - 1), "Yesterday");
        assert_eq!(age(86_400 * 2), "2 days ago");
        assert_eq!(age(86_400 * 30), "1 month ago");
        assert_eq!(age(86_400 * 90), "3 months ago");
        assert_eq!(age(86_400 * 365), "1 year ago");
        assert_eq!(age(86_400 * 365 * 3), "3 years ago");
    }

    // The count is written in the locale's plural form, not with `n == 1`, by the catalogs and by a
    // scope's words alike.
    #[test]
    fn relative_age_uses_the_locales_plural_forms() {
        let (i18n, problems) = I18n::new(CATALOGS, &["ru"]);
        assert!(problems.is_empty(), "{problems:?}");
        let ru = i18n.from_code("ru").unwrap_or(Lang::EN);
        let years = |years: u64| i18n.relative_age(ru, 86_400 * 365 * years, AgeStyle::Long);
        assert_eq!((years(1), years(2), years(5)), ("прошёл 1 год".to_owned(), "прошло 2 года".to_owned(), "прошло 5 лет".to_owned()));
        let format = i18n.format();
        let words = Words::from_i18n(std::sync::Arc::new(i18n), ru);
        let in_words = |years: u64| relative_age(&words, &format, 86_400 * 365 * years, AgeStyle::Long);
        assert_eq!((in_words(1), in_words(2), in_words(5)), ("прошёл 1 год".to_owned(), "прошло 2 года".to_owned(), "прошло 5 лет".to_owned()));
    }

    // Short: 30 s "Just now", 180 s "3 min ago", 7200 s "2 h ago", from a day on the long forms.
    #[test]
    fn short_ages_abbreviate_under_a_day() {
        let format = Format::from_tag("en-US");
        let words = Words::english();
        let age = |seconds| relative_age(&words, &format, seconds, AgeStyle::Short);
        assert_eq!(age(30), "Just now");
        assert_eq!(age(180), "3 min ago");
        assert_eq!(age(7200), "2 h ago");
        assert_eq!(age(86_400), "Yesterday");
        assert_eq!(age(86_400 * 4), "4 days ago");
    }
}
