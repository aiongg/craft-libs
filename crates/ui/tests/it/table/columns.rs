//! Ordering rows by the table's sort state.

use craft_behavior::ColumnKey;
use craft_ui::contract::ColumnSort;
use craft_ui::table::sort_rows;
use craft_ui::testing::trial;

const KIND: ColumnKey = ColumnKey::new("kind");
const NAME: ColumnKey = ColumnKey::new("name");

fn rows() -> Vec<(&'static str, u8)> {
    vec![("b", 1), ("a", 2), ("c", 1), ("d", 2)]
}

fn compare(a: &(&'static str, u8), b: &(&'static str, u8), column: &ColumnKey) -> std::cmp::Ordering {
    if *column == KIND { a.1.cmp(&b.1) } else { a.0.cmp(b.0) }
}

fn names(rows: &[(&'static str, u8)]) -> Vec<&'static str> {
    rows.iter().map(|row| row.0).collect()
}

#[trial]
fn descending_reverses_the_order_and_equal_rows_keep_theirs() {
    let mut ascending = rows();
    sort_rows(&mut ascending, Some(&ColumnSort::ascending(KIND)), compare);
    assert_eq!(names(&ascending), ["b", "c", "a", "d"]);
    let mut descending = rows();
    sort_rows(&mut descending, Some(&ColumnSort::descending(KIND)), compare);
    assert_eq!(names(&descending), ["a", "d", "b", "c"]);
    let mut by_name = rows();
    sort_rows(&mut by_name, Some(&ColumnSort::descending(NAME)), compare);
    assert_eq!(names(&by_name), ["d", "c", "b", "a"]);
}

#[trial]
fn no_sort_leaves_the_rows_alone() {
    let mut unsorted = rows();
    sort_rows(&mut unsorted, None, compare);
    assert_eq!(names(&unsorted), ["b", "a", "c", "d"]);
}
