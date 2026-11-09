//! Page range text and From/To ranges.

use super::*;

#[test]
fn range_text_parses_sorted_unique_and_bounded() {
    assert_eq!(parse_pages("5, 1-3; 2", 9), Ok(vec![0, 1, 2, 4]));
    assert_eq!(parse_pages("3-1", 9), Ok(vec![0, 1, 2]));
    assert_eq!(parse_pages("0", 9), Err(PageRangeError::OutOfRange { page: 0, count: 9 }));
    assert_eq!(parse_pages("12", 9), Err(PageRangeError::OutOfRange { page: 12, count: 9 }));
    assert_eq!(parse_pages("x", 9), Err(PageRangeError::NotAPage("x".into())));
    assert_eq!(parse_pages("  ", 9), Ok(vec![]));
    assert_eq!(parse_pages(&"1,".repeat(10_000), 9), Err(PageRangeError::TooLong));
    let overlaps = ["1-100000"; 11].join(",");
    assert_eq!(parse_pages(&overlaps, 100_000), Err(PageRangeError::TooLong));
    assert_eq!(PageRangeError::OutOfRange { page: 12, count: 9 }.to_string(), "page 12 is out of range (1\u{2013}9)");
}

#[test]
fn range_text_formats_runs() {
    let pages = [0, 1, 2, 4, 7, 8];
    assert_eq!(format_pages(&pages), "1-3, 5, 8-9");
    assert_eq!(parse_pages(&format_pages(&pages), 9), Ok(pages.to_vec()));
}

#[test]
fn from_to_ranges_take_subsets() {
    assert_eq!(PageRange::default().pages(5), [0, 1, 2, 3, 4]);
    let range = PageRange { all: false, from: 2, to: Some(9), subset: Subset::Even };
    assert_eq!(range.pages(5), [1, 3]);
}

// A range with no end runs to the last page of any document.
