//! Text columns (N3.27): PowerPoint's Columns dialog (Number and Spacing for a text box), Word's
//! More Columns dialog (presets One, Two, Three, Left, Right; number; width and spacing per
//! column; Equal column width; Line between) as one [`TextColumnsDialog`], and Word's Layout ›
//! Columns menu ([`columns_menu`]).

use std::cell::Cell;

use craft_ui::composites::{Field, FieldControl as _, FieldGroup, FormGrid, GroupBox};
use craft_ui::controls::expr::ExprUnits;
use craft_ui::controls::{Button, CheckState, Checkbox, ControlSize, NumberField, Select};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, InitialFocus, Outcome, StackDialog, Width};
use craft_ui::edit::{EditEvent, GestureDraft};
use craft_ui::localized::units::{Length, LengthUnit, UnitContext};
use craft_ui::localized::{Labels as _, LocaleNumbers, Units, fmt};
use craft_ui::menu::{MenuEntry, MenuItem};
use craft_ui::patterns::unit_number_field;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::{Context, Label, Rect, Sense, Ui, pos2, vec2};

/// The most columns PowerPoint takes (DeckCraft `crates/ui-egui/src/dialogs/columns.rs:10`); the
/// dialog's limit without [`word`](TextColumnsDialog::word) and the default for a host that
/// sets none.
pub const MAX_COLUMNS: u32 = 16;

/// The most columns Word takes (WordCraft `dialogs_insert/columns.rs:96` `clamp(1, 12)`); the
/// dialog's limit with [`word`](TextColumnsDialog::word).
pub const MAX_COLUMNS_WORD: u32 = 12;

/// The narrowest column of Word's dialog, points (WordCraft `dialogs_insert/columns.rs:81`
/// `MIN_COL_PT`: the engine refuses narrower ones).
pub const MIN_COLUMN_WIDTH: f32 = 18.0;

/// The largest spacing, points (22 in, PowerPoint).
pub const MAX_SPACING: f32 = 1584.0;

/// The spacing between columns Word gives a preset and a new section's columns, points: Word's
/// 0.5 in (wordcraft: `dialogs/dialogs_insert/columns.rs:182` `preset`, `doc/src/section.rs:617`).
const DEFAULT_SPACING: f32 = 0.5 * 72.0;
/// The Left and Right presets' narrow column is this share of the text width less one spacing,
/// the wide one the rest (wordcraft: `columns.rs:187-188`).
const NARROW_SHARE: f32 = 1.0 / 3.0;

craft_ui::words! {
    /// The dialog's and menu's words (DeckCraft's and WordCraft's strings).
    #[non_exhaustive]
    pub struct TextColumnsLabels {
        title: "columns.title" = "Columns";
        number: "columns.number" = "Number:";
        spacing: "columns.spacing" = "Spacing:";
        number_range: "columns.numberRange" = "Number must be a whole number from {min} to {max}";
        spacing_range: "columns.spacingRange" = "Spacing must be from {min} to {max}";
        presets: "columns.presets" = "Presets";
        one: "columns.one" = "One";
        two: "columns.two" = "Two";
        three: "columns.three" = "Three";
        left: "columns.left" = "Left";
        right: "columns.right" = "Right";
        width_and_spacing: "columns.widthAndSpacing" = "Width and spacing";
        column: "columns.columnN" = "Col {n}:";
        width: "columns.width" = "Width:";
        equal_width: "columns.equalWidth" = "Equal column width";
        line_between: "columns.lineBetween" = "Line between";
        more_columns: "columns.more" = "More Columns…";
        preview: "columns.preview" = "Preview";
        apply_to: "columns.applyTo" = "Apply to:";
        this_section: "columns.thisSection" = "This section";
        whole_document: "columns.wholeDocument" = "Whole document";
        this_point_forward: "columns.thisPointForward" = "This point forward";
        selected_text: "columns.selectedText" = "Selected text";
    }
}

/// Column settings; `None` where the selected text boxes or sections differ (a field left as
/// shown is left out of the result, so each keeps its own value).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextColumns {
    /// Number of columns.
    pub count: Option<u32>,
    /// The space between columns, points (all of them, with equal width).
    pub spacing: Option<f32>,
    /// Word: each column's width and the spacing after it, points; empty with equal width.
    pub widths: Vec<(f32, f32)>,
    /// Whether all columns have the same width.
    pub equal_width: Option<bool>,
    /// Whether a line is drawn between columns.
    pub line_between: Option<bool>,
    /// Word: where the columns apply. `Some` only for a dialog with [`apply_to`](TextColumnsDialog::apply_to),
    /// which always returns it with the changes (WordCraft sends it with every `layout.columns`).
    pub apply: Option<ColumnsApply>,
}

/// Word's Apply to: the part of the document the columns apply to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColumnsApply {
    /// The section the caret is in (offered when the document has more than one).
    Section,
    /// The whole document.
    Document,
    /// From the caret to the end: a new section starts here.
    Forward,
    /// The selected text, as a section of its own.
    Selection,
}

/// Word's Layout › Columns presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColumnsPreset {
    #[allow(missing_docs)]
    One,
    #[allow(missing_docs)]
    Two,
    #[allow(missing_docs)]
    Three,
    /// Two columns, the left one narrower.
    Left,
    /// Two columns, the right one narrower.
    Right,
}

/// The Columns dialog: only what changed comes back.
///
/// - [`word`](Self::word) off (PowerPoint): Number (a craft-ui `NumberField`, 1–[`MAX_COLUMNS`])
///   and Spacing (a `unit_number_field`, 0–[`MAX_SPACING`] points); values out of range keep the
///   dialog open with the labels' range messages under the fields in the field error style.
/// - On (Word): the presets row, Number, the width and spacing grid (one row per column, Col #
///   1–n; width fields disabled after the first with Equal column width), Equal column width,
///   Line between, and the page's text width for the grid's sums.
pub struct TextColumnsDialog {
    shown: TextColumns,
    edited: TextColumns,
    word: Option<f32>,
    max_columns: Option<u32>,
    apply_choices: Vec<ColumnsApply>,
    unit: LengthUnit,
    state: DialogState,
    labels: Option<TextColumnsLabels>,
    /// The edited columns from before the running gesture: Esc restores them.
    draft: GestureDraft<TextColumns>,
}

impl TextColumnsDialog {
    /// A dialog editing `columns`, with lengths in inches.
    pub fn new(columns: TextColumns) -> Self {
        TextColumnsDialog {
            shown: columns.clone(),
            edited: columns,
            word: None,
            max_columns: None,
            apply_choices: Vec::new(),
            unit: LengthUnit::Inches,
            state: DialogState::default(),
            labels: None,
            draft: GestureDraft::new(),
        }
    }

    /// Word's dialog, over a text width of `width` points.
    pub fn word(mut self, width: f32) -> Self {
        self.word = Some(width);
        self
    }

    /// The most columns Number takes, in place of the family's: [`MAX_COLUMNS_WORD`] with
    /// [`word`](Self::word), [`MAX_COLUMNS`] (PowerPoint, DeckCraft) without. At least 1.
    pub fn max_columns(mut self, max: u32) -> Self {
        self.max_columns = Some(max.max(1));
        self
    }

    /// Word's Apply to: the parts of the document the columns can apply to, in the order the
    /// list shows them, and the one chosen when the dialog opens (added to the list when it is
    /// missing). The answer carries the choice in [`TextColumns::apply`], changed or not.
    pub fn apply_to(mut self, choices: &[ColumnsApply], current: ColumnsApply) -> Self {
        self.apply_choices = choices.to_vec();
        if !self.apply_choices.contains(&current) {
            self.apply_choices.push(current);
        }
        self.shown.apply = Some(current);
        self.edited.apply = Some(current);
        self
    }

    /// Sets the unit the length fields show.
    pub fn unit(mut self, unit: LengthUnit) -> Self {
        self.unit = unit;
        self
    }

    /// Sets the translated labels; the default reads them from the interface locale.
    pub fn labels(mut self, labels: TextColumnsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The fields that differ from what the dialog showed; `None` when none does.
    fn changes(&self) -> Option<TextColumns> {
        let (shown, edited) = (&self.shown, &self.edited);
        fn kept<T: Clone + PartialEq>(a: &Option<T>, b: &Option<T>) -> Option<T> {
            if a != b { b.clone() } else { None }
        }
        let changes = TextColumns {
            count: kept(&shown.count, &edited.count),
            spacing: kept(&shown.spacing, &edited.spacing),
            widths: if shown.widths != edited.widths { edited.widths.clone() } else { Vec::new() },
            equal_width: kept(&shown.equal_width, &edited.equal_width),
            line_between: kept(&shown.line_between, &edited.line_between),
            apply: edited.apply.filter(|_| !self.apply_choices.is_empty()),
        };
        (changes != TextColumns::default()).then_some(changes)
    }

    /// The most columns Number takes.
    fn max(&self) -> u32 {
        self.max_columns.unwrap_or(if self.word.is_some() { MAX_COLUMNS_WORD } else { MAX_COLUMNS })
    }

    fn count_ok(&self, count: u32) -> bool {
        (1..=self.max()).contains(&count)
    }

    fn count_in_range(&self) -> bool {
        self.edited.count.is_none_or(|n| self.count_ok(n))
    }

    fn spacing_in_range(&self) -> bool {
        self.edited.spacing.is_none_or(|s| (0.0..=MAX_SPACING).contains(&s))
    }

    fn equal(&self) -> bool {
        self.edited.equal_width == Some(true)
    }

    /// The spacing the columns have for the arithmetic: the edited one, else the first column's.
    fn spacing_now(&self) -> f32 {
        self.edited.spacing.or(self.edited.widths.first().map(|w| w.1)).unwrap_or(DEFAULT_SPACING)
    }

    /// `count` equal columns of `spacing` filling `width`, each at least [`MIN_COLUMN_WIDTH`]: the
    /// spacing gives way when the columns would be narrower (WordCraft `even_out`).
    fn even_rows(width: f32, spacing: f32, count: u32) -> Vec<(f32, f32)> {
        let room = (width - MIN_COLUMN_WIDTH * count as f32).max(0.0);
        let spacing = if count > 1 { spacing.clamp(0.0, room / (count - 1) as f32) } else { spacing };
        let each = ((width - spacing * (count - 1) as f32) / count as f32).max(0.0);
        (0..count).map(|i| (each, if i + 1 < count { spacing } else { 0.0 })).collect()
    }

    /// The width and the spacing after each column, as the grid shows them.
    fn rows(&self) -> Vec<(f32, f32)> {
        let (Some(width), Some(count)) = (self.word, self.edited.count.filter(|n| self.count_ok(*n))) else {
            return Vec::new();
        };
        if !self.equal() && self.edited.widths.len() == count as usize {
            return self.edited.widths.clone();
        }
        Self::even_rows(width, self.spacing_now(), count)
    }

    fn set_count(&mut self, count: u32) {
        self.edited.count = Some(count);
        if self.word.is_some() && self.count_ok(count) && !self.equal() {
            self.edited.widths = self.rows_for(count);
        }
    }

    fn rows_for(&self, count: u32) -> Vec<(f32, f32)> {
        Self::even_rows(self.word.unwrap_or(0.0), self.spacing_now(), count)
    }

    fn apply_preset(&mut self, preset: ColumnsPreset) {
        let Some(width) = self.word else { return };
        let spacing = DEFAULT_SPACING;
        self.edited.spacing = Some(spacing);
        let (count, narrow_first) = match preset {
            ColumnsPreset::One => (1, None),
            ColumnsPreset::Two => (2, None),
            ColumnsPreset::Three => (3, None),
            ColumnsPreset::Left => (2, Some(true)),
            ColumnsPreset::Right => (2, Some(false)),
        };
        self.edited.count = Some(count);
        match narrow_first {
            None => {
                self.edited.equal_width = Some(true);
                self.edited.widths.clear();
            }
            Some(left) => {
                let room = (width - spacing).max(0.0);
                let narrow = room * NARROW_SHARE;
                let wide = room - narrow;
                let (first, second) = if left { (narrow, wide) } else { (wide, narrow) };
                self.edited.equal_width = Some(false);
                self.edited.widths = vec![(first, spacing), (second, 0.0)];
            }
        }
    }

    /// Column `row`'s width becomes `value`: with equal widths the spacing takes the difference,
    /// else the neighbouring column does.
    fn set_width(&mut self, row: usize, value: f32) {
        let (Some(width), Some(count)) = (self.word, self.edited.count) else { return };
        let value = value.clamp(MIN_COLUMN_WIDTH.min(width), width);
        if self.equal() {
            if count > 1 {
                self.edited.spacing = Some(((width - count as f32 * value) / (count - 1) as f32).clamp(0.0, MAX_SPACING));
            }
            return;
        }
        let mut rows = self.rows();
        let Some(old) = rows.get(row).map(|r| r.0) else { return };
        let other = if row + 1 < rows.len() { row + 1 } else { row.saturating_sub(1) };
        rows[row].0 = value;
        if other != row {
            // The neighbour gives the difference up to its minimum; the edited column takes back
            // what the neighbour could not give (WordCraft `rebalance`).
            let wanted = rows[other].0 - (value - old);
            let short = (MIN_COLUMN_WIDTH - wanted).max(0.0);
            rows[other].0 = wanted.max(MIN_COLUMN_WIDTH);
            if short > 0.0 {
                let from_width = short.min((rows[row].0 - MIN_COLUMN_WIDTH).max(0.0));
                rows[row].0 -= from_width;
                rows[row].1 = (rows[row].1 - (short - from_width)).max(0.0);
            }
        }
        self.edited.widths = rows;
    }

    /// The spacing after column `row` becomes `value`: with equal widths it is every column's;
    /// else the next column's width takes the difference.
    fn set_spacing_after(&mut self, row: usize, value: f32) {
        let value = value.clamp(0.0, MAX_SPACING);
        if self.equal() {
            self.edited.spacing = Some(value);
            return;
        }
        let mut rows = self.rows();
        let Some(old) = rows.get(row).map(|r| r.1) else { return };
        // The next column keeps its minimum width: the spacing stops growing there.
        let value = rows.get(row + 1).map_or(value, |next| value.min(old + (next.0 - MIN_COLUMN_WIDTH).max(0.0)));
        rows[row].1 = value;
        if let Some(next) = rows.get_mut(row + 1) {
            next.0 = (next.0 - (value - old)).max(MIN_COLUMN_WIDTH);
        }
        self.edited.widths = rows;
    }
}

fn in_unit(points: f32, unit: LengthUnit) -> f64 {
    UnitContext::default().convert(Length { value: f64::from(points), unit: LengthUnit::Points }, unit).unwrap_or(f64::from(points))
}

fn in_points(value: f64, unit: LengthUnit) -> f32 {
    UnitContext::default().convert(Length { value, unit }, LengthUnit::Points).unwrap_or(value) as f32
}

impl TextColumnsDialog {
    /// Applies a field's event through the draft: every `Update` shows in the dialog, and Esc
    /// (`Cancel`) restores the columns from before the gesture. `set` writes the field's value
    /// into the columns.
    fn edit<V>(&mut self, event: Option<EditEvent<V>>, set: impl FnOnce(&mut Self, V)) {
        let Some(event) = event else { return };
        let event = event.map(|value| {
            let before = self.edited.clone();
            set(self, value);
            std::mem::replace(&mut self.edited, before)
        });
        self.draft.apply(&mut self.edited, event);
    }

    /// The message under Number when it is out of range, its bounds in the locale's digits.
    pub(super) fn number_message(labels: &TextColumnsLabels, numbers: &LocaleNumbers, max: u32) -> String {
        let bound = |n: u32| numbers.format(f64::from(n), 0..=0);
        fmt(&labels.number_range, &[("min", &bound(1)), ("max", &bound(max))])
    }

    /// The Number field's event.
    pub(super) fn edit_count(&mut self, event: Option<EditEvent<Option<f64>>>) {
        self.edit(event, |me, v| {
            if let Some(v) = v {
                me.set_count(v.max(0.0) as u32);
            }
        });
    }

    /// The columns as edited so far.
    #[cfg(test)]
    pub(super) fn edited(&self) -> &TextColumns {
        &self.edited
    }

    fn spacing_message(&self, labels: &TextColumnsLabels, units: &Units) -> String {
        let show = |points: f32| units.length(self.unit, in_unit(points, self.unit));
        fmt(&labels.spacing_range, &[("min", &show(0.0)), ("max", &show(MAX_SPACING))])
    }

    /// One width or spacing field of the grid, named `caption`; its event in points.
    fn length_field(
        &self,
        ui: &mut Ui,
        numbers: &LocaleNumbers,
        salt: (&'static str, usize),
        caption: &str,
        points: Option<f32>,
        enabled: bool,
    ) -> craft_ui::edit::EditResponse<Option<f32>> {
        let units = ExprUnits { unit: Some(self.unit), suffix: self.unit.suffix(), context: UnitContext::default() };
        let shown = points.map_or(f64::NAN, |p| in_unit(p, self.unit));
        unit_number_field(salt, caption, shown, numbers, &units).enabled(enabled).show_optional(ui).map(|v| v.map(|v| in_points(v, self.unit)))
    }

    fn body(&mut self, ui: &mut Ui, labels: &TextColumnsLabels, numbers: &LocaleNumbers, valid: &Cell<bool>) {
        let gap = ui.spacing().item_spacing.y;
        if self.word.is_some() {
            GroupBox::new(labels.presets.as_str()).show(ui, |ui| self.presets(ui, labels));
            ui.add_space(gap);
        }
        FieldGroup::new("columns-fields").show(ui, |ui| {
            let bad = !self.count_in_range();
            let error = bad.then(|| Self::number_message(labels, numbers, self.max()));
            let shown = self.edited.count.map_or(f64::NAN, f64::from);
            let edit = Field::new(labels.number.as_str())
                .error(error)
                .show(ui, |ui, invalid| {
                    NumberField::new(("columns", "count"), labels.number.as_str(), shown, numbers).decimals(0..=0).invalid(invalid).show_optional(ui)
                })
                .inner;
            self.edit_count(edit.event);
            if self.word.is_none() {
                let bad = !self.spacing_in_range();
                let error = bad.then(|| self.spacing_message(labels, &Units::of(ui)));
                let spacing = self.edited.spacing;
                let edit = Field::new(labels.spacing.as_str())
                    .error(error)
                    .show(ui, |ui, invalid| {
                        let units = ExprUnits { unit: Some(self.unit), suffix: self.unit.suffix(), context: UnitContext::default() };
                        let shown = spacing.map_or(f64::NAN, |p| in_unit(p, self.unit));
                        unit_number_field(("columns", "spacing"), labels.spacing.as_str(), shown, numbers, &units).invalid(invalid).show_optional(ui)
                    })
                    .inner;
                let unit = self.unit;
                self.edit(edit.event, |me, v: Option<f64>| {
                    if let Some(v) = v {
                        me.edited.spacing = Some(in_points(v, unit));
                    }
                });
            }
        });
        if self.word.is_some() {
            ui.add_space(gap);
            let pane = Tokens::of(ui).component.dialog.preview_pane[0];
            ui.horizontal_top(|ui| {
                let room = (ui.available_width() - pane - ui.spacing().item_spacing.x).max(0.0);
                ui.scope(|ui| {
                    ui.set_width(room);
                    GroupBox::new(labels.width_and_spacing.as_str()).show(ui, |ui| self.grid(ui, labels, numbers));
                });
                ui.vertical(|ui| {
                    ui.add(Label::new(labels.preview.as_str()).selectable(false));
                    self.preview(ui, labels);
                });
            });
            ui.add_space(gap);
            let mut equal = self.edited.equal_width.map_or(CheckState::Mixed, CheckState::from);
            if Checkbox::new(labels.equal_width.as_str(), &mut equal).show(ui).response.changed() {
                self.edited.equal_width = Some(equal == CheckState::Checked);
                if equal == CheckState::Checked {
                    self.edited.spacing = Some(self.spacing_now());
                    self.edited.widths.clear();
                } else if let Some(count) = self.edited.count.filter(|n| self.count_ok(*n)) {
                    self.edited.widths = self.rows_for(count);
                }
            }
        }
        let mut line = self.edited.line_between.map_or(CheckState::Mixed, CheckState::from);
        if Checkbox::new(labels.line_between.as_str(), &mut line).show(ui).response.changed() {
            self.edited.line_between = Some(line == CheckState::Checked);
        }
        if !self.apply_choices.is_empty() {
            ui.add_space(gap);
            self.apply_select(ui, labels);
        }
        valid.set(self.count_in_range() && self.spacing_in_range());
    }

    /// Word's Apply to list.
    fn apply_select(&mut self, ui: &mut Ui, labels: &TextColumnsLabels) {
        let mut apply = self.edited.apply;
        Field::new(labels.apply_to.as_str()).show(ui, |ui, _| {
            let mut select = Select::new(("columns", "apply"), labels.apply_to.as_str(), &mut apply);
            for choice in &self.apply_choices {
                let text = match choice {
                    ColumnsApply::Section => &labels.this_section,
                    ColumnsApply::Document => &labels.whole_document,
                    ColumnsApply::Forward => &labels.this_point_forward,
                    ColumnsApply::Selection => &labels.selected_text,
                };
                select = select.option(Some(*choice), text.as_str());
            }
            select.show(ui)
        });
        self.edited.apply = apply;
    }

    /// The page preview: the columns and the line between them on a page, as Word's dialog draws it
    /// (WordCraft `columns_preview`).
    fn preview(&self, ui: &mut Ui, labels: &TextColumnsLabels) {
        let tokens = Tokens::of(ui);
        let [pane_width, pane_height] = tokens.component.dialog.preview_pane;
        let (pane, response) = ui.allocate_exact_size(vec2(pane_width, pane_height), Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, labels.preview.as_str()));
        if !ui.is_rect_visible(pane) {
            return;
        }
        let ppp = ui.pixels_per_point();
        let stroke = |color| egui::Stroke::new(craft_theme::Stroke::W1.width(ppp), color);
        let colors = &tokens.color;
        let height = pane.height().min(pane.width() / PAGE_ASPECT);
        let page = Rect::from_center_size(pane.center(), vec2(height * PAGE_ASPECT, height));
        let painter = ui.painter();
        painter.rect_filled(pane, 0.0, tokens.component.dialog.preview_fill.to_egui());
        painter.rect(page, 0.0, colors.surface.raised.to_egui(), stroke(colors.line.border.to_egui()), egui::StrokeKind::Inside);
        let plan = preview_plan(&self.rows(), self.word.unwrap_or(0.0), self.edited.line_between == Some(true), page);
        let pitch = ui.text_style_height(&egui::TextStyle::Body) * LINE_PITCH;
        for &(left, right) in &plan.columns {
            let mut y = plan.text.top();
            while y + pitch / 2.0 <= plan.text.bottom() {
                painter.line_segment([pos2(left, y), pos2(right, y)], stroke(colors.text.faint.to_egui()));
                y += pitch;
            }
        }
        for &x in &plan.separators {
            painter.line_segment([pos2(x, plan.text.top()), pos2(x, plan.text.bottom())], stroke(colors.text.normal.to_egui()));
        }
    }

    fn presets(&mut self, ui: &mut Ui, labels: &TextColumnsLabels) {
        let mut chosen = None;
        ui.horizontal(|ui| {
            let every = [
                (ColumnsPreset::One, &labels.one),
                (ColumnsPreset::Two, &labels.two),
                (ColumnsPreset::Three, &labels.three),
                (ColumnsPreset::Left, &labels.left),
                (ColumnsPreset::Right, &labels.right),
            ];
            for (preset, label) in every {
                if ui.add(Button::new(label.as_str()).size(ControlSize::Sm)).clicked() {
                    chosen = Some(preset);
                }
            }
        });
        if let Some(preset) = chosen {
            self.apply_preset(preset);
        }
    }

    /// One row per column: its number, then its width and the spacing after it.
    fn grid(&mut self, ui: &mut Ui, labels: &TextColumnsLabels, numbers: &LocaleNumbers) {
        let rows = self.rows();
        let equal = self.equal();
        let mut width_edit = None;
        let mut spacing_edit = None;
        let (width_name, spacing_name) = (bare(&labels.width), bare(&labels.spacing));
        FormGrid::new("columns-grid").show(ui, |form| {
            for (i, (width, spacing)) in rows.iter().enumerate() {
                let number = numbers.format(i as f64 + 1.0, 0..=0);
                let row = fmt(&labels.column, &[("n", &number)]);
                form.row(row, |ui, _| {
                    ui.add(Label::new(labels.width.as_str()).selectable(false));
                    let w = self.length_field(ui, numbers, ("columns-width", i), width_name, Some(*width), !equal || i == 0);
                    ui.add(Label::new(labels.spacing.as_str()).selectable(false));
                    let last = i + 1 == rows.len();
                    let s = self.length_field(ui, numbers, ("columns-gap", i), spacing_name, (!last).then_some(*spacing), !last);
                    // A row without an event leaves an earlier row's event alone.
                    width_edit = w.event.clone().map(|e| (i, e)).or(width_edit.take());
                    spacing_edit = s.event.clone().map(|e| (i, e)).or(spacing_edit.take());
                    (w.own_name(), s)
                });
            }
        });
        if let Some((i, event)) = width_edit {
            self.edit(Some(event), |me, v: Option<f32>| {
                if let Some(v) = v {
                    me.set_width(i, v);
                }
            });
        }
        if let Some((i, event)) = spacing_edit {
            self.edit(Some(event), |me, v: Option<f32>| {
                if let Some(v) = v {
                    me.set_spacing_after(i, v);
                }
            });
        }
    }
}

/// A page's width over its height (letter paper, WordCraft draws 84 by 110).
const PAGE_ASPECT: f32 = 84.0 / 110.0;
/// The margin around the preview's text, as a share of the page's width (WordCraft: 9 of 84).
const PAGE_MARGIN: f32 = 9.0 / 84.0;
/// The distance between the preview's text lines, as a share of a text row's height.
const LINE_PITCH: f32 = 0.5;

/// Where the preview draws: the text area, each column's left and right edge and the x of the
/// line between each two columns (when [`line_between`](TextColumns::line_between) is on).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PreviewPlan {
    pub(super) text: Rect,
    pub(super) columns: Vec<(f32, f32)>,
    pub(super) separators: Vec<f32>,
}

/// The preview of `rows` (each column's width and the spacing after it, points) over a text width
/// of `text_width` points on `page`.
pub(super) fn preview_plan(rows: &[(f32, f32)], text_width: f32, line_between: bool, page: Rect) -> PreviewPlan {
    let text = page.shrink(page.width() * PAGE_MARGIN);
    let scale = text.width() / text_width.max(f32::EPSILON);
    let mut x = text.left();
    let mut columns = Vec::new();
    let mut separators = Vec::new();
    for (i, (width, spacing)) in rows.iter().enumerate() {
        let right = (x + width * scale).min(text.right());
        columns.push((x, right));
        x = right;
        if i + 1 < rows.len() {
            let gap = spacing * scale;
            if line_between {
                separators.push(x + gap / 2.0);
            }
            x += gap;
        }
    }
    PreviewPlan { text, columns, separators }
}

/// A caption without its trailing colon: the name of a field whose row label carries the colon.
fn bare(caption: &str) -> &str {
    caption.trim_end_matches([':', '\u{ff1a}'])
}

impl StackDialog for TextColumnsDialog {
    /// The fields that changed; `None` when none did (OK applies nothing).
    type Result = Option<TextColumns>;

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("text-columns")
    }

    // deckcraft: crates/ui-egui/src/dialogs/columns.rs:32 `columns_dialog`, :6 `columns_value`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<Option<TextColumns>>> {
        let labels = self.labels.clone().unwrap_or_else(|| TextColumnsLabels::of_ctx(ctx));
        let numbers = LocaleNumbers::of_ctx(ctx);
        let valid = Cell::new(self.count_in_range() && self.spacing_in_range());
        let footer = Footer::standard().ok_enabled_with(|| valid.get());
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id(), labels.title.as_str())
            .stack(stack)
            .width(if self.word.is_some() { Width::Standard } else { Width::Compact })
            .initial_focus(InitialFocus::FirstField)
            .footer(footer)
            .show(ctx, &mut state, &mut (), |ui, ()| self.body(ui, &labels, &numbers, &valid));
        self.state = state;
        match shown.outcome {
            Some(Outcome::Ok) if valid.get() => Some(DialogResult::Ok(self.changes())),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}

/// A choice from the Columns menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColumnsChoice {
    /// One of the column presets.
    Preset(ColumnsPreset),
    /// More Columns…: open the dialog.
    More,
}

/// Layout › Columns as craft-ui menu entries (no `Ui`): One, Two, Three, Left, Right with the
/// matching one checked, a separator and More Columns…. The app shows them with
/// `menu::PressMenu` from its ribbon button or menu bar and applies the chosen value.
// wordcraft: crates/ui-egui/src/ribbon/layout.rs:36 `columns_items`.
pub fn columns_menu(current: Option<ColumnsPreset>, labels: &TextColumnsLabels) -> Vec<MenuEntry<ColumnsChoice>> {
    let presets = [
        (ColumnsPreset::One, &labels.one),
        (ColumnsPreset::Two, &labels.two),
        (ColumnsPreset::Three, &labels.three),
        (ColumnsPreset::Left, &labels.left),
        (ColumnsPreset::Right, &labels.right),
    ];
    let mut entries: Vec<MenuEntry<ColumnsChoice>> = presets
        .into_iter()
        .map(|(preset, label)| MenuItem::new(label.as_str(), ColumnsChoice::Preset(preset)).check(current == Some(preset)).into())
        .collect();
    entries.push(MenuEntry::Separator);
    entries.push(MenuItem::new(labels.more_columns.as_str(), ColumnsChoice::More).into());
    entries
}
