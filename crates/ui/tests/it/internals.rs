//! Row geometry, footer order, reorder arithmetic and the submenu pointer triangle, checked without a frame.

use craft_behavior::Axis;
use craft_ui::contract::DropMark;
use craft_ui::list::ListRow;
use craft_ui::sections::DialogRole;
use craft_ui::testing::internals::{
    footer_order, place_row as place, pointer_heads_toward, reorder_gap_at, reorder_move_all_to, reorder_move_to, row_indent_steps,
};
use craft_ui::testing::trial;
use egui::{Pos2, Rect, pos2, vec2};

#[trial]
fn optional_items_drop_out_before_the_label_gets_too_narrow() {
    let row = ListRow::new("Layer").trailing("ab").optional("abcd");
    assert!(place(&row, 150.0, false).items.iter().all(Option::is_some));
    let narrow = place(&row, 120.0, false);
    assert!(narrow.items[0].is_some() && narrow.items[1].is_none());
    assert!(narrow.label.width() >= narrow.min_label);
}

#[trial]
fn a_row_too_narrow_for_its_required_parts_keeps_the_label_at_its_least_width() {
    let row = ListRow::new("Layer").trailing("ab");
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
    let row = ListRow::new("Layer").trailing("ab");
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

fn column(count: usize) -> Vec<Rect> {
    (0..count).map(|n| Rect::from_min_size(Pos2::new(0.0, n as f32 * 20.0), vec2(100.0, 20.0))).collect()
}

#[trial]
fn a_moved_item_leaves_first() {
    assert_eq!(reorder_move_to(1, 3), 2);
    assert_eq!(reorder_move_to(3, 1), 1);
    assert_eq!(reorder_move_to(2, 2), 2);
    assert_eq!(reorder_move_all_to(&[0, 1], 4), 2);
}

#[trial]
fn the_gap_follows_the_pointer_along_the_axis() {
    let rects = column(3);
    let at = |y: f32| reorder_gap_at(&rects, Axis::Y, pos2(50.0, y)).map(|gap| gap.insert);
    assert_eq!((at(2.0), at(30.0), at(500.0)), (Some(0), Some(2), Some(3)));
    assert_eq!(reorder_gap_at(&[], Axis::Y, Pos2::ZERO).map(|gap| gap.insert), None);
}

#[trial]
fn the_gap_line_lies_on_the_edge_of_the_next_item() {
    let gap = reorder_gap_at(&column(2), Axis::Y, pos2(50.0, 500.0)).expect("a gap");
    assert_eq!(gap.mark, DropMark::line(pos2(0.0, 40.0), pos2(100.0, 40.0)));
}

#[trial]
fn the_pointer_heads_for_the_submenu_inside_the_triangle_to_its_near_edge() {
    let submenu = Rect::from_min_max(pos2(100.0, 0.0), pos2(200.0, 100.0));
    let from = pos2(0.0, 50.0);
    assert!(pointer_heads_toward(from, pos2(50.0, 50.0), submenu));
    assert!(pointer_heads_toward(from, pos2(50.0, 30.0), submenu));
    assert!(!pointer_heads_toward(from, pos2(50.0, 200.0), submenu));
    assert!(!pointer_heads_toward(from, pos2(-20.0, 50.0), submenu));
}
