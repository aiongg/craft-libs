//! Overlays over the grid: the filter button, the error tag and the statistics readout.

use std::collections::BTreeMap;

use craft_ui::testing::{events, symbols};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::{Event, Key, Modifiers, Painter, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::error_tag::ErrorTag;
use super::filter::{FilterButton, FilterButtonEvent};
use super::grid::{CellGrid, CellRange, CellRef, GridEdit, GridModel, GridState, LineIndex, SHEET_COLUMNS, SHEET_ROWS};
use super::stats::{SelectionStats, SelectionStatsReadout, StatPart, StatsLabels, StatsShown, stats_parts};
use super::style::SheetStyle;

struct Sheet {
    rows: LineIndex,
    columns: LineIndex,
}

impl GridModel for Sheet {
    fn rows(&self) -> &LineIndex {
        &self.rows
    }
    fn columns(&self) -> &LineIndex {
        &self.columns
    }
    fn frozen(&self) -> (u32, u32) {
        (0, 0)
    }
    fn merges(&self) -> &[CellRange] {
        &[]
    }
    fn last_used(&self) -> CellRef {
        CellRef::new(9, 9)
    }
    fn paint_cell(&self, _painter: &Painter, _rect: Rect, _c: CellRef, _zoom: f32) {}
    fn cell_text(&self, _c: CellRef) -> String {
        String::new()
    }
}

/// The header cell the filter button sits in, and the cell the error tag belongs to.
const FILTER_COLUMN: u32 = 2;
const TAG_CELL: CellRef = CellRef { row: 3, column: 2 };

struct App {
    sheet: Sheet,
    state: GridState,
    edits: Vec<GridEdit>,
    opened: Vec<FilterButtonEvent>,
    fix: Option<usize>,
    filter_rect: Rect,
    tag_rect: Rect,
    cell_rects: Vec<Rect>,
}

fn show(ui: &mut Ui, app: &mut App) {
    let style = SheetStyle::of(&Tokens::of(ui));
    let out = CellGrid::new("grid", &app.sheet, &mut app.state).show(ui);
    app.edits.extend(out.edits);
    let geometry = &out.geometry;
    let header = geometry.header_size();
    let side = style.filter_button_max;
    app.filter_rect = Rect::from_min_size(pos2(geometry.x(FILTER_COLUMN) + 30.0, geometry.cells().top() - header.y + 3.0), vec2(side, side));
    app.opened.extend(FilterButton::new("filter", "Region").show(ui, app.filter_rect).event);
    let tag = ErrorTag::new("tag", TAG_CELL, geometry, "Number Stored as Text", &["Convert to Number"]);
    app.tag_rect = tag.rect(&style).expect("inside the cells");
    let shown = tag.show(ui);
    app.fix = app.fix.or(shown.fix);
    app.cell_rects = (0..6).map(|row| geometry.cell_rect(CellRef::new(row, 1))).collect();
}

fn harness() -> Harness<'static, App> {
    let none = BTreeMap::new();
    let app = App {
        sheet: Sheet { rows: LineIndex::new(&none, 20.0, SHEET_ROWS), columns: LineIndex::new(&none, 80.0, SHEET_COLUMNS) },
        state: GridState::default(),
        edits: Vec::new(),
        opened: Vec::new(),
        fix: None,
        filter_rect: Rect::NOTHING,
        tag_rect: Rect::NOTHING,
        cell_rects: Vec::new(),
    };
    let mut harness = Harness::<App>::builder()
        .with_size([800.0, 500.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, app);
    harness.run();
    harness
}

fn play(harness: &mut Harness<'_, App>, frames: Vec<Vec<Event>>) {
    for frame in frames {
        for event in frame {
            harness.event(event);
        }
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

// A press on the filter button in a column header opens its menu and reaches nothing under it:
// the grid selects no column and begins no resize.
#[test]
fn a_filter_button_over_the_header_takes_the_press() {
    let mut harness = harness();
    let before = harness.state().state.selection.current();
    let at = harness.state().filter_rect.center();
    play(&mut harness, events::click(at, PointerButton::Primary, 1, Modifiers::NONE));
    assert_eq!(harness.state().opened, [FilterButtonEvent::Open]);
    assert_eq!(harness.state().state.selection.current(), before, "the column wasn't selected");
    assert!(harness.state().edits.is_empty(), "{:?}", harness.state().edits);
}

// A press on the error tag beside a cell reaches nothing under it: the cell under the tag stays
// unselected, and the tag's menu opens.
#[test]
fn an_error_tag_beside_a_cell_takes_the_press() {
    let mut harness = harness();
    let before = harness.state().state.selection.current();
    let at = harness.state().tag_rect.center();
    play(&mut harness, events::click(at, PointerButton::Primary, 1, Modifiers::NONE));
    assert_eq!(harness.state().state.selection.current(), before, "the cell under the tag wasn't selected");
    assert!(harness.state().edits.is_empty(), "{:?}", harness.state().edits);
    harness.get_by_label("Convert to Number").click();
    harness.run();
    assert_eq!(harness.state().fix, Some(0));
}

// Alt+Down on the focused filter button reports Open.
#[test]
fn alt_down_on_the_filter_button_opens_it() {
    let mut harness = harness();
    harness.get_by_label("Region (Showing All)").focus();
    harness.run();
    assert!(harness.state().opened.is_empty());
    play(&mut harness, events::key_press(Key::ArrowDown, Modifiers::ALT));
    assert_eq!(harness.state().opened, [FilterButtonEvent::Open]);
}

// A secondary drag of the fill handle reports the menu at release with the range it grew to, and
// fills and selects nothing.
#[test]
fn a_secondary_drag_of_the_fill_handle_reports_the_menu() {
    let mut harness = harness();
    let source = CellRef::new(1, 1);
    let at = harness.state().cell_rects[1].center();
    play(&mut harness, events::click(at, PointerButton::Primary, 1, Modifiers::NONE));
    let from = harness.state().cell_rects[1].right_bottom();
    let to = harness.state().cell_rects[4].center();
    let button = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    let mut frames = vec![vec![Event::PointerMoved(from)], vec![button(from, true)]];
    frames.extend((1..=4).map(|step| vec![Event::PointerMoved(from + (to - from) * (step as f32 / 4.0))]));
    frames.push(vec![button(to, false)]);
    harness.state_mut().edits.clear();
    play(&mut harness, frames);
    let menus: Vec<_> = harness.state().edits.iter().filter(|e| matches!(e, GridEdit::FillMenu { .. })).collect();
    assert_eq!(menus.len(), 1, "{:?}", harness.state().edits);
    match menus[0] {
        GridEdit::FillMenu { range, anchor } => {
            assert_eq!(*range, CellRange::new(source, CellRef::new(4, 1)));
            assert_eq!(*anchor, to);
        }
        _ => unreachable!(),
    }
    assert!(!harness.state().edits.iter().any(|e| matches!(e, GridEdit::Fill { .. })));
    assert_eq!(harness.state().state.selection.current(), CellRange::new(source, source));
}

struct StatsApp {
    parts: Vec<StatPart>,
    shown: StatsShown,
    copied: Vec<String>,
    rect: Rect,
}

fn stats_harness(parts: Vec<StatPart>) -> Harness<'static, StatsApp> {
    Harness::<StatsApp>::builder()
        .with_size([400.0, 200.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(
            |ui, app: &mut StatsApp| {
                app.rect = SelectionStatsReadout::new("stats", &app.parts, &mut app.shown).show(ui).rect;
                app.copied.extend(ui.ctx().output(|o| {
                    o.commands
                        .iter()
                        .filter_map(|c| if let egui::OutputCommand::CopyText(t) = c { Some(t.clone()) } else { None })
                        .collect::<Vec<_>>()
                }));
            },
            StatsApp { parts, shown: StatsShown::default(), copied: Vec::new(), rect: Rect::NOTHING },
        )
}

// A click on a statistic copies its value without the label.
#[test]
fn a_click_on_a_statistic_copies_its_value() {
    let stats = SelectionStats { count: 4, numeric: 2, sum: 7.0, min: 2.0, max: 5.0 };
    let parts = stats_parts(&stats, &StatsShown::default(), &StatsLabels::default(), |v| format!("{v:.1}"));
    let mut harness = stats_harness(parts);
    harness.run();
    let at = harness.get_by_label("Sum: 7.0").rect().center();
    let mut harness_frames = |frames| {
        for frame in frames {
            for event in frame {
                harness.event(event);
            }
            harness.run_steps(1);
        }
    };
    harness_frames(events::click(at, PointerButton::Primary, 1, Modifiers::NONE));
    assert_eq!(harness.state().copied, ["7.0"]);
}

// With no statistic shown the readout still takes the room it is given, so a secondary click opens
// the statistics menu, at least `sheet.status_menu_min_width` wide.
#[test]
fn an_empty_readout_opens_its_menu_at_the_least_width() {
    let mut style = None;
    egui::__run_test_ui(|ui| style = Some(SheetStyle::of(&Tokens::of(ui))));
    let style = style.unwrap();
    let mut harness = stats_harness(Vec::new());
    harness.run();
    let rect = harness.state().rect;
    assert!(rect.width() > 100.0 && rect.height() > 0.0, "{rect:?}");
    for frame in events::click(rect.center(), PointerButton::Secondary, 1, Modifiers::NONE) {
        for event in frame {
            harness.event(event);
        }
        harness.run_steps(1);
    }
    harness.run_steps(2);
    let width = harness.get_by_label("Average").rect().width();
    assert!(width >= style.status_menu_min_width, "{width} < {}", style.status_menu_min_width);
}
