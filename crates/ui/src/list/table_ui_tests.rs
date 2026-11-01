//! `Table` in a harness.

use egui::accesskit::{Role, SortDirection};
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::behavior::{RowInfo, Selection};
use crate::theme::{Base, install};

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
    let out = Table::new("files", "Files", columns, layout, rows.as_slice()).selection(selection).show(ui, |ui, row, column| {
        ui.label(format!("{column} {row}"));
    });
    events.extend(out.events);
}

fn harness() -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([420.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, Host::new());
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
