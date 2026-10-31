//! Page range text and From/To ranges.

// "5, 1-3; 2" in 9 pages is 0..=2 and 4; "3-1" reads forwards; "0" and "12" are out of range; "x" is not a page; blank is no pages; 20 000 bytes is too long.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn range_text_parses_sorted_unique_and_bounded() {
    todo!()
}

// [0, 1, 2, 4, 7, 8] formats as "1-3, 5, 8-9" and parses back to itself.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn range_text_formats_runs() {
    todo!()
}

// All pages of 5 is 0..5; from 2 to 9 of 5 even is pages 2 and 4 (indices 1 and 3).
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn from_to_ranges_take_subsets() {
    todo!()
}
