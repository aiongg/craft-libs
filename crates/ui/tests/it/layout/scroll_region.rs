//! A scrolled layout component is a node of its own, named by its label.

use craft_ui::contract::{Props as _, Show as _, ShowContainer as _};
use craft_ui::display::Text;
use craft_ui::layout_components::Stack;
use craft_ui::testing::access::violations;
use craft_ui::testing::{Fixture, trial};

fn unnamed_nodes(label: Option<&'static str>) -> Vec<String> {
    let mut fixture = Fixture::new(move |ui| {
        let stack = Stack::new().fill().scroll(craft_ui::contract::Scroll::vertical());
        let stack = if let Some(label) = label { stack.label(label) } else { stack };
        stack.show(ui, |ui, _| {
            Text::new("Content").show(ui);
        });
    });
    fixture.settle();
    violations(fixture.ctx(), fixture.tree())
}

#[trial]
fn a_scrolled_stack_with_a_label_is_a_named_node() {
    assert_eq!(unnamed_nodes(Some("Page")), Vec::<String>::new());
}

#[trial]
fn a_scrolled_stack_without_a_label_is_an_unnamed_interactive_node() {
    let found = unnamed_nodes(None);
    assert!(found.iter().any(|line| line.contains("no name")), "{found:?}");
}
