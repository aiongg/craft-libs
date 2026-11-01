//! [`TabStopsDialog`] (N3.09): Word's and PowerPoint's Tabs dialog on the dialog stack: the
//! default tab stops interval, the list of stops (position, alignment, leader, each editable in
//! place), Set, Clear and Clear All; OK applies them together.
//!
//! ```ignore
//! dialogs.open(TabStopsDialog::new(paragraph.tabs.clone(), doc.default_tab).unit(unit));
//! if let Some(DialogResult::Ok(tabs)) = dialogs.take::<TabStopsDialog>() {
//!     doc.set_tabs(tabs.stops, tabs.cleared, tabs.default_interval);
//! }
//! ```

use std::ops::RangeInclusive;
use std::sync::Arc;

use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::controls::expr::ExprUnits;
use craft_ui::controls::{Button, Select};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, InitialFocus, Outcome, StackDialog, Width};
use craft_ui::edit::{EditEvent, EditResponse};
use craft_ui::list::{Table, TableColumn, TableEvent, TableLayout};
use craft_ui::localized::units::{LengthUnit, UnitContext};
use craft_ui::localized::{LocaleNumbers, SharedLabels as _};
use craft_ui::patterns::unit_number_field;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{Context, Id, Ui};

use super::ruler::{MAX_POSITION, SAME_POSITION, TabAlign, TabLeader, TabStop};

craft_ui::words! {
    /// The dialog's words (WordCraft's strings; PowerPoint's dialog uses the same).
    #[non_exhaustive]
    pub struct TabStopsLabels {
        title: "tabs.title" = "Tabs";
        default_stops: "tabs.defaultStops" = "Default tab stops:";
        position: "tabs.position" = "Tab stop position:";
        alignment: "tabs.alignment" = "Alignment";
        leader: "tabs.leader" = "Leader";
        left: "tabs.left" = "Left";
        center: "tabs.center" = "Center";
        right: "tabs.right" = "Right";
        decimal: "tabs.decimal" = "Decimal";
        bar: "tabs.bar" = "Bar";
        leader_none: "tabs.leaderNone" = "None";
        set: "tabs.set" = "Set";
        clear: "tabs.clear" = "Clear";
        clear_all: "tabs.clearAll" = "Clear All";
        to_be_cleared: "tabs.toBeCleared" = "Tab stops to be cleared:";
    }
}

/// What OK returns.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabStopsResult {
    /// The stops, sorted, none within [`SAME_POSITION`](super::ruler::SAME_POSITION) of another.
    pub stops: Vec<TabStop>,
    /// Positions to clear (stops the paragraph inherits from its style, cleared here).
    pub cleared: Vec<f32>,
    /// The default interval, points.
    pub default_interval: f32,
}

/// The most stops the dialog reads from a paragraph (Word's 64).
pub const MAX_STOPS: usize = 64;

/// The Tabs dialog.
///
/// - The stops are a craft-ui [`Table`](craft_ui::list::Table) (columns Position, Alignment,
///   Leader; one Tab stop with the list keys and selection) whose cells are a `unit_number_field`
///   in the unit and two `Select`s (the leader's showing just the fill when closed); editing a
///   row in place keeps the table in row order until the edit ends, then
///   [`TabStopsDialog::normalize`] restores position order and merges stops moved onto one
///   another (the edited row wins).
/// - Set adds a stop at the next free multiple of the default interval after the last stop, left
///   aligned, no leader, selected and ready to edit; Clear moves the selected stop to "to be
///   cleared"; Clear All clears every stop.
/// - Positions are clamped to ±[`MAX_POSITION`](super::ruler::MAX_POSITION); at most
///   [`MAX_STOPS`] stops are read from a paragraph.
/// - Keys: Enter is OK unless a row's field has the keyboard (it commits the field), Esc Cancel.
pub struct TabStopsDialog {
    stops: Vec<TabStop>,
    cleared: Vec<f32>,
    default_interval: f32,
    selected: Option<usize>,
    edited: Option<usize>,
    unit: LengthUnit,
    labels: Option<TabStopsLabels>,
    state: DialogState,
    layout: TableLayout<Col>,
    /// The row whose position field takes the keyboard in the next pass (a stop Set just added).
    to_edit: Option<usize>,
    /// The row and position an in-place edit began from, for Esc.
    before: Option<(usize, f32)>,
}

/// The stops table's columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Col {
    Position,
    Alignment,
    Leader,
}

/// The rows the stops table shows before it scrolls.
const VISIBLE_ROWS: usize = 6;

/// The interval Set steps by when the document's is not a positive number: Word's 0.5 in.
const FALLBACK_INTERVAL: f32 = 36.0;

impl TabStopsDialog {
    /// The dialog over a paragraph's `stops` and the document's default interval (points).
    pub fn new(mut stops: Vec<TabStop>, default_interval: f32) -> Self {
        stops.retain(|stop| stop.position.is_finite());
        stops.truncate(MAX_STOPS);
        TabStopsDialog {
            stops,
            cleared: Vec::new(),
            default_interval,
            selected: None,
            edited: None,
            unit: LengthUnit::Inches,
            labels: None,
            state: DialogState::new(),
            layout: TableLayout { order: vec![Col::Position, Col::Alignment, Col::Leader], hidden: Vec::new(), widths: Vec::new(), sort: None },
            to_edit: None,
            before: None,
        }
    }

    pub fn unit(mut self, unit: LengthUnit) -> Self {
        self.unit = unit;
        self
    }

    pub fn labels(mut self, labels: TabStopsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Set: a stop at the next free multiple of the default interval after the last stop.
    // wordcraft: crates/ui-egui/src/dialogs/dialogs_para.rs:75 `TabsForm::new_stop`.
    pub fn new_stop(&mut self) {
        if self.stops.len() >= MAX_STOPS {
            return;
        }
        let step = if self.default_interval.is_finite() && self.default_interval > 0.0 { self.default_interval } else { FALLBACK_INTERVAL };
        let taken = |at: f32| {
            self.stops.iter().any(|stop| (stop.position - at).abs() < SAME_POSITION)
                || self.cleared.iter().any(|gone| (gone - at).abs() < SAME_POSITION)
        };
        let last = self.stops.iter().map(|stop| stop.position).fold(0.0, f32::max);
        let mut position = (last / step).floor() * step + step;
        while taken(position) && position <= MAX_POSITION {
            position += step;
        }
        if !position.is_finite() || position > MAX_POSITION {
            return;
        }
        self.stops.push(TabStop { position, align: TabAlign::Left, leader: TabLeader::None });
        self.stops.sort_by(|a, b| a.position.total_cmp(&b.position));
        self.selected = self.stops.iter().position(|stop| (stop.position - position).abs() < SAME_POSITION);
        self.to_edit = self.selected;
        self.edited = None;
    }

    /// Position order after in-place edits, merging stops within [`SAME_POSITION`](super::ruler::SAME_POSITION) (the edited
    /// row wins), the selection kept on its stop.
    // wordcraft: crates/ui-egui/src/dialogs/dialogs_para.rs:123 `TabsForm::normalize`.
    pub fn normalize(&mut self) {
        let Some(edited) = self.edited.take() else { return };
        let followed = self.selected.and_then(|at| self.stops.get(at)).copied();
        let mut kept: Vec<(usize, TabStop)> = Vec::with_capacity(self.stops.len());
        for (at, stop) in self.stops.iter().enumerate() {
            match kept.iter().position(|(_, other)| (other.position - stop.position).abs() < SAME_POSITION) {
                // The edited row holds its ground; the row that drifted onto it goes.
                Some(held) if kept.get(held).is_some_and(|(was, _)| *was == edited) => {}
                Some(held) => {
                    kept.remove(held);
                    kept.push((at, *stop));
                }
                None => kept.push((at, *stop)),
            }
        }
        kept.sort_by(|a, b| a.1.position.total_cmp(&b.1.position));
        self.stops = kept.into_iter().map(|(_, stop)| stop).collect();
        self.selected = followed.and_then(|moved| self.stops.iter().position(|stop| (stop.position - moved.position).abs() < SAME_POSITION));
    }

    /// An in-place edit of the stop at `at`: clamped, and a cleared position it lands on is no
    /// longer cleared. The rows keep their order until [`normalize`](Self::normalize).
    pub(super) fn edit_position(&mut self, at: usize, position: f32) {
        if !position.is_finite() {
            return;
        }
        let Some(stop) = self.stops.get_mut(at) else { return };
        stop.position = position.clamp(-MAX_POSITION, MAX_POSITION);
        let landed = stop.position;
        self.cleared.retain(|gone| (gone - landed).abs() >= SAME_POSITION);
        self.edited = Some(at);
    }

    pub(super) fn select(&mut self, at: Option<usize>) {
        self.selected = at.filter(|at| *at < self.stops.len());
    }

    /// Clear: the selected stop moves to "to be cleared".
    pub(super) fn clear(&mut self) {
        let Some(at) = self.selected.filter(|at| *at < self.stops.len()) else { return };
        let stop = self.stops.remove(at);
        self.cleared.push(stop.position);
        self.selected = None;
        self.edited = None;
    }

    /// Clear All.
    pub(super) fn clear_all(&mut self) {
        self.cleared.extend(self.stops.drain(..).map(|stop| stop.position));
        self.selected = None;
        self.edited = None;
    }

    /// What OK returns.
    pub(super) fn result(&mut self) -> TabStopsResult {
        self.normalize();
        TabStopsResult { stops: self.stops.clone(), cleared: self.cleared.clone(), default_interval: self.default_interval }
    }
}

impl StackDialog for TabStopsDialog {
    type Result = TabStopsResult;

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("tab-stops")
    }

    // wordcraft: crates/ui-egui/src/dialogs/dialogs_para.rs:195 `tabs`; deckcraft:
    // crates/ui-egui/src/dialogs/tabs.rs:7 `tabs_dialog` (PowerPoint's: default spacing, list, Set, Clear All).
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<TabStopsResult>> {
        let labels = match &self.labels {
            Some(labels) => Arc::new(labels.clone()),
            None => TabStopsLabels::shared_ctx(ctx),
        };
        let numbers = LocaleNumbers::of_ctx(ctx);
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id(), labels.title.as_str())
            .stack(stack)
            .width(Width::Standard)
            .initial_focus(InitialFocus::Field(self.field_id("default")))
            .footer(Footer::standard())
            .show(ctx, &mut state, &mut (), |ui, ()| self.body(ui, &labels, &numbers));
        self.state = state;
        match shown.outcome {
            Some(Outcome::Ok) => Some(DialogResult::Ok(self.result())),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}

/// A length field in `unit` over points.
struct Lengths<'a> {
    unit: LengthUnit,
    numbers: &'a LocaleNumbers,
}

fn rounded(value: f64, unit: LengthUnit) -> f64 {
    let scale = 10f64.powi(unit.decimals() as i32);
    (value * scale).round() / scale
}

impl Lengths<'_> {
    /// The field for `points`; its events carry points.
    fn field(&self, ui: &mut Ui, salt: impl egui::AsIdSalt, caption: &str, points: f32, range: RangeInclusive<f32>, id: Id) -> EditResponse<f32> {
        let context = UnitContext::default();
        let convert = |points: f64| context.from_pixels(points, self.unit).unwrap_or(points);
        let shown = rounded(convert(f64::from(points)), self.unit);
        let units = ExprUnits { unit: Some(self.unit), suffix: self.unit.suffix(), context };
        let field = unit_number_field(salt, caption, shown, self.numbers, &units)
            .range(convert(f64::from(*range.start()))..=convert(f64::from(*range.end())))
            .with_id(id);
        field.show(ui).map(|value| context.to_pixels(value, self.unit).map_or(points, |points| points as f32))
    }

    /// `points` as the number in the unit with its suffix.
    fn text(&self, points: f32) -> String {
        let value = UnitContext::default().from_pixels(f64::from(points), self.unit).unwrap_or(f64::from(points));
        format!("{value:.*} {}", self.unit.decimals(), self.unit.suffix())
    }
}

/// A change made in a cell of the stops table.
enum CellEdit {
    Position(usize, EditEvent<f32>),
    Align(usize, TabAlign),
    Leader(usize, TabLeader),
}

impl TabStopsDialog {
    fn field_id(&self, name: &str) -> Id {
        self.id().with(name)
    }

    fn body(&mut self, ui: &mut Ui, labels: &TabStopsLabels, numbers: &LocaleNumbers) {
        let lengths = Lengths { unit: self.unit, numbers };
        self.default_row(ui, labels, &lengths);
        let edits = self.table(ui, labels, &lengths);
        for edit in edits {
            self.apply(edit);
        }
        self.buttons(ui, labels);
        if !self.cleared.is_empty() {
            ui.label(labels.to_be_cleared.as_str());
            let positions: Vec<String> = self.cleared.iter().map(|gone| lengths.text(*gone)).collect();
            ui.label(positions.join(", "));
        }
    }

    fn default_row(&mut self, ui: &mut Ui, labels: &TabStopsLabels, lengths: &Lengths<'_>) {
        ui.horizontal(|ui| {
            ui.label(labels.default_stops.as_str());
            let edit = lengths.field(
                ui,
                "tabs-default",
                &labels.default_stops,
                self.default_interval,
                SAME_POSITION..=MAX_POSITION,
                self.field_id("default"),
            );
            if let Some(value) = edit.event.as_ref().and_then(|event| match event {
                EditEvent::Commit { changed: false, .. } => None,
                event => event.value().copied(),
            }) {
                self.default_interval = value;
            }
        });
    }

    fn columns(labels: &TabStopsLabels) -> Vec<TableColumn<Col>> {
        let column = |key, label: &str| TableColumn::new(key, label.trim_end_matches([':', '\u{ff1a}'])).sortable(false);
        vec![column(Col::Position, &labels.position), column(Col::Alignment, &labels.alignment), column(Col::Leader, &labels.leader)]
    }

    /// The stops table; returns the edits its cells made.
    fn table(&mut self, ui: &mut Ui, labels: &TabStopsLabels, lengths: &Lengths<'_>) -> Vec<CellEdit> {
        let columns = Self::columns(labels);
        let rows: Vec<RowInfo<usize>> = (0..self.stops.len()).map(RowInfo::leaf).collect();
        let style = Table::style(&Tokens::of(ui));
        let body = self.stops.len().clamp(1, VISIBLE_ROWS) as f32 * style.row_height;
        let mut selection = self.selected.map_or_else(Selection::new, Selection::only);
        let (stops, to_edit) = (self.stops.clone(), self.to_edit.take());
        let edit_ids: Vec<Id> = (0..stops.len()).map(|at| self.field_id("position").with(at)).collect();
        let mut edits = Vec::new();
        let output = Table::new("tabs-stops", labels.title.as_str(), &columns, &mut self.layout, rows.as_slice())
            .selection(&mut selection)
            .mode(SelectionMode::Single)
            .movable(false)
            .max_height(body)
            .show(ui, |ui, &at, column, _| {
                let Some(stop) = stops.get(at) else { return };
                match column {
                    Col::Position => {
                        let id = edit_ids.get(at).copied().unwrap_or_else(|| ui.scope_id().with(("tabs-position", at)));
                        if to_edit == Some(at) {
                            ui.memory_mut(|m| m.request_focus(id));
                        }
                        let edit = lengths.field(ui, ("tabs-position", at), &labels.position, stop.position, -MAX_POSITION..=MAX_POSITION, id);
                        edits.extend(edit.event.map(|event| CellEdit::Position(at, event)));
                    }
                    Col::Alignment => {
                        let mut align = stop.align;
                        let _ = align_select(ui, at, labels, &mut align);
                        if align != stop.align {
                            edits.push(CellEdit::Align(at, align));
                        }
                    }
                    Col::Leader => {
                        let mut leader = stop.leader;
                        let _ = leader_select(ui, at, labels, &mut leader);
                        if leader != stop.leader {
                            edits.push(CellEdit::Leader(at, leader));
                        }
                    }
                }
            });
        for event in output.events {
            if let TableEvent::Selection(change) = event {
                self.select(change.selection.lead);
            }
        }
        edits
    }

    fn apply(&mut self, edit: CellEdit) {
        match edit {
            CellEdit::Position(at, event) => self.apply_position(at, event),
            CellEdit::Align(at, align) => {
                if let Some(stop) = self.stops.get_mut(at) {
                    stop.align = align;
                }
            }
            CellEdit::Leader(at, leader) => {
                if let Some(stop) = self.stops.get_mut(at) {
                    stop.leader = leader;
                }
            }
        }
    }

    /// An in-place position edit: rows keep their order until the edit ends.
    fn apply_position(&mut self, at: usize, event: EditEvent<f32>) {
        match event {
            EditEvent::Begin { value, .. } => {
                self.before = self.stops.get(at).map(|stop| (at, stop.position));
                if let Some(value) = value {
                    self.edit_position(at, value);
                }
            }
            EditEvent::Update { value, .. } => self.edit_position(at, value),
            EditEvent::Commit { value, changed, .. } => {
                if changed {
                    self.edit_position(at, value);
                }
                self.before = None;
                self.edited = self.edited.or(Some(at));
                self.normalize();
            }
            EditEvent::Set { value, .. } => {
                self.edit_position(at, value);
                self.normalize();
            }
            EditEvent::Cancel { .. } => {
                if let Some((row, position)) = self.before.take() {
                    self.edit_position(row, position);
                }
                self.normalize();
            }
            _ => {}
        }
    }

    fn buttons(&mut self, ui: &mut Ui, labels: &TabStopsLabels) {
        ui.horizontal(|ui| {
            let button =
                |ui: &mut Ui, name: &str, label: &str, enabled: bool| ui.add(Button::new(label).enabled(enabled).id_salt(("tabs", name))).clicked();
            if button(ui, "set", &labels.set, self.stops.len() < MAX_STOPS) {
                self.new_stop();
            }
            if button(ui, "clear", &labels.clear, self.selected.is_some()) {
                self.clear();
            }
            if button(ui, "clear-all", &labels.clear_all, !self.stops.is_empty()) {
                self.clear_all();
            }
        });
    }
}

fn align_select(ui: &mut Ui, at: usize, labels: &TabStopsLabels, align: &mut TabAlign) -> EditResponse<TabAlign> {
    Select::new(("tabs-align", at), labels.alignment.as_str(), align)
        .option(TabAlign::Left, labels.left.as_str())
        .option(TabAlign::Center, labels.center.as_str())
        .option(TabAlign::Right, labels.right.as_str())
        .option(TabAlign::Decimal, labels.decimal.as_str())
        .option(TabAlign::Bar, labels.bar.as_str())
        .show(ui)
}

/// The leader's options read as their fill, so the closed box shows just the fill.
fn leader_select(ui: &mut Ui, at: usize, labels: &TabStopsLabels, leader: &mut TabLeader) -> EditResponse<TabLeader> {
    let mut select = Select::new(("tabs-leader", at), labels.leader.as_str(), leader);
    for kind in TabLeader::ALL {
        let text = if kind == TabLeader::None { labels.leader_none.as_str() } else { kind.sample() };
        select = select.option(kind, text);
    }
    select.show(ui)
}
