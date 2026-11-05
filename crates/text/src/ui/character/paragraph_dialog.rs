//! [`ParagraphDialog`] (N3.15): the Paragraph dialog of PowerPoint (Home › Paragraph launcher)
//! and Word: alignment, indentation (before text, and a first-line or hanging indent by an
//! amount), spacing before and after, line spacing, with a live sample of the result and a Tabs…
//! button. OK applies everything as one change.
//!
//! The Paragraph controls ([`ParagraphControls`](super::ParagraphControls)) are Photoshop's
//! panel; this is the office apps' dialog over the same paragraph.
//!
//! ```ignore
//! dialogs.open(ParagraphDialog::new(format_at_caret).unit(unit));
//! if let Some(DialogResult::Ok(answer)) = dialogs.take::<ParagraphDialog>() {
//!     if answer.open_tabs { dialogs.open(TabStopsDialog::new(..)) } else { doc.apply_paragraph(answer.format) }
//! }
//! ```

use std::cell::Cell;

use craft_ui::composites::{Field, FieldGroup, GroupBox};
use craft_ui::controls::expr::ExprUnits;
use craft_ui::controls::{NumberField, Select};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, InitialFocus, Outcome, StackDialog, Width};
use craft_ui::edit::{EditEvent, GestureDraft};
use craft_ui::localized::units::{Length, LengthUnit, UnitContext};
use craft_ui::localized::{Labels as _, LocaleNumbers, UnitKind, Units, fmt};
use craft_ui::patterns::unit_number_field;
use craft_ui::primitives::{Content, Surface, SurfaceKind};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::{Context, Rect, Sense, Ui, pos2, vec2};

craft_ui::words! {
    /// The dialog's words (DeckCraft's strings).
    #[non_exhaustive]
    pub struct ParagraphDialogLabels {
        title: "paragraphDialog.title" = "Paragraph";
        general: "paragraphDialog.general" = "General";
        alignment: "paragraphDialog.alignment" = "Alignment";
        left: "paragraphDialog.left" = "Left";
        center: "paragraphDialog.center" = "Center";
        right: "paragraphDialog.right" = "Right";
        justify: "paragraphDialog.justify" = "Justify";
        justify_low: "paragraphDialog.justifyLow" = "Justify low";
        distributed: "paragraphDialog.distributed" = "Distributed";
        indentation: "paragraphDialog.indentation" = "Indentation";
        before_text: "paragraphDialog.beforeText" = "Before text";
        after_text: "paragraphDialog.afterText" = "After text";
        special: "paragraphDialog.special" = "Special";
        special_none: "paragraphDialog.specialNone" = "None";
        first_line: "paragraphDialog.firstLine" = "First line";
        hanging: "paragraphDialog.hanging" = "Hanging";
        by: "paragraphDialog.by" = "By";
        spacing: "paragraphDialog.spacing" = "Spacing";
        spacing_before: "paragraphDialog.spaceBefore" = "Spacing before";
        spacing_after: "paragraphDialog.spaceAfter" = "Spacing after";
        line_spacing: "paragraphDialog.lineSpacing" = "Line spacing";
        single: "paragraphDialog.single" = "Single";
        one_and_half: "paragraphDialog.oneAndHalf" = "1.5 lines";
        double: "paragraphDialog.double" = "Double";
        multiple: "paragraphDialog.multiple" = "Multiple";
        exactly: "paragraphDialog.exactly" = "Exactly";
        at: "paragraphDialog.at" = "At";
        preview: "paragraphDialog.preview" = "Preview";
        tabs: "paragraphDialog.tabs" = "Tabs…";
        at_least: "paragraphDialog.atLeast" = "At least";
        sample: "paragraphDialog.sample" = "Sample Text";
        out_of_range: "paragraphDialog.outOfRange" = "{field} must be from {min} to {max}";
    }
}

/// PowerPoint's alignments (Word has the first four).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ParagraphAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
    JustifyLow,
    Distributed,
}

/// The first line against the others.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpecialIndent {
    #[default]
    None,
    /// The first line starts `by` further in.
    FirstLine,
    /// The other lines start `by` further in (the first line hangs).
    Hanging,
}

/// What a paragraph's spacing before or after counts, as DeckCraft keeps it (`spaceBeforeKind`
/// and `spaceAfterKind`, `pts` or `pct`; engine `model/src/text.rs:186` `Spacing`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpacingKind {
    /// A distance in points.
    #[default]
    Points,
    /// A multiple of the line (DeckCraft's `pct`: 1 is a single line).
    Lines,
}

/// Line spacing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineSpacing {
    /// A multiple of single spacing: Single is 1, 1.5 lines 1.5, Double 2, Multiple any other.
    Multiple(f32),
    /// A fixed distance, points.
    Exactly(f32),
    /// At least this distance, points (Word's At least).
    AtLeast(f32),
}

impl Default for LineSpacing {
    fn default() -> Self {
        LineSpacing::Multiple(1.0)
    }
}

/// What the dialog edits; `None` where the selected paragraphs differ (the field starts empty
/// and a field left empty is left as it was).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParagraphFormat {
    pub align: Option<ParagraphAlign>,
    /// Points.
    pub before_text: Option<f32>,
    /// Word's right indent ("After text"). The outer `None` hides the field (PowerPoint has
    /// none); `Some(None)` shows it empty, for paragraphs that differ.
    pub after_text: Option<Option<f32>>,
    pub special: Option<SpecialIndent>,
    pub by: Option<f32>,
    /// Points, or lines when [`space_before_kind`](Self::space_before_kind) says so.
    pub space_before: Option<f32>,
    /// Points, or lines when [`space_after_kind`](Self::space_after_kind) says so.
    pub space_after: Option<f32>,
    /// What `space_before` counts. The dialog keeps the kind while the value is the one it
    /// opened with and answers [`Points`](SpacingKind::Points) once the value changes (DeckCraft
    /// `dialogs/paragraph.rs:33`: the kind is kept while the text equals its seed).
    pub space_before_kind: SpacingKind,
    /// What `space_after` counts, kept as `space_before_kind` is.
    pub space_after_kind: SpacingKind,
    pub line_spacing: Option<LineSpacing>,
}

/// OK's answer: the format, and whether Tabs… was pressed (the app applies the format, then opens
/// the Tabs dialog, and after it the Paragraph dialog shows what was applied).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParagraphAnswer {
    pub format: ParagraphFormat,
    pub open_tabs: bool,
}

/// The Paragraph dialog.
///
/// - Alignment and Special (None, First line, Hanging) and Line spacing (Single, 1.5 lines,
///   Double, Multiple, Exactly; Word adds At least) are craft-ui `Select`s; Before text, After
///   text and By are `unit_number_field`s in the dialog's unit, Spacing before and after
///   `unit_number_field`s in points (PowerPoint and Word both show points there); At is a
///   `NumberField` (lines or points, by the line spacing).
///   Choosing Exactly with an At of 9.99 or less sets 18 pt; choosing a multiple with an At above
///   9.99 sets 1 (DeckCraft).
/// - Lengths in the dialog's unit (inches by default, as PowerPoint). Typed numbers are clamped
///   by their field; text that is not a number is not taken (the field marks it, and OK waits until it is fixed or
///   left). Esc in a field's gesture restores the value the gesture began from. A format the
///   app hands over with a value out of range shows the labels' range message under its field
///   (the field error style) and OK waits until it is fixed.
/// - Preview (Word's; DeckCraft has none): three sample paragraphs, the outer two grey and the
///   middle one in the text colour, laid out with the values as they are now (Word's Preview
///   box), redrawn as fields change. A typographic point is a point of the screen there.
/// - Keys: Enter is OK, Esc Cancel; first focus on Alignment.
pub struct ParagraphDialog {
    format: ParagraphFormat,
    unit: LengthUnit,
    word: bool,
    labels: Option<ParagraphDialogLabels>,
    state: DialogState,
    /// The format the dialog opened with: a spacing that returns to its value takes its kind back.
    opened: ParagraphFormat,
    /// One draft per field, indexed by [`Row`]: Esc in a gesture restores the value it began from.
    drafts: [GestureDraft<Option<f32>>; ROWS],
}

impl ParagraphDialog {
    pub fn new(format: ParagraphFormat) -> Self {
        ParagraphDialog {
            opened: format.clone(),
            format,
            unit: LengthUnit::Inches,
            word: false,
            labels: None,
            state: DialogState::new(),
            drafts: Default::default(),
        }
    }

    pub fn unit(mut self, unit: LengthUnit) -> Self {
        self.unit = unit;
        self
    }

    /// Word's form: four alignments, and At least among the line spacings.
    pub fn word(mut self, word: bool) -> Self {
        self.word = word;
        self
    }

    pub fn labels(mut self, labels: ParagraphDialogLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The format as edited so far.
    pub fn format(&self) -> &ParagraphFormat {
        &self.format
    }
}

/// The largest length Word takes, points.
const MAX_POINTS: f32 = 1584.0;
/// PowerPoint's switch between a multiple and a distance in At.
const MULTIPLE_LIMIT: f32 = 9.99;
/// The distance Exactly and At least start at, points.
const DEFAULT_DISTANCE: f32 = 18.0;
/// The indent a first-line or hanging indent starts at, points (Word's half inch).
const DEFAULT_BY: f32 = 36.0;
/// The most a multiple of single spacing can be (Word).
const MAX_MULTIPLE: f32 = 132.0;
/// The least a multiple of single spacing can be (Word).
const MIN_MULTIPLE: f32 = 0.06;
/// The least an exact distance can be, points (Word).
const MIN_EXACT: f32 = 0.7;
/// The most spacing before or after of kind lines (DeckCraft `write_spacing` clamps `pct` to 10).
const MAX_SPACING_LINES: f32 = 10.0;
/// Decimals the At field shows.
const AT_DECIMALS: usize = 2;
/// Lines in each sample paragraph of the preview.
const SAMPLE_LINES: usize = 3;

/// The entries of Line spacing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinePreset {
    Single,
    OneAndHalf,
    Double,
    Multiple,
    AtLeast,
    Exactly,
}

impl LinePreset {
    fn of(spacing: LineSpacing) -> Self {
        match spacing {
            LineSpacing::Multiple(1.0) => LinePreset::Single,
            LineSpacing::Multiple(1.5) => LinePreset::OneAndHalf,
            LineSpacing::Multiple(2.0) => LinePreset::Double,
            LineSpacing::Multiple(_) => LinePreset::Multiple,
            LineSpacing::AtLeast(_) => LinePreset::AtLeast,
            LineSpacing::Exactly(_) => LinePreset::Exactly,
        }
    }

    /// The spacing choosing this entry gives when `current` was set.
    fn choose(self, current: Option<LineSpacing>) -> LineSpacing {
        let at = match current {
            Some(LineSpacing::Multiple(x) | LineSpacing::Exactly(x) | LineSpacing::AtLeast(x)) => Some(x),
            None => None,
        };
        let distance = at.filter(|x| *x > MULTIPLE_LIMIT).unwrap_or(DEFAULT_DISTANCE);
        match self {
            LinePreset::Single => LineSpacing::Multiple(1.0),
            LinePreset::OneAndHalf => LineSpacing::Multiple(1.5),
            LinePreset::Double => LineSpacing::Multiple(2.0),
            LinePreset::Multiple => LineSpacing::Multiple(at.filter(|x| *x <= MULTIPLE_LIMIT).unwrap_or(1.0)),
            LinePreset::AtLeast => LineSpacing::AtLeast(distance),
            LinePreset::Exactly => LineSpacing::Exactly(distance),
        }
    }
}

/// The values a spacing before or after of `kind` can take: points up to Word's largest length,
/// lines up to the most DeckCraft keeps (`engine/src/cmd/format/paragraph.rs:246`).
fn spacing_bounds(kind: SpacingKind) -> (f32, f32) {
    match kind {
        SpacingKind::Points => (0.0, MAX_POINTS),
        SpacingKind::Lines => (0.0, MAX_SPACING_LINES),
    }
}

/// The number of [`Row`]s.
const ROWS: usize = 6;

/// The fields that can be out of range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Row {
    BeforeText,
    AfterText,
    By,
    SpaceBefore,
    SpaceAfter,
    At,
}

fn in_unit(points: f32, unit: LengthUnit) -> f64 {
    let context = UnitContext::default();
    context.convert(Length { value: f64::from(points), unit: LengthUnit::Points }, unit).unwrap_or(f64::from(points))
}

fn in_points(value: f64, unit: LengthUnit) -> f32 {
    let context = UnitContext::default();
    context.convert(Length { value, unit }, LengthUnit::Points).unwrap_or(value) as f32
}

/// The bounds of At for a line spacing, in lines or points.
fn at_bounds(spacing: LineSpacing) -> (f32, f32) {
    match spacing {
        LineSpacing::Multiple(_) => (MIN_MULTIPLE, MAX_MULTIPLE),
        LineSpacing::Exactly(_) => (MIN_EXACT, MAX_POINTS),
        LineSpacing::AtLeast(_) => (0.0, MAX_POINTS),
    }
}

fn at_value(spacing: LineSpacing) -> f32 {
    match spacing {
        LineSpacing::Multiple(x) | LineSpacing::Exactly(x) | LineSpacing::AtLeast(x) => x,
    }
}

fn with_at(spacing: LineSpacing, at: f32) -> LineSpacing {
    match spacing {
        LineSpacing::Multiple(_) => LineSpacing::Multiple(at),
        LineSpacing::Exactly(_) => LineSpacing::Exactly(at),
        LineSpacing::AtLeast(_) => LineSpacing::AtLeast(at),
    }
}

/// What a length field reported this pass.
pub(super) struct LengthEdit {
    pub(super) event: Option<EditEvent<Option<f32>>>,
    /// The field's text can't become a value.
    pub(super) rejected: bool,
}

impl ParagraphDialog {
    /// The rows whose value is outside its bounds.
    fn out_of_range(&self) -> Vec<Row> {
        let f = &self.format;
        let outside = |value: Option<f32>, (lo, hi): (f32, f32)| value.is_some_and(|v| !(lo..=hi).contains(&v));
        let mut rows = Vec::new();
        let mut check = |row, bad| {
            if bad {
                rows.push(row);
            }
        };
        check(Row::BeforeText, outside(f.before_text, (-MAX_POINTS, MAX_POINTS)));
        check(Row::AfterText, outside(f.after_text.flatten(), (-MAX_POINTS, MAX_POINTS)));
        check(Row::By, outside(f.by, (0.0, MAX_POINTS)));
        check(Row::SpaceBefore, outside(f.space_before, spacing_bounds(f.space_before_kind)));
        check(Row::SpaceAfter, outside(f.space_after, spacing_bounds(f.space_after_kind)));
        check(Row::At, f.line_spacing.is_some_and(|l| outside(Some(at_value(l)), at_bounds(l))));
        rows
    }

    fn message(
        &self,
        labels: &ParagraphDialogLabels,
        numbers: &LocaleNumbers,
        units: &Units,
        caption: &str,
        bounds: (f32, f32),
        unit: Option<LengthUnit>,
    ) -> String {
        let show = |points: f32| match unit {
            Some(unit) => units.length(unit, in_unit(points, unit)),
            None => numbers.format(f64::from(points), 0..=AT_DECIMALS),
        };
        fmt(&labels.out_of_range, &[("field", caption), ("min", &show(bounds.0)), ("max", &show(bounds.1))])
    }

    /// One length field of the form: `value` in points, shown in `unit`; without a unit the
    /// value is a number of lines, shown as it is.
    #[allow(clippy::too_many_arguments, reason = "one field of the form; the pieces are its parts")]
    fn length_field(
        &self,
        ui: &mut Ui,
        labels: &ParagraphDialogLabels,
        numbers: &LocaleNumbers,
        salt: &'static str,
        caption: &str,
        value: Option<f32>,
        unit: Option<LengthUnit>,
        bounds: (f32, f32),
        enabled: bool,
        row: Row,
    ) -> LengthEdit {
        let invalid = self.out_of_range().contains(&row);
        let error = invalid.then(|| self.message(labels, numbers, &Units::of(ui), caption, bounds, unit));
        let rejected = Cell::new(false);
        let edit = Field::new(caption)
            .error(error)
            .show(ui, |ui, invalid| {
                let output = match unit {
                    Some(unit) => {
                        let shown = value.map_or(f64::NAN, |v| in_unit(v, unit));
                        let units = ExprUnits { unit: Some(unit), suffix: unit.suffix(), context: UnitContext::default() };
                        unit_number_field(("paragraph", salt), caption, shown, numbers, &units)
                            .range(in_unit(bounds.0, unit)..=in_unit(bounds.1, unit))
                            .enabled(enabled)
                            .invalid(invalid)
                            .show_optional_full(ui)
                    }
                    // No range: a number typed here replaces the lines with points, whatever it is.
                    None => NumberField::new(("paragraph", salt), caption, value.map_or(f64::NAN, f64::from), numbers)
                        .decimals(0..=AT_DECIMALS)
                        .enabled(enabled)
                        .invalid(invalid)
                        .show_optional_full(ui),
                };
                rejected.set(output.rejected);
                output.edit
            })
            .inner;
        // Lines are the value as typed; points go through the unit.
        let to_value = |v: f64| unit.map_or(v as f32, |unit| in_points(v, unit));
        LengthEdit { event: edit.event.map(|event| event.map(|v| v.map(to_value))), rejected: rejected.get() }
    }

    /// Applies a field's event to its row's value through the row's draft, and refuses OK while
    /// the field's text is rejected.
    pub(super) fn edit(&mut self, row: Row, edit: LengthEdit, valid: &Cell<bool>) {
        if edit.rejected {
            valid.set(false);
        }
        let Some(event) = edit.event else { return };

        let mut value = self.value(row);
        self.drafts[row as usize].apply(&mut value, event);
        self.set_value(row, value);
    }

    fn value(&self, row: Row) -> Option<f32> {
        let f = &self.format;
        match row {
            Row::BeforeText => f.before_text,
            Row::AfterText => f.after_text.flatten(),
            Row::By => f.by,
            Row::SpaceBefore => f.space_before,
            Row::SpaceAfter => f.space_after,
            Row::At => f.line_spacing.map(at_value),
        }
    }

    fn set_value(&mut self, row: Row, value: Option<f32>) {
        let f = &mut self.format;
        match row {
            Row::BeforeText => f.before_text = value,
            Row::AfterText => f.after_text = Some(value),
            Row::By => f.by = value,
            Row::SpaceBefore => {
                f.space_before = value;
                f.space_before_kind = if value == self.opened.space_before { self.opened.space_before_kind } else { SpacingKind::Points };
            }
            Row::SpaceAfter => {
                f.space_after = value;
                f.space_after_kind = if value == self.opened.space_after { self.opened.space_after_kind } else { SpacingKind::Points };
            }
            // An emptied At keeps the spacing's number.
            Row::At => f.line_spacing = f.line_spacing.map(|l| with_at(l, value.unwrap_or_else(|| at_value(l)))),
        }
    }

    fn body(&mut self, ui: &mut Ui, labels: &ParagraphDialogLabels, numbers: &LocaleNumbers, valid: &Cell<bool>) {
        valid.set(true);
        let unit = self.unit;
        let gap = ui.spacing().item_spacing.y;
        FieldGroup::new("paragraph-fields").show(ui, |ui| {
            GroupBox::new(labels.general.as_str()).show(ui, |ui| self.alignment(ui, labels));
            ui.add_space(gap);
            GroupBox::new(labels.indentation.as_str()).show(ui, |ui| self.indentation(ui, labels, numbers, unit, valid));
            ui.add_space(gap);
            GroupBox::new(labels.spacing.as_str()).show(ui, |ui| self.spacing(ui, labels, numbers, valid));
            ui.add_space(gap);
            GroupBox::new(labels.preview.as_str()).show(ui, |ui| self.preview(ui, labels));
        });
        if !self.out_of_range().is_empty() {
            valid.set(false);
        }
    }

    fn alignment(&mut self, ui: &mut Ui, labels: &ParagraphDialogLabels) {
        let mut align = self.format.align;
        Field::new(labels.alignment.as_str()).show(ui, |ui, _| {
            let mut select = Select::new(("paragraph", "align"), labels.alignment.as_str(), &mut align).placeholder("");
            let every = [
                (ParagraphAlign::Left, &labels.left),
                (ParagraphAlign::Center, &labels.center),
                (ParagraphAlign::Right, &labels.right),
                (ParagraphAlign::Justify, &labels.justify),
                (ParagraphAlign::JustifyLow, &labels.justify_low),
                (ParagraphAlign::Distributed, &labels.distributed),
            ];
            for (value, text) in every {
                let word_only = matches!(value, ParagraphAlign::JustifyLow | ParagraphAlign::Distributed);
                if !self.word || !word_only || self.format.align == Some(value) {
                    select = select.option(Some(value), text.as_str());
                }
            }
            select.show(ui)
        });
        self.format.align = align;
    }

    fn indentation(&mut self, ui: &mut Ui, labels: &ParagraphDialogLabels, numbers: &LocaleNumbers, unit: LengthUnit, valid: &Cell<bool>) {
        let indent = (-MAX_POINTS, MAX_POINTS);
        let before = self.format.before_text;
        let edit = self.length_field(ui, labels, numbers, "before", &labels.before_text, before, Some(unit), indent, true, Row::BeforeText);
        self.edit(Row::BeforeText, edit, valid);
        if let Some(after) = self.format.after_text {
            let edit = self.length_field(ui, labels, numbers, "after", &labels.after_text, after, Some(unit), indent, true, Row::AfterText);
            self.edit(Row::AfterText, edit, valid);
        }
        let mut special = self.format.special;
        Field::new(labels.special.as_str()).show(ui, |ui, _| {
            Select::new(("paragraph", "special"), labels.special.as_str(), &mut special)
                .placeholder("")
                .option(Some(SpecialIndent::None), labels.special_none.as_str())
                .option(Some(SpecialIndent::FirstLine), labels.first_line.as_str())
                .option(Some(SpecialIndent::Hanging), labels.hanging.as_str())
                .show(ui)
        });
        if special != self.format.special {
            self.format.special = special;
            if matches!(special, Some(SpecialIndent::FirstLine | SpecialIndent::Hanging)) && self.format.by.is_none_or(|by| by == 0.0) {
                self.format.by = Some(DEFAULT_BY);
            }
        }
        let by_enabled = matches!(self.format.special, Some(SpecialIndent::FirstLine | SpecialIndent::Hanging));
        let by = self.format.by;
        let edit = self.length_field(ui, labels, numbers, "by", &labels.by, by, Some(unit), (0.0, MAX_POINTS), by_enabled, Row::By);
        self.edit(Row::By, edit, valid);
    }

    fn spacing(&mut self, ui: &mut Ui, labels: &ParagraphDialogLabels, numbers: &LocaleNumbers, valid: &Cell<bool>) {
        let (before, after) = (self.format.space_before, self.format.space_after);
        let (kind_before, kind_after) = (self.format.space_before_kind, self.format.space_after_kind);
        let unit_of = |kind| (kind == SpacingKind::Points).then_some(LengthUnit::Points);
        let edit = self.length_field(
            ui,
            labels,
            numbers,
            "space-before",
            &labels.spacing_before,
            before,
            unit_of(kind_before),
            spacing_bounds(kind_before),
            true,
            Row::SpaceBefore,
        );
        self.edit(Row::SpaceBefore, edit, valid);
        let edit = self.length_field(
            ui,
            labels,
            numbers,
            "space-after",
            &labels.spacing_after,
            after,
            unit_of(kind_after),
            spacing_bounds(kind_after),
            true,
            Row::SpaceAfter,
        );
        self.edit(Row::SpaceAfter, edit, valid);

        let mut preset = self.format.line_spacing.map(LinePreset::of);
        let at_least = self.word || preset == Some(LinePreset::AtLeast);
        Field::new(labels.line_spacing.as_str()).show(ui, |ui, _| {
            let mut select = Select::new(("paragraph", "line"), labels.line_spacing.as_str(), &mut preset)
                .placeholder("")
                .option(Some(LinePreset::Single), labels.single.as_str())
                .option(Some(LinePreset::OneAndHalf), labels.one_and_half.as_str())
                .option(Some(LinePreset::Double), labels.double.as_str())
                .option(Some(LinePreset::Multiple), labels.multiple.as_str());
            if at_least {
                select = select.option(Some(LinePreset::AtLeast), labels.at_least.as_str());
            }
            select.option(Some(LinePreset::Exactly), labels.exactly.as_str()).show(ui)
        });
        if let Some(chosen) = preset.filter(|p| Some(*p) != self.format.line_spacing.map(LinePreset::of)) {
            self.format.line_spacing = Some(chosen.choose(self.format.line_spacing));
        }

        let current = self.format.line_spacing;
        let (lo, hi) = current.map_or((0.0, MAX_POINTS), at_bounds);
        let invalid = self.out_of_range().contains(&Row::At);
        let in_lines = matches!(current, Some(LineSpacing::Multiple(_)));
        let error = invalid.then(|| self.message(labels, numbers, &Units::of(ui), &labels.at, (lo, hi), (!in_lines).then_some(LengthUnit::Points)));
        let shown = current.map_or(f64::NAN, |l| f64::from(at_value(l)));
        let rejected = Cell::new(false);
        let edit = Field::new(labels.at.as_str())
            .error(error)
            .show(ui, |ui, invalid| {
                let mut field = NumberField::new(("paragraph", "at"), labels.at.as_str(), shown, numbers)
                    .range(f64::from(lo)..=f64::from(hi))
                    .decimals(0..=AT_DECIMALS);
                if !in_lines {
                    field = field.unit_kind(UnitKind::Points);
                }
                let output = field.enabled(current.is_some()).invalid(invalid).show_optional_full(ui);
                rejected.set(output.rejected);
                output.edit
            })
            .inner;
        let event = edit.event.map(|event| event.map(|v| v.map(|v| v as f32)));
        self.edit(Row::At, LengthEdit { event, rejected: rejected.get() }, valid);
    }

    fn preview(&self, ui: &mut Ui, labels: &ParagraphDialogLabels) {
        let tokens = Tokens::of(ui);
        let font = egui::TextStyle::Body.resolve(ui.style());
        let sample = labels.sample.as_str();
        let (subject, context) = (Content::Normal.color(&tokens).to_egui(), Content::Muted.color(&tokens).to_egui());
        Surface::new(SurfaceKind::Sunken).show(ui, |ui| {
            let width = ui.available_width();
            let one = ui.painter().layout_no_wrap(sample.to_owned(), font.clone(), subject);
            let plan = sample_plan(&self.format, width, one.size().y);
            let (rect, response) = ui.allocate_exact_size(vec2(width, plan.height), Sense::hover());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, labels.preview.as_str()));
            let painter = ui.painter().with_clip_rect(rect);
            for row in &plan.rows {
                let color = if row.subject { subject } else { context };
                let slot = row.rect.translate(rect.min.to_vec2());
                let text = if row.full { format!("{sample} ").repeat((slot.width() / one.size().x).ceil() as usize + 1) } else { sample.to_owned() };
                let galley = painter.layout_no_wrap(text, font.clone(), color);
                let x = match row.last_align {
                    ParagraphAlign::Center if !row.full => slot.center().x - galley.size().x / 2.0,
                    ParagraphAlign::Right if !row.full => slot.right() - galley.size().x,
                    _ => slot.left(),
                };
                painter.with_clip_rect(slot.intersect(rect)).galley(pos2(x, slot.top()), galley, color);
            }
        });
    }
}

/// One line of the preview.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SampleRow {
    /// The line's room, relative to the preview's corner.
    pub(super) rect: Rect,
    /// Whether the line belongs to the paragraph being edited.
    pub(super) subject: bool,
    /// Whether the line is full (not the last of its paragraph, or a distributed last line).
    pub(super) full: bool,
    /// Where a short line sits.
    pub(super) last_align: ParagraphAlign,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SamplePlan {
    pub(super) rows: Vec<SampleRow>,
    pub(super) height: f32,
}

/// The preview's lines for `format` in a box `width` wide, `row_height` being one line of text:
/// a paragraph before and after in the default format, and the edited one between. Fields that
/// differ among the paragraphs (`None`) take the default.
pub(super) fn sample_plan(format: &ParagraphFormat, width: f32, row_height: f32) -> SamplePlan {
    let plain = ParagraphFormat::default();
    let mut rows = Vec::new();
    let mut y = 0.0_f32;
    for (index, f) in [&plain, format, &plain].into_iter().enumerate() {
        let align = f.align.unwrap_or_default();
        let left = f.before_text.unwrap_or(0.0);
        let right = f.after_text.flatten().unwrap_or(0.0);
        let by = f.by.unwrap_or(0.0);
        let (first, rest) = match f.special.unwrap_or_default() {
            SpecialIndent::None => (0.0, 0.0),
            SpecialIndent::FirstLine => (by, 0.0),
            SpecialIndent::Hanging => (0.0, by),
        };
        let pitch = match f.line_spacing.unwrap_or_default() {
            LineSpacing::Multiple(m) => row_height * m,
            LineSpacing::Exactly(e) => e,
            LineSpacing::AtLeast(a) => row_height.max(a),
        };
        y += f.space_before.unwrap_or(0.0);
        for line in 0..SAMPLE_LINES {
            let start = (left + if line == 0 { first } else { rest }).clamp(0.0, width);
            let end = (width - right).clamp(start, width);
            let last = line + 1 == SAMPLE_LINES;
            rows.push(SampleRow {
                rect: Rect::from_min_max(pos2(start, y), pos2(end, y + pitch)),
                subject: index == 1,
                full: !last || align == ParagraphAlign::Distributed,
                last_align: align,
            });
            y += pitch;
        }
        y += f.space_after.unwrap_or(0.0);
    }
    SamplePlan { rows, height: y }
}

impl StackDialog for ParagraphDialog {
    type Result = ParagraphAnswer;

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("paragraph")
    }

    // deckcraft: crates/ui-egui/src/dialogs/paragraph.rs:57 `paragraph_dialog`, :3
    // `seed_paragraph_fields`, :17 `paragraph_return_params`, :43 `line_preset_label`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<ParagraphAnswer>> {
        let labels = self.labels.clone().unwrap_or_else(|| ParagraphDialogLabels::of_ctx(ctx));
        let numbers = LocaleNumbers::of_ctx(ctx);
        let valid = Cell::new(self.out_of_range().is_empty());
        let footer = Footer::side().extra(labels.tabs.as_str()).ok_enabled_with(|| valid.get());
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id(), labels.title.as_str())
            .stack(stack)
            .width(Width::Standard)
            .initial_focus(InitialFocus::FirstField)
            .footer(footer)
            .show(ctx, &mut state, &mut (), |ui, ()| self.body(ui, &labels, &numbers, &valid));
        self.state = state;
        let answer = |open_tabs| ParagraphAnswer { format: self.format.clone(), open_tabs };
        if shown.extra.is_some() && valid.get() {
            return Some(DialogResult::Ok(answer(true)));
        }
        match shown.outcome {
            Some(Outcome::Ok) if valid.get() => Some(DialogResult::Ok(answer(false))),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}
