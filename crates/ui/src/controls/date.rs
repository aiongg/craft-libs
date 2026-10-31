#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`DatePicker`]: a calendar button whose popup picks a year, a month or a day, and the
//! [`PickedDate`] it edits.

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::ControlSize;

crate::words! {
    /// The picker's words, English by default. Month and weekday names come from the interface
    /// language's date formats.
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
/// its precision: `2026` means the whole year.
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
        todo!()
    }

    /// `2026`, `2026-08` or `2026-08-14`, years padded to four digits.
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:87 iso
    pub fn iso(&self) -> String {
        todo!()
    }

    pub fn precision(&self) -> DatePrecision {
        match (self.month, self.day) {
            (Some(_), Some(_)) => DatePrecision::Day,
            (Some(_), None) => DatePrecision::Month,
            _ => DatePrecision::Year,
        }
    }

    /// The first day it covers, as (year, month, day).
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:104 first_day
    pub fn first_day(&self) -> (i32, u8, u8) {
        todo!()
    }

    /// The last day it covers.
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:109 last_day
    pub fn last_day(&self) -> (i32, u8, u8) {
        todo!()
    }

    /// Whether any day of it lies between `min` and `max`, each optional and inclusive.
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:141 in_bounds
    pub fn in_bounds(&self, min: Option<&PickedDate>, max: Option<&PickedDate>) -> bool {
        todo!()
    }
}

/// The days of `month` (1 to 12) of `year` in the proleptic Gregorian calendar.
// lightcraft: crates/ui-egui/src/widgets/date_picker.rs:116 days_in_month
pub fn days_in_month(year: i32, month: u8) -> u8 {
    todo!()
}

/// The month's days laid out in weeks starting on `first_weekday` (0 Monday to 6 Sunday): `None`
/// before the 1st and after the last day, so the length is a multiple of 7. LightCraft always
/// starts on Monday; the week start here is the interface locale's.
// lightcraft: crates/ui-egui/src/widgets/date_picker.rs:129 month_grid
pub fn month_grid(year: i32, month: u8, first_weekday: u8) -> Vec<Option<u8>> {
    todo!()
}

/// A calendar button that edits an optional [`PickedDate`], after LightCraft's date picker.
///
/// ```ignore
/// if DatePicker::new("from", "Captured from", &mut rule.from).precisions(&[DatePrecision::Day]).show(ui).changed() { save() }
/// ```
///
/// - **Button.** An [`IconButton`](crate::controls::IconButton) with the calendar icon (a new
///   `calendar` icon; LightCraft's 24 × 22) named [`DateLabels::pick`]; the host shows the value
///   beside it in its own words (craft-i18n's date format).
/// - **Popup.** A [`Popover`](crate::popover::Popover) under the button. A
///   [`Segmented`](crate::controls::Segmented) of the allowed precisions (Year, Month, Day) when
///   more than one is allowed; under it Previous, the page's title and Next; then the page: Day
///   shows the month as a 7-column grid with weekday initials over it (the locale's week start
///   and short names), Month the twelve months of the year in 3 × 4, Year twelve years (2016 to
///   2027) in 3 × 4. Paging stops at years 0000 and 9999 (LightCraft's `step`). Cells are 26
///   points wide and 2 apart; the value's cell is selected (`listbox.active.*`), today's has an
///   outline ring in `accent.solid`, cells outside `min`/`max` are disabled. Below, Today and
///   Clear as link buttons.
/// - **Choosing.** A click on a cell sets the value at the page's precision and closes the
///   popup; Today sets today at the current precision; Clear sets `None`.
/// - **Keys.** In the grid the arrows move the active cell (a week up and down on the Day page),
///   Page Up and Page Down page, Home and End go to the row's ends, Enter and Space choose, Esc
///   closes and gives the focus back to the button. The popup opens with the value's cell (or
///   today's) active.
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
        }
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

    /// What today is (tests and screenshots fix it); the local clock by default.
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

    /// The button; `changed()` when a cell, Today or Clear changed the value.
    // lightcraft: crates/ui-egui/src/widgets/date_picker.rs:238 show
    // needs: services craft-i18n::format (N11.02) for month and weekday names, the week start and the long date.
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
