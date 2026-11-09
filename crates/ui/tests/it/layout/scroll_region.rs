//! A scrolled or labelled layout component is a group node of its own, named by its label; an unlabelled unscrolled
//! one is no node. The suite's text-size case lets a scroll region's view cut its rows, and no other part.

use craft_ui::contract::{Props as _, Role, Scroll, Show as _, ShowContainer as _};
use craft_ui::display::Text;
use craft_ui::layout_components::Stack;
use craft_ui::testing::access::{name, violations};
use craft_ui::testing::story::{Story, StoryData, StoryOutput};
use craft_ui::testing::suite::text_size_case;
use craft_ui::testing::{Fixture, trial};
use egui::Ui;

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
fn a_scrolled_or_labelled_stack_is_a_group_named_by_its_label_and_an_unlabelled_unscrolled_one_is_no_node() {
    const LABEL: &str = "Page";
    // (scrolled, label, the group nodes' names: none for no node, `None` inside for an unnamed group)
    let cases: [(bool, Option<&str>, Vec<Option<&str>>); 4] =
        [(true, Some(LABEL), vec![Some(LABEL)]), (true, None, vec![None]), (false, Some(LABEL), vec![Some(LABEL)]), (false, None, vec![])];
    for (scroll, label, expected) in cases {
        let (names, violations) = groups(scroll, label);
        let expected: Vec<Option<String>> = expected.into_iter().map(|name| name.map(str::to_owned)).collect();
        assert_eq!(names, expected, "scrolled: {scroll}, label: {label:?}");
        if scroll && label.is_some() {
            assert!(violations.is_empty(), "{violations:?}");
        }
    }
}

/// More rows than the fixture's room holds, so one of them crosses the room's bottom edge.
const ROWS: usize = 40;

/// A stack that fills the room and shows `ROWS` rows, scrolling when `scroll` is set.
fn rows(ui: &mut Ui, scroll: bool) -> StoryOutput {
    let mut stack = Stack::new().fill();
    if scroll {
        stack = stack.scroll(Scroll::vertical());
    }
    stack.show(ui, |ui, _| {
        for row in 1..=ROWS {
            Text::new(format!("Row {row}")).show(ui);
        }
    });
    StoryOutput::default()
}

fn scrolling_rows(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    rows(ui, true)
}

fn fixed_rows(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    rows(ui, false)
}

/// A row that a scrolling body's view cuts at its edge stays in the content that scrolls, so the case passes.
#[trial]
fn the_text_size_case_lets_a_scroll_view_cut_a_row() {
    assert_eq!(text_size_case(&Story::new("scrolling-rows", scrolling_rows)), Ok(()));
}

/// Rows that grow past a part that does not scroll leave it under the larger text size.
#[trial]
fn the_text_size_case_fails_text_that_grows_past_a_fixed_part() {
    let failed = text_size_case(&Story::new("fixed-rows", fixed_rows)).expect_err("the rows grow past the room");
    assert!(failed.contains("under text size"), "{failed}");
}

/// A container shown at a rect of its own, nested in a scrolling body, bounds the room of what it holds: a child that
/// fills there fills the rect, not its content.
#[trial]
fn a_container_shown_at_a_rect_bounds_the_room_inside_a_scrolling_body() {
    const HEIGHT: f32 = 120.0;
    let seen = std::rc::Rc::new(std::cell::Cell::new((0.0_f32, 0.0_f32)));
    let kept = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        Stack::new().fill().scroll(Scroll::vertical()).show(ui, |ui, _| {
            let at = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(200.0, HEIGHT));
            Stack::new().show_at(ui, at, |ui, _| {
                let filled = Stack::new().fill().show(ui, |ui, _| {
                    Text::new("Content").show(ui);
                });
                kept.set((filled.rect.height(), at.height()));
            });
        });
    });
    fixture.settle();
    let (filled, rect) = seen.get();
    assert_eq!(filled, rect, "the filling child fills the rect it is shown in");
}
