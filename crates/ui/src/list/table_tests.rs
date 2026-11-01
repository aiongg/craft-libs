//! `TableSort` and `TableLayout` rules.

use super::{SortOrder, TableColumn, TableLayout, TableSort};

fn columns() -> Vec<TableColumn<&'static str>> {
    vec![
        TableColumn::new("name", "Name").min_width(120.0),
        TableColumn::new("pages", "Pages").min_width(60.0).hideable(true),
        TableColumn::new("size", "Size").min_width(60.0).hideable(true),
    ]
}

// `TableSort::clicked`: the sorting column flips its order; another column starts ascending.
#[test]
fn a_header_click_flips_the_sorting_column_and_sorts_another_ascending() {
    let first = TableSort::clicked(None, "name");
    assert_eq!(first, TableSort { column: "name", order: SortOrder::Ascending });
    let flipped = TableSort::clicked(Some(&first), "name");
    assert_eq!(flipped.order, SortOrder::Descending);
    let other = TableSort::clicked(Some(&flipped), "size");
    assert_eq!(other, TableSort { column: "size", order: SortOrder::Ascending });
}

// `TableSort::apply` is stable and reverses the comparison for Descending.
#[test]
fn descending_reverses_and_ties_keep_their_order() {
    let mut rows = vec![("a", 2), ("b", 1), ("c", 2), ("d", 1)];
    let compare = |x: &(&str, i32), y: &(&str, i32), _: &&str| x.1.cmp(&y.1);
    TableSort { column: "n", order: SortOrder::Ascending }.apply(&mut rows, compare);
    assert_eq!(rows.iter().map(|r| r.0).collect::<String>(), "bdac");
    TableSort { column: "n", order: SortOrder::Descending }.apply(&mut rows, compare);
    assert_eq!(rows.iter().map(|r| r.0).collect::<String>(), "acbd", "ties keep the order they had");
}

// `set_visible(false)` then `(true)` leaves `order` unchanged.
#[test]
fn a_hidden_column_comes_back_where_it_was() {
    let mut layout = TableLayout::new(&columns());
    layout.set_visible(&"pages", false);
    assert!(!layout.is_visible(&"pages"));
    assert_eq!(layout.order, ["name", "pages", "size"]);
    layout.set_visible(&"pages", true);
    assert!(layout.is_visible(&"pages"));
    assert_eq!(layout.order, ["name", "pages", "size"]);
}

// `repaired` drops unknown keys, appends missing columns, drops non-finite widths and clamps the rest.
#[test]
fn a_stored_layout_is_repaired_against_the_columns() {
    let stored = TableLayout {
        order: vec!["size", "gone", "name", "size"],
        hidden: vec!["gone", "pages", "name"],
        widths: vec![("name", 10.0), ("pages", f32::NAN), ("size", 9000.0), ("gone", 50.0)],
        sort: Some(TableSort { column: "gone", order: SortOrder::Descending }),
    };
    let repaired = stored.repaired(&columns());
    assert_eq!(repaired.order, ["size", "name", "pages"]);
    assert_eq!(repaired.hidden, ["pages"], "only hideable known columns stay hidden");
    assert_eq!(repaired.widths, [("name", 120.0), ("size", 4000.0)]);
    assert_eq!(repaired.sort, None);
}

// `move_column` with and without `after`.
#[test]
fn a_dropped_header_moves_its_column_before_or_after_the_target() {
    let mut layout = TableLayout::new(&columns());
    layout.move_column(&"name", &"size", false);
    assert_eq!(layout.order, ["pages", "name", "size"]);
    layout.move_column(&"pages", &"size", true);
    assert_eq!(layout.order, ["name", "size", "pages"]);
    layout.move_column(&"name", &"name", true);
    assert_eq!(layout.order, ["name", "size", "pages"], "a column dropped on itself stays");
}
