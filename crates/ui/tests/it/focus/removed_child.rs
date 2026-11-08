//! Focus after a focused child of a collection row leaves the tree.

use craft_behavior::{DisplayOrder, ItemSource};
use craft_ui::contract::{Bind, Role, SelectMode, Show as _, ShowCollection as _};
use craft_ui::list::{List, ListRow};
use craft_ui::testing::access::focused_key;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::HexField;
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

struct Two;

impl DisplayOrder<usize> for Two {
    fn len(&self) -> usize {
        2
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for Two {}

/// A row child that holds focus and then leaves the tree hands focus back to its row (the collection's stop), not to
/// the control that follows the list.
#[trial]
fn removing_a_focused_row_child_leaves_focus_on_the_row() {
    let hosted = Rc::new(Cell::new(true));
    let flag = hosted.clone();
    let mut fixture = Fixture::new(move |ui| {
        let hosted = flag.get();
        let _ = List::new("Items").select(SelectMode::Single).show_items(ui, &Two, &mut |key, _| {
            let row = ListRow::new(format!("Row {key}"));
            if *key == 0 && hosted { row.trailing_field(HexField::hex("Hex", Bind::given(craft_theme::Rgba::rgb(0, 0, 0))), |_| {}) } else { row }
        });
        let _ = craft_ui::buttons::Button::new("After the list").show(ui);
    });
    fixture.settle();
    let row = fixture.by_label("Row 0").expect("the row has a node");
    fixture.click(&row);
    fixture.idle(2);
    let on_row = focused_key(fixture.tree());
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    let field = fixture.by_role(Role::TEXT_INPUT).into_iter().next().expect("the row hosts a field");
    assert_eq!(focused_key(fixture.tree()), field.node, "Forward from the row reaches the field");
    hosted.set(false);
    fixture.idle(3);
    let after = fixture.by_label("After the list").expect("the control after the list has a node");
    assert_ne!(focused_key(fixture.tree()), after.node, "focus did not skip to the next control");
    assert_eq!(focused_key(fixture.tree()), on_row, "focus returned to the row");
}
