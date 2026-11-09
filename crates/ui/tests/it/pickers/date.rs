//! Date values, codec, calendar maths and the calendar's keys.

use craft_behavior::{Codec, NavMove};
use craft_i18n::{CalendarDate, DateFormat, Region, Weekday};
use craft_ui::contract::Show as _;
use craft_ui::pickers::{Calendar, CalendarView, DateCodec, DatePrecision, PickedDate, local_month_grid, month_grid};
use craft_ui::testing::Fixture;
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;
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

/// A step past the month's first or last row goes on into the neighbouring month, a day or a week from the cell; paging
/// keeps the place in the month at most the new month's last day.
#[craft_ui::testing::trial]
fn the_calendar_crosses_months_past_the_grids_ends_and_pages() {
    let dates = DateFormat::from_tag("en-US");
    let january = PickedDate::day(YEAR, JANUARY, 1);
    let view = CalendarView::open(Some(january), None, &[DatePrecision::Day], &dates);
    assert_eq!((view.year, view.month, view.active), (YEAR, JANUARY, 1));
    let before = view.crossed(NavMove::Back, 1, &dates);
    assert_eq!((before.year, before.month, before.active), (YEAR - 1, 12, i32::from(LAST_JANUARY_DAY)), "the day before the 1st");
    let above = view.crossed(NavMove::Up, 3, &dates);
    assert_eq!((above.year, above.month, above.active), (YEAR - 1, 12, 27), "a week before January 3rd");
    let below = view.crossed(NavMove::Down, i32::from(LAST_JANUARY_DAY), &dates);
    assert_eq!((below.month, below.active), (FEBRUARY, WEEK), "a week after January 31st");

    let last = PickedDate::day(YEAR, JANUARY, LAST_JANUARY_DAY);
    let end = CalendarView::open(Some(last), None, &[DatePrecision::Day], &dates);
    let next = end.crossed(NavMove::PageDown, end.active, &dates);
    assert_eq!((next.month, i32::from(FEBRUARY_DAYS)), (FEBRUARY, next.active));
    assert_eq!(next.crossed(NavMove::PageUp, next.active, &dates).month, JANUARY);
    assert_eq!(view.crossed(NavMove::Start, 1, &dates), view, "Home is the grid's");
}

/// On a Month page a step past the first or last month goes on into the neighbouring year's page, at the mirrored cell;
/// on a Year page a step past the last row goes on into the next twelve years.
#[craft_ui::testing::trial]
fn month_and_year_pages_cross_their_edges() {
    const DECEMBER: i32 = 12;
    const COLUMNS: i32 = 4;
    let dates = DateFormat::from_tag("en-US");
    let months = CalendarView::open(Some(PickedDate::month(YEAR, 12)), None, &[DatePrecision::Month], &dates);
    assert_eq!(months.precision, DatePrecision::Month);
    let next = months.crossed(NavMove::Forward, DECEMBER, &dates);
    assert_eq!((next.year, next.active), (YEAR + 1, 1), "Right on the last month");
    let below = months.crossed(NavMove::Down, DECEMBER - 1, &dates);
    assert_eq!((below.year, below.active), (YEAR + 1, 3), "Down from the last row keeps the column");
    let before = next.crossed(NavMove::Back, 1, &dates);
    assert_eq!((before.year, before.active), (YEAR, DECEMBER), "Left on the first month");
    let above = next.crossed(NavMove::Up, 2, &dates);
    assert_eq!((above.year, above.active), (YEAR, DECEMBER - COLUMNS + 2), "Up from the first row keeps the column");

    let years = CalendarView::open(Some(PickedDate::year(YEAR)), None, &[DatePrecision::Year], &dates);
    let last_row = YEAR - 1;
    let onward = years.crossed(NavMove::Down, last_row, &dates);
    assert_eq!(onward.active, last_row + COLUMNS, "Down from the last row of the page");
    assert!(onward.year > YEAR, "the next page of twelve years");
    assert!(onward.land, "focus follows");
}

/// A month whose first day sits in the fourth column of the week, in the region the fixture runs in.
fn month_starting_in_fourth_column() -> (u8, CalendarView, DateFormat) {
    let dates = DateFormat::new(&Region::system());
    let month = (2..=12u8)
        .find(|month| local_month_grid(&dates, YEAR, *month).iter().position(Option::is_some) == Some(3))
        .expect("every weekday starts a month between February and December");
    let view = CalendarView::open(Some(PickedDate::day(YEAR, month, 1)), None, &[DatePrecision::Day], &dates);
    (month, view, dates)
}

type CalendarRig = (Fixture, Rc<Cell<CalendarView>>, Rc<Cell<Option<PickedDate>>>);

/// A calendar over `view` that records its page and the day picked.
fn calendar(view: CalendarView) -> CalendarRig {
    let page = Rc::new(Cell::new(view));
    let picked = Rc::new(Cell::new(None));
    let (shown, chose) = (page.clone(), picked.clone());
    let mut fixture = Fixture::new(move |ui| {
        let out = Calendar::new("Day", shown.get(), None).show(ui);
        if let Some(next) = out.moved {
            shown.set(next);
        }
        if out.picked.is_some() {
            chose.set(out.picked);
        }
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    (fixture, page, picked)
}

/// Down from the 1st of a month that starts in the fourth column lands on the 8th, in the same column of the next
/// row, and Enter picks it.
#[craft_ui::testing::trial]
fn down_from_the_first_of_a_month_lands_on_the_same_weekday_a_week_later() {
    let (month, view, _) = month_starting_in_fourth_column();
    let (mut fixture, _, picked) = calendar(view);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(picked.get(), Some(PickedDate::day(YEAR, month, WEEK as u8 + 1)));
}

/// Up from the 1st has no row above: the previous month shows and focus lands a week before the 1st.
#[craft_ui::testing::trial]
fn up_from_the_first_moves_to_the_previous_month() {
    let (month, view, _) = month_starting_in_fourth_column();
    let (mut fixture, page, picked) = calendar(view);
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    fixture.idle(3);
    assert_eq!(page.get().month, month - 1);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    let days = CalendarDate::days_in_month(i64::from(YEAR), month - 1);
    assert_eq!(picked.get(), Some(PickedDate::day(YEAR, month - 1, days - WEEK as u8 + 1)));
}

/// Enter on the focused day picks it.
#[craft_ui::testing::trial]
fn enter_on_a_focused_day_picks_it() {
    let (month, view, _) = month_starting_in_fourth_column();
    let (mut fixture, _, picked) = calendar(view);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(picked.get(), Some(PickedDate::day(YEAR, month, 2)));
}
