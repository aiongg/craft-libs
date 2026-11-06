//! `Table` in a harness.

use egui::accesskit::{Role, SortDirection};
use egui::{DragAndDrop, Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::style::list::ListStyle;
use crate::behavior::{Axis, DropPosition, ListDrag, Move, Reorder, RowInfo, Selection};
use crate::controls::Button;
use crate::direction::{self, Direction};
use crate::edit::{CancelReason, EditEvent};
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install};

type Col = &'static str;

struct Host {
    columns: Vec<TableColumn<Col>>,
    layout: TableLayout<Col>,
    rows: Vec<RowInfo<usize>>,
    selection: Selection<usize>,
    events: Vec<TableEvent<usize, Col>>,
    /// Shows the rows as a tree named "Row {key}".
    tree: bool,
    /// Rows drag (named "Row {key}"); a button "Target" below the table takes the dropped rows.
    reorder: bool,
    /// With `reorder`, `accepts` refuses every row target.
    refuse: bool,
    /// The rows rename in the Name column (named "Row {key}").
    rename: bool,
    thumbnails: ThumbSize,
    /// Each pass's drop target while a drag is under way.
    targets: Vec<Option<Move<usize>>>,
    /// The payload the "Target" button took.
    dropped: Option<ListDrag<usize>>,
}

impl Host {
    fn new() -> Self {
        let columns = vec![
            TableColumn::new("name", "Name").min_width(60.0),
            TableColumn::new("pages", "Pages").min_width(50.0).hideable(true).numeric(),
            TableColumn::new("size", "Size").min_width(50.0).hideable(true),
        ];
        let layout = TableLayout::new(&columns);
        Host {
            columns,
            layout,
            rows: (0..5).map(RowInfo::leaf).collect(),
            selection: Selection::only(0),
            events: Vec::new(),
            tree: false,
            reorder: false,
            refuse: false,
            rename: false,
            thumbnails: ThumbSize::None,
            targets: Vec::new(),
            dropped: None,
        }
    }

    fn take_events(&mut self) -> Vec<TableEvent<usize, Col>> {
        std::mem::take(&mut self.events)
    }
}

fn show(ui: &mut egui::Ui, host: &mut Host) {
    let Host { columns, layout, rows, selection, events, tree, reorder, refuse, rename, thumbnails, targets, dropped } = host;
    let mut table = Table::new("files", "Files", columns, layout, rows.as_slice())
        .selection(selection)
        .row_name(|key| format!("Row {key}"))
        .thumbnails(*thumbnails);
    if *tree {
        table = table.tree(|key| format!("Row {key}"));
    }
    if *reorder {
        let refuse = *refuse;
        table = table.reorder(Reorder::new(Axis::Vertical).accepts(move |_, _| !refuse)).max_height(160.0);
    }
    if *rename {
        table = table.renamable("name");
    }
    let out = table.show(ui, |ui, row, column, cx| {
        if *column == "name" {
            cx.name(ui, format!("{column} {row}"));
        } else {
            cx.label(ui, format!("{column} {row}"));
        }
    });
    if *reorder {
        targets.push(out.drop_target.clone());
        if let Some(drag) = ui.add(Button::new("Target")).dnd_release_payload::<ListDrag<usize>>() {
            *dropped = Some((*drag).clone());
        }
    }
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
    let tokens = crate::theme::current(&harness.ctx);
    let format = |text: &str| painted_format(&harness, text).expect("the cell is painted");
    assert_eq!(format("name 0").color, tokens.color.text.on_selected.to_egui(), "row 0 is selected");
    assert_eq!(format("name 1").color, tokens.color.text.normal.to_egui());
    assert!(!format("pages 1").font_features.is_empty(), "Pages is numeric");
    assert!(format("name 1").font_features.is_empty(), "Name is not");
}

// A hovered striped row keeps its stripe under the hover fill.
#[test]
fn a_hovered_row_keeps_its_stripe() {
    let mut harness = harness();
    let stripe = ListStyle::new(&crate::theme::current(&harness.ctx)).stripe.to_egui();
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
    let pad = crate::theme::current(&harness.ctx).space.pad.control;
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

// A column without a least width takes the table's default least width.
#[test]
fn a_column_without_a_least_width_takes_the_default() {
    let mut host = Host::new();
    host.columns[0] = TableColumn::new("name", "Name");
    let mut harness = harness_with(host, Direction::Ltr);
    let min = table_style(&crate::theme::current(&harness.ctx)).min_column;
    let grip = edge(&harness, "Name").rect().center();
    drag(&mut harness, grip, grip - egui::vec2(900.0, 0.0));
    assert_eq!(width(&harness, "Name"), f64::from(min));
}

// An edge shows the dock dividers' fill while it is dragged.
#[test]
fn a_dragged_edge_has_the_dock_dividers_fill() {
    let mut harness = harness();
    let fill = crate::theme::current(&harness.ctx).color.accent.solid.to_egui();
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

/// Row 0 is an open group with a leaf (1) and a closed group (2) inside it; row 3 is a top-level leaf.
fn tree_host() -> Host {
    let mut host = Host::new();
    host.tree = true;
    host.rows = vec![RowInfo::group(0, true), RowInfo::leaf(1).at_depth(1), RowInfo::group(2, false).at_depth(1), RowInfo::leaf(3)];
    host
}

#[expect(clippy::unwrap_used, reason = "a test helper")]
fn list_style() -> ListRowStyle {
    let harness = Harness::builder().with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(|_| {});
    let mut style = None;
    harness.ctx.run_ui(egui::RawInput::default(), |ui| style = Some(ListRow::<usize>::style(&Tokens::of(ui)))).drop_without_applying_deltas();
    style.unwrap()
}

#[expect(clippy::expect_used, reason = "a test helper")]
fn name_x(harness: &Harness<'_, Host>, row: usize) -> f32 {
    painted_x(harness, &format!("name {row}")).expect("the cell is painted")
}

fn request(harness: &Harness<'_, Host>, node: &egui_kittest::Node<'_>, action: egui::accesskit::Action) {
    let (target_node, target_tree) = node.accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(egui::accesskit::ActionRequest { action, target_tree, target_node, data: None }));
}

fn opens(events: Vec<TableEvent<usize, Col>>) -> Vec<OpenChange<usize>> {
    events.into_iter().filter_map(|event| if let TableEvent::Open(change) = event { Some(change) } else { None }).collect()
}

// A tree table: the first shown column's cell starts `size.icon.md` per level plus the disclosure
// column after the row's edge; a click on a closed group's triangle reports Open { open: true },
// with Alt `all: true`; mirrored in a right-to-left layout.
#[test]
fn a_tree_table_indents_the_first_column_and_opens_by_its_triangle() {
    let style = list_style();
    for (dir, sign) in [(Direction::Ltr, 1.0), (Direction::Rtl, -1.0)] {
        let mut harness = harness_with(tree_host(), dir);
        let base = name_x(&harness, 3);
        let near = |got: f32, want: f32| assert!((got - base - sign * want).abs() < 1.5, "{dir:?}: {} vs {want}", got - base);
        near(name_x(&harness, 0), style.disclosure);
        near(name_x(&harness, 1), 2.0 * style.indent);
        near(name_x(&harness, 2), style.indent + style.disclosure);
        // Other columns carry no tree.
        assert!((painted_x(&harness, "pages 1").unwrap() - painted_x(&harness, "pages 3").unwrap()).abs() < 0.5);

        harness.state_mut().take_events();
        harness.get_by_label("Expand Row 2").click();
        harness.run();
        assert_eq!(opens(harness.state_mut().take_events()), [OpenChange { row: 2, open: true, all: false }], "{dir:?}");
        harness.get_by_label("Expand Row 2").click_modifiers(Modifiers::ALT);
        harness.run();
        assert_eq!(opens(harness.state_mut().take_events()), [OpenChange { row: 2, open: true, all: true }], "{dir:?}");
    }
}

// Tree keys in the focused body: → on a closed group reports Open, → on an open one selects its
// first child, ← on a child selects its parent, Alt+← reports Open { open: false, all: true } and
// the lead moves to its topmost group; ←/→ are swapped right to left; without `tree` ←/→ do nothing.
#[test]
fn a_tree_table_takes_the_tree_keys() {
    let mut harness = harness_with(tree_host(), Direction::Ltr);
    harness.get_by_role(Role::TreeGrid).focus();
    harness.run();
    let selected = |harness: &Harness<'_, Host>| harness.state().selection.clone();
    harness.state_mut().take_events();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(selected(&harness), Selection::only(1), "→ on an open group enters its first child");
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.state_mut().take_events();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(opens(harness.state_mut().take_events()), [OpenChange { row: 2, open: true, all: false }], "→ on a closed group opens it");
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(selected(&harness), Selection::only(0), "← on a closed row goes to its parent");
    harness.state_mut().take_events();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(opens(harness.state_mut().take_events()), [OpenChange { row: 0, open: false, all: true }]);

    // Right to left swaps the arrows.
    let mut harness = harness_with(tree_host(), Direction::Rtl);
    harness.get_by_role(Role::TreeGrid).focus();
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection, Selection::only(1), "← enters in a right-to-left layout");

    // A plain table leaves the arrows alone.
    let mut harness = super::table_ui_tests::harness();
    harness.get_by_role(Role::Table).focus();
    harness.run();
    harness.state_mut().take_events();
    harness.key_press(Key::ArrowRight);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(harness.state_mut().take_events().is_empty());
    assert_eq!(harness.state().selection, Selection::only(0));
}

// AccessKit: a tree table is a TreeGrid and its rows TreeItems with level, expanded on groups,
// position_in_set and size_of_set among siblings; a screen reader's Expand reports Open.
#[test]
fn a_tree_table_exposes_tree_items() {
    let mut harness = harness_with(tree_host(), Direction::Ltr);
    harness.get_by_role(Role::TreeGrid);
    let items: Vec<_> = harness
        .query_all_by_role(Role::TreeItem)
        .map(|n| {
            let node = n.accesskit_node();
            (node.level(), node.data().is_expanded(), node.position_in_set(), node.size_of_set())
        })
        .collect();
    assert_eq!(
        items,
        [
            (Some(0), Some(true), Some(0), Some(2)),
            (Some(1), None, Some(0), Some(2)),
            (Some(1), Some(false), Some(1), Some(2)),
            (Some(0), None, Some(1), Some(2))
        ]
    );
    harness.state_mut().take_events();
    let closed = harness.query_all_by_role(Role::TreeItem).nth(2).unwrap();
    assert!(closed.accesskit_node().data().supports_action(egui::accesskit::Action::Expand));
    request(&harness, &closed, egui::accesskit::Action::Expand);
    harness.run();
    assert_eq!(opens(harness.state_mut().take_events()), [OpenChange { row: 2, open: true, all: false }]);
}

fn drag_host() -> Host {
    let mut host = Host::new();
    host.reorder = true;
    host
}

/// The rect of the row at `at` in the display order.
fn row_rect(harness: &Harness<'_, Host>, at: usize) -> egui::Rect {
    rows(harness)[at]
}

/// A point in the row at `at`, `share` of the way down.
fn in_row(harness: &Harness<'_, Host>, at: usize, share: f32) -> Pos2 {
    let rect = row_rect(harness, at);
    Pos2::new(rect.center().x, rect.top() + rect.height() * share)
}

fn hover(harness: &mut Harness<'_, Host>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    harness.run();
}

/// Presses on the row at `at` and moves past egui's drag threshold.
fn grab(harness: &mut Harness<'_, Host>, at: usize) {
    let start = in_row(harness, at, 0.5);
    hover(harness, start);
    press(harness, start, PointerButton::Primary, true);
    hover(harness, start + egui::vec2(0.0, 8.0));
}

fn release(harness: &mut Harness<'_, Host>, pos: Pos2) {
    hover(harness, pos);
    press(harness, pos, PointerButton::Primary, false);
}

fn table_payload(harness: &Harness<'_, Host>) -> Option<ListDrag<usize>> {
    DragAndDrop::payload::<ListDrag<usize>>(&harness.ctx).map(|p| (*p).clone())
}

/// The reorder events since the last call.
fn reorders(harness: &mut Harness<'_, Host>) -> Vec<EditEvent<Move<usize>>> {
    harness
        .state_mut()
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            TableEvent::Reorder(event) => Some(event),
            _ => None,
        })
        .collect()
}

// A row drag carries the `ListDrag` payload with the grabbed row and its name; over another row the
// table resolves `Reorder`'s target and draws its marker; the release reports the move as a Commit
// and takes the payload.
#[test]
fn a_row_dragged_onto_another_row_reports_the_move() {
    let mut harness = harness_with(drag_host(), Direction::Ltr);
    grab(&mut harness, 1);
    assert!(matches!(reorders(&mut harness)[..], [EditEvent::Begin { .. }]));
    let drag = table_payload(&harness).expect("the payload");
    assert_eq!((drag.items.as_slice(), drag.grabbed, drag.label.as_str()), (&[1][..], 1, "Row 1"));

    harness.state_mut().targets.clear();
    let over = in_row(&harness, 3, 0.75);
    hover(&mut harness, over);
    let after = Move { items: vec![1], target: 3, position: DropPosition::After, copy: false };
    assert_eq!(harness.state().targets.last(), Some(&Some(after.clone())));
    let line = row_rect(&harness, 3);
    let drop = list_style().drop;
    assert!(
        shapes(&harness).iter().any(|shape| matches!(shape, egui::Shape::Rect(r) if r.fill == drop && (r.rect.bottom() - line.bottom()).abs() < 4.0)),
        "the After marker on the target row"
    );
    release(&mut harness, over);
    let ends = reorders(&mut harness);
    assert!(matches!(&ends[..], [EditEvent::Commit { value, .. }] if *value == after), "{ends:?}");
    assert!(table_payload(&harness).is_none(), "a drop on the table takes the payload");
    assert_eq!(harness.state().dropped, None);
    assert_eq!(harness.state().selection.items, [0], "a drag doesn't change the selection");
}

// A table whose `accepts` refuses every row target drags only to other widgets: no target over its
// rows, and a release over a widget after it leaves that widget the payload (the selection, with
// the grabbed row) while the table reports Cancel(Dismissed).
#[test]
fn a_row_dragged_to_another_widget_leaves_it_the_payload() {
    let mut host = drag_host();
    host.refuse = true;
    host.selection = Selection { items: vec![2, 1], lead: Some(2), anchor: Some(1) };
    let mut harness = harness_with(host, Direction::Ltr);
    grab(&mut harness, 1);
    reorders(&mut harness);
    harness.state_mut().targets.clear();
    let over = in_row(&harness, 3, 0.75);
    hover(&mut harness, over);
    assert_eq!(harness.state().targets.last(), Some(&None), "every row target refused");
    let target = harness.get_by_role_and_label(Role::Button, "Target").rect().center();
    release(&mut harness, target);
    let ends = reorders(&mut harness);
    assert!(matches!(ends[..], [EditEvent::Cancel { reason: CancelReason::Dismissed, .. }]), "{ends:?}");
    let dropped = harness.state().dropped.clone().expect("the button took the rows");
    assert_eq!((dropped.items.as_slice(), dropped.grabbed), (&[1, 2][..], 1), "the selection in display order, and the row grabbed");
}

// Esc ends a row drag as Cancel(Escape) and drops the payload; the release after it moves nothing.
#[test]
fn escape_cancels_a_row_drag() {
    let mut harness = harness_with(drag_host(), Direction::Ltr);
    grab(&mut harness, 1);
    reorders(&mut harness);
    let over = in_row(&harness, 3, 0.75);
    hover(&mut harness, over);
    harness.key_press(Key::Escape);
    harness.run();
    let ends = reorders(&mut harness);
    assert!(matches!(ends[..], [EditEvent::Cancel { reason: CancelReason::Escape, .. }]), "{ends:?}");
    assert!(table_payload(&harness).is_none());
    release(&mut harness, over);
    assert!(reorders(&mut harness).is_empty(), "the rest of a cancelled drag does nothing");
}

fn rename_host() -> Host {
    let mut host = Host::new();
    host.rename = true;
    host
}

/// The renames reported since the last call.
fn renames(harness: &mut Harness<'_, Host>) -> Vec<TableEvent<usize, Col>> {
    harness.state_mut().take_events().into_iter().filter(|e| matches!(e, TableEvent::Renamed { .. } | TableEvent::RenameEnded { .. })).collect()
}

fn click_at(harness: &mut Harness<'_, Host>, pos: Pos2) {
    hover(harness, pos);
    press(harness, pos, PointerButton::Primary, true);
    press(harness, pos, PointerButton::Primary, false);
}

// F2 opens the lead's rename field with its name; typed text and Enter report Renamed then
// RenameEnded(Enter). F2 and Esc report only RenameEnded(Escape).
#[test]
fn f2_renames_the_lead_and_esc_cancels() {
    let mut harness = harness_with(rename_host(), Direction::Ltr);
    let row = in_row(&harness, 1, 0.5);
    click_at(&mut harness, row);
    harness.run_steps(10);
    renames(&mut harness);
    harness.key_press(Key::F2);
    harness.run();
    let field = harness.get_by_role(Role::TextInput);
    assert_eq!(field.value().as_deref(), Some("Row 1"), "the field starts from the row's name");
    harness.event(Event::Text("Cover".into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        renames(&mut harness),
        [TableEvent::Renamed { row: 1, text: "Cover".into() }, TableEvent::RenameEnded { row: 1, end: RenameEnd::Enter }]
    );
    assert!(harness.query_by_role(Role::TextInput).is_none());

    harness.key_press(Key::F2);
    harness.run();
    harness.event(Event::Text("Draft".into()));
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(renames(&mut harness), [TableEvent::RenameEnded { row: 1, end: RenameEnd::Escape }]);
}

// A click on the name of the row that is already the only one selected opens its rename once the
// double-click time has passed; a double click there activates the row instead.
#[test]
fn a_slow_click_on_the_selected_rows_name_renames_it() {
    let mut harness = harness_with(rename_host(), Direction::Ltr);
    let name = harness.get_by_label("name 0").rect().center();
    click_at(&mut harness, name);
    assert!(harness.query_by_role(Role::TextInput).is_none(), "not before the double-click time");
    harness.run_steps(10);
    assert!(harness.query_by_role(Role::TextInput).is_some(), "the slow click opened the rename");
    harness.key_press(Key::Escape);
    harness.run_steps(10);
    harness.state_mut().take_events();

    hover(&mut harness, name);
    // One pass per press and release: `run` may step on toward the slow click's repaint.
    for pressed in [true, false, true, false] {
        harness.event(Event::PointerButton { pos: name, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        harness.step();
    }
    harness.run_steps(10);
    assert!(harness.query_by_role(Role::TextInput).is_none(), "a double click doesn't rename");
    assert!(harness.state_mut().take_events().contains(&TableEvent::Activate(0)));
}

// Rows take a list row's height for the thumbnail size (`ListRowStyle::height`), not
// `size.control.md`: Small is taller than no thumbnail and Large taller than Medium. Small and
// Medium both reach `ListStyle::thumb_row_min` in the suite's tokens.
#[test]
fn rows_take_the_list_row_height_of_the_thumbnail_size() {
    let height = |thumbnails| {
        let mut host = Host::new();
        host.thumbnails = thumbnails;
        rows(&harness_with(host, Direction::Ltr))[0].height()
    };
    let style = list_style();
    for size in [ThumbSize::None, ThumbSize::Small, ThumbSize::Medium, ThumbSize::Large] {
        assert_eq!(height(size), style.height(size, false), "{size:?}");
    }
    assert!(height(ThumbSize::None) < height(ThumbSize::Small));
    assert!(height(ThumbSize::Medium) < height(ThumbSize::Large));
}
