//! [`DatePicker`]: a calendar button whose popup picks a year, a month or a day, and the
//! [`PickedDate`] it edits.

use std::sync::Arc;

use craft_host::Host;
use craft_i18n::{CalendarDate, DateFormat, DateLength, DateStyle, DateTime, NameLength, TimeZone, Weekday};
use egui::accesskit::Role;
use egui::{Align2, AsIdSalt, Color32, CornerRadius, Id, IdSalt, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};

use super::option_list::text_galley;
use super::{ControlSize, IconButton, Link, ListboxStyle, Segmented, Select};
use crate::behavior::{GridKey, GridNav};
use crate::direction;
use crate::edit::{EditEvent, EditResponse};
use crate::icons::Icon;
use crate::localized::Labels as _;
use crate::popover::{Popover, PopoverAnchor, PopoverClose, PopoverSize};
use crate::primitives::Interactive;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

/// The last year that can be picked (four digits).
const LAST_YEAR: i32 = 9999;
/// Years on a Year page.
const YEARS_PER_PAGE: i32 = 12;

crate::words! {
    /// The picker's words, English by default. Month and weekday names come from the region's
    /// date format (services' `craft_i18n::DateFormat::{month_name, weekday_name}`).
    #[non_exhaustive]
    pub struct DateLabels {
        /// The calendar button's tooltip and accessible name (LightCraft's).
        pick: "date.pick" = "Pick a date";
        year: "date.year" = "Year";
        month: "date.month" = "Month";
        day: "date.day" = "Day";
        previous: "date.previous" = "Previous";
        next: "date.next" = "Next";
        today: "date.today" = "Today";
        clear: "date.clear" = "Clear";
        /// The precision switch's accessible name.
        precision: "date.precision" = "Pick by";
    }
}

/// How much of a date a value names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DatePrecision {
    Year,
    Month,
    Day,
}

impl DatePrecision {
    pub const ALL: [DatePrecision; 3] = [DatePrecision::Year, DatePrecision::Month, DatePrecision::Day];
}

/// A year, a month of it or a day of that month: `2026`, `2026-08`, `2026-08-14`. A value keeps
/// its precision: `2026` means the whole year. A day-precision value is one calendar date;
/// conversions to and from services' `craft_i18n::CalendarDate` are added when the two parts
/// merge, and the calendar arithmetic (days in a month, weekdays, paging by months) is
/// `CalendarDate`'s, not a copy here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PickedDate {
    pub year: i32,
    /// 1 to 12.
    pub month: Option<u8>,
    /// 1 to the month's last day; only with a month.
    pub day: Option<u8>,
}

impl PickedDate {
    pub fn year(year: i32) -> Self {
        Self { year, month: None, day: None }
    }

    pub fn month(year: i32, month: u8) -> Self {
        Self { year, month: Some(month), day: None }
    }

    pub fn day(year: i32, month: u8, day: u8) -> Self {
        Self { year, month: Some(month), day: Some(day) }
    }

    /// Reads `2026`, `2026-08` or `2026-08-14`; anything after a day's `T` or a space (a time) is
    /// read past. `None` for anything else or a date that doesn't exist (`2026-02-30`).
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:59 parse
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let number =
            |from: usize, to: usize| text.get(from..to).filter(|t| t.bytes().all(|c| c.is_ascii_digit())).and_then(|t| t.parse::<u32>().ok());
        let year = i32::try_from(number(0, 4)?).ok()?;
        if text.len() == 4 {
            return Some(Self::year(year));
        }
        let month = u8::try_from(number(5, 7)?).ok().filter(|m| (1..=12).contains(m))?;
        if text.get(4..5) != Some("-") {
            return None;
        }
        if text.len() == 7 {
            return Some(Self::month(year, month));
        }
        let day = u8::try_from(number(8, 10)?).ok().filter(|d| (1..=CalendarDate::days_in_month(i64::from(year), month)).contains(d))?;
        // A time after the day is read past, but it must be one: T10, T10:00 or T10:00:00, the T
        // or a space.
        let time_ok = match text.len() {
            10 => true,
            13 | 16 | 19 => {
                matches!(text.get(10..11), Some("T" | " "))
                    && text.char_indices().skip(11).all(|(i, c)| if i == 13 || i == 16 { c == ':' } else { c.is_ascii_digit() })
            }
            _ => false,
        };
        (text.get(7..8) == Some("-") && time_ok).then_some(Self::day(year, month, day))
    }

    /// `2026`, `2026-08` or `2026-08-14`, years padded to four digits.
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:87 iso
    pub fn iso(&self) -> String {
        match (self.month, self.day) {
            (Some(month), Some(day)) => format!("{:04}-{month:02}-{day:02}", self.year),
            (Some(month), None) => format!("{:04}-{month:02}", self.year),
            _ => format!("{:04}", self.year),
        }
    }

    pub fn precision(&self) -> DatePrecision {
        match (self.month, self.day) {
            (Some(_), Some(_)) => DatePrecision::Day,
            (Some(_), None) => DatePrecision::Month,
            _ => DatePrecision::Year,
        }
    }

    /// The first day it covers, at day precision.
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:104 first_day
    pub fn first_day(&self) -> PickedDate {
        Self::day(self.year, self.month.unwrap_or(1), self.day.unwrap_or(1))
    }

    /// The last day it covers, at day precision (`CalendarDate::days_in_month` for a month's).
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:109 last_day
    // needs: services craft_i18n::CalendarDate::days_in_month (N11.02).
    pub fn last_day(&self) -> PickedDate {
        let month = self.month.unwrap_or(12);
        Self::day(self.year, month, self.day.unwrap_or_else(|| CalendarDate::days_in_month(i64::from(self.year), month)))
    }

    /// The calendar date of its first day, for `craft_i18n::DateFormat::format`.
    pub fn calendar_date(&self) -> Option<CalendarDate> {
        let first = self.first_day();
        CalendarDate::new(i64::from(first.year), first.month?, first.day?)
    }

    /// The day-precision value of `date`; `None` for a year outside the four digits.
    pub fn from_calendar_date(date: CalendarDate) -> Option<Self> {
        let year = i32::try_from(date.year).ok().filter(|y| (0..=LAST_YEAR).contains(y))?;
        Some(Self::day(year, date.month, date.day))
    }

    fn day_key(&self) -> (i32, u8, u8) {
        (self.year, self.month.unwrap_or(1), self.day.unwrap_or(1))
    }

    /// Whether any day of it lies between `min` and `max`, each optional and inclusive.
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:141 in_bounds
    pub fn in_bounds(&self, min: Option<&PickedDate>, max: Option<&PickedDate>) -> bool {
        min.is_none_or(|m| self.last_day().day_key() >= m.first_day().day_key())
            && max.is_none_or(|m| self.first_day().day_key() <= m.last_day().day_key())
    }
}

/// The month's days laid out in weeks starting on `first_weekday`: `None` before the 1st and
/// after the last day, so the length is a multiple of 7; empty for a month outside 1 to 12.
/// LightCraft always starts on Monday; the picker passes the region's week start
/// (`craft_i18n::DateFormat::first_weekday`).
// lightcraft: crates/ui-egui/src/widgets/date_picker.rs:129 month_grid
pub fn month_grid(year: i32, month: u8, first_weekday: Weekday) -> Vec<Option<u8>> {
    let Some(first) = CalendarDate::new(i64::from(year), month, 1) else {
        return Vec::new();
    };
    let lead = (first.weekday() as usize + 7 - first_weekday as usize) % 7;
    let mut cells: Vec<Option<u8>> =
        std::iter::repeat_n(None, lead).chain((1..=CalendarDate::days_in_month(i64::from(year), month)).map(Some)).collect();
    while !cells.len().is_multiple_of(7) {
        cells.push(None);
    }
    cells
}

/// A calendar button that edits an optional [`PickedDate`], after LightCraft's date picker.
///
/// ```ignore
/// if DatePicker::new("from", "Captured from", &mut rule.from).precisions(&[DatePrecision::Day]).show(ui).event.is_some() { save() }
/// ```
///
/// - **Button.** An [`IconButton`](crate::controls::IconButton) with the calendar icon (a new
///   `calendar` icon; LightCraft's 24 × 22) named [`DateLabels::pick`]; the host shows the value
///   beside it in its own words (`craft_i18n::DateFormat::format`).
/// - **Popup.** A [`Popover`](crate::popover::Popover) under the button. A
///   [`Segmented`](crate::controls::Segmented) of the allowed precisions (Year, Month, Day) when
///   more than one is allowed; under it Previous, the page's title and Next; then the page: Day
///   shows the month as a 7-column grid with weekday initials over it (the locale's week start
///   and narrow names), Month the twelve months of the year in 3 rows of 4, Year twelve years
///   (2016 to 2027) in 3 rows of 4. Paging stops at years 0000 and 9999 (LightCraft's `step`). Cells are 26
///   points wide and 2 apart; the value's cell is selected (`listbox.active.*`), today's has an
///   outline ring in `accent.solid`, cells outside `min`/`max` are disabled. Below, Today and
///   Clear as link buttons.
/// - **Choosing.** A click on a cell sets the value at the page's precision and closes the
///   popup; Today sets today at the current precision; Clear sets `None`. Each is a one-shot
///   [`Set`](crate::edit::EditEvent::Set) in the [`EditResponse`], in the pass it happened; the
///   value is already set. Paging, changing the precision and Esc report nothing. The press that
///   closes the popup outside it does nothing else (the menus' closing-press rule).
/// - **Keys.** In the grid the arrows move the active cell (a week up and down on the Day page),
///   Page Up and Page Down page, Home and End go to the row's ends, Enter and Space choose, Esc
///   closes and gives the focus back to the button. The popup opens with the value's cell (or
///   today's) active. The grid is one Tab stop
///   ([`GridNav::wrapped`](crate::behavior::GridNav::wrapped) with 7 or 4 columns, the active cell
///   the one that can take Tab); while the popup is open it owns the keys
///   ([`focus::key_owner`](crate::focus::key_owner)).
/// - **Names.** The Day page's title is the month's name and the year; the Month and Year pages
///   show the year and the span of years.
/// - **AccessKit.** The page is a `Grid` named by its title; cells are `GridCell`s named by the
///   full date in the locale's long format, `selected` for the value.
pub struct DatePicker<'a> {
    id_salt: IdSalt,
    label: String,
    value: &'a mut Option<PickedDate>,
    precisions: &'a [DatePrecision],
    min: Option<PickedDate>,
    max: Option<PickedDate>,
    today: Option<PickedDate>,
    size: ControlSize,
    enabled: bool,
    labels: Option<DateLabels>,
}

impl<'a> DatePicker<'a> {
    /// A picker named `label` editing `value`, at every precision.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a mut Option<PickedDate>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            precisions: &DatePrecision::ALL,
            min: None,
            max: None,
            today: None,
            size: ControlSize::Md,
            enabled: true,
            labels: None,
        }
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: DateLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The precisions offered; the first is the popup's page when the value has none of them.
    pub fn precisions(mut self, precisions: &'a [DatePrecision]) -> Self {
        self.precisions = precisions;
        self
    }

    /// The earliest value allowed: a value is allowed when any of its days is in range.
    pub fn min(mut self, min: Option<PickedDate>) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: Option<PickedDate>) -> Self {
        self.max = max;
        self
    }

    /// What today is (tests and screenshots fix it); by default the local date from services'
    /// `craft_i18n::DateTime` in the local time zone.
    pub fn today(mut self, today: PickedDate) -> Self {
        self.today = Some(today);
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The button, and the popup while it is open.
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:238 show
    // needs: services craft_i18n::{DateFormat, DateTime, CalendarDate, NameLength} (N11.02) for month and weekday names, the week start, the long date and today.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Option<PickedDate>> {
        let Self { id_salt, label, value, precisions, min, max, today, size, enabled, labels } = self;
        let labels = labels.unwrap_or_else(|| DateLabels::of(ui));
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(id_salt);
        let enabled = enabled && ui.is_enabled();
        let precisions: &[DatePrecision] = if precisions.is_empty() { &DatePrecision::ALL } else { precisions };
        let today = || today.or_else(system_today);

        let open_id = id.with("open");
        let view_id = id.with("view");
        let mut open = enabled && ctx.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        let button_id = id.with("button");
        let response = ui.add(IconButton::new(Icon::Calendar, labels.pick.as_str()).size(size).enabled(enabled).with_id(button_id));
        if enabled && response.clicked() {
            open = !open;
            if open {
                ctx.data_mut(|d| d.insert_temp(view_id, View::open(*value, today(), precisions)));
            }
        }

        let current = *value;
        let mut chosen: Option<Option<PickedDate>> = None;
        let popup_salt = (id_salt, "popup");
        let popup_id = Popover::id_in(ui, popup_salt);
        let shown = Popover::new(popup_salt, PopoverAnchor::Control(response.clone()))
            .size(PopoverSize::Fit)
            .close(PopoverClose::default())
            .toggle(button_id)
            .label(label.as_str())
            .show(ui, &mut open, |ui, pop| {
                let mut view = ctx.data(|d| d.get_temp::<View>(view_id)).unwrap_or_else(|| View::open(current, today(), precisions));
                let page = Popup { id, current, today: today(), min, max, precisions, labels: &labels, size };
                let picked = page.show(ui, &mut view, |cell| pop.initial_focus(cell));
                ctx.data_mut(|d| d.insert_temp(view_id, view));
                if picked.is_some() {
                    pop.close();
                }
                picked
            });
        if let Some(shown) = shown {
            chosen = shown.inner;
            ctx.memory_mut(|m| {
                m.swallow_presses_outside(egui::LayerId::new(egui::Order::Foreground, popup_id));
                m.allow_presses_in(response.rect);
            });
        } else {
            ctx.data_mut(|d| d.remove::<View>(view_id));
        }
        ctx.data_mut(|d| d.insert_temp(open_id, open));

        let mut response = response;
        let event = chosen.map(|date| {
            *value = date;
            response.mark_changed();
            ctx.memory_mut(|m| m.request_focus(button_id));
            EditEvent::set(date, ui.input(|i| i.modifiers))
        });
        EditResponse::new(response, event)
    }
}

/// The local date, where the platform has a clock.
fn system_today() -> Option<PickedDate> {
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        PickedDate::from_calendar_date(DateTime::from_system(std::time::SystemTime::now(), TimeZone::Local).date)
    }
}

/// The installed host's date format, else English (US) conventions kept in the context.
fn date_format(ctx: &egui::Context) -> Arc<DateFormat> {
    if let Some(dates) = Host::try_of_ctx(ctx).and_then(|host| host.i18n().map(|i18n| i18n.dates())) {
        return dates;
    }
    let key = Id::NULL.with("craft-ui-date-fallback");
    if let Some(dates) = ctx.data(|d| d.get_temp::<Arc<DateFormat>>(key)) {
        return dates;
    }
    let dates = Arc::new(DateFormat::from_tag("en-US"));
    ctx.data_mut(|d| d.insert_temp(key, Arc::clone(&dates)));
    dates
}

/// Which page the open popup shows, and the cell that has the keyboard (its day, month number
/// or year, by the page).
#[derive(Clone, Copy, Debug, PartialEq)]
struct View {
    precision: DatePrecision,
    year: i32,
    month: u8,
    active: i32,
}

impl View {
    /// The view of a popup opening on `value`, else today, at the value's precision when it is
    /// offered, else days when offered, else the first offered.
    fn open(value: Option<PickedDate>, today: Option<PickedDate>, precisions: &[DatePrecision]) -> View {
        let at = value.or(today).unwrap_or(PickedDate::year(1970));
        let fallback =
            if precisions.contains(&DatePrecision::Day) { DatePrecision::Day } else { precisions.first().copied().unwrap_or(DatePrecision::Day) };
        let precision = value.map(|v| v.precision()).filter(|p| precisions.contains(p)).unwrap_or(fallback);
        let mut view = View { precision, year: at.year.clamp(0, LAST_YEAR), month: at.month.unwrap_or(1).clamp(1, 12), active: 0 };
        view.active = view.landing(value, today);
        view
    }

    /// The first year of a Year page.
    fn first_year(&self) -> i32 {
        self.year.div_euclid(YEARS_PER_PAGE) * YEARS_PER_PAGE
    }

    /// The cell that has the keyboard when the page shows: the value's, else today's, else the
    /// first.
    fn landing(&self, value: Option<PickedDate>, today: Option<PickedDate>) -> i32 {
        let on_page = |date: PickedDate| match self.precision {
            DatePrecision::Day => (date.year == self.year && date.month == Some(self.month)).then(|| i32::from(date.day.unwrap_or(1))),
            DatePrecision::Month => (date.year == self.year).then(|| i32::from(date.month.unwrap_or(1))),
            DatePrecision::Year => (self.first_year()..self.first_year() + YEARS_PER_PAGE).contains(&date.year).then_some(date.year),
        };
        value.and_then(on_page).or_else(|| today.and_then(on_page)).unwrap_or(match self.precision {
            DatePrecision::Day | DatePrecision::Month => 1,
            DatePrecision::Year => self.first_year().min(LAST_YEAR),
        })
    }

    /// The page before or after: a month, a year or twelve years, with the active cell kept at the
    /// same place. Paging stops at years 0000 and 9999 rather than wrapping.
    fn paged(self, forward: bool) -> View {
        let sign = if forward { 1 } else { -1 };
        let (year, month) = match self.precision {
            DatePrecision::Day => {
                let m = i32::from(self.month) - 1 + sign;
                (self.year.saturating_add(m.div_euclid(12)), (m.rem_euclid(12) + 1) as u8)
            }
            DatePrecision::Month => (self.year.saturating_add(sign), self.month),
            DatePrecision::Year => (self.first_year().saturating_add(sign * YEARS_PER_PAGE), self.month),
        };
        if !(0..=LAST_YEAR).contains(&year) {
            return self;
        }
        let mut view = View { year, month, ..self };
        view.active = match self.precision {
            DatePrecision::Day => self.active.min(i32::from(CalendarDate::days_in_month(i64::from(year), month))),
            DatePrecision::Month => self.active,
            DatePrecision::Year => (self.active + sign * YEARS_PER_PAGE).clamp(0, LAST_YEAR),
        };
        view
    }
}

/// One cell of a page.
struct Cell {
    /// The day, the month number or the year.
    key: i32,
    date: PickedDate,
    text: String,
    /// The accessible name.
    name: String,
}

/// A page's cells in reading order; `None` is the blank before the 1st and after the last day.
struct Page {
    cells: Vec<Option<Cell>>,
    columns: usize,
}

impl Page {
    fn of(view: &View, dates: &DateFormat) -> Page {
        let long = |date: PickedDate| match date.calendar_date() {
            Some(day) => dates.format(DateTime { date: day, hour: 0, minute: 0, second: 0 }, DateStyle { date: Some(DateLength::Long), time: None }),
            None => date.iso(),
        };
        match view.precision {
            DatePrecision::Day => Page {
                columns: 7,
                cells: month_grid(view.year, view.month, dates.first_weekday())
                    .into_iter()
                    .map(|day| {
                        day.map(|d| {
                            let date = PickedDate::day(view.year, view.month, d);
                            Cell { key: i32::from(d), date, text: d.to_string(), name: long(date) }
                        })
                    })
                    .collect(),
            },
            DatePrecision::Month => Page {
                columns: 4,
                cells: (1..=12u8)
                    .map(|m| {
                        let date = PickedDate::month(view.year, m);
                        let name = format!("{} {:04}", dates.month_name(m, NameLength::Wide), view.year);
                        Some(Cell { key: i32::from(m), date, text: dates.month_name(m, NameLength::Abbreviated), name })
                    })
                    .collect(),
            },
            DatePrecision::Year => Page {
                columns: 4,
                cells: (view.first_year()..view.first_year() + YEARS_PER_PAGE)
                    .filter(|y| *y <= LAST_YEAR)
                    .map(|y| Some(Cell { key: y, date: PickedDate::year(y), text: format!("{y:04}"), name: format!("{y:04}") }))
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

    /// Where `key` takes the active cell from `from`, never onto a blank.
    fn moved(&self, from: usize, key: Key, rtl: bool) -> usize {
        let nav = GridNav::wrapped(self.cells.len(), self.columns).rtl(rtl);
        let to = match key {
            Key::Home => nav.row_range(nav.row_of(from)).start,
            Key::End => nav.row_range(nav.row_of(from)).end.saturating_sub(1),
            other => GridKey::from_key(other).map_or(from, |k| nav.step(from, k, None).index),
        };
        to.clamp(self.first(), self.last())
    }

    /// The title over the page.
    fn title(&self, view: &View, dates: &DateFormat) -> String {
        match view.precision {
            DatePrecision::Day => format!("{} {:04}", dates.month_name(view.month, NameLength::Wide), view.year),
            DatePrecision::Month => format!("{:04}", view.year),
            DatePrecision::Year => {
                let first = view.first_year();
                format!("{first:04}\u{2013}{:04}", (first + YEARS_PER_PAGE - 1).min(LAST_YEAR))
            }
        }
    }
}

/// What the popup needs from the picker while it is drawn.
struct Popup<'a> {
    id: Id,
    current: Option<PickedDate>,
    today: Option<PickedDate>,
    min: Option<PickedDate>,
    max: Option<PickedDate>,
    precisions: &'a [DatePrecision],
    labels: &'a DateLabels,
    size: ControlSize,
}

impl Popup<'_> {
    fn cell_id(&self, view: &View, key: i32) -> Id {
        self.id.with(("cell", view.precision as u8, key))
    }

    /// Draws the popup's contents and returns the value chosen in this pass: `Some(None)` for
    /// Clear.
    fn show(&self, ui: &mut Ui, view: &mut View, initial_focus: impl Fn(Id)) -> Option<Option<PickedDate>> {
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let date = tokens.component.date;
        let dates = date_format(&ctx);
        let list = Select::style(&tokens, self.size).list;
        let rtl = direction::is_rtl(ui);
        let allowed = |date: &PickedDate| date.in_bounds(self.min.as_ref(), self.max.as_ref());
        ui.set_width(date.popup_width);
        let mut picked: Option<Option<PickedDate>> = None;

        if self.precisions.len() > 1 {
            let before = view.precision;
            let mut precision = view.precision;
            let mut group = Segmented::new((self.id, "precision"), self.labels.precision.as_str(), &mut precision).size(self.size);
            for p in self.precisions {
                let name = match p {
                    DatePrecision::Year => &self.labels.year,
                    DatePrecision::Month => &self.labels.month,
                    DatePrecision::Day => &self.labels.day,
                };
                group = group.item(*p, name.as_str());
            }
            group.show(ui);
            if precision != before {
                view.precision = precision;
                view.active = view.landing(self.current, self.today);
            }
            ui.add_space(ui.spacing().item_spacing.y);
        }

        // Keys: the arrows, Home and End move the active cell, Page Up and Page Down the page.
        // They apply before the page is built, so it draws the new page and cell in this pass.
        let focused = {
            let page = Page::of(view, &dates);
            (0..page.cells.len()).filter_map(|at| page.cells[at].as_ref()).any(|c| ctx.memory(|m| m.has_focus(self.cell_id(view, c.key))))
        };
        let mut moved = false;
        if focused {
            let page = Page::of(view, &dates);
            if let Some(at) = page.position(view.active) {
                ctx.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        self.cell_id(view, view.active),
                        egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() },
                    )
                });
                let mut position = at;
                let mut paging = None;
                ui.input_mut(|i| {
                    for key in [Key::ArrowLeft, Key::ArrowRight, Key::ArrowUp, Key::ArrowDown, Key::Home, Key::End] {
                        if i.consume_key(Modifiers::NONE, key) {
                            position = page.moved(position, key, rtl);
                            moved = true;
                        }
                    }
                    for (key, forward) in [(Key::PageUp, false), (Key::PageDown, true)] {
                        if i.consume_key(Modifiers::NONE, key) {
                            paging = Some(forward);
                        }
                    }
                });
                if let Some(key) = page.cells[position].as_ref().map(|c| c.key) {
                    view.active = key;
                }
                if let Some(forward) = paging {
                    *view = view.paged(forward);
                    moved = true;
                }
            }
        }

        let width = ui.available_width();

        // Previous, the title, Next.
        let (strip, _) = ui.allocate_exact_size(vec2(width, date.page_button), Sense::hover());
        let forward = direction::forward(ui);
        let start = direction::start_x(ui, strip);
        let end = direction::end_x(ui, strip);
        let button = |from: f32, to: f32| Rect::from_x_y_ranges(from.min(to)..=from.max(to), strip.y_range());
        let (previous_rect, next_rect) = (button(start, start + forward * date.page_button), button(end, end - forward * date.page_button));
        if IconButton::new(Icon::ChevronLeft, self.labels.previous.as_str()).size(self.size).show_at(ui, previous_rect).clicked() {
            *view = view.paged(false);
        }
        if IconButton::new(Icon::ChevronRight, self.labels.next.as_str()).size(self.size).show_at(ui, next_rect).clicked() {
            *view = view.paged(true);
        }
        let page = Page::of(view, &dates);
        let title = page.title(view, &dates);
        ui.painter().text(
            strip.center(),
            Align2::CENTER_CENTER,
            &title,
            tokens.font(craft_theme::TypeRole::BodyStrong),
            tokens.color.text.normal.to_egui(),
        );

        // The page.
        let active_key = view.active;
        let widest = page
            .cells
            .iter()
            .flatten()
            .map(|c| text_galley(ui, list.label, &c.text, Color32::PLACEHOLDER, f32::INFINITY).size().x + 2.0 * date.cell_pad)
            .fold(date.cell, f32::max);
        let columns = page.columns;
        let grid_width = columns as f32 * widest + (columns - 1) as f32 * date.gap;
        let ids: Vec<Option<Id>> = page.cells.iter().map(|c| c.as_ref().map(|c| self.cell_id(view, c.key))).collect();
        if let Some(Some(id)) = page.position(active_key).map(|at| ids[at]) {
            initial_focus(id);
            if moved {
                ctx.memory_mut(|m| m.request_focus(id));
            }
        }
        let mut grid = ui.new_child(
            UiBuilder::new()
                .id_salt((self.id, "page", view.precision as u8))
                .max_rect(ui.available_rect_before_wrap())
                .accessibility_role(Role::Grid)
                .accessibility_label(title.as_str()),
        );
        let origin = direction::start_x(&grid, grid.max_rect()) + forward * ((width - grid_width) / 2.0).max(0.0);
        let x_of = |column: usize| origin + forward * column as f32 * (widest + date.gap);
        let cell_rect = |top: f32, column: usize, height: f32| {
            let (a, b) = (x_of(column), x_of(column) + forward * widest);
            Rect::from_min_max(pos2(a.min(b), top), pos2(a.max(b), top + height))
        };
        if view.precision == DatePrecision::Day {
            let (_, row) = grid.allocate_space(vec2(width, list.height));
            let first = dates.first_weekday() as usize;
            let muted = tokens.color.text.muted.to_egui();
            for column in 0..7 {
                let name = dates.weekday_name(Weekday::ALL[(first + column) % 7], NameLength::Narrow);
                let rect = cell_rect(row.top(), column, list.height);
                grid.painter().text(rect.center(), Align2::CENTER_CENTER, name, tokens.font(craft_theme::TypeRole::Caption), muted);
            }
            grid.add_space(date.gap);
        }
        for (row, cells) in page.cells.chunks(columns).enumerate() {
            let (_, strip) = grid.allocate_space(vec2(width, list.height));
            for (column, cell) in cells.iter().enumerate() {
                let Some(cell) = cell else { continue };
                let rect = cell_rect(strip.top(), column, list.height);
                let permitted = allowed(&cell.date);
                let mut element = Interactive::with_id(ids[row * columns + column].unwrap_or(Id::NULL), Role::GridCell, cell.name.as_str())
                    .corner_radius(list.radius)
                    .state_layer(false)
                    .selected(self.current == Some(cell.date))
                    .enabled(permitted);
                if cell.key != active_key {
                    element = element.sense(Sense::CLICK);
                }
                let response = element.interact_at(&grid, rect).finish(&grid);
                if grid.is_rect_visible(rect) {
                    self.paint(&grid, &tokens, &list, rect, cell, &response);
                }
                if moved && cell.key == active_key {
                    response.scroll_to_me(None);
                }
                if permitted && response.clicked() {
                    picked = Some(Some(cell.date));
                }
            }
            grid.add_space(date.gap);
        }
        ui.advance_cursor_after_rect(grid.min_rect());

        // Today and Clear.
        let today = self.today.map(|t| match view.precision {
            DatePrecision::Day => t,
            DatePrecision::Month => PickedDate::month(t.year, t.month.unwrap_or(1)),
            DatePrecision::Year => PickedDate::year(t.year),
        });
        ui.horizontal(|ui| {
            let today_ok = today.as_ref().is_some_and(allowed);
            if let Some(today) = today.filter(|_| self.today.is_some())
                && ui.add_enabled(today_ok, Link::new(self.labels.today.as_str()).id_salt((self.id, "today"))).clicked()
            {
                picked = Some(Some(today));
            }
            if ui.add_enabled(self.current.is_some(), Link::new(self.labels.clear.as_str()).id_salt((self.id, "clear"))).clicked() {
                picked = Some(None);
            }
        });
        picked
    }

    fn paint(&self, ui: &Ui, tokens: &Tokens, list: &ListboxStyle, rect: Rect, cell: &Cell, response: &egui::Response) {
        let permitted = cell.date.in_bounds(self.min.as_ref(), self.max.as_ref());
        let selected = self.current == Some(cell.date);
        let painter = ui.painter();
        let radius: CornerRadius = list.radius;
        if selected && permitted {
            painter.rect_filled(rect, radius, list.active_fill);
            painter.rect_stroke(rect, radius, Stroke::new(1.0, list.active_border), StrokeKind::Inside);
        } else if response.hovered() && permitted {
            painter.rect_filled(rect, radius, tokens.color.state.hover.to_egui());
        }
        let holds_today = self
            .today
            .is_some_and(|t| t.first_day().day_key() >= cell.date.first_day().day_key() && t.first_day().day_key() <= cell.date.last_day().day_key());
        if holds_today {
            painter.rect_stroke(rect, radius, Stroke::new(1.0, tokens.color.accent.solid.to_egui()), StrokeKind::Inside);
        }
        let color = match () {
            () if !permitted => list.text_disabled,
            () if selected => list.active_text,
            () => list.text,
        };
        let galley = text_galley(ui, list.label, &cell.text, color, rect.width());
        painter.galley(rect.center() - galley.size() / 2.0, galley, color);
    }
}
