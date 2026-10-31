//! Text columns (N3.27): PowerPoint's Columns dialog (Number and Spacing for a text box), Word's
//! More Columns dialog (presets One, Two, Three, Left, Right; number; width and spacing per
//! column; Equal column width; Line between) as one [`TextColumnsDialog`], and Word's Layout ›
//! Columns menu ([`columns_menu`]).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
use craft_ui::localized::units::LengthUnit;
use egui::{Context, Ui};

/// The most columns (PowerPoint's 16).
pub const MAX_COLUMNS: u32 = 16;

/// The largest spacing, points (22 in, PowerPoint).
pub const MAX_SPACING: f32 = 1584.0;

craft_ui::words! {
    /// The dialog's and menu's words (DeckCraft's and WordCraft's strings).
    #[non_exhaustive]
    pub struct TextColumnsLabels {
        title: "columns.title" = "Columns";
        number: "columns.number" = "Number:";
        spacing: "columns.spacing" = "Spacing:";
        number_range: "columns.numberRange" = "Number must be a whole number from 1 to 16";
        spacing_range: "columns.spacingRange" = "Spacing must be from 0 to 22 inches";
        presets: "columns.presets" = "Presets";
        one: "columns.one" = "One";
        two: "columns.two" = "Two";
        three: "columns.three" = "Three";
        left: "columns.left" = "Left";
        right: "columns.right" = "Right";
        width_and_spacing: "columns.widthAndSpacing" = "Width and spacing";
        column: "columns.column" = "Col #:";
        width: "columns.width" = "Width:";
        equal_width: "columns.equalWidth" = "Equal column width";
        line_between: "columns.lineBetween" = "Line between";
        more_columns: "columns.more" = "More Columns…";
    }
}

/// Column settings; `None` where the selected text boxes or sections differ (a field left as
/// shown is left out of the result, so each keeps its own value).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextColumns {
    pub count: Option<u32>,
    /// The space between columns, points (all of them, with equal width).
    pub spacing: Option<f32>,
    /// Word: each column's width and the spacing after it, points; empty with equal width.
    pub widths: Vec<(f32, f32)>,
    pub equal_width: Option<bool>,
    pub line_between: Option<bool>,
}

/// Word's Layout › Columns presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColumnsPreset {
    One,
    Two,
    Three,
    /// Two columns, the left one narrower.
    Left,
    /// Two columns, the right one narrower.
    Right,
}

/// The Columns dialog: only what changed comes back.
///
/// - [`word`](Self::word) off (PowerPoint): Number (1–[`MAX_COLUMNS`]) and Spacing (0–22 in);
///   values out of range keep the dialog open with "Number must be a whole number from 1 to 16" /
///   "Spacing must be from 0 to 22 inches" in red under the fields.
/// - On (Word): the presets row, Number, the width and spacing grid (one row per column, Col #
///   1–n; width fields disabled after the first with Equal column width), Equal column width,
///   Line between, and the page's text width for the grid's sums.
pub struct TextColumnsDialog {
    shown: TextColumns,
    edited: TextColumns,
    word: Option<f32>,
    unit: LengthUnit,
    error: Option<String>,
    labels: Option<TextColumnsLabels>,
}

impl TextColumnsDialog {
    pub fn new(columns: TextColumns) -> Self {
        TextColumnsDialog { shown: columns.clone(), edited: columns, word: None, unit: LengthUnit::Inches, error: None, labels: None }
    }

    /// Word's dialog, over a text width of `width` points.
    pub fn word(mut self, width: f32) -> Self {
        self.word = Some(width);
        self
    }

    pub fn unit(mut self, unit: LengthUnit) -> Self {
        self.unit = unit;
        self
    }

    pub fn labels(mut self, labels: TextColumnsLabels) -> Self {
        self.labels = Some(labels);
        self
    }
}

impl StackDialog for TextColumnsDialog {
    /// The fields that changed; `None` when none did (OK applies nothing).
    type Result = Option<TextColumns>;

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("text-columns")
    }

    // deckcraft: crates/ui-egui/src/dialogs/columns.rs:32 `columns_dialog`, :6 `columns_value`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<Option<TextColumns>>> {
        todo!()
    }
}

/// A choice from the Columns menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColumnsChoice {
    Preset(ColumnsPreset),
    /// More Columns…: open the dialog.
    More,
}

/// Layout › Columns: One, Two, Three, Left, Right with the matching one checked, a separator and
/// More Columns…. Returns the choice.
// wordcraft: crates/ui-egui/src/ribbon/layout.rs:36 `columns_items`.
pub fn columns_menu(ui: &mut Ui, current: Option<ColumnsPreset>, labels: &TextColumnsLabels) -> Option<ColumnsChoice> {
    todo!()
}
