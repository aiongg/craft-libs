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

