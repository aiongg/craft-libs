//! `PickedDate` rules and the calendar grid.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `parse`: "2026", "2026-08", "2026-08-14T10:00" read; "2026-02-30" and "2026-13" do not; `iso` writes them back.
#[test]
#[ignore = "skeleton"]
fn dates_parse_at_their_precision_and_reject_impossible_days() {
    todo!()
}

// `in_bounds`: the year 2026 is inside a min of 2026-12-31; a month before a min is not.
#[test]
#[ignore = "skeleton"]
fn a_value_is_in_bounds_when_any_day_is() {
    todo!()
}

// `month_grid`: August 2026 starts on a Saturday: five blanks from Monday, six from Sunday; the length is a multiple of 7.
#[test]
#[ignore = "skeleton"]
fn the_month_grid_starts_on_the_week_start() {
    todo!()
}

// In a harness: on the Day page a click on the 14th sets 2026-08-14 and closes the popup; Clear sets `None`.
#[test]
#[ignore = "skeleton"]
fn picking_a_cell_sets_the_value_and_closes() {
    todo!()
}
