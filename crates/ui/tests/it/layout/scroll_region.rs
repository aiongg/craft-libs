//! A scrolled layout component is a group node of its own, named by its label; an unscrolled one is no node.

use craft_ui::contract::{Props as _, Role, Scroll, Show as _, ShowContainer as _};
use craft_ui::display::Text;
use craft_ui::layout_components::Stack;
use craft_ui::testing::access::{name, violations};
use craft_ui::testing::{Fixture, trial};

/// The names of the group nodes (`None` for an unnamed one) and the suite's rule violations of a stack shown with
/// `scroll` and `label`.
fn groups(scroll: bool, label: Option<&'static str>) -> (Vec<Option<String>>, Vec<String>) {
    let mut fixture = Fixture::new(move |ui| {
        let mut stack = Stack::new().fill();
        if scroll {
            stack = stack.scroll(Scroll::vertical());
        }
        if let Some(label) = label {
            stack = stack.label(label);
        }
        stack.show(ui, |ui, _| {
            Text::new("Content").show(ui);
        });
    });
    fixture.settle();
    let tree = fixture.tree();
    let names = tree.with_role(Role::GROUP).into_iter().map(|key| name(&tree, key).map(str::to_owned)).collect();
    (names, violations(fixture.ctx(), tree))
}

#[trial]
fn a_scrolled_stack_is_a_group_named_by_its_label() {
    assert_eq!(groups(true, Some("Page")), (vec![Some("Page".to_owned())], Vec::new()));
}

#[trial]
fn a_scrolled_stack_without_a_label_is_an_unnamed_group() {
    assert_eq!(groups(true, None).0, vec![None]);
}

#[trial]
fn an_unscrolled_stack_is_no_node() {
    assert_eq!(groups(false, Some("Page")).0, Vec::<Option<String>>::new());
}
