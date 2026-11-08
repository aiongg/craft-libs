//! The document tables, the text selection tool, the accessibility bridge and the reading view.

use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{CellAt, CellRange};
use craft_pages::ui::access::{AccessBuilder, AccessDescriber, AccessNode, AccessPosition, BlockRole, MAX_NODES};
use craft_pages::ui::fixtures::{PAPER, demo_placed};
use craft_pages::ui::reading::{ReadingMetrics, ReadingState, reading_size, reading_viewport, swipe_turn, wheel_turns};
use craft_pages::ui::table::{
    BorderEdges, BorderPreset, EdgeKey, EdgePicker, RowHeight, TableChange, TableGeometry, TableProperties, TableSizePicker, TableSizePickerOutput,
    TableValues,
};
use craft_pages::ui::text_selection::{PageTextSource, PageTexts, TextSelectState, TextSelectTool, TextSelection, TextSpan};
use craft_pages::ui::view::{PageView, PageViewState, PlacedPage};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, pos2, vec2};

// Edges

#[trial]
fn presets_set_or_toggle_edges() {
    let mut inner = BorderEdges::default();
    inner.set(EdgeKey::InsideVertical, true);
    let outside = BorderPreset::Outside.apply(inner);
    assert_eq!(EdgeKey::ALL.map(|k| outside.get(k)), [true, true, true, true, false, true]);
    let toggled = BorderPreset::Edge(EdgeKey::Top).apply(outside);
    assert!(!toggled.get(EdgeKey::Top) && toggled.get(EdgeKey::Bottom));
    assert_eq!(BorderPreset::None.apply(outside), BorderEdges::default());
}

fn edge_picker() -> (Fixture, Rc<Cell<BorderEdges>>) {
    let edges = Rc::new(Cell::new(BorderEdges::default()));
    let shared = Rc::clone(&edges);
    let mut fixture = Fixture::new(move |ui| {
        let mut now = shared.get();
        EdgePicker::new("Borders", Bind::bound(&mut now)).show(ui);
        shared.set(now);
    });
    fixture.settle();
    (fixture, edges)
}

fn chosen(edges: &Cell<BorderEdges>) -> Vec<EdgeKey> {
    EdgeKey::ALL.into_iter().filter(|&key| edges.get().get(key)).collect()
}

#[trial]
fn edge_clicks_toggle_double_picks_a_group_triple_toggles_all() {
    let (mut fixture, edges) = edge_picker();
    let top = fixture.by_label("Top Border").expect("the top line is a check box").center();
    fixture.click_at(top);
    assert_eq!(chosen(&edges), vec![EdgeKey::Top]);
    fixture.click_at(top);
    assert_eq!(chosen(&edges), vec![EdgeKey::Top, EdgeKey::Bottom, EdgeKey::Left, EdgeKey::Right], "the double click picks the outer group");
    fixture.click_at(top);
    assert_eq!(chosen(&edges), EdgeKey::ALL.to_vec(), "the triple click turns all on, the choice before it being none");
    fixture.advance(2.0);
    fixture.idle(2);
    for _ in 0..3 {
        fixture.click_at(top);
    }
    assert!(chosen(&edges).is_empty(), "a triple click from all on turns all off");
}

// Size picker

fn size_picker(hovered: Rc<RefCell<Option<(usize, usize)>>>, picked: Rc<RefCell<Vec<(usize, usize)>>>) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let out: TableSizePickerOutput = TableSizePicker::new("Insert Table").show(ui);
        *hovered.borrow_mut() = out.hovered;
        picked.borrow_mut().extend(out.picked);
    });
    fixture.settle();
    fixture
}

#[trial]
fn the_pointer_sizes_rows_by_columns_and_a_click_picks() {
    let (hovered, picked): (Rc<RefCell<Option<(usize, usize)>>>, Rc<RefCell<Vec<(usize, usize)>>>) = Default::default();
    let mut fixture = size_picker(Rc::clone(&hovered), Rc::clone(&picked));
    let square = fixture.by_label("3 × 4").expect("each square is named by its size").center();
    fixture.frame(vec![pointer::moved(square)]);
    assert_eq!(*hovered.borrow(), Some((3, 4)), "rows, then columns");
    fixture.click_at(square);
    assert_eq!(*picked.borrow(), vec![(3, 4)]);
}

#[trial]
fn the_size_picker_keys_stop_at_the_edges() {
    let (hovered, picked): (Rc<RefCell<Option<(usize, usize)>>>, Rc<RefCell<Vec<(usize, usize)>>>) = Default::default();
    let mut fixture = size_picker(Rc::clone(&hovered), Rc::clone(&picked));
    fixture.key(Key::Tab, Modifiers::NONE);
    assert_eq!(*hovered.borrow(), Some((1, 1)), "the first square takes the focus");
    fixture.key(Key::ArrowLeft, Modifiers::NONE).key(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(*hovered.borrow(), Some((1, 1)), "the arrows stop at the grid's edges");
    fixture.key(Key::ArrowRight, Modifiers::NONE).key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(*hovered.borrow(), Some((2, 2)));
    fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(*picked.borrow(), vec![(2, 2)]);
}

// Properties

#[trial]
fn the_properties_panel_reports_the_change_made() {
    let mut start = TableValues::default();
    (start.rows, start.columns, start.row_height_mode) = (3, 4, Some(RowHeight::AtLeast));
    let values = Rc::new(RefCell::new(start));
    let changes: Rc<RefCell<Vec<TableChange>>> = Rc::default();
    let (shown, seen) = (Rc::clone(&values), Rc::clone(&changes));
    let mut fixture = Fixture::new(move |ui| {
        let out = TableProperties::new(&shown.borrow(), UnitKind::Points).show(ui);
        seen.borrow_mut().extend(out.change);
    });
    fixture.room(vec2(360.0, 900.0));
    fixture.settle();
    let rows = fixture.by_label("Rows").expect("the rows field");
    fixture.click(&rows);
    fixture.key(Key::A, Modifiers::COMMAND).type_text("5").key(Key::Enter, Modifiers::NONE);
    assert_eq!(changes.borrow().last(), Some(&TableChange::Rows(5)));
    changes.borrow_mut().clear();
    let merge = fixture.by_label("Merge Cells").expect("the merge button");
    fixture.click(&merge);
    assert!(changes.borrow().is_empty(), "Merge Cells is disabled with one cell");
    values.borrow_mut().several_cells = true;
    fixture.settle();
    fixture.click(&merge);
    assert_eq!(*changes.borrow(), vec![TableChange::Merge]);
}

// Geometry

#[trial]
fn table_geometry_finds_cells_and_clamps_past_the_last_edge() {
    let geometry = TableGeometry::new(vec![0.0, 20.0, 50.0], vec![0.0, 40.0, 80.0, 120.0]);
    assert_eq!(geometry.cell_at(pos2(40.0, 19.9)), Some(CellAt::new(0, 1)), "a band owns its leading edge");
    assert_eq!(geometry.cell_at(pos2(39.9, 20.0)), Some(CellAt::new(1, 0)));
    assert_eq!(geometry.cell_at(pos2(120.0, 10.0)), None, "the last edge is outside");
    let past = CellRange::new(1, 4, 1..3, 2);
    assert_eq!(geometry.rect(&past), egui::Rect::from_min_max(pos2(40.0, 20.0), pos2(120.0, 50.0)));
}

// Text selection

/// Three lines of seven glyphs, 10 wide and 12 high, lines 20 apart, page point = screen point.
const LINES: [&str; 3] = ["one two", "six ten", "ace dog"];

struct Mock;

impl Mock {
    fn chars() -> Vec<char> {
        LINES.iter().flat_map(|line| line.chars()).collect()
    }

    fn box_of(glyph: usize) -> Rect {
        let (line, column) = ((glyph / 7) as f64, (glyph % 7) as f64);
        Rect::new(column * 10.0, line * 20.0, column * 10.0 + 10.0, line * 20.0 + 12.0)
    }

    fn center(glyph: usize) -> Pos2 {
        let c = Self::box_of(glyph).center();
        pos2(c.x as f32, c.y as f32)
    }

    fn joined(glyphs: impl Iterator<Item = usize>) -> String {
        let chars = Self::chars();
        let mut out = String::new();
        let mut line = None;
        for glyph in glyphs {
            if line.is_some_and(|l| l != glyph / 7) {
                out.push('\n');
            }
            line = Some(glyph / 7);
            out.push(chars[glyph]);
        }
        out
    }
}

impl PageTextSource for Mock {
    fn glyph_count(&self) -> usize {
        21
    }

    fn glyph_at(&self, p: Point) -> Option<usize> {
        (0..21).find(|glyph| Self::box_of(*glyph).inflate(4.0, 4.0).contains(p))
    }

    fn glyph_box_at(&self, p: Point) -> Option<usize> {
        (0..21).find(|glyph| Self::box_of(*glyph).contains(p))
    }

    fn line_rects(&self, glyphs: Range<usize>) -> Vec<Rect> {
        (0..3)
            .filter_map(|line| {
                let own: Vec<usize> = glyphs.clone().filter(|g| g / 7 == line).collect();
                own.first().zip(own.last()).map(|(a, b)| Self::box_of(*a).union(Self::box_of(*b)))
            })
            .collect()
    }

    fn glyphs_in(&self, rect: Rect) -> Vec<usize> {
        (0..21).filter(|glyph| rect.contains_rect(Self::box_of(*glyph))).collect()
    }

    fn word_at(&self, glyph: usize) -> Option<Range<usize>> {
        let chars = Self::chars();
        if chars[glyph] == ' ' {
            return None;
        }
        let line = glyph / 7;
        let start = (line * 7..=glyph).rev().take_while(|g| chars[*g] != ' ').last()?;
        let end = (glyph..line * 7 + 7).take_while(|g| chars[*g] != ' ').last()? + 1;
        Some(start..end)
    }

    fn line_at(&self, glyph: usize) -> Option<Range<usize>> {
        Some(glyph / 7 * 7..glyph / 7 * 7 + 7)
    }

    fn text_of(&self, glyphs: Range<usize>) -> String {
        Mock::joined(glyphs)
    }

    fn column_text(&self, glyphs: &[usize]) -> String {
        Mock::joined(glyphs.iter().copied())
    }
}

struct Texts;

impl PageTexts for Texts {
    fn page(&self, page: usize) -> Option<&dyn PageTextSource> {
        (page == 0).then_some(&Mock)
    }
}

/// The select tool run over a canvas that fills a 200 by 100 room, with the page at its top left.
struct Rig {
    fixture: Fixture,
    state: Rc<RefCell<TextSelectState>>,
}

fn rig(rtl: bool) -> Rig {
    let state: Rc<RefCell<TextSelectState>> = Rc::default();
    let shown = Rc::clone(&state);
    let rect = egui::Rect::from_min_size(Pos2::ZERO, vec2(200.0, 100.0));
    let pages = vec![PlacedPage::new(0, rect, 1.0, (0.0, 100.0))];
    let mut fixture = Fixture::new(move |ui| {
        let canvas = Canvas::new("Page").show(ui, |_, _| ());
        let mut state = shown.borrow_mut();
        let mut tool = TextSelectTool::new(&pages, &Texts, &mut state).right_to_left(rtl);
        let _ = craft_ui::canvas::ToolHost::new().run(ui, &canvas.output, &mut tool);
    });
    fixture.room(vec2(200.0, 100.0));
    fixture.settle();
    Rig { fixture, state }
}

impl Rig {
    fn selection(&self) -> Option<TextSelection> {
        self.state.borrow().selection
    }

    fn press(&mut self, at: Pos2) {
        self.fixture.frame(vec![pointer::moved(at)]);
        self.fixture.frame(vec![pointer::press(at)]);
    }

    fn release(&mut self, at: Pos2) {
        self.fixture.frame(vec![pointer::release(at)]);
    }

    fn move_to(&mut self, at: Pos2) {
        self.fixture.frame(vec![pointer::moved(at)]);
    }
}

fn run(anchor: usize, head: usize) -> Option<TextSelection> {
    Some(TextSelection::new(0, TextSpan::run(anchor, head)))
}

#[trial]
fn a_drag_selects_a_run_anchored_at_the_press() {
    let mut rig = rig(false);
    rig.press(Mock::center(4));
    rig.move_to(Mock::center(10));
    assert_eq!(rig.selection(), run(4, 10));
    let selected = rig.selection().expect("selected");
    assert_eq!(selected.text(&Mock), "two\nsix ");
    assert_eq!(selected.rects(&Mock).len(), 2, "one rectangle per line piece");
    rig.release(Mock::center(10));
    assert_eq!(rig.selection(), run(4, 10), "the release keeps it");
}

#[trial]
fn an_alt_drag_selects_a_column() {
    let mut rig = rig(false);
    rig.fixture.hold(Modifiers::ALT);
    rig.press(pos2(0.0, 0.0));
    rig.move_to(pos2(31.0, 56.0));
    rig.release(pos2(31.0, 56.0));
    let selected = rig.selection().expect("selected");
    assert!(matches!(selected.span, TextSpan::Column { .. }));
    assert_eq!(selected.text(&Mock), "one\nsix\nace");
}

#[trial]
fn multi_clicks_select_word_line_and_page() {
    let mut rig = rig(false);
    let at = Mock::center(5);
    for (clicks, expected) in [(1, run(5, 5)), (2, run(4, 6)), (3, run(0, 6)), (4, run(0, 20))] {
        rig.press(at);
        assert_eq!(rig.selection(), expected, "{clicks} clicks");
        rig.release(at);
    }
    rig.fixture.advance(2.0);
    rig.press(pos2(150.0, 90.0));
    assert_eq!(rig.selection(), None, "a press off any glyph clears the selection");
}

#[trial]
fn shift_arrows_extend_toward_the_arrows_side_in_either_direction() {
    for (rtl, key, head) in [(false, Key::ArrowRight, 5), (false, Key::ArrowLeft, 3), (true, Key::ArrowLeft, 5), (true, Key::ArrowRight, 3)] {
        let mut rig = rig(rtl);
        rig.press(Mock::center(4));
        rig.release(Mock::center(4));
        rig.fixture.key(key, Modifiers::SHIFT);
        assert_eq!(rig.selection(), run(4, head), "rtl {rtl} {key:?}");
    }
}

#[trial]
fn escape_clears_the_selection_and_cancels_a_drag() {
    let mut rig = rig(false);
    rig.press(Mock::center(4));
    rig.release(Mock::center(10));
    rig.press(Mock::center(15));
    rig.move_to(Mock::center(19));
    assert_eq!(rig.selection(), run(15, 19));
    rig.fixture.key(Key::Escape, Modifiers::NONE);
    assert_eq!(rig.selection(), run(4, 10), "Escape during the drag brings the earlier selection back");
    rig.fixture.key(Key::Escape, Modifiers::NONE);
    assert_eq!(rig.selection(), None, "Escape clears the selection");
}

// Accessibility bridge

/// What a fake document describes.
#[derive(Clone, Copy)]
enum Content {
    /// A heading, a two-item list and a 2 by 2 table.
    Mixed,
    /// This many paragraphs.
    Paragraphs(usize),
}

struct Fake {
    content: Content,
    /// The pages of the last `describe` call.
    described: RefCell<Vec<usize>>,
}

fn line(out: &mut AccessBuilder, parent: AccessNode, text: &str, top: f32) -> Option<AccessNode> {
    let chars: Vec<(f32, f32)> = (0..text.chars().count()).map(|i| (10.0 + i as f32 * 8.0, 8.0)).collect();
    out.line(parent, text, &chars, egui::Rect::from_min_size(pos2(10.0, top), vec2(100.0, 12.0)))
}

impl AccessDescriber for Fake {
    fn describe(&self, placed: &[PlacedPage], out: &mut AccessBuilder) {
        *self.described.borrow_mut() = placed.iter().map(|page| page.page).collect();
        match self.content {
            Content::Mixed => {
                if let Some(heading) = out.block(None, BlockRole::Heading(2)) {
                    line(out, heading, "Title page", 0.0);
                }
                for (i, text) in ["first", "second"].into_iter().enumerate() {
                    let marker = format!("{}.", i + 1);
                    if let Some(item) = out.block(None, BlockRole::list_item(0, marker)) {
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
        None
    }
}

/// A page view of `pages` pages that describes `content`.
fn described(content: Content, pages: usize) -> (Fixture, Rc<Fake>) {
    let fake = Rc::new(Fake { content, described: RefCell::default() });
    let shown = Rc::clone(&fake);
    let source = craft_pages::ui::fixtures::DemoPages::new(pages);
    let state = Rc::new(RefCell::new(PageViewState::default()));
    let mut fixture = Fixture::new(move |ui| {
        let mut state = state.borrow_mut();
        PageView::new("Document", &source, &mut state, PAPER).access(&*shown).show(ui, |_, _| ());
    });
    fixture.settle();
    (fixture, fake)
}

#[trial]
fn blocks_nest_as_a_document_a_screen_reader_can_walk() {
    let (fixture, _) = described(Content::Mixed, 1);
    let tree = fixture.tree();
    let heading = tree.with_role(Role::HEADING);
    assert_eq!(heading.len(), 1);
    let runs = tree.children_of(heading[0]);
    assert_eq!(runs.len(), 1, "a heading holds the run of its line");
    let table = tree.with_role(Role::TABLE);
    let rows = tree.children_of(table[0]);
    assert_eq!(rows.len(), 2);
    assert_eq!(tree.grid_of(table[0]).row_count, Some(2));
    assert_eq!(tree.grid_of(table[0]).column_count, Some(2));
    let cells = tree.children_of(rows[1]);
    assert_eq!((tree.grid_of(cells[1]).row_index, tree.grid_of(cells[1]).column_index), (Some(1), Some(1)));
    let lists = tree.with_role(Role::LIST);
    assert_eq!(lists.len(), 1, "consecutive items at one level share a list");
    assert_eq!(tree.children_of(lists[0]).len(), 2);
}

#[trial]
fn the_node_count_is_capped() {
    let (fixture, _) = described(Content::Paragraphs(MAX_NODES + 500), 1);
    let described = fixture.tree().with_role(Role::PARAGRAPH).len();
    assert_eq!(described, MAX_NODES, "the builder stops at the cap");
}

#[trial]
fn only_the_pages_in_view_and_one_either_side_are_described() {
    let placed = demo_placed(10, pos2(0.0, 0.0));
    let (_, fake) = described(Content::Paragraphs(1), 10);
    let shown = fake.described.borrow().clone();
    assert!(shown.len() < placed.len(), "pages {shown:?} of 10 are described, not all of them");
    assert!(shown.windows(2).all(|pair| pair[1] == pair[0] + 1), "the pages are consecutive: {shown:?}");
}

// Reading view

fn metrics() -> ReadingMetrics {
    ReadingMetrics::new(20.0, 40.0)
}

#[trial]
fn the_reading_size_fits_whole_columns() {
    let metrics = metrics();
    let (width, height) = reading_size(vec2(1200.0, 900.0), 1.0, &metrics, None);
    assert!(width >= 192.0, "no column is narrower than the least width: {width}");
    assert!((4.0 * width + 5.0 * metrics.column_gap - 1200.0).abs() < 1e-3, "four columns fill the width: {width}");
    assert_eq!(height, 900.0 - metrics.column_gap - metrics.nav_bar);
    let (requested, _) = reading_size(vec2(500.0, 900.0), 1.0, &metrics, Some(612.0));
    assert_eq!(requested, 500.0 - 2.0 * metrics.column_gap, "a chosen width gives way to a narrow window");
    let viewport = reading_viewport(egui::Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0)), 300.0, 20.0, 1.0);
    assert!(viewport.width() <= 1000.0 && (viewport.width() - 20.0) % 320.0 == 0.0, "whole columns only: {}", viewport.width());
}

#[trial]
fn turns_stay_within_the_columns() {
    let mut state = ReadingState::default();
    state.turn(-1, 4);
    assert_eq!(state.column, 0, "no turn back from the first column");
    state.turn(2, 4);
    state.turn(5, 4);
    assert_eq!(state.column, 3, "no turn on from the last column");
    let mut carry = 0.0;
    assert_eq!(wheel_turns(vec2(0.0, -40.0), &mut carry), 1, "a notch down turns forward");
    assert_eq!(wheel_turns(vec2(0.0, -15.0), &mut carry), 0, "a touchpad's small steps add up");
    assert_eq!(wheel_turns(vec2(0.0, -30.0), &mut carry), 1);
    assert_eq!(swipe_turn(pos2(100.0, 50.0), pos2(20.0, 52.0), 300.0, 1.0), 1, "a swipe against the columns turns forward");
    assert_eq!(swipe_turn(pos2(100.0, 50.0), pos2(20.0, 52.0), 300.0, -1.0), -1, "and the other way round in a right-to-left row");
}
