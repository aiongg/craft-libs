//! Row geometry and footer order, checked without a frame.

use craft_ui::list::ListRow;
use craft_ui::sections::DialogRole;
use craft_ui::testing::internals::{footer_order, place_row as place, row_indent_steps};
use craft_ui::testing::trial;

#[trial]
fn optional_items_drop_out_before_the_label_gets_too_narrow() {
    let row = ListRow::new("Layer").trailing(1.0, |_, _| {}).optional(2.0, |_, _| {});
    assert!(place(&row, 150.0, false).items.iter().all(Option::is_some));
    let narrow = place(&row, 120.0, false);
    assert!(narrow.items[0].is_some() && narrow.items[1].is_none());
    assert!(narrow.label.width() >= narrow.min_label);
}

#[trial]
fn a_row_too_narrow_for_its_required_parts_keeps_the_label_at_its_least_width() {
    let row = ListRow::new("Layer").trailing(1.0, |_, _| {});
    let placed = place(&row, 60.0, false);
    assert_eq!(placed.label.width(), placed.min_label);
    assert!(placed.items[0].is_some_and(|item| item.right() > 60.0), "the item moves out with the label and the row's edge cuts it");
}

#[trial]
fn the_indent_shrinks_before_the_label_does_and_the_disclosure_column_keeps_its_room() {
    let row = ListRow::new("Group").depth(5).group(true);
    let placed = place(&row, 120.0, false);
    assert_eq!(placed.label.width(), placed.min_label);
    let disclosure = placed.disclosure.expect("a group row has a disclosure column");
    assert!(disclosure.right() <= placed.label.left());
    assert_eq!(placed.label.left() - disclosure.right(), 0.0);
}

#[trial]
fn the_row_mirrors_under_right_to_left() {
    let row = ListRow::new("Layer").trailing(1.0, |_, _| {});
    let (ltr, rtl) = (place(&row, 200.0, false), place(&row, 200.0, true));
    assert_eq!(ltr.label.left(), 200.0 - rtl.label.right());
    assert_eq!(ltr.items[0].map(|item| item.left()), rtl.items[0].map(|item| 200.0 - item.right()));
}

#[trial]
fn a_leaf_inside_a_group_indents_one_level_more_than_its_depth() {
    let leaf = ListRow::new("Leaf").depth(1);
    let top = ListRow::new("Leaf").depth(0);
    assert_eq!(row_indent_steps(&leaf), 2);
    assert_eq!(row_indent_steps(&top), 0);
    assert_eq!(row_indent_steps(&ListRow::new("Group").depth(1).group(false)), 1);
}
#[trial]
fn accept_is_last_off_windows_and_first_on_windows() {
    let roles = [DialogRole::Accept, DialogRole::Cancel, DialogRole::Other];
    assert_eq!(footer_order(&roles, false), vec![2, 1, 0]);
    assert_eq!(footer_order(&roles, true), vec![0, 1, 2]);
}
