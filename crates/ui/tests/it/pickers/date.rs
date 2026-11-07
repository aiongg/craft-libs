//! Date values, codec, calendar maths and the calendar's keys.

use craft_behavior::{Codec, NavMove};
use craft_i18n::{DateFormat, Format, Weekday};
use craft_ui::pickers::{CalendarView, DateCodec, DatePrecision, PickedDate, month_grid};
use std::sync::Arc;

const YEAR: i32 = 2026;
const JANUARY: u8 = 1;
const FEBRUARY: u8 = 2;
/// February 2026 has 28 days and starts on a Sunday.
const FEBRUARY_DAYS: u8 = 28;
const LAST_JANUARY_DAY: u8 = 31;
const WEEK: i32 = 7;

fn dates() -> Arc<DateFormat> {
    Arc::new(DateFormat::from_tag("en-US"))
}

/// A value reads as its year, month or day, past a trailing time, and a date that does not exist is none.
#[craft_ui::testing::trial]
fn picked_dates_parse_at_their_precision() {
    assert_eq!(PickedDate::parse("2026"), Some(PickedDate::year(YEAR)));
    assert_eq!(PickedDate::parse("2026-08"), Some(PickedDate::month(YEAR, 8)));
    assert_eq!(PickedDate::parse("2026-08-14"), Some(PickedDate::day(YEAR, 8, 14)));
    assert_eq!(PickedDate::parse("2026-08-14T10:30"), Some(PickedDate::day(YEAR, 8, 14)));
    assert_eq!(PickedDate::parse("2026-02-30"), None);
    assert_eq!(PickedDate::parse("2026-8"), None);
    assert_eq!(PickedDate::year(7).iso(), "0007");
}

/// A month or year is in bounds when any of its days is.
#[craft_ui::testing::trial]
fn bounds_admit_a_period_when_any_of_its_days_is_inside() {
    let august = PickedDate::month(YEAR, 8);
    assert!(august.in_bounds(Some(&PickedDate::day(YEAR, 8, 31)), None));
    assert!(!august.in_bounds(Some(&PickedDate::day(YEAR, 9, 1)), None));
    assert!(august.in_bounds(None, Some(&PickedDate::day(YEAR, 8, 1))));
    assert!(!august.in_bounds(None, Some(&PickedDate::day(YEAR, 7, 31))));
}

/// A month's weeks start on the week's first day, with blanks around the days.
#[craft_ui::testing::trial]
fn a_month_grid_pads_to_whole_weeks() {
    let grid = month_grid(YEAR, FEBRUARY, Weekday::Monday);
    assert_eq!(grid.len() % 7, 0);
    assert_eq!(grid.iter().position(Option::is_some), Some(6));
    assert_eq!(grid.iter().flatten().count(), usize::from(FEBRUARY_DAYS));
    assert!(month_grid(YEAR, 13, Weekday::Monday).is_empty());
}

/// Typed text reads at the precisions offered and inside the bounds; empty text clears unless the field is mixed.
#[craft_ui::testing::trial]
fn the_codec_reads_what_the_picker_offers() {
    let day = PickedDate::day(YEAR, 8, 14);
    let codec = DateCodec::new(dates(), &[DatePrecision::Day]).bounded(Some(PickedDate::day(YEAR, 8, 1)), None);
    assert_eq!(codec.parse("2026-08-14", &None), Ok(Some(day)));
    assert!(codec.parse("2026-08", &None).is_err(), "a month is not offered");
    assert!(codec.parse("2026-07-31", &None).is_err(), "before the minimum");
    assert!(codec.parse("soon", &None).is_err());
    assert_eq!(codec.parse("  ", &None), Ok(None));
    assert!(codec.clone().mixed(true).parse("", &None).is_err());
    assert_eq!(codec.parse(&codec.format(&Some(day)), &None), Ok(Some(day)), "written text reads back");
}

/// The active day moves a week with Up and Down, stays at the month's ends, and paging keeps the place in the month
/// at most the new month's last day.
#[craft_ui::testing::trial]
fn the_calendar_moves_the_active_day_and_pages() {
    let (dates, format) = (DateFormat::from_tag("en-US"), Format::from_tag("en-US"));
    let january = PickedDate::day(YEAR, JANUARY, 1);
    let view = CalendarView::open(Some(january), None, &[DatePrecision::Day], &dates);
    assert_eq!((view.year, view.month, view.active), (YEAR, JANUARY, 1));
    assert_eq!(view.stepped(NavMove::Down, &dates, &format).active, 1 + WEEK);
    assert_eq!(view.stepped(NavMove::Back, &dates, &format).active, 1, "nothing before the 1st");
    assert_eq!(view.stepped(NavMove::Up, &dates, &format).active, 1);

    let last = PickedDate::day(YEAR, JANUARY, LAST_JANUARY_DAY);
    let end = CalendarView::open(Some(last), None, &[DatePrecision::Day], &dates);
    let next = end.stepped(NavMove::PageDown, &dates, &format);
    assert_eq!((next.month, i32::from(FEBRUARY_DAYS)), (FEBRUARY, next.active));
    assert_eq!(next.stepped(NavMove::PageUp, &dates, &format).month, JANUARY);
}
