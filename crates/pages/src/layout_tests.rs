//! Page placement, fits and spreads.

// Three Letter pages at 100 % with ACROBAT options sit centred, 14 pt apart, from 28 pt down; with gap 0 their heights are whole device pixels.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn single_pages_centre_and_stack_with_gaps() {
    todo!()
}

// Five pages two-up with a cover: rows [0], [1, 2], [3, 4], page 0 right of the centre line; shown_row and spread_first agree for every page.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn spreads_follow_the_cover_rule() {
    todo!()
}

// Word options in a 2000 pt wide viewport at 50 % put four pages per row, never more than the page count, at least min_columns.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn columns_fit_as_many_pages_as_the_width_holds() {
    todo!()
}

// Fit width with a wide page 3 fits page 3's width when continuous, the current page's when single and not continuous; the scale stays inside ZOOM_RANGE.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn fits_use_the_largest_page_unless_one_page_shows() {
    todo!()
}

// Bodies of three pages collapse to bands 6 pt beyond the text with 1 pt seams; page_at misses a hidden margin; on_page_gap finds the seam.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn hide_white_space_cuts_margins_and_keeps_coordinates() {
    todo!()
}

// Every display and continuous pair survives display_token then parse_display_token; "Two-Up-Single" reads as two-page; "twoup" is refused.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn display_tokens_round_trip() {
    todo!()
}

// fit-width, fit-page and 125% round-trip; 6500% and "fit" are refused.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn zoom_names_round_trip_and_refuse_out_of_range() {
    todo!()
}
