//! [`relative_age`] steps and the locale's plural forms.

use std::sync::Arc;

use craft_host::{AppInfo, Host};
use craft_i18n::{Embedded, I18n, Lang};
use egui::Context;

use super::{AgeStyle, relative_age};

const APP: AppInfo = AppInfo { id: "test", name: "Test", version: "1.0" };

static CATALOGS: &Embedded = &[("ru", "@plural\t{n} year ago|{n} years ago\tпрошёл {n} год|прошло {n} года|прошло {n} лет\n", "")];

// 30 s is "Just now", then minutes, hours, days, months and years.
#[test]
fn relative_age_steps() {
    let ctx = Context::default();
    let age = |seconds| relative_age(&ctx, seconds, AgeStyle::Long);
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

// The count is written in the locale's plural form and digits, not with `n == 1`.
#[test]
fn relative_age_uses_the_locales_plural_forms() {
    let (i18n, problems) = I18n::new(CATALOGS, &["ru"]);
    assert!(problems.is_empty(), "{problems:?}");
    let ru = i18n.from_code("ru").unwrap_or(Lang::EN);
    let ctx = Context::default();
    Host::builder(APP).i18n(Arc::new(i18n)).lang(ru).install(&ctx);
    let age = |years: u64| relative_age(&ctx, 86_400 * 365 * years, AgeStyle::Long);
    assert_eq!((age(1), age(2), age(5)), ("прошёл 1 год".to_owned(), "прошло 2 года".to_owned(), "прошло 5 лет".to_owned()));
}

// Short: 30 s "Just now", 180 s "3 min ago", 7200 s "2 h ago", from a day on the Long forms
// ("Yesterday", "4 days ago").
#[test]
fn short_ages_abbreviate_under_a_day() {
    let ctx = Context::default();
    let age = |seconds| relative_age(&ctx, seconds, AgeStyle::Short);
    assert_eq!(age(30), "Just now");
    assert_eq!(age(180), "3 min ago");
    assert_eq!(age(7200), "2 h ago");
    assert_eq!(age(86_400), "Yesterday");
    assert_eq!(age(86_400 * 4), "4 days ago");
}
