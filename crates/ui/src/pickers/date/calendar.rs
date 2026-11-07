//! `Calendar`: one page of a date picker's popover: the title with the previous and next buttons, and the days,
//! months or years of the page as cells.
//!
//! The page the calendar shows is its [`CalendarView`], which the host keeps and the calendar's output moves. One cell
//! is the focus stop (the active cell, the part `ACTIVE`): arrows reach `update`, which moves the active cell over
//! the grid, blanks included, so a week stays a row. The other cells are pointer parts that name the active cell as
//! their keyboard path.

use super::{DatePrecision, LAST_YEAR, PickedDate, dates_of};
use crate::buttons::IconButton;
use crate::contract::{
    Access, Common, Component, Cx, MeasureCx, Name, Outcome, PaintUi, PartsCx, Pointer as _, Props as _, Role, Styled as _, WordSetCx as _,
};
use crate::pickers::words::PickerWords;
use crate::primitives::{HAlign, PaintShape, StrokeWidth, TextRun};
use crate::runtime::access::GridPlace;
use crate::runtime::focus::{KeepKeys, KeyUse};
use crate::style::{ComponentStyle, Flags, FocusLook, Ink, StyleColor, StyleCx, TextLook, TypeRole, UiTokens};
use craft_behavior::{Flow, HitLevel, HitPriority, IconId, Metrics, NavMove, PartKey, PartRef, Sense};
use craft_i18n::{CalendarDate, DateFormat, DateLength, DateStyle, DateTime, Format, LocalDate, NameLength, Weekday};
use craft_theme::Stateful;
use egui::{CornerRadius, Rect, Response, Vec2, vec2};
use std::sync::Arc;

/// Years on a Year page.
const YEARS_PER_PAGE: i32 = 12;
/// Columns of a Day page, a Month page and a Year page.
const DAY_COLUMNS: usize = 7;
const PICK_COLUMNS: usize = 4;
/// Rows a page is sized for: the most weeks a month spans.
const ROWS: usize = 6;

/// The cell that holds the keyboard.
const ACTIVE: PartKey = PartKey::new("active");
/// Any other cell.
const DAY: PartKey = PartKey::new("day");
const PREVIOUS: PartKey = PartKey::new("previous");
const NEXT: PartKey = PartKey::new("next");
const TITLE: PartKey = PartKey::new("title");

const PREVIOUS_ICON: &str = "chevron_left";
const NEXT_ICON: &str = "chevron_right";

/// The month's days laid out in weeks starting on `first_weekday`: `None` before the 1st and after the last day, so
/// the length is a multiple of 7; empty for a month outside 1 to 12. The months are Gregorian; [`local_month_grid`]
/// lays out a month of the region's calendar.
pub fn month_grid(year: i32, month: u8, first_weekday: Weekday) -> Vec<Option<u8>> {
    let Some(first) = CalendarDate::new(i64::from(year), month, 1) else {
        return Vec::new();
    };
    weeks(first.weekday(), CalendarDate::days_in_month(i64::from(year), month), first_weekday)
}

/// [`month_grid`] for a month of the calendar `dates` writes (Persian months start on other days and differ in
/// length); empty for a month that does not exist.
pub fn local_month_grid(dates: &DateFormat, year: i32, month: u8) -> Vec<Option<u8>> {
    let days = dates.local_days_in_month(year, month);
    match dates.from_local(LocalDate { year, month, day: 1 }) {
        Some(first) if days > 0 => weeks(first.weekday(), days, dates.first_weekday()),
        _ => Vec::new(),
    }
}

fn weeks(first_day: Weekday, days: u8, first_weekday: Weekday) -> Vec<Option<u8>> {
    let lead = (first_day as usize + 7 - first_weekday as usize) % 7;
    let mut cells: Vec<Option<u8>> = std::iter::repeat_n(None, lead).chain((1..=days).map(Some)).collect();
    while !cells.len().is_multiple_of(7) {
        cells.push(None);
    }
    cells
}

/// The numbers of a page in the scope's digits, without grouping.
struct Numbers<'a>(&'a Format);

impl Numbers<'_> {
    fn of(&self, n: i32) -> String {
        self.0.number_ungrouped(f64::from(n), 0)
    }
}

/// Which page the calendar shows, and the cell that has the keyboard (its day, month number or year, by the page).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CalendarView {
    /// What the page lists: days, months or years.
    pub precision: DatePrecision,
    /// The page's year (a Year page: any year of it).
    pub year: i32,
    /// The page's month, 1-based.
    pub month: u8,
    /// The active cell's day, month number or year, by the page.
    pub active: i32,
    /// The Month and Year pages only move the Day page, in the display calendar (a Day-only picker), instead of
    /// choosing a Gregorian month or year.
    pub nav: bool,
}

impl CalendarView {
    /// The Gregorian date of a local day, `None` outside the four digits a value can hold.
    fn picked(dates: &DateFormat, local: LocalDate) -> Option<PickedDate> {
        dates.from_local(local).and_then(PickedDate::from_calendar_date)
    }

    /// The year number a value page adds to a value's Gregorian year: the display calendar's when its months are the
    /// Gregorian months (Buddhist: 543), else none, since a month or year of a calendar with other month starts is
    /// not a Gregorian month or year.
    fn frame_offset(dates: &DateFormat) -> i32 {
        dates.local_year_offset().unwrap_or(0)
    }

    /// Whether the page counts days, months and years of the display calendar: the Day page, and the navigation
    /// pages. Else it counts Gregorian ones (shifted by [`Self::frame_offset`]).
    fn local(&self) -> bool {
        self.precision == DatePrecision::Day || self.nav
    }

    /// The years a page can show, as the page counts them.
    fn year_range(local: bool, dates: &DateFormat) -> (i32, i32) {
        if local { dates.local_year_range().unwrap_or((0, LAST_YEAR)) } else { (Self::frame_offset(dates), LAST_YEAR + Self::frame_offset(dates)) }
    }

    /// `date`'s first day as a page counts it: year, month, day.
    fn coords(&self, date: PickedDate, dates: &DateFormat) -> Option<(i32, u8, u8)> {
        if self.local() {
            let local = dates.to_local(date.calendar_date()?)?;
            Some((local.year, local.month, local.day))
        } else {
            Some((date.year + Self::frame_offset(dates), date.month.unwrap_or(1), date.day.unwrap_or(1)))
        }
    }

    /// The view of a popup opening on `value`, else today, at the value's precision when it is offered, else days
    /// when offered, else the first offered.
    pub fn open(value: Option<PickedDate>, today: Option<PickedDate>, precisions: &[DatePrecision], dates: &DateFormat) -> CalendarView {
        let at = value.or(today).unwrap_or(PickedDate::year(1970));
        let fallback =
            if precisions.contains(&DatePrecision::Day) { DatePrecision::Day } else { precisions.first().copied().unwrap_or(DatePrecision::Day) };
        let precision = value.map(|v| v.precision()).filter(|p| precisions.contains(p)).unwrap_or(fallback);
        let nav = precision == DatePrecision::Day && precisions.iter().all(|p| *p == DatePrecision::Day);
        let mut view = CalendarView { precision, year: 0, month: 1, active: 0, nav };
        let (year, month, _) = view.coords(at, dates).unwrap_or((at.year, at.month.unwrap_or(1), 1));
        let (first, last) = Self::year_range(view.local(), dates);
        view.year = year.clamp(first, last);
        view.month = month.max(1);
        view.active = view.landing(value, today, dates);
        view
    }

    /// This view with the active cell on the value's, else today's, else the first of its page.
    #[must_use]
    pub fn landed(self, value: Option<PickedDate>, today: Option<PickedDate>, dates: &DateFormat) -> CalendarView {
        CalendarView { active: self.landing(value, today, dates), ..self }
    }

    /// This view on another precision's page, over the same day.
    #[must_use]
    pub fn switched(self, precision: DatePrecision, dates: &DateFormat) -> CalendarView {
        let (year, month) = match (self.precision, precision) {
            (DatePrecision::Day, DatePrecision::Day) => (self.year, self.month),
            (DatePrecision::Day, _) => match Self::picked(dates, LocalDate { year: self.year, month: self.month, day: 1 }) {
                Some(first) => (first.year + Self::frame_offset(dates), first.month.unwrap_or(1)),
                None => (self.year, self.month),
            },
            (_, DatePrecision::Day) => {
                let first = PickedDate::day(self.year - Self::frame_offset(dates), self.month, 1);
                first.calendar_date().and_then(|d| dates.to_local(d)).map_or((self.year, self.month), |l| (l.year, l.month))
            }
            _ => (self.year, self.month),
        };
        let (first, last) = Self::year_range(precision == DatePrecision::Day, dates);
        CalendarView { precision, year: year.clamp(first, last), month: month.max(1), nav: false, ..self }
    }

    /// The navigation page one level up from the Day page: Month, then Year.
    #[must_use]
    pub fn up(self, dates: &DateFormat, value: Option<PickedDate>, today: Option<PickedDate>) -> CalendarView {
        let view = match (self.nav, self.precision) {
            (true, DatePrecision::Day) => CalendarView { precision: DatePrecision::Month, ..self },
            (true, DatePrecision::Month) => CalendarView { precision: DatePrecision::Year, ..self },
            _ => self,
        };
        view.landed(value, today, dates)
    }

    /// The view after choosing the cell `key` of a navigation page: a Year page's year opens its Month page, a Month
    /// page's month its Day page.
    #[must_use]
    pub fn entered(self, key: i32, dates: &DateFormat, value: Option<PickedDate>, today: Option<PickedDate>) -> CalendarView {
        let view = match self.precision {
            DatePrecision::Year => CalendarView { precision: DatePrecision::Month, year: key, ..self },
            DatePrecision::Month => CalendarView { precision: DatePrecision::Day, month: u8::try_from(key).unwrap_or(1), ..self },
            _ => self,
        };
        view.landed(value, today, dates)
    }

    /// The first year of a Year page.
    fn first_year(&self) -> i32 {
        self.year.div_euclid(YEARS_PER_PAGE) * YEARS_PER_PAGE
    }

    /// The cell that has the keyboard when the page shows: the value's, else today's, else the first.
    fn landing(&self, value: Option<PickedDate>, today: Option<PickedDate>, dates: &DateFormat) -> i32 {
        let (_, last) = Self::year_range(self.local(), dates);
        let on_page = |date: PickedDate| {
            let (year, month, day) = self.coords(date, dates)?;
            match self.precision {
                DatePrecision::Day => {
                    (date.month.is_some() && year == self.year && month == self.month).then(|| i32::from(date.day.map_or(1, |_| day)))
                }
                DatePrecision::Month => (year == self.year).then_some(i32::from(month)),
                _ => (self.first_year()..self.first_year() + YEARS_PER_PAGE).contains(&year).then_some(year),
            }
        };
        value.and_then(on_page).or_else(|| today.and_then(on_page)).unwrap_or(match self.precision {
            DatePrecision::Day | DatePrecision::Month => 1,
            _ => self.first_year().min(last),
        })
    }

    /// The page before or after: a month, a year or twelve years, with the active cell kept at the same place.
    /// Paging stops at the first and last year rather than wrapping.
    #[must_use]
    pub fn paged(self, forward: bool, dates: &DateFormat) -> CalendarView {
        let sign = if forward { 1 } else { -1 };
        let (first, last) = Self::year_range(self.local(), dates);
        let (year, month) = match self.precision {
            DatePrecision::Day => {
                let months = i32::from(dates.local_months_in_year(self.year).max(1));
                let m = i32::from(self.month) - 1 + sign;
                if m < 0 {
                    let year = self.year.saturating_sub(1);
                    (year, dates.local_months_in_year(year).max(1))
                } else if m >= months {
                    (self.year.saturating_add(1), 1)
                } else {
                    (self.year, (m + 1) as u8)
                }
            }
            DatePrecision::Month => (self.year.saturating_add(sign), self.month),
            _ => (self.first_year().saturating_add(sign * YEARS_PER_PAGE), self.month),
        };
        if !(first..=last).contains(&year) {
            return self;
        }
        let mut view = CalendarView { year, month, ..self };
        view.active = match self.precision {
            DatePrecision::Day => self.active.min(i32::from(dates.local_days_in_month(year, month))),
            DatePrecision::Month => self.active,
            _ => (self.active + sign * YEARS_PER_PAGE).clamp(first, last),
        };
        view
    }

    /// The view after `mv`: arrows move the active cell over the page (never onto a blank), Home and End go to the
    /// row's ends, Page Up and Page Down page.
    #[must_use]
    pub fn stepped(self, mv: NavMove, dates: &DateFormat, format: &Format) -> CalendarView {
        match mv {
            NavMove::PageUp => return self.paged(false, dates),
            NavMove::PageDown => return self.paged(true, dates),
            _ => {}
        }
        let page = Page::of(&self, dates, &Numbers(format), false);
        let Some(from) = page.position(self.active) else { return self };
        let to = page.moved(from, mv);
        match page.cells[to].as_ref() {
            Some(cell) => CalendarView { active: cell.key, ..self },
            None => self,
        }
    }
}

/// One cell of a page.
struct Cell {
    /// The day, the month number or the year.
    key: i32,
    /// What choosing it sets; a navigation cell's first day.
    date: PickedDate,
    /// The first and last Gregorian day it covers.
    span: (PickedDate, PickedDate),
    /// Whether choosing it only moves the page.
    nav: bool,
    text: String,
    /// The accessible name; empty unless the page was built for the access tree.
    name: String,
}

/// A page's cells in reading order; `None` is the blank before the 1st and after the last day.
struct Page {
    cells: Vec<Option<Cell>>,
    columns: usize,
}

impl Page {
    fn of(view: &CalendarView, dates: &DateFormat, numbers: &Numbers<'_>, named: bool) -> Page {
        let long = |date: PickedDate| match date.calendar_date() {
            Some(day) => dates.format(DateTime { date: day, hour: 0, minute: 0, second: 0 }, DateStyle { date: Some(DateLength::Long), time: None }),
            None => date.iso(),
        };
        let named_as = |name: &dyn Fn() -> String| if named { name() } else { String::new() };
        let cell = |key: i32, date: PickedDate, text: String, name: String| Cell {
            key,
            date,
            span: (date.first_day(), date.last_day()),
            nav: false,
            text,
            name,
        };
        let offset = if view.local() { 0 } else { CalendarView::frame_offset(dates) };
        let (first, last) = CalendarView::year_range(view.local(), dates);
        // A local month or year as the Gregorian days it covers; the ends of the range are clamped.
        let span_of = |year: i32, first_month: u8, last_month: u8| {
            let start = CalendarView::picked(dates, LocalDate { year, month: first_month, day: 1 }).unwrap_or(PickedDate::day(0, 1, 1));
            let end = CalendarView::picked(dates, LocalDate { year, month: last_month, day: dates.local_days_in_month(year, last_month) })
                .unwrap_or(PickedDate::day(LAST_YEAR, 12, 31));
            (start, end)
        };
        match (view.precision, view.nav) {
            (DatePrecision::Day, _) => Page {
                columns: DAY_COLUMNS,
                cells: local_month_grid(dates, view.year, view.month)
                    .into_iter()
                    .map(|day| {
                        let d = day?;
                        let date = CalendarView::picked(dates, LocalDate { year: view.year, month: view.month, day: d })?;
                        Some(cell(i32::from(d), date, numbers.of(i32::from(d)), named_as(&|| long(date))))
                    })
                    .collect(),
            },
            (DatePrecision::Month, true) => Page {
                columns: PICK_COLUMNS,
                cells: (1..=dates.local_months_in_year(view.year))
                    .map(|m| {
                        let span = span_of(view.year, m, m);
                        Some(Cell {
                            key: i32::from(m),
                            date: span.0,
                            span,
                            nav: true,
                            text: dates.local_month_name(m, NameLength::Abbreviated),
                            name: named_as(&|| dates.local_year_month(view.year, m)),
                        })
                    })
                    .collect(),
            },
            (DatePrecision::Month, false) => Page {
                columns: PICK_COLUMNS,
                cells: (1..=12u8)
                    .map(|m| {
                        let date = PickedDate::month(view.year - offset, m);
                        let name = || if offset == 0 { dates.year_month(i64::from(view.year), m) } else { dates.local_year_month(view.year, m) };
                        Some(cell(i32::from(m), date, dates.month_name(m, NameLength::Abbreviated), named_as(&name)))
                    })
                    .collect(),
            },
            (_, true) => Page {
                columns: PICK_COLUMNS,
                cells: (view.first_year()..view.first_year() + YEARS_PER_PAGE)
                    .map(|y| {
                        (first..=last).contains(&y).then(|| {
                            let span = span_of(y, 1, dates.local_months_in_year(y));
                            Cell { key: y, date: span.0, span, nav: true, text: numbers.of(y), name: named_as(&|| numbers.of(y)) }
                        })
                    })
                    .collect(),
            },
            (_, false) => Page {
                columns: PICK_COLUMNS,
                cells: (view.first_year()..view.first_year() + YEARS_PER_PAGE)
                    .map(|y| (first..=last).contains(&y).then(|| cell(y, PickedDate::year(y - offset), numbers.of(y), named_as(&|| numbers.of(y)))))
                    .collect(),
            },
        }
    }

    /// The position of the cell with `key`.
    fn position(&self, key: i32) -> Option<usize> {
        self.cells.iter().position(|c| c.as_ref().is_some_and(|c| c.key == key))
    }

    fn first(&self) -> usize {
        self.cells.iter().position(Option::is_some).unwrap_or(0)
    }

    fn last(&self) -> usize {
        self.cells.iter().rposition(Option::is_some).unwrap_or(0)
    }

    /// Where `mv` takes the active cell from `from`: a step past the page's cells stays, and Home and End go to the
    /// row's ends clamped to the cells that exist.
    fn moved(&self, from: usize, mv: NavMove) -> usize {
        let columns = self.columns;
        let row = from / columns;
        let step = |to: Option<usize>| to.filter(|to| (self.first()..=self.last()).contains(to)).unwrap_or(from);
        match mv {
            NavMove::Back => step(from.checked_sub(1)),
            NavMove::Forward => step(from.checked_add(1)),
            NavMove::Up => step(from.checked_sub(columns)),
            NavMove::Down => step(from.checked_add(columns)),
            NavMove::Start => (row * columns).clamp(self.first(), self.last()),
            NavMove::End => ((row + 1) * columns - 1).clamp(self.first(), self.last()),
            _ => from,
        }
    }

    /// The title over the page.
    fn title(&self, view: &CalendarView, numbers: &Numbers<'_>, dates: &DateFormat) -> String {
        match view.precision {
            DatePrecision::Day => dates.local_year_month(view.year, view.month),
            DatePrecision::Month => numbers.of(view.year),
            _ => {
                let first = view.first_year();
                let last = (first + YEARS_PER_PAGE - 1).min(CalendarView::year_range(view.local(), dates).1);
                format!("{}\u{2013}{}", numbers.of(first), numbers.of(last))
            }
        }
    }

    fn rows(&self) -> usize {
        self.cells.len().div_ceil(self.columns)
    }
}

/// The part of the cell `key` on the page of `view`.
fn part_of(view: &CalendarView, key: i32) -> PartRef {
    if key == view.active { ACTIVE.into() } else { DAY.at(u64::from(key.unsigned_abs())) }
}

/// The style of a calendar: the sizes of a page and the looks of its text and cells.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct CalendarStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The page title.
    #[style(text)]
    title: TextLook,
    /// The weekday names.
    #[style(text)]
    weekday: TextLook,
    /// A cell's text.
    #[style(text)]
    text: TextLook,
    /// The text of a cell outside the bounds.
    #[style(text)]
    disabled: TextLook,
    /// The fill under the pointer, of the value's cell, and of the value's cell under the pointer.
    hover: StyleColor,
    selected: StyleColor,
    selected_hover: StyleColor,
    /// The ring around the cell that holds today.
    today: StyleColor,
    radius: CornerRadius,
    /// A cell's height and least width, the gap between cells and the height of the title strip and its buttons.
    cell: f32,
    gap: f32,
    strip: f32,
    /// The width of the page: seven cells and six gaps.
    width: f32,
}

/// What a calendar reports this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct CalendarOutput {
    /// The calendar's response.
    pub response: Response,
    /// The day, month or year the user chose, as the value it sets; a cell of a navigation page only moves the view.
    pub picked: Option<PickedDate>,
    /// The enabled cell under the pointer.
    pub hovered: Option<PickedDate>,
    /// The view after this pass's keys, paging and navigation, when it moved.
    pub moved: Option<CalendarView>,
}

impl Outcome for CalendarOutput {
    type Value = crate::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        Outcome::story_facts(&self.response)
    }
}

/// The page of a date picker's popover. The host owns the [`CalendarView`] and applies `moved`.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct Calendar {
    #[common]
    common: Common,
    view: CalendarView,
    value: Option<PickedDate>,
    /// The earliest value allowed: a cell is allowed when any of its days is in range.
    #[prop]
    min: Option<PickedDate>,
    /// The latest value allowed.
    #[prop]
    max: Option<PickedDate>,
    /// The day that carries the ring.
    #[prop]
    today: Option<PickedDate>,
    picked: Option<PickedDate>,
    hovered: Option<PickedDate>,
    moved: Option<CalendarView>,
    /// The heights of the title, weekday and cell text, measured for centring.
    lines: [f32; 3],
}

impl Calendar {
    /// The page of `view` named `label`, with the cell of `value` selected.
    pub fn new(label: impl Into<crate::runtime::env::Label>, view: CalendarView, value: Option<PickedDate>) -> Self {
        Calendar { common: Common::new(), view, value, min: None, max: None, today: None, picked: None, hovered: None, moved: None, lines: [0.0; 3] }
            .label(label)
    }

    /// Whether any day of the cell lies within the bounds.
    fn permitted(&self, cell: &Cell) -> bool {
        self.min.is_none_or(|m| cell.span.1.day_key() >= m.first_day().day_key())
            && self.max.is_none_or(|m| cell.span.0.day_key() <= m.last_day().day_key())
    }

    /// Whether the value is the cell's, or for a navigation cell falls within it.
    fn selected(&self, cell: &Cell) -> bool {
        if cell.nav {
            self.value.is_some_and(|c| (cell.span.0.day_key()..=cell.span.1.day_key()).contains(&c.first_day().day_key()))
        } else {
            self.value == Some(cell.date)
        }
    }

    fn holds_today(&self, cell: &Cell) -> bool {
        self.today.is_some_and(|t| (cell.span.0.day_key()..=cell.span.1.day_key()).contains(&t.first_day().day_key()))
    }

    /// Whether the title is a button that goes up a level.
    fn title_goes_up(&self) -> bool {
        self.view.nav && self.view.precision != DatePrecision::Year
    }

    fn measure_lines(&mut self, cx: &MeasureCx<'_, Self>) {
        let line = |role: TypeRole| cx.layout(&TextRun::new("0").role(role)).size().y;
        self.lines = [line(cx.style.title.role), line(cx.style.weekday.role), line(cx.style.text.role)];
    }
}

/// Where the page's pieces sit inside a rect.
struct Geometry {
    previous: Rect,
    next: Rect,
    title: Rect,
    weekdays: Option<Rect>,
    top: f32,
    min_x: f32,
    max_x: f32,
    cell: f32,
    gap: f32,
    columns: usize,
    rtl: bool,
}

impl Geometry {
    fn of(rect: Rect, style: &CalendarStyle, rtl: bool, view: &CalendarView, columns: usize) -> Geometry {
        let strip = Rect::from_min_size(rect.min, vec2(rect.width(), style.strip));
        let (previous, rest) = Flow::new(strip, rtl).take_start(style.strip);
        let (next, rest) = rest.take_end(style.strip);
        let below = strip.max.y + style.gap;
        let (weekdays, top) = if view.precision == DatePrecision::Day {
            let row = Rect::from_min_size(egui::pos2(rect.min.x, below), vec2(rect.width(), style.cell));
            (Some(row), below + style.cell + style.gap)
        } else {
            (None, below)
        };
        Geometry {
            previous,
            next,
            title: rest.rect(),
            weekdays,
            top,
            min_x: rect.min.x,
            max_x: rect.max.x,
            cell: style.cell,
            gap: style.gap,
            columns,
            rtl,
        }
    }

    fn width(&self) -> f32 {
        (self.max_x - self.min_x - (self.columns - 1) as f32 * self.gap) / self.columns as f32
    }

    fn column(&self, column: usize) -> Rect {
        let width = self.width();
        let at = column as f32 * (width + self.gap);
        let (a, b) = if self.rtl { (self.max_x - at - width, self.max_x - at) } else { (self.min_x + at, self.min_x + at + width) };
        Rect::from_min_max(egui::pos2(a, 0.0), egui::pos2(b, 0.0))
    }

    /// The rect of the cell at `index` in reading order.
    fn cell(&self, index: usize) -> Rect {
        let (row, column) = (index / self.columns, index % self.columns);
        let x = self.column(column);
        let y = self.top + row as f32 * (self.cell + self.gap);
        Rect::from_min_max(egui::pos2(x.min.x, y), egui::pos2(x.max.x, y + self.cell))
    }
}

impl Component for Calendar {
    const ROLE: Role = Role::TABLE;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::of(crate::contract::Input::Pointer).with_keep(KeepKeys::BlockArrows);
    type Style = CalendarStyle;
    type Output = CalendarOutput;

    fn style(cx: &StyleCx<'_>, _: CalendarStyleKey) -> CalendarStyle {
        let colors = &cx.tokens.color;
        let (cell, gap) = (cx.tokens.size.control.sm, cx.tokens.space.gap.tight);
        let ink = |color| TextLook::new(TypeRole::Body, Stateful::new(color));
        CalendarStyle {
            metrics: Metrics::new(),
            focus: cx.focus_look(),
            title: TextLook::new(TypeRole::BodyStrong, Stateful::new(cx.ink(Ink::OnPanel))),
            weekday: TextLook::new(TypeRole::Caption, Stateful::new(cx.color(colors.text.muted))),
            text: ink(cx.ink(Ink::OnPanel)),
            disabled: ink(cx.color(colors.text.disabled)),
            hover: cx.color(colors.state.hover),
            selected: cx.color(colors.state.selected),
            selected_hover: cx.color(colors.state.selected_hover),
            today: cx.color(colors.accent.solid),
            radius: UiTokens::corner(cx.tokens.shape.radius.control),
            cell,
            gap,
            strip: cx.tokens.size.control.md,
            width: 7.0 * cell + 6.0 * gap,
        }
    }

    /// The page's size does not depend on its view: a strip, a weekday row and six rows of cells.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        self.measure_lines(cx);
        let style = cx.style;
        Vec2::new(style.width, style.strip + style.gap + style.cell + style.gap + ROWS as f32 * (style.cell + style.gap))
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        self.measure_lines(cx);
        let dates = dates_of(cx.env.host());
        let numbers = Numbers(cx.env.format());
        let page = Page::of(&self.view, &dates, &numbers, false);
        let geometry = Geometry::of(rect, cx.style, cx.flow.is_rtl(), &self.view, page.columns);
        let cell_part = |cx: &mut PartsCx<'_, Self>, cell: &Cell, at: usize, focusable: bool| {
            let part = cx.part(part_of(&self.view, cell.key), geometry.cell(at));
            if !self.permitted(cell) && !focusable {
                return;
            }
            let part = if focusable {
                part.sense(Sense::CLICK | Sense::FOCUS).focusable()
            } else {
                part.sense(Sense::CLICK | Sense::HOVER).keyboard_via(ACTIVE)
            };
            if self.selected(cell) {
                part.flags(Flags::SELECTED);
            }
        };
        // The active cell is declared first so that a popup opening on the page puts focus on it.
        if let Some(at) = page.position(self.view.active)
            && let Some(cell) = page.cells[at].as_ref()
        {
            cell_part(cx, cell, at, true);
        }
        for (at, cell) in page.cells.iter().enumerate() {
            if let Some(cell) = cell.as_ref().filter(|cell| cell.key != self.view.active) {
                cell_part(cx, cell, at, false);
            }
        }
        cx.part(PREVIOUS, geometry.previous).sense(Sense::CLICK | Sense::FOCUS).focusable();
        cx.part(NEXT, geometry.next).sense(Sense::CLICK | Sense::FOCUS).focusable();
        if self.title_goes_up() {
            cx.part(TITLE, geometry.title).sense(Sense::CLICK | Sense::FOCUS).priority(HitPriority::new(HitLevel::Body, 0)).focusable();
        }
    }

    /// Arrows, Home, End, Page Up and Page Down move the view; a click, Enter or Space on a cell picks it, or on a
    /// navigation cell opens it; the buttons and the title page.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let dates = dates_of(cx.env.host());
        let format = cx.env.format();
        let page = Page::of(&self.view, &dates, &Numbers(format), false);
        let mut view = self.view;
        while let Some((mv, _)) = cx.intents().take_move() {
            view = view.stepped(mv, &dates, format);
        }
        for cell in page.cells.iter().flatten() {
            let part = part_of(&self.view, cell.key);
            if !self.permitted(cell) {
                continue;
            }
            if cx.part_activated(part).is_some() {
                if cell.nav {
                    view = view.entered(cell.key, &dates, self.value, self.today);
                } else {
                    self.picked = Some(cell.date);
                }
            }
            if !cell.nav && cx.part(part).hovered {
                self.hovered = Some(cell.date);
            }
        }
        if cx.part_activated(PREVIOUS).is_some() {
            view = view.paged(false, &dates);
        }
        if cx.part_activated(NEXT).is_some() {
            view = view.paged(true, &dates);
        }
        if self.title_goes_up() && cx.part_activated(TITLE).is_some() {
            view = view.up(&dates, self.value, self.today);
        }
        self.moved = (view != self.view).then_some(view);
    }

    /// The grid's size, its cells with their places, and the title's button.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let dates = dates_of(cx.env.host());
        let numbers = Numbers(cx.env.format());
        let page = Page::of(&self.view, &dates, &numbers, true);
        access.grid(GridPlace::grid(page.rows(), page.columns));
        for (at, cell) in page.cells.iter().enumerate() {
            let Some(cell) = cell else { continue };
            let node = access.node(part_of(&self.view, cell.key), Role::CELL);
            node.name = Name::Text(cell.name.clone());
            node.grid = GridPlace::cell(at / page.columns, at % page.columns);
        }
        access.node(PREVIOUS, Role::BUTTON);
        access.node(NEXT, Role::BUTTON);
        if self.title_goes_up() {
            let node = access.node(TITLE, Role::BUTTON);
            node.name = Name::Text(page.title(&self.view, &numbers, &dates));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let words = cx.words::<PickerWords>();
        ui.face(PREVIOUS, IconButton::new(IconId::new(PREVIOUS_ICON)).label(words.date_previous.clone()));
        ui.face(NEXT, IconButton::new(IconId::new(NEXT_ICON)).label(words.date_next.clone()));
        let dates: Arc<DateFormat> = dates_of(cx.env.host());
        let numbers = Numbers(cx.env.format());
        let page = Page::of(&self.view, &dates, &numbers, false);
        let style = cx.style;
        let geometry = Geometry::of(cx.rect, style, cx.flow.is_rtl(), &self.view, page.columns);
        let draw = ui.draw();
        let centred = |rect: Rect, text: &str, look: &TextLook, line: f32| {
            let at = Rect::from_center_size(rect.center(), vec2(rect.width(), line));
            draw.text(at, &TextRun::new(text).role(look.role).color(cx.resolve(&look.color)).align(HAlign::Center));
        };
        centred(geometry.title, &page.title(&self.view, &numbers, &dates), &style.title, self.lines[0]);
        if let Some(row) = geometry.weekdays {
            let first = dates.first_weekday() as usize;
            for column in 0..DAY_COLUMNS {
                let name = dates.weekday_name(Weekday::ALL[(first + column) % 7], NameLength::Narrow);
                let x = geometry.column(column);
                centred(Rect::from_min_max(egui::pos2(x.min.x, row.min.y), egui::pos2(x.max.x, row.max.y)), &name, &style.weekday, self.lines[1]);
            }
        }
        for (at, cell) in page.cells.iter().enumerate() {
            let Some(cell) = cell else { continue };
            let rect = geometry.cell(at);
            let permitted = self.permitted(cell);
            let hovered = cx.part(part_of(&self.view, cell.key)).hovered;
            let shape = PaintShape::Rect(rect, style.radius);
            match (self.selected(cell) && permitted, hovered && permitted) {
                (true, true) => draw.fill(&shape, style.selected_hover),
                (true, false) => draw.fill(&shape, style.selected),
                (false, true) => draw.fill(&shape, style.hover),
                _ => {}
            }
            if self.holds_today(cell) {
                draw.stroke(&shape, StrokeWidth::Thin, style.today);
            }
            centred(rect, &cell.text, if permitted { &style.text } else { &style.disabled }, self.lines[2]);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> CalendarOutput {
        CalendarOutput { response: cx.response.clone(), picked: self.picked, hovered: self.hovered, moved: self.moved }
    }
}
