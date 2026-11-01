//! `Table` in a harness.

use egui::accesskit::{Role, SortDirection};
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::behavior::{RowInfo, Selection};
use crate::direction::{self, Direction};
use crate::theme::{Base, ToEgui as _, install};

type Col = &'static str;

struct Host {
    columns: Vec<TableColumn<Col>>,
    layout: TableLayout<Col>,
    rows: Vec<RowInfo<usize>>,
    selection: Selection<usize>,
    events: Vec<TableEvent<usize, Col>>,
}

impl Host {
    fn new() -> Self {
        let columns = vec![
            TableColumn::new("name", "Name").min_width(60.0),
            TableColumn::new("pages", "Pages").min_width(50.0).hideable(true).numeric(),
            TableColumn::new("size", "Size").min_width(50.0).hideable(true),
        ];
        let layout = TableLayout::new(&columns);
        Host { columns, layout, rows: (0..5).map(RowInfo::leaf).collect(), selection: Selection::only(0), events: Vec::new() }
    }

    fn take_events(&mut self) -> Vec<TableEvent<usize, Col>> {
        std::mem::take(&mut self.events)
    }
}

fn show(ui: &mut egui::Ui, host: &mut Host) {
    let Host { columns, layout, rows, selection, events } = host;
    let out = Table::new("files", "Files", columns, layout, rows.as_slice()).selection(selection).show(ui, |ui, row, column, cx| {
        cx.label(ui, format!("{column} {row}"));
    });
    events.extend(out.events);
}

fn harness() -> Harness<'static, Host> {
    harness_with(Host::new(), Direction::Ltr)
}

fn harness_with(host: Host, dir: Direction) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([420.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn header<'h>(harness: &'h Harness<'_, Host>, label: &'h str) -> egui_kittest::Node<'h> {
    harness.get_by_role_and_label(Role::ColumnHeader, label)
}

fn edge<'h>(harness: &'h Harness<'_, Host>, column: &str) -> egui_kittest::Node<'h> {
    let name = format!("Resize {column}");
    harness.get_by(move |n| n.role() == Role::Splitter && n.label().as_deref() == Some(name.as_str()))
}

fn width(harness: &Harness<'_, Host>, column: &str) -> f64 {
    edge(harness, column).accesskit_node().numeric_value().unwrap_or_default()
}

fn press(harness: &mut Harness<'_, Host>, pos: Pos2, button: PointerButton, pressed: bool) {
    harness.event(Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE });
    harness.run();
}

/// A pointer drag from `from` to `to` in a few moves.
fn drag(harness: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.run();
    press(harness, from, PointerButton::Primary, true);
    for step in 1..=4 {
        harness.event(Event::PointerMoved(from + (to - from) * (step as f32 / 4.0)));
        harness.run();
    }
    press(harness, to, PointerButton::Primary, false);
}

// A header click reports `TableEvent::Sort`; the header then reads "Name, sorted ascending" with `sort_direction`.
#[test]
fn clicking_a_header_sorts_and_reads_its_order() {
    let mut harness = harness();
    header(&harness, "Name").click();
    harness.run();
    let events = harness.state_mut().take_events();
    assert!(events.contains(&TableEvent::Sort(TableSort { column: "name", order: SortOrder::Ascending })), "{events:?}");
    let sorted = header(&harness, "Name, sorted ascending");
    assert_eq!(sorted.accesskit_node().sort_direction(), Some(SortDirection::Ascending));
    sorted.click();
    harness.run();
    assert_eq!(header(&harness, "Name, sorted descending").accesskit_node().sort_direction(), Some(SortDirection::Descending));
}

// A drag on the edge between two headers widens the left column, not past its least width, and writes every width into the layout.
#[test]
fn dragging_a_column_edge_resizes_it_and_keeps_the_widths() {
    let mut harness = harness();
    assert!(harness.state().layout.widths.is_empty(), "the columns fit until the first resize");
    let before = width(&harness, "Name");
    let grip = edge(&harness, "Name").rect().center();
    drag(&mut harness, grip, grip + egui::vec2(30.0, 0.0));
    let widened = width(&harness, "Name");
    assert!((widened - before - 30.0).abs() < 4.0, "{before} -> {widened}");
    assert_eq!(harness.state().layout.widths.len(), 3, "every shown column's width is written");
    assert!(harness.state_mut().take_events().contains(&TableEvent::Layout));

    let grip = edge(&harness, "Name").rect().center();
    drag(&mut harness, grip, grip - egui::vec2(900.0, 0.0));
    assert_eq!(width(&harness, "Name"), 60.0, "not past the least width");
}

// The header menu's check row hides a hideable column and the menu stays open; Reset Columns shows it again.
#[test]
fn the_header_menu_hides_a_column_and_stays_open() {
    let mut harness = harness();
    header(&harness, "Pages").click_secondary();
    harness.run();
    harness.get_by_label("Columns");
    let row = |harness: &Harness<'_, Host>, label: &str| {
        harness.query_all_by_label(label).find(|n| n.accesskit_node().role() != Role::ColumnHeader).map(|n| n.rect())
    };
    let pages = row(&harness, "Pages").expect("a check row for Pages");
    assert!(row(&harness, "Name").is_none(), "a column that can't be hidden has no row");
    harness.event(Event::PointerMoved(pages.center()));
    harness.run();
    press(&mut harness, pages.center(), PointerButton::Primary, true);
    press(&mut harness, pages.center(), PointerButton::Primary, false);
    assert_eq!(harness.state().layout.hidden, ["pages"]);
    assert!(harness.state_mut().take_events().contains(&TableEvent::Layout));
    harness.get_by_label("Reset Columns");
    assert!(harness.query_all_by_role_and_label(Role::ColumnHeader, "Pages").next().is_none(), "the column is gone");

    harness.get_by_label("Reset Columns").click();
    harness.run();
    assert!(harness.state().layout.hidden.is_empty());
    header(&harness, "Pages");
}

// Dropping a header past another's centre moves its column after that one and reports `Layout`.
#[test]
fn a_header_dragged_onto_another_moves_the_column() {
    let mut harness = harness();
    let from = header(&harness, "Name").rect().center();
    let target = header(&harness, "Size").rect();
    drag(&mut harness, from, egui::pos2(target.right() - 6.0, target.center().y));
    assert_eq!(harness.state().layout.order, ["pages", "size", "name"]);
    assert!(harness.state_mut().take_events().contains(&TableEvent::Layout));
}

// Tab reaches the body once; ↓ and Shift+↓ select as a `List` does; Enter reports `Activate`.
#[test]
fn the_body_is_one_tab_stop_with_the_list_keys() {
    let mut harness = harness();
    harness.key_press(Key::Tab);
    harness.run();
    let table = harness.get_by_role(Role::Table);
    assert!(table.accesskit_node().active_descendant().is_some(), "Tab reaches the body");
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().selection.items, [1]);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().selection.items, [1, 2]);
    harness.state_mut().take_events();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [TableEvent::Activate(2)]);
}

// Tab from the first header reaches its edge; → widens the column by 8 points and ← narrows it,
// not below its least width; each press reports `Layout` and writes the widths; the edge reads as
// a `Splitter` "Resize Name" with the width as its value, and SetValue sets it.
#[test]
fn arrow_keys_on_a_focused_column_edge_resize_the_column() {
    let mut harness = harness();
    for _ in 0..3 {
        harness.key_press(Key::Tab);
        harness.run();
    }
    assert!(edge(&harness, "Name").is_focused(), "body, header, then its edge");
    let start = width(&harness, "Name");
    harness.state_mut().take_events();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(width(&harness, "Name"), start + 8.0);
    assert_eq!(harness.state_mut().take_events(), [TableEvent::Layout]);
    assert_eq!(harness.state().layout.widths.len(), 3);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(width(&harness, "Name"), start);
    edge(&harness, "Name").set_value_accesskit(61.0);
    harness.run();
    assert_eq!(width(&harness, "Name"), 61.0);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(width(&harness, "Name"), 60.0, "not below the least width");
}

/// The x of the text shape that draws `text`, as painted this pass.
fn painted_x(harness: &Harness<'_, Host>, text: &str) -> Option<f32> {
    fn find(shape: &egui::Shape, text: &str) -> Option<f32> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == text => Some(t.pos.x),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, text)),
            _ => None,
        }
    }
    harness.output().shapes.iter().find_map(|s| find(&s.shape, text))
}

// During an edge drag the header cell and the body cell of a column start at the same x in the same pass.
#[test]
fn the_header_and_the_body_agree_on_widths_during_an_edge_drag() {
    let mut harness = harness();
    let grip = edge(&harness, "Name").rect().center();
    harness.event(Event::PointerMoved(grip));
    harness.run();
    press(&mut harness, grip, PointerButton::Primary, true);
    let start = painted_x(&harness, "Size").expect("the header is painted");
    for step in 1..=3 {
        harness.event(Event::PointerMoved(grip + egui::vec2(12.0 * step as f32, 0.0)));
        harness.step();
        let (header, body) = (painted_x(&harness, "Size").unwrap_or_default(), painted_x(&harness, "size 0").unwrap_or(f32::MAX));
        assert!((header - body).abs() < 0.6, "step {step}: header text at {header}, body text at {body}");
        assert!(header > start, "the column to the left is widening");
    }
}

/// The shapes painted this pass, flattened.
fn shapes(harness: &Harness<'_, Host>) -> Vec<egui::Shape> {
    fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    let mut out = Vec::new();
    harness.output().shapes.iter().for_each(|s| flatten(&s.shape, &mut out));
    out
}

/// The first text section's format of the text shape that draws `text`.
fn painted_format(harness: &Harness<'_, Host>, text: &str) -> Option<egui::TextFormat> {
    shapes(harness).into_iter().find_map(|shape| match shape {
        egui::Shape::Text(t) if t.galley.text() == text => Some(t.galley.job.sections[0].format.clone()),
        _ => None,
    })
}

fn painted_width(harness: &Harness<'_, Host>, text: &str) -> Option<f32> {
    shapes(harness).into_iter().find_map(|shape| match shape {
        egui::Shape::Text(t) if t.galley.text() == text => Some(t.galley.size().x),
        _ => None,
    })
}

fn filled(harness: &Harness<'_, Host>, fill: egui::Color32) -> Vec<egui::Rect> {
    shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(r) if r.fill == fill => Some(r.rect),
            _ => None,
        })
        .collect()
}

fn rows(harness: &Harness<'_, Host>) -> Vec<egui::Rect> {
    let mut rects: Vec<_> = harness.get_all_by_role(Role::Row).map(|n| n.rect()).collect();
    rects.sort_by(|a, b| a.top().total_cmp(&b.top()));
    rects
}

// A selected row's cells take the selected text colour through the cell context; the others keep
// the row text; a numeric column writes tabular figures.
#[test]
fn cells_take_the_row_text_colour_and_numeric_ones_tabular_figures() {
    let harness = harness();
    let list = &crate::theme::current(&harness.ctx).component.list;
    let format = |text: &str| painted_format(&harness, text).expect("the cell is painted");
    assert_eq!(format("name 0").color, list.selected_text.to_egui(), "row 0 is selected");
    assert_eq!(format("name 1").color, list.text.to_egui());
    assert!(!format("pages 1").font_features.is_empty(), "Pages is numeric");
    assert!(format("name 1").font_features.is_empty(), "Name is not");
}

// A hovered striped row keeps its stripe under the hover fill.
#[test]
fn a_hovered_row_keeps_its_stripe() {
    let mut harness = harness();
    let stripe = crate::theme::current(&harness.ctx).component.list.stripe.to_egui();
    let row = rows(&harness)[1];
    assert!(filled(&harness, stripe).iter().any(|r| (r.top() - row.top()).abs() < 0.5), "row 1 is striped");
    harness.event(Event::PointerMoved(row.center()));
    harness.run();
    assert!(filled(&harness, stripe).iter().any(|r| (r.top() - row.top()).abs() < 0.5), "still striped under the hover");
}

// Space and Enter on a focused header sort, Ctrl+→ moves its column, the Menu key opens the header menu.
#[test]
fn header_keys_sort_move_and_open_the_menu() {
    let mut harness = harness();
    header(&harness, "Name").focus();
    harness.run();
    harness.state_mut().take_events();
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [TableEvent::Sort(TableSort { column: "name", order: SortOrder::Ascending })]);
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [TableEvent::Sort(TableSort { column: "name", order: SortOrder::Descending })]);
    assert!(harness.state().selection.items == [0], "the keys didn't reach the body");

    harness.key_press_modifiers(Modifiers::COMMAND, Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().layout.order, ["pages", "name", "size"]);
    assert_eq!(harness.state_mut().take_events(), [TableEvent::Layout]);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().layout.order, ["name", "pages", "size"]);

    assert!(harness.query_by_label("Reset Columns").is_none());
    harness.key_press(Key::Menu);
    harness.run();
    harness.get_by_label("Reset Columns");
}

// Esc during an edge drag puts back the widths from before it, and the release then changes nothing.
#[test]
fn escape_during_an_edge_drag_restores_the_widths() {
    let mut harness = harness();
    let before = width(&harness, "Name");
    let grip = edge(&harness, "Name").rect().center();
    harness.event(Event::PointerMoved(grip));
    harness.run();
    press(&mut harness, grip, PointerButton::Primary, true);
    for step in 1..=3 {
        harness.event(Event::PointerMoved(grip + egui::vec2(10.0 * step as f32, 0.0)));
        harness.run();
    }
    assert!(width(&harness, "Name") > before + 20.0, "the drag widens the column");
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(width(&harness, "Name"), before);
    assert!(harness.state().layout.widths.is_empty(), "the layout is back to fitting");
    harness.event(Event::PointerMoved(grip + egui::vec2(60.0, 0.0)));
    harness.run();
    press(&mut harness, grip + egui::vec2(60.0, 0.0), PointerButton::Primary, false);
    assert_eq!(width(&harness, "Name"), before, "the drag is over");
}

// Right to left: the headers run from the right, the edge is the column's left one, a drag to the left widens, and ← widens by a key step.
#[test]
fn edges_resize_right_to_left() {
    let mut harness = harness_with(Host::new(), Direction::Rtl);
    assert!(header(&harness, "Name").rect().center().x > header(&harness, "Size").rect().center().x, "Name is at the right");
    let name = header(&harness, "Name").rect();
    let grip = edge(&harness, "Name").rect().center();
    assert!((grip.x - name.left()).abs() < 4.0, "the edge is the column's left one");
    let before = width(&harness, "Name");
    drag(&mut harness, grip, grip - egui::vec2(30.0, 0.0));
    let widened = width(&harness, "Name");
    assert!((widened - before - 30.0).abs() < 4.0, "{before} -> {widened}");

    let mut harness = harness_with(Host::new(), Direction::Rtl);
    for _ in 0..3 {
        harness.key_press(Key::Tab);
        harness.run();
    }
    assert!(edge(&harness, "Name").is_focused());
    let start = width(&harness, "Name");
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(width(&harness, "Name"), start + 8.0);
}

// A double click on an edge fits the column to its widest cell in view, with the padding, and
// reports `Layout`.
#[test]
fn a_double_click_on_an_edge_fits_the_column() {
    let mut host = Host::new();
    host.columns[0] = TableColumn::new("name", "Name").min_width(10.0);
    let mut harness = harness_with(host, Direction::Ltr);
    let before = width(&harness, "Name");
    let widest = (0..5).filter_map(|row| painted_width(&harness, &format!("name {row}"))).fold(0.0, f32::max);
    let pad = crate::theme::current(&harness.ctx).component.list.pad_x;
    let grip = edge(&harness, "Name").rect().center();
    harness.event(Event::PointerMoved(grip));
    harness.run();
    harness.state_mut().take_events();
    for pressed in [true, false, true, false] {
        harness.event(Event::PointerButton { pos: grip, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    harness.run();
    let fitted = width(&harness, "Name");
    assert!(fitted < before, "{before} -> {fitted}");
    assert!((fitted - f64::from(widest + 2.0 * pad)).abs() < 1.0, "fitted {fitted}, widest cell {widest}, padding {pad}");
    assert!(harness.state_mut().take_events().contains(&TableEvent::Layout));
}

// A column without a least width takes `table.min_column`.
#[test]
fn a_column_without_a_least_width_takes_the_token() {
    let mut host = Host::new();
    host.columns[0] = TableColumn::new("name", "Name");
    let mut harness = harness_with(host, Direction::Ltr);
    let min = crate::theme::current(&harness.ctx).component.table.min_column;
    let grip = edge(&harness, "Name").rect().center();
    drag(&mut harness, grip, grip - egui::vec2(900.0, 0.0));
    assert_eq!(width(&harness, "Name"), f64::from(min));
}

// An edge shows the dock dividers' fill while it is dragged.
#[test]
fn a_dragged_edge_has_the_dock_dividers_fill() {
    let mut harness = harness();
    let fill = crate::theme::current(&harness.ctx).component.dock.resize.to_egui();
    let grip = edge(&harness, "Name").rect().center();
    harness.event(Event::PointerMoved(grip));
    harness.run();
    press(&mut harness, grip, PointerButton::Primary, true);
    harness.event(Event::PointerMoved(grip + egui::vec2(10.0, 0.0)));
    harness.run();
    assert!(!filled(&harness, fill).is_empty(), "the edge is filled in the dock's resize colour");
}

// Hiding the sorting column clears the sort and reports `Unsorted`; the header no longer reads as sorted.
#[test]
fn hiding_the_sorting_column_clears_the_sort() {
    let mut harness = harness();
    header(&harness, "Pages").click();
    harness.run();
    assert!(harness.state().layout.sort.is_some());
    header(&harness, "Pages, sorted ascending").click_secondary();
    harness.run();
    let row = harness.query_all_by_label("Pages").find(|n| n.accesskit_node().role() != Role::ColumnHeader).map(|n| n.rect()).expect("a check row");
    harness.state_mut().take_events();
    harness.event(Event::PointerMoved(row.center()));
    harness.run();
    press(&mut harness, row.center(), PointerButton::Primary, true);
    press(&mut harness, row.center(), PointerButton::Primary, false);
    assert_eq!(harness.state().layout.sort, None);
    let events = harness.state_mut().take_events();
    assert!(events.contains(&TableEvent::Unsorted) && events.contains(&TableEvent::Layout), "{events:?}");
}

// A column's first order is what the first click sorts in.
#[test]
fn a_descending_first_column_sorts_descending_on_the_first_click() {
    let mut host = Host::new();
    host.columns[1] = TableColumn::new("pages", "Pages").min_width(50.0).hideable(true).numeric().first_order(SortOrder::Descending);
    let mut harness = harness_with(host, Direction::Ltr);
    header(&harness, "Pages").click();
    harness.run();
    let events = harness.state_mut().take_events();
    assert!(events.contains(&TableEvent::Sort(TableSort { column: "pages", order: SortOrder::Descending })), "{events:?}");
    header(&harness, "Pages, sorted descending").click();
    harness.run();
    assert_eq!(harness.state().layout.sort, Some(TableSort { column: "pages", order: SortOrder::Ascending }), "the second click flips");
}

// Columns wider than the table scroll sideways with the header, and the vertical scroll bar stays at the
// view's edge; the header stays at the top while the rows scroll.
#[test]
fn wide_columns_keep_the_vertical_bar_in_view_and_the_header_in_place() {
    let mut host = Host::new();
    host.rows = (0..40).map(RowInfo::leaf).collect();
    host.layout.widths = vec![("name", 300.0), ("pages", 300.0), ("size", 300.0)];
    let mut harness = harness_with(host, Direction::Ltr);
    let table = harness.get_by_role(Role::Table).rect();
    let bars: Vec<_> = harness.query_all_by_role(Role::ScrollBar).map(|n| n.rect()).collect();
    assert!(bars.iter().any(|bar| bar.right() <= table.right() + 0.5 && bar.height() > 100.0), "a vertical bar inside the table: {bars:?}");
    let top = header(&harness, "Name").rect().top();
    harness.event(Event::PointerMoved(table.center()));
    harness.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -120.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(20);
    assert_eq!(header(&harness, "Name").rect().top(), top, "the header doesn't scroll away");
    let first = rows(&harness)[0];
    assert!(first.top() < top + 60.0, "the rows scrolled under it: {first:?}");
}

// A tree table: the first shown column's cell starts `list.indent` per level plus the disclosure
// column after the row's edge; a click on a closed group's triangle reports Open { open: true },
// with Alt `all: true`; mirrored in a right-to-left layout.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
fn a_tree_table_indents_the_first_column_and_opens_by_its_triangle() {
    todo!()
}

// Tree keys in the focused body: → on a closed group reports Open, → on an open one selects its
// first child, ← on a child selects its parent, Alt+← reports Open { open: false, all: true } and
// the lead moves to its topmost group; ←/→ are swapped right to left; without `tree` ←/→ do nothing.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
fn a_tree_table_takes_the_tree_keys() {
    todo!()
}

// AccessKit: a tree table is a TreeGrid and its rows TreeItems with level, expanded on groups,
// position_in_set and size_of_set among siblings; a screen reader's Expand reports Open.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
fn a_tree_table_exposes_tree_items() {
    todo!()
}
