//! `PickedDate` rules, the calendar grid and `DatePicker` in a harness.

use craft_i18n::{DateFormat, NameLength, Weekday};

use super::date::{Page, View};
use crate::localized::LocaleNumbers;

use super::*;

fn parse(text: &str) -> Option<PickedDate> {
    PickedDate::parse(text)
}

// The grid, its titles and the Month cells' names follow the region: ar-EG writes the digits of its
// own, de-DE puts the month name first in the title and its own names on the cells.
#[test]
fn pages_write_the_regions_digits_and_year_month_titles() {
    let (ar_numbers, de_numbers) = (LocaleNumbers::from_tag("ar-EG"), LocaleNumbers::from_tag("de-DE"));
    let (de, us) = (DateFormat::from_tag("de-DE"), DateFormat::from_tag("en-US"));
    let day_view = View { precision: DatePrecision::Day, year: 2026, month: 10, active: 1, nav: false };
    let page = Page::of(&day_view, &de, &ar_numbers);
    let first = page.cells.iter().flatten().next().unwrap();
    assert_eq!(first.text, "\u{661}", "the day in Arabic-Indic digits");
    assert_eq!(page.title(&day_view, &ar_numbers, &de), "Oktober 2026");
    assert_eq!(page.title(&day_view, &de_numbers, &us), "October 2026");

    let month_view = View { precision: DatePrecision::Month, ..day_view };
    let page = Page::of(&month_view, &de, &de_numbers);
    assert_eq!(page.cells[9].as_ref().unwrap().name, "Oktober 2026");
    let year_view = View { precision: DatePrecision::Year, ..day_view };
    let page = Page::of(&year_view, &us, &ar_numbers);
    assert_eq!(page.cells[0].as_ref().unwrap().text, ar_numbers.year(2016));
    assert_eq!(ar_numbers.year(987), "\u{660}\u{669}\u{668}\u{667}", "years pad to four digits");
    assert_eq!(de_numbers.year(2026), "2026", "no grouping");
}

// `parse`: "2026", "2026-08", "2026-08-14T10:00" read; "2026-02-30" and "2026-13" do not; `iso` writes them back.
#[test]
fn dates_parse_at_their_precision_and_reject_impossible_days() {
    assert_eq!(parse("2026"), Some(PickedDate::year(2026)));
    assert_eq!(parse("2026-08"), Some(PickedDate::month(2026, 8)));
    assert_eq!(parse("2026-08-14"), Some(PickedDate::day(2026, 8, 14)));
    assert_eq!(parse("2026-08-14T10:00").map(|d| d.iso()).as_deref(), Some("2026-08-14"), "a time is read past");
    assert_eq!(parse(" 2028-02-29 "), Some(PickedDate::day(2028, 2, 29)), "a leap day");
    for bad in ["", "26", "2026-13", "2026-02-30", "2026-02-29", "banana", "2026/08", "2026-08-14x", "2026-08-14T1"] {
        assert_eq!(parse(bad), None, "{bad}");
    }
    for text in ["2026", "2026-08", "2026-08-14", "0001-01-01", "9999-12-31"] {
        assert_eq!(parse(text).map(|d| d.iso()).as_deref(), Some(text));
    }
    assert_eq!(parse("2026-08").map(|d| d.precision()), Some(DatePrecision::Month));
    assert_eq!(PickedDate::month(2026, 2).last_day(), PickedDate::day(2026, 2, 28));
    assert_eq!(PickedDate::year(2028).last_day(), PickedDate::day(2028, 12, 31));
    assert_eq!(PickedDate::year(2028).first_day(), PickedDate::day(2028, 1, 1));
}

// `in_bounds`: the year 2026 is inside a min of 2026-12-31; a month before a min is not.
#[test]
fn a_value_is_in_bounds_when_any_day_is() {
    let min = PickedDate::day(2026, 12, 31);
    assert!(PickedDate::year(2026).in_bounds(Some(&min), None), "its last day is the min");
    assert!(!PickedDate::month(2026, 11).in_bounds(Some(&min), None));
    assert!(!PickedDate::day(2026, 12, 30).in_bounds(Some(&min), None));
    let max = PickedDate::month(2026, 3);
    assert!(PickedDate::day(2026, 3, 31).in_bounds(None, Some(&max)), "a month bound covers its whole month");
    assert!(!PickedDate::day(2026, 4, 1).in_bounds(None, Some(&max)));
    assert!(PickedDate::year(2026).in_bounds(Some(&min), Some(&PickedDate::year(2026))));
    assert!(PickedDate::year(1999).in_bounds(None, None));
}

// Thailand: the Day page's title and a Year page count Buddhist years, and the cells still hold Gregorian dates.
#[test]
fn thai_pages_show_buddhist_years_and_pick_gregorian_dates() {
    let (th, numbers) = (DateFormat::from_tag("th-TH"), LocaleNumbers::from_tag("th-TH"));
    let day_view = View::open(Some(PickedDate::day(2026, 10, 31)), None, &DatePrecision::ALL, &th);
    assert_eq!((day_view.year, day_view.month, day_view.active), (2569, 10, 31));
    let page = Page::of(&day_view, &th, &numbers);
    assert!(page.title(&day_view, &numbers, &th).ends_with("2569"), "{}", page.title(&day_view, &numbers, &th));
    let last = page.cells.iter().flatten().find(|c| c.key == 31).unwrap();
    assert_eq!(last.date, PickedDate::day(2026, 10, 31));

    let year_view = View { precision: DatePrecision::Year, ..day_view }.switched(DatePrecision::Year, &th);
    let page = Page::of(&year_view, &th, &numbers);
    let year = page.cells.iter().flatten().find(|c| c.key == 2569).unwrap();
    assert_eq!((year.text.as_str(), year.date), ("2569", PickedDate::year(2026)));
}

// Persian: Aban 1405 has 30 days and starts on a Friday (2026-10-23), six blanks after a Saturday week start; a picked day is its Gregorian date.
#[test]
fn a_persian_month_grid_has_its_own_length_and_start() {
    let (fa, numbers) = (DateFormat::from_tag("fa-IR"), LocaleNumbers::from_tag("fa-IR"));
    let grid = local_month_grid(&fa, 1405, 8);
    assert_eq!((grid.iter().take_while(|d| d.is_none()).count(), grid.iter().flatten().count(), grid.len() % 7), (6, 30, 0));
    assert_eq!(local_month_grid(&fa, 1405, 1).iter().flatten().count(), 31);
    assert_eq!(local_month_grid(&fa, 1405, 12).iter().flatten().count(), 29);
    let view = View { precision: DatePrecision::Day, year: 1405, month: 8, active: 1, nav: false };
    let page = Page::of(&view, &fa, &numbers);
    let day = |key| page.cells.iter().flatten().find(|c| c.key == key).unwrap();
    assert_eq!((day(1).date, day(9).date), (PickedDate::day(2026, 10, 23), PickedDate::day(2026, 10, 31)));
    assert_eq!(day(1).text, "\u{6f1}", "Persian digits");
}

// Paging runs from Esfand (29 days) into the next Farvardin and back, keeping the active day within the month.
#[test]
fn paging_crosses_the_persian_year_end() {
    let fa = DateFormat::from_tag("fa-IR");
    let esfand = View { precision: DatePrecision::Day, year: 1405, month: 12, active: 29, nav: false };
    let next = esfand.paged(true, &fa);
    assert_eq!((next.year, next.month, next.active), (1406, 1, 29));
    let back = View { active: 31, ..next }.paged(false, &fa);
    assert_eq!((back.year, back.month, back.active), (1405, 12, 29), "Esfand 1405 has 29 days");
    let page = Page::of(&next, &fa, &LocaleNumbers::from_tag("fa-IR"));
    assert_eq!(page.cells.iter().flatten().next().unwrap().date, PickedDate::day(2027, 3, 21), "1 Farvardin 1406");
}

// A Persian month or year is no Gregorian month, so the Month and Year pages count Gregorian ones.
#[test]
fn persian_month_and_year_pages_count_gregorian_months() {
    let fa = DateFormat::from_tag("fa-IR");
    let day = View { precision: DatePrecision::Day, year: 1405, month: 8, active: 9, nav: false };
    let month = day.switched(DatePrecision::Month, &fa);
    assert_eq!((month.year, month.month), (2026, 10));
    let back = month.switched(DatePrecision::Day, &fa);
    assert_eq!((back.year, back.month), (1405, 7), "2026-10-01 falls in Mehr");
}

// A Day-only Persian picker: the title goes up to the Month page, which lists Persian months, and choosing one opens that month's days.
#[test]
fn navigating_a_persian_day_picker_lands_on_the_chosen_persian_month() {
    let (fa, numbers) = (DateFormat::from_tag("fa-IR"), LocaleNumbers::from_tag("fa-IR"));
    let value = Some(PickedDate::day(2026, 10, 31));
    let day = View::open(value, None, &[DatePrecision::Day], &fa);
    assert_eq!((day.year, day.month, day.nav), (1405, 8, true));
    let months = day.up(&fa, value, None);
    assert_eq!(months.precision, DatePrecision::Month);
    let page = Page::of(&months, &fa, &numbers);
    assert_eq!(page.cells.iter().flatten().count(), 12);
    assert_eq!(page.cells[7].as_ref().unwrap().text, fa.local_month_name(8, NameLength::Abbreviated));
    assert!(page.cells[7].as_ref().unwrap().text.contains("آبان"), "Aban");
    assert_eq!(months.active, 8, "the value's month has the keyboard");
    let mehr = months.entered(7, &fa, value, None);
    assert_eq!((mehr.precision, mehr.year, mehr.month), (DatePrecision::Day, 1405, 7));
    let first = Page::of(&mehr, &fa, &numbers).cells.iter().flatten().next().unwrap().date;
    assert_eq!(first, PickedDate::day(2026, 9, 23), "1 Mehr 1405");
    let years = months.up(&fa, value, None);
    assert_eq!(Page::of(&years, &fa, &numbers).cells.iter().flatten().find(|c| c.key == 1405).map(|c| c.text.clone()), Some(numbers.year(1405)));
    assert_eq!(years.entered(1404, &fa, value, None).precision, DatePrecision::Month);
}

// With month precision offered, the value is a Gregorian month, so the Persian picker's Month page picks one.
#[test]
fn a_persian_month_precision_picker_returns_a_gregorian_month() {
    let (fa, numbers) = (DateFormat::from_tag("fa-IR"), LocaleNumbers::from_tag("fa-IR"));
    let view = View::open(Some(PickedDate::month(2026, 10)), None, &[DatePrecision::Month, DatePrecision::Day], &fa);
    assert_eq!((view.precision, view.year, view.nav), (DatePrecision::Month, 2026, false));
    let page = Page::of(&view, &fa, &numbers);
    let cell = page.cells[9].as_ref().unwrap();
    assert_eq!((cell.date, cell.nav), (PickedDate::month(2026, 10), false));
}

// The screen-reader name of a cell is the date in the calendar the grid shows.
#[test]
fn cell_names_follow_the_shown_calendar() {
    for (tag, year) in [("th-TH", "2569"), ("fa-IR", "\u{6f1}\u{6f4}\u{6f0}\u{6f5}"), ("en-US", "2026")] {
        let (dates, numbers) = (DateFormat::from_tag(tag), LocaleNumbers::from_tag(tag));
        let view = View::open(Some(PickedDate::day(2026, 10, 31)), None, &DatePrecision::ALL, &dates);
        let page = Page::of(&view, &dates, &numbers);
        let cell = page.cells.iter().flatten().find(|c| c.date == PickedDate::day(2026, 10, 31)).unwrap();
        assert!(cell.name.contains(year), "{tag}: {}", cell.name);
    }
}

// `month_grid`: August 2026 starts on a Saturday: five blanks from Monday, six from Sunday; the length is a multiple of 7.
#[test]
fn the_month_grid_starts_on_the_week_start() {
    let monday = month_grid(2026, 8, Weekday::Monday);
    assert_eq!(monday.len() % 7, 0);
    assert_eq!(monday.iter().take_while(|d| d.is_none()).count(), 5);
    assert_eq!(monday.iter().flatten().count(), 31);
    let sunday = month_grid(2026, 8, Weekday::Sunday);
    assert_eq!(sunday.iter().take_while(|d| d.is_none()).count(), 6);
    assert_eq!(month_grid(2026, 6, Weekday::Monday).first(), Some(&Some(1)), "June 2026 starts on a Monday");
    assert_eq!(month_grid(2026, 2, Weekday::Monday).iter().flatten().count(), 28);
    assert!(month_grid(2026, 13, Weekday::Monday).is_empty());
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod picker {
    use egui::accesskit::Role;
    use egui::{Key, Ui};
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    use egui_kittest::{Harness, HarnessBuilder};

    use super::super::DatePicker;
    use super::*;
    use crate::edit::EditEvent;
    use crate::theme::{Base, install};

    struct Host {
        value: Option<PickedDate>,
        events: Vec<EditEvent<Option<PickedDate>>>,
    }

    fn harness() -> Harness<'static, Host> {
        HarnessBuilder::default().with_size([320.0, 420.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, host: &mut Host| {
                let out = DatePicker::new("from", "Captured from", &mut host.value).today(PickedDate::day(2026, 8, 20)).show(ui);
                host.events.extend(out.event);
            },
            Host { value: Some(PickedDate::day(2026, 8, 3)), events: Vec::new() },
        )
    }

    fn open(harness: &mut Harness<'_, Host>) {
        harness.run();
        harness.get_by_label("Pick a date").click();
        harness.run();
        harness.run();
    }

    /// A press and release over the cell with a pass between them, as a pointer does.
    fn click(harness: &mut Harness<'_, Host>, label: &str) {
        let at = harness.get_by_label(label).rect().center();
        harness.hover_at(at);
        harness.run();
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
            harness.run();
        }
        harness.run();
    }

    fn cells(harness: &Harness<'_, Host>) -> usize {
        harness.query_all_by_role(Role::GridCell).count()
    }

    // In a harness: on the Day page a click on the 14th sets 2026-08-14 and closes the popup; Clear sets `None`.
    #[test]
    fn picking_a_cell_sets_the_value_and_closes() {
        let mut harness = harness();
        open(&mut harness);
        assert!(cells(&harness) >= 31, "the month's days show");
        assert!(harness.get_by_label("August 3, 2026").accesskit_node().is_selected().unwrap_or(false), "the value's cell is selected");
        click(&mut harness, "August 14, 2026");
        assert_eq!(harness.state().value, Some(PickedDate::day(2026, 8, 14)));
        assert!(matches!(harness.state().events.as_slice(), [EditEvent::Set { value: Some(d), .. }] if *d == PickedDate::day(2026, 8, 14)));
        assert_eq!(cells(&harness), 0, "the popup closed");

        open(&mut harness);
        harness.get_by_label("Clear").click();
        harness.run();
        harness.run();
        assert_eq!(harness.state().value, None);
        assert!(matches!(harness.state().events.last(), Some(EditEvent::Set { value: None, .. })));
        assert_eq!(cells(&harness), 0);
    }

    // The popup opens on the value's cell; → moves a day, Page Down pages a month, Esc closes without an event.
    #[test]
    fn the_keys_move_the_active_day_and_page() {
        let mut harness = harness();
        open(&mut harness);
        assert!(harness.get_by_label("August 3, 2026").accesskit_node().is_focused(), "the value's cell has the focus");
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert!(harness.get_by_label("August 4, 2026").accesskit_node().is_focused());
        harness.key_press(Key::ArrowDown);
        harness.run();
        assert!(harness.get_by_label("August 11, 2026").accesskit_node().is_focused(), "a week down");
        harness.key_press(Key::PageDown);
        harness.run();
        harness.run();
        assert!(harness.get_by_label("September 11, 2026").accesskit_node().is_focused(), "the page follows, the day stays");
        harness.key_press(Key::Enter);
        harness.run();
        harness.run();
        assert_eq!(harness.state().value, Some(PickedDate::day(2026, 9, 11)));

        open(&mut harness);
        harness.key_press(Key::Escape);
        harness.run();
        harness.run();
        assert_eq!(cells(&harness), 0, "Esc closed it");
        assert_eq!(harness.state().events.len(), 1, "Esc reports nothing");
    }

    // A press outside the popup closes it and reports nothing.
    #[test]
    fn a_press_outside_closes_without_an_event() {
        let mut harness = harness();
        open(&mut harness);
        let away = egui::pos2(310.0, 410.0);
        harness.hover_at(away);
        harness.run();
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton { pos: away, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
            harness.run();
        }
        harness.run();
        assert_eq!(cells(&harness), 0);
        assert!(harness.state().events.is_empty());
        assert_eq!(harness.state().value, Some(PickedDate::day(2026, 8, 3)));
    }
}
