//! Mixed values.
#![allow(clippy::expect_used, reason = "tests")]

use super::mixed::{ApplyValue, MixedValue};

// `MixedValue::of`: none gives `None`, equal values `Same`, any difference `Mixed`.
#[test]
fn the_shared_value_or_mixed() {
    assert_eq!(MixedValue::<i32>::of([]), None);
    assert_eq!(MixedValue::of([3, 3, 3]), Some(MixedValue::Same(3)));
    assert_eq!(MixedValue::of([3, 3, 4]), Some(MixedValue::Mixed));
    assert_eq!(MixedValue::of([3, 4]).and_then(MixedValue::same), None);
}

// `ApplyValue::change` is `None` until `apply` is on, and `None` again when the value equals the current one.
#[test]
fn an_apply_row_changes_only_when_applied_and_different() {
    let mut row = ApplyValue::of([1, 2]).expect("two items");
    assert!(row.mixed && !row.apply, "opening the dialog applies nothing");
    assert_eq!(row.change(&2), None);
    row.apply = true;
    assert_eq!(row.change(&2), Some(1));
    assert_eq!(row.change(&1), None);
    assert!(!ApplyValue::of([5, 5]).expect("two items").mixed);
}
