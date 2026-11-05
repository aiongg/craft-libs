//! Page labels.

use crate::numbering::{NumberStyle, PageNumbering};

// Start 3, lower roman, prefix "A-" included: page 0 is "A-iii", page 1 "A-iv"; without include_prefix "iv".
#[test]
fn labels_count_from_the_start_with_the_prefix() {
    let mut numbering = PageNumbering { style: NumberStyle::LowerRoman, start: 3, prefix: "A-".into(), include_prefix: true };
    assert_eq!((numbering.label(0), numbering.label(1)), ("A-iii".to_string(), "A-iv".to_string()));
    numbering.include_prefix = false;
    assert_eq!(numbering.label(1), "iv");
    assert_eq!(PageNumbering { start: u32::MAX, ..PageNumbering::default() }.label(5), u32::MAX.to_string());
}
