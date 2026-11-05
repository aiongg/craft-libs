//! [`TableProperties`] (N3.12): InDesign's Table panel (and the Table section of Properties):
//! rows and columns, row height (At Least or Exactly) and column width, Distribute Rows Evenly,
//! cell text alignment and insets, merge and unmerge, insert and delete rows and columns, the
//! table's direction, alternating row fills, header and footer rows. Every change goes out as
//! one [`TableChange`]; the table stays the app's.

use craft_ui::composites::{Field, GroupBox};
use craft_ui::controls::expr::ExprUnits;
use craft_ui::controls::{Button, Checkbox, ControlSize, NumberField, Select};
use craft_ui::edit::EditEvent;
use craft_ui::localized::units::{Length, LengthUnit, UnitContext};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::patterns::unit_number_field;
use egui::{Modifiers, Response, Ui, UiBuilder};

craft_ui::words! {
    /// The panel's words (DesignCraft's strings).
    #[non_exhaustive]
    pub struct TablePropertiesLabels {
        rows: "tableProperties.rows" = "Rows";
        columns: "tableProperties.columns" = "Columns";
        row_height: "tableProperties.rowHeight" = "Row Height";
        /// The row height's length field (the select above it is "Row Height").
        row_height_value: "tableProperties.rowHeightValue" = "Height";
        at_least: "tableProperties.atLeast" = "At Least";
        exactly: "tableProperties.exactly" = "Exactly";
        column_width: "tableProperties.columnWidth" = "Column Width";
        distribute_rows: "tableProperties.distributeRows" = "Distribute Rows Evenly";
        distribute_rows_tip: "tableProperties.distributeRowsTip" = "Equal fixed heights for the selected rows; cell text may become overset.";
        cell_text: "tableProperties.cellText" = "Cell Text";
        align_top: "tableProperties.alignTop" = "Top";
        align_center: "tableProperties.alignCenter" = "Center";
        align_bottom: "tableProperties.alignBottom" = "Bottom";
        align_justify: "tableProperties.alignJustify" = "Justify";
        inset_top: "tableProperties.insetTop" = "Top";
        inset_left: "tableProperties.insetLeft" = "Left";
        inset_bottom: "tableProperties.insetBottom" = "Bottom";
        inset_right: "tableProperties.insetRight" = "Right";
        merge: "tableProperties.merge" = "Merge Cells";
        unmerge: "tableProperties.unmerge" = "Unmerge";
        insert_row: "tableProperties.insertRow" = "Insert Row";
        insert_column: "tableProperties.insertColumn" = "Insert Column";
        delete_row: "tableProperties.deleteRow" = "Delete Row";
        delete_column: "tableProperties.deleteColumn" = "Delete Column";
        direction: "tableProperties.direction" = "Direction";
        left_to_right: "tableProperties.leftToRight" = "Left to Right";
        right_to_left: "tableProperties.rightToLeft" = "Right to Left";
        alternating_fills: "tableProperties.alternatingFills" = "Alternating Row Fills";
        repeat_header: "tableProperties.repeatHeader" = "Repeat Header Rows";
        header_rows: "tableProperties.headerRows" = "Header Rows";
        footer_rows: "tableProperties.footerRows" = "Footer Rows";
    }
}

/// How a row's height is set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RowHeight {
    #[allow(missing_docs)]
    #[default]
    AtLeast,
    #[allow(missing_docs)]
    Exactly,
}

/// Cell text's vertical justification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CellAlign {
    #[allow(missing_docs)]
    #[default]
    Top,
    #[allow(missing_docs)]
    Center,
    #[allow(missing_docs)]
    Bottom,
    #[allow(missing_docs)]
    Justify,
}

/// Which way a cell's text runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextDirection {
    #[allow(missing_docs)]
    #[default]
    LeftToRight,
    #[allow(missing_docs)]
    RightToLeft,
}

/// The values at the cell selection; `None` where the selected cells differ (the field shows
/// mixed).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableValues {
    /// Number of rows in the table.
    pub rows: usize,
    /// Number of columns in the table.
    pub columns: usize,
    #[allow(missing_docs)]
    pub row_height_mode: Option<RowHeight>,
    /// Points.
    pub row_height: Option<f64>,
    /// Column width in points.
    pub column_width: Option<f64>,
    #[allow(missing_docs)]
    pub cell_align: Option<CellAlign>,
    /// Top, left, bottom, right, points.
    pub insets: [Option<f64>; 4],
    #[allow(missing_docs)]
    pub direction: TextDirection,
    #[allow(missing_docs)]
    pub alternating_fills: bool,
    #[allow(missing_docs)]
    pub repeat_header: bool,
    /// Number of header rows.
    pub header_rows: usize,
    /// Number of footer rows.
    pub footer_rows: usize,
    /// More than one cell is selected (Merge Cells is enabled).
    pub several_cells: bool,
    /// The selection holds a merged cell (Unmerge is enabled).
    pub merged: bool,
}

/// One change the user made.
#[derive(Clone, Debug, PartialEq)]
pub enum TableChange {
    #[allow(missing_docs)]
    Rows(usize),
    #[allow(missing_docs)]
    Columns(usize),
    #[allow(missing_docs)]
    RowHeightMode(RowHeight),
    /// Row height in points.
    RowHeight(f64),
    /// Column width in points.
    ColumnWidth(f64),
    /// Make the selected rows equally tall.
    DistributeRows,
    #[allow(missing_docs)]
    CellAlign(CellAlign),
    /// Inset `index` (top, left, bottom, right) in points.
    Inset(usize, f64),
    /// Merge the selected cells.
    Merge,
    /// Split the selected merged cell.
    Unmerge,
    #[allow(missing_docs)]
    InsertRow,
    #[allow(missing_docs)]
    InsertColumn,
    #[allow(missing_docs)]
    DeleteRow,
    #[allow(missing_docs)]
    DeleteColumn,
    #[allow(missing_docs)]
    Direction(TextDirection),
    #[allow(missing_docs)]
    AlternatingFills(bool),
    #[allow(missing_docs)]
    RepeatHeader(bool),
    /// Number of header rows.
    HeaderRows(usize),
    /// Number of footer rows.
    FooterRows(usize),
}

/// The panel's result.
#[derive(Debug)]
#[must_use = "the app applies the panel's edits"]
pub struct TablePropertiesOutput {
    #[allow(missing_docs)]
    pub response: Response,
    /// What the user did this pass, in order, as craft-ui edit events: a number field's typing
    /// run or scrub is one gesture (`Begin`, `Update`s, `Commit`, or `Cancel` on Esc) carrying a
    /// [`TableChange`] with the field's value, one undo step; a button, a menu choice or a
    /// checkbox is one `Set`.
    pub edits: Vec<EditEvent<TableChange>>,
}

/// InDesign's Table panel body, built per pass (look InDesign; behaviour Photoshop's panel
/// fields, then DesignCraft).
///
/// - Number fields are craft-ui `NumberField`s (counts) and `unit_number_field`s (lengths in
///   the document's unit), mixed (empty) where [`TableValues`] has `None`; row height mode,
///   cell alignment and direction are `Select`s or icon `Segmented` controls; Merge, Unmerge,
///   Insert and Delete are `Button`s, enabled from `several_cells` and `merged`.
/// - The number fields have no upper bound (counts from 1, or 0 for header and footer rows;
///   lengths from 0), as DesignCraft's table panel; the app refuses what its table can't take.
/// - Keys: Tab moves through the fields in reading order; each control's own keys.
pub struct TableProperties<'a> {
    id_salt: egui::IdSalt,
    values: &'a TableValues,
    unit: craft_ui::localized::units::LengthUnit,
    labels: Option<TablePropertiesLabels>,
}

impl<'a> TableProperties<'a> {
    /// A panel showing `values`, with lengths in `unit`.
    pub fn new(id_salt: impl egui::AsIdSalt, values: &'a TableValues, unit: craft_ui::localized::units::LengthUnit) -> Self {
        TableProperties { id_salt: egui::IdSalt::new(id_salt), values, unit, labels: None }
    }

    /// Sets the translated labels; the default reads them from the interface locale.
    pub fn labels(mut self, labels: TablePropertiesLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // designcraft: crates/ui-egui/src/panels/table.rs:11 `show`.
    /// Paints the panel and returns the response with the changes the user made.
    pub fn show(self, ui: &mut Ui) -> TablePropertiesOutput {
        let labels = self.labels.clone().unwrap_or_else(|| TablePropertiesLabels::of(ui));
        let panel = Panel { numbers: LocaleNumbers::of(ui), unit: self.unit };
        let mut edits = Vec::new();
        let scope = ui.scope_builder(UiBuilder::new().id_salt(self.id_salt), |ui| panel.body(ui, &labels, self.values, &mut edits));
        TablePropertiesOutput { response: scope.response, edits }
    }
}

type Edits = Vec<EditEvent<TableChange>>;

/// What the controls share in one pass.
struct Panel {
    numbers: std::sync::Arc<LocaleNumbers>,
    unit: LengthUnit,
}

impl Panel {
    fn body(&self, ui: &mut Ui, labels: &TablePropertiesLabels, v: &TableValues, edits: &mut Edits) {
        self.count(ui, edits, &labels.rows, "rows", v.rows, 1, TableChange::Rows);
        self.count(ui, edits, &labels.columns, "columns", v.columns, 1, TableChange::Columns);

        let mode_text = |mode: RowHeight| match mode {
            RowHeight::AtLeast => labels.at_least.as_str(),
            RowHeight::Exactly => labels.exactly.as_str(),
        };
        choose(
            ui,
            edits,
            &labels.row_height,
            "row-height-mode",
            v.row_height_mode,
            TableChange::RowHeightMode,
            mode_text,
            &[RowHeight::AtLeast, RowHeight::Exactly],
        );
        self.length(ui, edits, &labels.row_height_value, "row-height", v.row_height, TableChange::RowHeight);
        self.length(ui, edits, &labels.column_width, "column-width", v.column_width, TableChange::ColumnWidth);
        if button(ui, &labels.distribute_rows, true, Some(&labels.distribute_rows_tip)) {
            set(edits, TableChange::DistributeRows);
        }

        GroupBox::new(labels.cell_text.as_str()).show(ui, |ui| {
            let align_text = |align: CellAlign| match align {
                CellAlign::Top => labels.align_top.as_str(),
                CellAlign::Center => labels.align_center.as_str(),
                CellAlign::Bottom => labels.align_bottom.as_str(),
                CellAlign::Justify => labels.align_justify.as_str(),
            };
            let every = [CellAlign::Top, CellAlign::Center, CellAlign::Bottom, CellAlign::Justify];
            choose(ui, edits, &labels.cell_text, "cell-align", v.cell_align, TableChange::CellAlign, align_text, &every);
            let insets = [
                (&labels.inset_top, "inset-top", TableChange::Inset as fn(usize, f64) -> TableChange),
                (&labels.inset_left, "inset-left", TableChange::Inset),
                (&labels.inset_bottom, "inset-bottom", TableChange::Inset),
                (&labels.inset_right, "inset-right", TableChange::Inset),
            ];
            for (index, (caption, salt, change)) in insets.into_iter().enumerate() {
                self.length(ui, edits, caption, salt, v.insets[index], move |points| change(index, points));
            }
        });

        if button(ui, &labels.merge, v.several_cells, None) {
            set(edits, TableChange::Merge);
        }
        if button(ui, &labels.unmerge, v.merged, None) {
            set(edits, TableChange::Unmerge);
        }
        for (label, change) in [
            (&labels.insert_row, TableChange::InsertRow),
            (&labels.insert_column, TableChange::InsertColumn),
            (&labels.delete_row, TableChange::DeleteRow),
            (&labels.delete_column, TableChange::DeleteColumn),
        ] {
            if button(ui, label, true, None) {
                set(edits, change);
            }
        }

        let direction_text = |direction: TextDirection| match direction {
            TextDirection::LeftToRight => labels.left_to_right.as_str(),
            TextDirection::RightToLeft => labels.right_to_left.as_str(),
        };
        let both = [TextDirection::LeftToRight, TextDirection::RightToLeft];
        choose(ui, edits, &labels.direction, "direction", Some(v.direction), TableChange::Direction, direction_text, &both);
        check(ui, edits, &labels.alternating_fills, "alternating-fills", v.alternating_fills, TableChange::AlternatingFills);
        check(ui, edits, &labels.repeat_header, "repeat-header", v.repeat_header, TableChange::RepeatHeader);
        self.count(ui, edits, &labels.header_rows, "header-rows", v.header_rows, 0, TableChange::HeaderRows);
        self.count(ui, edits, &labels.footer_rows, "footer-rows", v.footer_rows, 0, TableChange::FooterRows);
    }

    /// A whole-number field from `least`.
    #[allow(clippy::too_many_arguments, reason = "one control's parts")]
    fn count(&self, ui: &mut Ui, edits: &mut Edits, caption: &str, salt: &'static str, value: usize, least: usize, change: fn(usize) -> TableChange) {
        let edit = Field::new(caption)
            .show(ui, |ui, _| {
                NumberField::new(salt, caption, value as f64, &self.numbers)
                    .range(least as f64..=f64::INFINITY)
                    .decimals(0..=0)
                    .step(1.0)
                    .size(ControlSize::Sm)
                    .show(ui)
            })
            .inner;
        edits.extend(edit.event.map(|event| event.map(|n| change(n.round().max(least as f64) as usize))));
    }

    /// A length field: `points` shown in the document's unit.
    fn length(&self, ui: &mut Ui, edits: &mut Edits, caption: &str, salt: &'static str, points: Option<f64>, change: impl Fn(f64) -> TableChange) {
        let unit = self.unit;
        let units = ExprUnits { unit: Some(unit), suffix: unit.suffix(), context: UnitContext::default() };
        let shown = points.map_or(0.0, |p| in_unit(p, unit));
        let edit = Field::new(caption)
            .show(ui, |ui, _| {
                unit_number_field(salt, caption, shown, &self.numbers, &units).range(0.0..=f64::INFINITY).mixed(points.is_none()).show(ui)
            })
            .inner;
        edits.extend(edit.event.map(|event| event.map(|value| change(in_points(value, unit)))));
    }
}

fn set(edits: &mut Edits, change: TableChange) {
    edits.push(EditEvent::set(change, Modifiers::NONE));
}

/// A select over `choices`; `None` while the selected cells differ (shown as a dash).
#[allow(clippy::too_many_arguments, reason = "one control's parts")]
fn choose<'a, T: Copy + PartialEq + Send + Sync + 'static>(
    ui: &mut Ui,
    edits: &mut Edits,
    caption: &str,
    salt: &'static str,
    value: Option<T>,
    change: fn(T) -> TableChange,
    text: impl Fn(T) -> &'a str,
    choices: &[T],
) {
    let mut shown = value;
    let edit = Field::new(caption)
        .show(ui, |ui, _| {
            let mut select = Select::new(salt, caption, &mut shown).placeholder("—").size(ControlSize::Sm);
            for &choice in choices {
                select = select.option(Some(choice), text(choice));
            }
            select.show(ui)
        })
        .inner;
    if let Some(Some(picked)) = edit.committed() {
        set(edits, change(*picked));
    }
}

fn check(ui: &mut Ui, edits: &mut Edits, caption: &str, salt: &'static str, value: bool, change: fn(bool) -> TableChange) {
    let mut on = value;
    let edit = Checkbox::bool(caption, &mut on).size(ControlSize::Sm).id_salt(salt).show(ui);
    edits.extend(edit.event.map(|event| event.map(change)));
}

fn button(ui: &mut Ui, caption: &str, enabled: bool, tip: Option<&str>) -> bool {
    let response = ui.add(Button::new(caption).size(ControlSize::Sm).enabled(enabled));
    let response = match tip {
        Some(tip) => response.on_hover_text(tip),
        None => response,
    };
    response.clicked()
}

fn in_unit(points: f64, unit: LengthUnit) -> f64 {
    UnitContext::default().convert(Length { value: points, unit: LengthUnit::Points }, unit).unwrap_or(points)
}

fn in_points(value: f64, unit: LengthUnit) -> f64 {
    UnitContext::default().convert(Length { value, unit }, LengthUnit::Points).unwrap_or(value)
}
