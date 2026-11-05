//! Stories of the table widgets. Not registered yet: registering them would change the registry
//! dump and run their bodies. The registration task adds them to the gallery, the words of
//! [`TableStoryLabels`] to the words registry, and regenerates the dump.
//!
//! Each story keeps its own demo state in egui memory and applies the widget's events to it as
//! an app would; the grid stories share one fake sheet ([`Sheet`], a [`GridModel`] over 40 rows
//! and 12 columns of sample sales). The line under a story names the last event. Sample names,
//! categories and field labels are words of [`TableStoryLabels`].
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/cell-grid/sheet` | 40 × 12 cells, a frozen header row, a wide column, two selected ranges, an error tag and filter buttons |
//! | `patterns/cell-grid/outline` | two levels of row groups (one collapsed) and a column group |
//! | `patterns/condition-builder/rules` | Match all of: a rating, a date range and a nested any-group |
//! | `patterns/field-list/pane` | five fields, Region in Rows and Sales in Values |
//! | `composites/chart/primitives` | a column chart and a doughnut |
//! | `composites/slicer/tiles` | a two-column slicer with three of six values on |
//! | `composites/selection-stats/readout` | Average, Count and Sum of a selection |

use std::collections::BTreeMap;

use craft_theme::TypeRole;
use craft_ui::behavior::{DropPosition, Move};
use craft_ui::controls::PickedDate;
use craft_ui::edit::EditEvent;
use craft_ui::list::SortOrder;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, NoArgs, Story, StoryLabels, lock, shared};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};
use craft_ui::typed_story;
use egui::{Align2, Color32, CornerRadius, FontId, Painter, Rect, Sense, Ui, UiBuilder, Vec2, pos2, vec2};

use super::chart::{ChartPrimitive, HAlign, VAlign, paint_chart};
use super::conditions::{Condition, ConditionBuilder, ConditionFields, ConditionGroup, ConditionItem, ConditionKind, ConditionValue, Match};
use super::error_tag::ErrorTag;
use super::fields::{FieldArea, FieldListEdit, FieldListPane, PivotField};
use super::filter::{FilterButton, filter_button_rect};
use super::grid::{CellGrid, CellRange, CellRef, CellSelection, GridEdit, GridModel, GridState, LineIndex};
use super::outline::{OutlineAction, OutlineAxis, OutlineBand, OutlineGroup};
use super::slicer::{Slicer, SlicerAction};
use super::stats::{SelectionStats, SelectionStatsReadout, StatsLabels, StatsShown, stats_parts};
use super::style::SheetStyle;

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/cell-grid/sheet", Level::Patterns, "Cell grid", "Sheet", NoArgs {}, cell_grid),
        typed_story!("patterns/cell-grid/outline", Level::Patterns, "Cell grid", "Outline", NoArgs {}, outline),
        typed_story!("patterns/condition-builder/rules", Level::Patterns, "Condition builder", "Rules", NoArgs {}, conditions),
        typed_story!("patterns/field-list/pane", Level::Patterns, "Field list", "Pane", NoArgs {}, fields),
        typed_story!("composites/chart/primitives", Level::Composites, "Chart", "Primitives", NoArgs {}, chart),
        typed_story!("composites/slicer/tiles", Level::Composites, "Slicer", "Tiles", NoArgs {}, slicer),
        typed_story!("composites/selection-stats/readout", Level::Composites, "Selection stats", "Readout", NoArgs {}, stats),
    ]
}

craft_ui::words! {
    /// The words the table stories add: the sample sheet's headings, categories and notes, and
    /// the condition builder's and field list's sample fields.
    #[non_exhaustive]
    pub(crate) struct TableStoryLabels {
        /// The sample sheet's column headings.
        head_region: "tableStory.headRegion" = "Region";
        head_product: "tableStory.headProduct" = "Product";
        head_channel: "tableStory.headChannel" = "Channel";
        head_quarter: "tableStory.headQuarter" = "Quarter";
        head_units: "tableStory.headUnits" = "Units";
        head_price: "tableStory.headPrice" = "Price";
        head_sales: "tableStory.headSales" = "Sales";
        head_discount: "tableStory.headDiscount" = "Discount";
        head_cost: "tableStory.headCost" = "Cost";
        head_margin: "tableStory.headMargin" = "Margin";
        head_rating: "tableStory.headRating" = "Rating";
        head_notes: "tableStory.headNotes" = "Notes";
        head_year: "tableStory.headYear" = "Year";
        /// The six regions (also the slicer's values).
        region_north: "tableStory.regionNorth" = "North";
        region_south: "tableStory.regionSouth" = "South";
        region_east: "tableStory.regionEast" = "East";
        region_west: "tableStory.regionWest" = "West";
        region_central: "tableStory.regionCentral" = "Central";
        region_export: "tableStory.regionExport" = "Export";
        /// The four products.
        product_widget: "tableStory.productWidget" = "Widget";
        product_gadget: "tableStory.productGadget" = "Gadget";
        product_gizmo: "tableStory.productGizmo" = "Gizmo";
        product_doohickey: "tableStory.productDoohickey" = "Doohickey";
        /// The two channels.
        channel_online: "tableStory.channelOnline" = "Online";
        channel_retail: "tableStory.channelRetail" = "Retail";
        /// A quarter's name; `{n}` is 1 to 4.
        quarter: "tableStory.quarter" = "Q{n}";
        note_review: "tableStory.noteReview" = "Check figures";
        /// The error tag's problem and fixes.
        problem_text_number: "tableStory.problemTextNumber" = "Number Stored as Text";
        fix_convert: "tableStory.fixConvert" = "Convert to Number";
        fix_ignore: "tableStory.fixIgnore" = "Ignore Error";
        /// The condition builder's sample fields, operators and units.
        field_rating: "tableStory.fieldRating" = "Rating";
        field_date: "tableStory.fieldDate" = "Capture date";
        field_keyword: "tableStory.fieldKeyword" = "Keyword";
        group_photo: "tableStory.groupPhoto" = "Photo";
        group_metadata: "tableStory.groupMetadata" = "Metadata";
        op_at_least: "tableStory.opAtLeast" = "is at least";
        op_is: "tableStory.opIs" = "is";
        op_contains: "tableStory.opContains" = "contains";
        op_in_last: "tableStory.opInLast" = "is in the last";
        op_on: "tableStory.opOn" = "is on";
        unit_days: "tableStory.unitDays" = "days";
        unit_weeks: "tableStory.unitWeeks" = "weeks";
        unit_months: "tableStory.unitMonths" = "months";
        keyword_sunset: "tableStory.keywordSunset" = "Sunset";
        keyword_beach: "tableStory.keywordBeach" = "Beach";
        /// The charts' titles.
        chart_columns: "tableStory.chartColumns" = "Sales by region";
        chart_doughnut: "tableStory.chartDoughnut" = "Share of sales";
    }
}

impl TableStoryLabels {
    fn regions(&self) -> [&str; 6] {
        [&self.region_north, &self.region_south, &self.region_east, &self.region_west, &self.region_central, &self.region_export]
    }

    fn products(&self) -> [&str; 4] {
        [&self.product_widget, &self.product_gadget, &self.product_gizmo, &self.product_doohickey]
    }

    fn headings(&self) -> [&str; 12] {
        [
            &self.head_region,
            &self.head_product,
            &self.head_channel,
            &self.head_quarter,
            &self.head_units,
            &self.head_price,
            &self.head_sales,
            &self.head_discount,
            &self.head_cost,
            &self.head_margin,
            &self.head_rating,
            &self.head_notes,
        ]
    }
}

/// The sample sheet's size.
const ROWS: u32 = 40;
const COLUMNS: u32 = 12;

/// The height of a grid over its width.
const GRID_ASPECT: f32 = 0.7;

fn last_event(ui: &mut Ui, event: Option<&str>) {
    if let Some(event) = event {
        let line = fmt(&StoryLabels::of(ui).last_event, &[("event", event)]);
        ui.add(Text::new(line).content(Content::Muted));
    }
}

fn rgba(color: Color32) -> [u8; 4] {
    color.to_array()
}

/// The sample sheet: the header row, then sales rows made from the story words, with the numbers
/// kept for the statistics.
struct Sheet {
    rows: LineIndex,
    columns: LineIndex,
    texts: Vec<Vec<String>>,
    numbers: Vec<Vec<Option<f64>>>,
    /// Row heights and column widths at 100 %.
    default_row: f32,
    default_column: f32,
}

impl Sheet {
    fn new(labels: &TableStoryLabels, style: &SheetStyle) -> Self {
        let (regions, products) = (labels.regions(), labels.products());
        let channels = [&labels.channel_online, &labels.channel_retail];
        let mut texts = vec![labels.headings().iter().map(|h| (*h).to_owned()).collect::<Vec<_>>()];
        let mut numbers = vec![vec![None; COLUMNS as usize]];
        for i in 0..(ROWS as usize - 1) {
            let units = (20 + (i * 37) % 80) as f64;
            let price = 4.5 + ((i * 13) % 20) as f64 * 0.5;
            let sales = units * price;
            let cost = sales * 0.6;
            let discount = ((i * 7) % 5) as f64 * 5.0;
            let rating = (1 + (i * 3) % 5) as f64;
            let quarter = fmt(&labels.quarter, &[("n", &((i / 4) % 4 + 1).to_string())]);
            let note = if i % 9 == 3 { labels.note_review.clone() } else { String::new() };
            texts.push(vec![
                regions[i % 6].to_owned(),
                products[(i / 2) % 4].to_owned(),
                channels[i % 2].clone(),
                quarter,
                format!("{units:.0}"),
                format!("{price:.2}"),
                format!("{sales:.2}"),
                format!("{discount:.0}%"),
                format!("{cost:.2}"),
                format!("{:.2}", sales - cost),
                format!("{rating:.0}"),
                note,
            ]);
            // The units of the error tag's cell (row 3) are text, not a number.
            numbers.push(vec![
                None,
                None,
                None,
                None,
                (i != 2).then_some(units),
                Some(price),
                Some(sales),
                Some(discount),
                Some(cost),
                Some(sales - cost),
                Some(rating),
                None,
            ]);
        }
        let (default_row, default_column) = (style.column_header_height, style.column_header_height * 3.0);
        let mut sheet = Self {
            rows: LineIndex::new(&BTreeMap::new(), default_row, ROWS),
            columns: LineIndex::new(&BTreeMap::new(), default_column, COLUMNS),
            texts,
            numbers,
            default_row,
            default_column,
        };
        sheet.set_lines(&BTreeMap::new(), &BTreeMap::new());
        sheet
    }

    /// Rows and columns whose size is not the default (0 hides).
    fn set_lines(&mut self, rows: &BTreeMap<u32, f32>, columns: &BTreeMap<u32, f32>) {
        self.rows = LineIndex::new(rows, self.default_row, ROWS);
        self.columns = LineIndex::new(columns, self.default_column, COLUMNS);
    }

    fn stats(&self, selection: &CellSelection) -> SelectionStats {
        let mut stats = SelectionStats { min: f64::INFINITY, max: f64::NEG_INFINITY, ..SelectionStats::default() };
        for range in selection.ranges() {
            for row in range.rows() {
                for column in range.columns() {
                    let (row, column) = (row as usize, column as usize);
                    if self.texts[row][column].is_empty() {
                        continue;
                    }
                    stats.count += 1;
                    if let Some(value) = self.numbers[row][column] {
                        stats.numeric += 1;
                        stats.sum += value;
                        stats.min = stats.min.min(value);
                        stats.max = stats.max.max(value);
                    }
                }
            }
        }
        stats
    }
}

impl GridModel for Sheet {
    fn rows(&self) -> &LineIndex {
        &self.rows
    }

    fn columns(&self) -> &LineIndex {
        &self.columns
    }

    fn frozen(&self) -> (u32, u32) {
        (1, 0)
    }

    fn last_used(&self) -> CellRef {
        CellRef::new(ROWS - 1, COLUMNS - 1)
    }

    fn paint_cell(&self, painter: &Painter, rect: Rect, c: CellRef, zoom: f32) {
        let tokens = craft_ui::theme::current(painter.ctx());
        let header = c.row == 0;
        if header {
            painter.rect_filled(rect, CornerRadius::ZERO, tokens.color.surface.chrome.to_egui());
        }
        let text = self.cell_text(c);
        if text.is_empty() {
            return;
        }
        let right = self.numbers[c.row as usize][c.column as usize].is_some();
        let pad = tokens.space.gap.inline * zoom;
        let font = tokens.font(if header { TypeRole::BodyStrong } else { TypeRole::Body });
        let font = FontId::new(font.size * zoom, font.family);
        let (anchor, x) = if right { (Align2::RIGHT_CENTER, rect.right() - pad) } else { (Align2::LEFT_CENTER, rect.left() + pad) };
        painter.with_clip_rect(rect).text(pos2(x, rect.center().y), anchor, text, font, tokens.color.text.normal.to_egui());
    }

    fn cell_text(&self, c: CellRef) -> String {
        self.texts.get(c.row as usize).and_then(|row| row.get(c.column as usize)).cloned().unwrap_or_default()
    }
}

/// A resize gesture's size once it ends.
fn settled(edit: &EditEvent<f32>) -> Option<f32> {
    matches!(edit, EditEvent::Commit { .. } | EditEvent::Set { .. }).then(|| edit.value().copied()).flatten()
}

struct GridDemo {
    sheet: Sheet,
    state: GridState,
    rows: BTreeMap<u32, f32>,
    columns: BTreeMap<u32, f32>,
    last: Option<String>,
}

// new: 40 × 12 cells of sample data, a frozen header row, a custom-width column, a two-range
// selection with the active cell, an error tag and filter buttons on the header row.
fn cell_grid(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TableStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let style = SheetStyle::of(&tokens);
    let demo = shared(ui, "grid", || {
        let mut sheet = Sheet::new(&labels, &style);
        // The Product column is wide.
        let columns = BTreeMap::from([(1, sheet.default_column * 1.75)]);
        sheet.set_lines(&BTreeMap::new(), &columns);
        let selection = CellSelection::from_ranges(
            vec![CellRange::new(CellRef::new(2, 1), CellRef::new(5, 3)), CellRange::new(CellRef::new(8, 4), CellRef::new(10, 6))],
            CellRef::new(2, 1),
        )
        .unwrap_or_default();
        GridDemo { sheet, state: GridState { selection, ..GridState::default() }, rows: BTreeMap::new(), columns, last: None }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;

    let width = ui.available_width();
    let (area, _) = ui.allocate_exact_size(vec2(width, width * GRID_ASPECT), Sense::hover());
    let out = ui.scope_builder(UiBuilder::new().max_rect(area), |ui| CellGrid::new("grid", &demo.sheet, &mut demo.state).show(ui)).inner;

    // The tag and the filter buttons sit over the cells, clipped to them.
    let geometry = &out.geometry;
    ui.scope_builder(UiBuilder::new().max_rect(area), |ui| {
        ui.set_clip_rect(geometry.cells().intersect(ui.clip_rect()));
        let fixes = [labels.fix_convert.as_str(), labels.fix_ignore.as_str()];
        let tag = ErrorTag::new("tag", CellRef::new(3, 4), geometry, &labels.problem_text_number, &fixes).show(ui);
        if let Some(fix) = tag.fix {
            demo.last = Some(format!("Fix({fix})"));
        }
        for column in 0..COLUMNS {
            let cell = geometry.cell_rect(CellRef::new(0, column));
            if !cell.intersects(geometry.cells()) {
                continue;
            }
            let rect = filter_button_rect(cell, demo.state.zoom, style.filter_button_max, &style);
            let button = FilterButton::new(("filter", column), labels.headings()[column as usize])
                .filtered(column == 2)
                .sorted((column == 0).then_some(SortOrder::Ascending));
            if button.show(ui, rect).event.is_some() {
                demo.last = Some(format!("Filter({column})"));
            }
        }
    });

    for edit in out.edits {
        demo.last = Some(format!("{edit:?}"));
        match edit {
            GridEdit::ResizeColumn { column, width } => {
                if let Some(width) = settled(&width) {
                    demo.columns.insert(column, width);
                }
            }
            GridEdit::ResizeRow { row, height } => {
                if let Some(height) = settled(&height) {
                    demo.rows.insert(row, height);
                }
            }
            _ => {}
        }
    }
    demo.sheet.set_lines(&demo.rows, &demo.columns);
    last_event(ui, demo.last.as_deref());
}

struct OutlineDemo {
    sheet: Sheet,
    state: GridState,
    rows: OutlineAxis,
    columns: OutlineAxis,
    last: Option<String>,
}

fn group(start: u32, end: u32, level: u8, collapsed: bool) -> OutlineGroup {
    OutlineGroup { start, end, level, collapsed, summary_before: false }
}

/// The lines a collapsed group hides: its details, not the summary line after it.
fn hidden_lines(axis: &OutlineAxis) -> BTreeMap<u32, f32> {
    axis.groups.iter().filter(|g| g.collapsed).flat_map(|g| g.start..=g.end).map(|line| (line, 0.0)).collect()
}

fn apply_outline(axis: &mut OutlineAxis, action: OutlineAction, rows: bool) {
    match action {
        OutlineAction::Level { rows: for_rows, level } if for_rows == rows => {
            for group in &mut axis.groups {
                group.collapsed = group.level >= level;
            }
        }
        OutlineAction::Toggle { rows: for_rows, group } if for_rows == rows => {
            if let Some(found) = axis.groups.iter_mut().find(|g| g.start == group.start && g.end == group.end && g.level == group.level) {
                found.collapsed = !found.collapsed;
            }
        }
        _ => {}
    }
}

// new: two levels of row groups, one collapsed, and a column group.
fn outline(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TableStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let style = SheetStyle::of(&tokens);
    let demo = shared(ui, "outline", || OutlineDemo {
        sheet: Sheet::new(&labels, &style),
        state: GridState::default(),
        rows: OutlineAxis { levels: 2, groups: vec![group(1, 10, 1, false), group(3, 5, 2, true), group(13, 20, 1, false)] },
        columns: OutlineAxis { levels: 1, groups: vec![group(3, 5, 1, false)] },
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let (rows, columns) = (hidden_lines(&demo.rows), hidden_lines(&demo.columns));
    demo.sheet.set_lines(&rows, &columns);

    let width = ui.available_width();
    let (area, _) = ui.allocate_exact_size(vec2(width, width * GRID_ASPECT), Sense::hover());
    let (row_levels, column_levels) = (demo.rows.levels, demo.columns.levels);
    let out = ui
        .scope_builder(UiBuilder::new().max_rect(area), |ui| {
            CellGrid::new("grid", &demo.sheet, &mut demo.state).outline_levels(row_levels, column_levels).show(ui)
        })
        .inner;
    let band = OutlineBand::new("outline", &demo.rows, &demo.columns, &out.geometry).show(ui);
    for action in band.actions {
        demo.last = Some(format!("{action:?}"));
        apply_outline(&mut demo.rows, action, true);
        apply_outline(&mut demo.columns, action, false);
    }
    for edit in out.edits {
        demo.last = Some(format!("{edit:?}"));
    }
    last_event(ui, demo.last.as_deref());
}

/// The sample fields of the condition builder, labelled by the story words.
struct PhotoFields<'a>(&'a TableStoryLabels);

impl ConditionFields for PhotoFields<'_> {
    fn fields(&self) -> Vec<(String, String, Option<String>)> {
        let l = self.0;
        vec![
            ("rating".into(), l.field_rating.clone(), Some(l.group_photo.clone())),
            ("date".into(), l.field_date.clone(), Some(l.group_metadata.clone())),
            ("keyword".into(), l.field_keyword.clone(), Some(l.group_metadata.clone())),
        ]
    }

    fn kind(&self, field: &str) -> ConditionKind {
        match field {
            "rating" => ConditionKind::Number,
            "date" => ConditionKind::Date,
            _ => ConditionKind::Text,
        }
    }

    fn operators(&self, field: &str) -> Vec<(String, String)> {
        let l = self.0;
        let pair = |id: &str, label: &String| (id.to_owned(), label.clone());
        match field {
            "rating" => vec![pair("gte", &l.op_at_least), pair("is", &l.op_is)],
            "date" => vec![pair("last", &l.op_in_last), pair("on", &l.op_on)],
            _ => vec![pair("is", &l.op_is), pair("contains", &l.op_contains)],
        }
    }

    fn default_value(&self, field: &str, operator: &str) -> ConditionValue {
        match (field, operator) {
            ("rating", _) => ConditionValue::Number(3.0),
            ("date", "last") => ConditionValue::Last { n: 30, unit: "days".into() },
            ("date", _) => ConditionValue::Date(PickedDate { year: 2026, month: Some(10), day: Some(1) }),
            _ => ConditionValue::Text(String::new()),
        }
    }

    fn choices(&self, field: &str) -> Vec<(String, String)> {
        let l = self.0;
        match field {
            "date" => vec![("days".into(), l.unit_days.clone()), ("weeks".into(), l.unit_weeks.clone()), ("months".into(), l.unit_months.clone())],
            _ => Vec::new(),
        }
    }
}

fn rule(field: &str, operator: &str, value: ConditionValue) -> ConditionItem {
    ConditionItem::Rule(Condition { field: field.into(), operator: operator.into(), value })
}

// new: Match all of: Rating is at least 3, Date in the last 30 days, and a nested any-group.
fn conditions(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TableStoryLabels::of(ui);
    let demo = shared(ui, "conditions", || ConditionGroup {
        matching: Match::All,
        items: vec![
            rule("rating", "gte", ConditionValue::Number(3.0)),
            rule("date", "last", ConditionValue::Last { n: 30, unit: "days".into() }),
            ConditionItem::Group(ConditionGroup {
                matching: Match::Any,
                items: vec![
                    rule("keyword", "is", ConditionValue::Text(labels.keyword_sunset.clone())),
                    rule("keyword", "is", ConditionValue::Text(labels.keyword_beach.clone())),
                ],
            }),
        ],
    });
    let mut group = lock(&demo);
    ConditionBuilder::new("rules", &mut group, &PhotoFields(&labels)).show(ui);
}

struct FieldsDemo {
    fields: Vec<PivotField>,
    areas: Vec<(FieldArea, Vec<String>)>,
    last: Option<String>,
}

/// Takes the moved fields out of their areas and puts them at the drop in `to`.
fn move_fields(areas: &mut [(FieldArea, Vec<String>)], to: FieldArea, mv: &Move<String>) {
    if !mv.copy {
        for (_, list) in areas.iter_mut() {
            list.retain(|f| !mv.items.contains(f) || *f == mv.target);
        }
    }
    let Some((_, list)) = areas.iter_mut().find(|(area, _)| *area == to) else { return };
    let target = list.iter().position(|f| *f == mv.target);
    let index = match (target, mv.position) {
        (Some(at), DropPosition::Before) => at,
        (Some(at), _) => at + 1,
        (None, _) => list.len(),
    };
    // A moved field that is its own target stays where it is.
    let items: Vec<String> = mv.items.iter().filter(|f| **f != mv.target).cloned().collect();
    for (n, item) in items.into_iter().enumerate() {
        list.insert((index + n).min(list.len()), item);
    }
}

// new: five fields, Region in Rows and Sales in Values.
fn fields(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TableStoryLabels::of(ui);
    let demo = shared(ui, "fields", || {
        let field = |name: &str, numeric| PivotField { name: name.into(), numeric };
        FieldsDemo {
            fields: vec![
                field(&labels.head_region, false),
                field(&labels.head_product, false),
                field(&labels.head_year, false),
                field(&labels.head_sales, true),
                field(&labels.head_units, true),
            ],
            areas: vec![
                (FieldArea::Filters, Vec::new()),
                (FieldArea::Columns, Vec::new()),
                (FieldArea::Rows, vec![labels.head_region.clone()]),
                (FieldArea::Values, vec![labels.head_sales.clone()]),
            ],
            last: None,
        }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let width = ui.available_width() * 0.5;
    let out = ui.allocate_ui(vec2(width, width * 1.4), |ui| FieldListPane::new("fields", &demo.fields, &demo.areas).show(ui)).inner;
    for edit in out.edits {
        demo.last = Some(format!("{edit:?}"));
        match edit {
            FieldListEdit::Add { field } => {
                let numeric = demo.fields.iter().any(|f| f.name == field && f.numeric);
                let target = if numeric { FieldArea::Values } else { FieldArea::Rows };
                if let Some((_, list)) = demo.areas.iter_mut().find(|(area, _)| *area == target)
                    && !list.contains(&field)
                {
                    list.push(field);
                }
            }
            FieldListEdit::Remove { field } => demo.areas.iter_mut().for_each(|(_, list)| list.retain(|f| *f != field)),
            FieldListEdit::Move { to, change } => {
                if matches!(change, EditEvent::Commit { .. } | EditEvent::Set { .. })
                    && let Some(mv) = change.value()
                {
                    move_fields(&mut demo.areas, to, mv);
                }
            }
            FieldListEdit::Refresh | FieldListEdit::CalculatedField | FieldListEdit::Layout(_) => {}
        }
    }
    last_event(ui, demo.last.as_deref());
}

/// The share of a chart's width and height left empty at each edge.
const CHART_MARGIN: f32 = 0.12;

/// A column chart: axes, dashed gridlines, a bar and a label per category.
fn column_chart(tokens: &UiTokens, ppp: f32, size: Vec2, title: &str, categories: &[&str], values: &[f32]) -> Vec<ChartPrimitive> {
    let ink = rgba(tokens.color.text.normal.to_egui());
    let axis = rgba(tokens.color.line.border.to_egui());
    let bar = rgba(tokens.color.accent.solid.to_egui());
    let hairline = craft_ui::theme::scale::Stroke::Hairline.width(ppp);
    let text_size = tokens.font(TypeRole::Caption).size;
    let (left, right) = (size.x * CHART_MARGIN, size.x * (1.0 - CHART_MARGIN));
    let (top, bottom) = (size.y * CHART_MARGIN * 1.5, size.y * (1.0 - CHART_MARGIN));
    let mut prims = vec![ChartPrimitive::Text {
        x: size.x / 2.0,
        y: top / 2.0,
        text: title.to_owned(),
        size: tokens.font(TypeRole::BodyStrong).size,
        color: ink,
        bold: true,
        align: HAlign::Center,
        valign: VAlign::Middle,
        rotation: 0.0,
    }];
    for step in 1..=3 {
        let y = bottom - (bottom - top) * step as f32 / 4.0;
        prims.push(ChartPrimitive::Line { points: vec![[left, y], [right, y]], color: axis, width: hairline, dash: true });
    }
    prims.push(ChartPrimitive::Line { points: vec![[left, top], [left, bottom], [right, bottom]], color: axis, width: hairline, dash: false });
    let max = values.iter().copied().fold(f32::MIN, f32::max).max(f32::EPSILON);
    let slot = (right - left) / values.len() as f32;
    for (i, (value, name)) in values.iter().zip(categories).enumerate() {
        let w = slot * 0.6;
        let h = (bottom - top) * value / max;
        let x = left + slot * i as f32 + (slot - w) / 2.0;
        prims.push(ChartPrimitive::Rect { x, y: bottom - h, w, h, fill: Some(bar), stroke: None, radius: 0.0 });
        prims.push(ChartPrimitive::Text {
            x: x + w / 2.0,
            y: bottom + text_size / 2.0,
            text: (*name).to_owned(),
            size: text_size,
            color: ink,
            bold: false,
            align: HAlign::Center,
            valign: VAlign::Top,
            rotation: 0.0,
        });
    }
    prims
}

/// A doughnut: a wedge per share, from 12 o'clock clockwise.
fn doughnut_chart(tokens: &UiTokens, ppp: f32, size: Vec2, title: &str, shares: &[f32]) -> Vec<ChartPrimitive> {
    let status = &tokens.color.status;
    let colors = [tokens.color.accent.solid, status.good.solid, status.warn.solid, status.bad.solid].map(|c| rgba(c.to_egui()));
    let ink = rgba(tokens.color.text.normal.to_egui());
    let hairline = craft_ui::theme::scale::Stroke::Hairline.width(ppp);
    let gap = Some((rgba(tokens.color.surface.raised.to_egui()), hairline));
    let (cx, cy) = (size.x / 2.0, size.y * 0.55);
    let r_outer = size.x.min(size.y) * 0.34;
    let total: f32 = shares.iter().sum();
    let mut prims = vec![ChartPrimitive::Text {
        x: cx,
        y: size.y * CHART_MARGIN,
        text: title.to_owned(),
        size: tokens.font(TypeRole::BodyStrong).size,
        color: ink,
        bold: true,
        align: HAlign::Center,
        valign: VAlign::Middle,
        rotation: 0.0,
    }];
    let mut from = 0.0;
    for (i, share) in shares.iter().enumerate() {
        let to = from + std::f32::consts::TAU * share / total;
        prims.push(ChartPrimitive::Wedge { cx, cy, r_outer, r_inner: r_outer * 0.55, a0: from, a1: to, fill: colors[i % colors.len()], stroke: gap });
        from = to;
    }
    prims
}

// new: a column chart and a doughnut from hand-written primitives.
fn chart(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TableStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let ppp = ui.ctx().pixels_per_point();
    let gap = ui.spacing().item_spacing.x;
    let width = (ui.available_width() - gap) / 2.0;
    let size = vec2(width, width * 0.7);
    ui.horizontal(|ui| {
        let regions = labels.regions();
        let columns = column_chart(&tokens, ppp, size, &labels.chart_columns, &regions[..4], &[42.0, 65.0, 38.0, 80.0]);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        paint_chart(&ui.painter_at(rect), rect.min, &columns, 1.0);
        let doughnut = doughnut_chart(&tokens, ppp, size, &labels.chart_doughnut, &[40.0, 30.0, 20.0, 10.0]);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        paint_chart(&ui.painter_at(rect), rect.min, &doughnut, 1.0);
    });
}

struct SlicerDemo {
    items: Vec<(String, bool)>,
    last: Option<String>,
}

// new: a two-column slicer with three of six values on.
fn slicer(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TableStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let style = SheetStyle::of(&tokens);
    let demo = shared(ui, "slicer", || SlicerDemo {
        items: labels.regions().iter().enumerate().map(|(i, name)| ((*name).to_owned(), i % 2 == 0)).collect(),
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let columns = 2u32;
    let rows = demo.items.len().div_ceil(columns as usize) as f32;
    let height = style.slicer_header + style.slicer_padding * 2.0 + rows * style.slicer_tile + (rows - 1.0) * style.slicer_gap;
    let width = ui.available_width() * 0.4;
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let filtered = demo.items.iter().any(|(_, on)| !on);
    let out = Slicer::new("slicer", &labels.head_region, &demo.items).columns(columns).filtered(filtered).show(ui, rect);
    for action in out.actions {
        demo.last = Some(format!("{action:?}"));
        match action {
            SlicerAction::Toggle(name) => {
                if let Some(item) = demo.items.iter_mut().find(|(n, _)| *n == name) {
                    item.1 = !item.1;
                }
            }
            SlicerAction::Clear => demo.items.iter_mut().for_each(|(_, on)| *on = true),
        }
    }
    last_event(ui, demo.last.as_deref());
}

struct StatsDemo {
    sheet: Sheet,
    shown: StatsShown,
    selection: CellSelection,
}

// new: Average, Count and Sum of a selection.
fn stats(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TableStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let style = SheetStyle::of(&tokens);
    let demo = shared(ui, "stats", || StatsDemo {
        sheet: Sheet::new(&labels, &style),
        shown: StatsShown::default(),
        selection: CellSelection::range(CellRange::new(CellRef::new(1, 4), CellRef::new(12, 6))),
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let totals = demo.sheet.stats(&demo.selection);
    let parts = stats_parts(&totals, &demo.shown, &StatsLabels::of(ui), |value| format!("{value:.2}"));
    SelectionStatsReadout::new("stats", &parts, &mut demo.shown).show(ui);
}

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests {
    use craft_ui::story::Args;
    use craft_ui::testing::symbols;
    use craft_ui::theme::{Base, install};
    use egui_kittest::Harness;

    // every story draws its widget over its sample data without panicking.
    #[test]
    fn every_story_draws() {
        for story in super::stories() {
            let mut harness = Harness::<Args>::builder()
                .with_size([720.0, 520.0])
                .with_context_setup(|ctx| {
                    install(ctx, &Base::suite_default());
                    symbols::install(ctx);
                })
                .build_ui_state(
                    move |ui, args| {
                        // The reaction emoji have no face in the test fonts.
                        ui.ctx().set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
                        (story.show)(ui, args);
                    },
                    story.defaults(),
                );
            harness.run_steps(4);
        }
    }
}
