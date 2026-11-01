//! The accessibility bridge.
#![allow(clippy::expect_used, reason = "tests")]

use egui::accesskit::Role;
use egui::{Context, Id, Rect, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::access::*;
use super::view::PlacedPage;

const WIDTH: f32 = 8.0;
fn view() -> Id {
    Id::NULL.with("document")
}

/// What a fake document describes.
#[derive(Clone, Copy)]
enum Content {
    /// A heading, a two-item list and a 2 × 2 table.
    Mixed,
    /// This many paragraphs.
    Paragraphs(usize),
}

struct Fake {
    content: Content,
    calls: usize,
    selection: Option<(AccessPosition, AccessPosition)>,
}

/// A line of `text` at `top`, a character `WIDTH` wide each.
fn line(out: &mut AccessBuilder, parent: AccessNode, text: &str, top: f32) -> Option<AccessNode> {
    let chars: Vec<(f32, f32)> = (0..text.chars().count()).map(|i| (10.0 + i as f32 * WIDTH, WIDTH)).collect();
    out.line(parent, text, &chars, Rect::from_min_size(pos2(10.0, top), vec2(100.0, 12.0)))
}

impl AccessDescriber for Fake {
    fn describe(&mut self, _placed: &[PlacedPage], out: &mut AccessBuilder) {
        self.calls += 1;
        match self.content {
            Content::Mixed => {
                if let Some(heading) = out.block(None, BlockRole::Heading(1)) {
                    line(out, heading, "Title page", 0.0);
                }
                for (i, text) in ["first", "second"].into_iter().enumerate() {
                    let marker = format!("{}.", i + 1);
                    if let Some(item) = out.block(None, BlockRole::ListItem { level: 0, marker }) {
                        line(out, item, text, 20.0 + 14.0 * i as f32);
                    }
                }
                if let Some(table) = out.block(None, BlockRole::Table) {
                    for row in 0..2 {
                        let Some(row_node) = out.block(Some(table), BlockRole::Row) else { return };
                        for column in 0..2 {
                            let Some(cell) = out.block(Some(row_node), BlockRole::Cell) else { return };
                            let Some(paragraph) = out.block(Some(cell), BlockRole::Paragraph) else { return };
                            line(out, paragraph, "cell", 60.0 + 14.0 * (row * 2 + column) as f32);
                        }
                    }
                }
            }
            Content::Paragraphs(count) => {
                for _ in 0..count {
                    if out.block(None, BlockRole::Paragraph).is_none() {
                        break;
                    }
                }
            }
        }
    }

    fn selection(&self) -> Option<(AccessPosition, AccessPosition)> {
        self.selection
    }
}

struct App {
    placed: Vec<PlacedPage>,
    revision: u64,
    fake: Fake,
}

fn app(content: Content) -> App {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 400.0));
    App { placed: vec![PlacedPage { page: 0, rect, scale: 1.0, band: (0.0, 400.0) }], revision: 0, fake: Fake { content, calls: 0, selection: None } }
}

fn harness(app: App) -> Harness<'static, App> {
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(400.0, 500.0))
        .build_ui_state(|ui, app: &mut App| PagesAccess::new(view(), &app.placed).revision(app.revision).show(ui.ctx(), &mut app.fake), app);
    harness.run();
    harness
}

// Without AccessKit output requested the describer is never called.
#[test]
fn nothing_is_built_without_a_screen_reader() {
    let mut app = app(Content::Mixed);
    let ctx = Context::default();
    for _ in 0..3 {
        let mut output = ctx.run_pass(egui::RawInput::default(), |ctx| {
            PagesAccess::new(view(), &app.placed).show(ctx, &mut app.fake);
        });
        output.textures_delta.clear();
    }
    assert_eq!(app.fake.calls, 0);
}

// A describer with a heading, a two-item list and a 2 × 2 table yields Document › Heading, List › ListItem × 2 with markers, Table › Row › Cell, each line a TextRun with character positions.
#[test]
fn paragraphs_lines_lists_and_tables_form_the_tree() {
    let harness = harness(app(Content::Mixed));
    let count = |role| harness.query_all_by_role(role).count();
    assert_eq!((count(Role::Heading), count(Role::List), count(Role::ListItem), count(Role::ListMarker)), (1, 1, 2, 2));
    assert_eq!((count(Role::Table), count(Role::Row), count(Role::Cell), count(Role::TextRun)), (1, 2, 4, 7));

    let document = harness.get_by_role(Role::Document);
    let roles: Vec<Role> = document.children().map(|child| child.accesskit_node().role()).collect();
    assert_eq!(roles, [Role::Heading, Role::List, Role::Table]);

    let list = harness.get_by_role(Role::List);
    assert_eq!(list.children().count(), 2, "both items sit in the one list");
    let markers: Vec<String> = harness.query_all_by_role(Role::ListMarker).filter_map(|m| m.accesskit_node().value()).collect();
    assert_eq!(markers, ["1.", "2."]);

    let heading = harness.get_by_role(Role::Heading);
    assert_eq!(heading.accesskit_node().level(), Some(1));
    let run = heading.children().next().expect("the heading's line");
    let node = run.accesskit_node();
    assert_eq!(node.role(), Role::TextRun);
    assert_eq!(node.value().as_deref(), Some("Title page"));
    let positions = node.data().character_positions().expect("positions");
    assert_eq!(positions.len(), "Title page".len());
    assert_eq!((positions[0], positions[1]), (0.0, WIDTH));
    assert_eq!(node.data().word_starts(), &[6u8][..]);

    let table = harness.get_by_role(Role::Table);
    assert_eq!((table.accesskit_node().data().row_count(), table.accesskit_node().data().column_count()), (Some(2), Some(2)));
}

// 7000 blocks stop at MAX_NODES; the same revision and placement reuse last pass's tree without calling the describer.
#[test]
fn the_tree_is_capped_and_reused() {
    let mut harness = harness(app(Content::Paragraphs(7000)));
    assert_eq!(harness.query_all_by_role(Role::Paragraph).count(), MAX_NODES);
    assert_eq!(harness.state().fake.calls, 1);

    harness.run_steps(3);
    assert_eq!(harness.state().fake.calls, 1, "same revision and placement");

    harness.state_mut().revision = 1;
    harness.run();
    assert_eq!(harness.state().fake.calls, 2, "a new revision rebuilds");

    harness.state_mut().placed[0].rect = harness.state().placed[0].rect.translate(vec2(0.0, 10.0));
    harness.run();
    assert_eq!(harness.state().fake.calls, 3, "a moved page rebuilds");
}
