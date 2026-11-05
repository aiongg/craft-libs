//! The field list pane of a pivot table (N6.04).

use craft_ui::behavior::{Axis, DropPosition, Move, Reorder, RowInfo, Selection};
use craft_ui::controls::{Button, CheckState, ControlSize};
use craft_ui::direction;
use craft_ui::edit::{EditEvent, GestureId};
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, RowEvent, ToggleColumn, ToggleEdit};
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt, PressMenu};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, margin};
use egui::{AsIdSalt, Frame, Id, IdSalt, Modifiers, Response, RichText, Ui};

use super::style::SheetStyle;

craft_ui::words! {
    /// The pane's words, English by default (GridCraft's, Excel's).
    #[non_exhaustive]
    pub struct FieldListLabels {
        title: "fieldList.title" = "PivotTable Fields";
        choose: "fieldList.choose" = "Choose fields to add to report:";
        areas_hint: "fieldList.areasHint" = "Use a field's menu to move it between areas:";
        filters: "fieldList.filters" = "Filters";
        columns: "fieldList.columns" = "Columns";
        rows: "fieldList.rows" = "Rows";
        values: "fieldList.values" = "Values";
        move_to: "fieldList.moveTo" = "Move to {area}";
        move_up: "fieldList.moveUp" = "Move Up";
        move_down: "fieldList.moveDown" = "Move Down";
        remove: "fieldList.remove" = "Remove Field";
        refresh: "fieldList.refresh" = "Refresh";
        calculated: "fieldList.calculated" = "Calculated Field…";
        layout: "fieldList.layout" = "Report Layout";
        compact: "fieldList.compact" = "Show in Compact Form";
        outline: "fieldList.outline" = "Show in Outline Form";
        tabular: "fieldList.tabular" = "Show in Tabular Form";
        values_field: "fieldList.valuesField" = "Σ Values";
        samples_separator: "fieldList.samplesSeparator" = ", ";
        summarize_sum: "fieldList.summarizeSum" = "Summarize by Sum";
        summarize_count: "fieldList.summarizeCount" = "Summarize by Count";
        summarize_average: "fieldList.summarizeAverage" = "Summarize by Average";
        summarize_max: "fieldList.summarizeMax" = "Summarize by Max";
        summarize_min: "fieldList.summarizeMin" = "Summarize by Min";
        show_as_normal: "fieldList.showAsNormal" = "Show as No Calculation";
        show_as_percent_of_grand_total: "fieldList.showAsPercentOfGrandTotal" = "Show as % of Grand Total";
        show_as_running_total: "fieldList.showAsRunningTotal" = "Show as Running Total";
        show_as_rank: "fieldList.showAsRank" = "Show as Rank";
        sort_ascending: "fieldList.sortAscending" = "Sort A to Z";
        sort_descending: "fieldList.sortDescending" = "Sort Z to A";
        group_years: "fieldList.groupYears" = "Group by Years";
        group_quarters: "fieldList.groupQuarters" = "Group by Quarters";
        group_months: "fieldList.groupMonths" = "Group by Months";
        ungroup: "fieldList.ungroup" = "Ungroup";
    }
}

/// A pivot area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldArea {
    Filters,
    Columns,
    Rows,
    Values,
}

impl FieldArea {
    /// Excel's order in the pane.
    pub const ALL: [FieldArea; 4] = [FieldArea::Filters, FieldArea::Columns, FieldArea::Rows, FieldArea::Values];
}

/// A source field: its name, whether it holds numbers (ticked, it goes to Values; text to Rows).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PivotField {
    pub name: String,
    pub numeric: bool,
    /// A few of the field's values, shown as the checklist row's tooltip (GridCraft
    /// `pivot_pane.rs:56-57`).
    pub samples: Vec<String>,
}

/// Where the "Σ Values" pseudo-field sits: the app passes it when the pivot has two or more value
/// fields, in Rows or Columns (GridCraft `pivot_pane.rs:72-83`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValuesRow {
    /// Rows or Columns.
    pub area: FieldArea,
    /// Its place among that area's fields; past the end puts it last.
    pub at: usize,
}

/// How a value field is summarized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Summary {
    Sum,
    Count,
    Average,
    Max,
    Min,
}

/// How a value field is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShowAs {
    NoCalculation,
    PercentOfGrandTotal,
    RunningTotal,
    Rank,
}

/// A change of a value field's settings: its summary or how it is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueSetting {
    Summarize(Summary),
    ShowAs(ShowAs),
}

/// The order a field's items sort in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FieldSort {
    Ascending,
    Descending,
}

/// How a date field groups its items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FieldGroup {
    Years,
    Quarters,
    Months,
    Ungroup,
}

/// What the pane asks of the app.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FieldListEdit {
    /// Ticked: add it to its default area. A tick sweep reports one `Add` or `Remove` per row it
    /// changes, however late the app applies them; Esc reports the opposite edits.
    Add {
        field: String,
    },
    /// Unticked, or Remove Field from an area's menu.
    Remove {
        field: String,
        /// The area the field's menu removed it from; `None` for a tick of the checklist, which
        /// removes it from every area (GridCraft `pivot_pane.rs:164-166`).
        from: Option<FieldArea>,
    },
    /// A field's menu (Move to …, Move Up, Move Down), or a drag between or inside the areas,
    /// as one gesture of the kernel's [`Move`] (the list's reorder):
    /// the app previews `Update`s and applies the `Commit`.
    Move {
        to: FieldArea,
        change: EditEvent<Move<String>>,
    },
    /// The "Σ Values" row moved to Rows or Columns, by its menu or a drop. The app places it.
    MoveValues {
        to: FieldArea,
    },
    /// A value field's Summarize by and Show as entries.
    ValueSettings {
        field: String,
        setting: ValueSetting,
    },
    /// A field's Sort A to Z and Z to A entries.
    Sort {
        field: String,
        order: FieldSort,
    },
    /// A field's Group by and Ungroup entries.
    Group {
        field: String,
        by: FieldGroup,
    },
    Refresh,
    CalculatedField,
    /// The Report Layout menu.
    Layout(ReportLayout),
}

/// The Report Layout menu's choices (Excel's).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReportLayout {
    Compact,
    Outline,
    Tabular,
}

/// The PivotTable Fields task pane (Excel's, as GridCraft lays it out): a field checklist, then
/// the four areas each listing its fields; Refresh, Calculated Field… and the Report Layout
/// [`PressMenu`](craft_ui::menu::PressMenu) at the bottom.
///
/// - The checklist is a kernel [`List`](craft_ui::list::List) whose rows carry a check-box
///   [`ToggleColumn`](craft_ui::list::ToggleColumn) (ticked: [`FieldListEdit::Add`], unticked:
///   [`FieldListEdit::Remove`]); a row's tooltip lists the field's samples. Its height is at most
///   `sheet.field_list_checklist_max`, each area's at least `sheet.field_list_area_min`.
/// - Each area is a `List` with [`List::reorder`](craft_ui::list::List) and the
///   [`ListDrag`](craft_ui::behavior::ListDrag) payload, so fields drag inside an area and between
///   areas ([`List::accept_from`]) with the list's drop marker and ghost; each row's context menu
///   (and the Menu key) has Move to … each other area, Move Up, Move Down, and for a value field
///   Summarize by and Show as, for another field Sort and Group, then Remove Field. The "Σ Values"
///   row ([`values_row`](FieldListPane::values_row)) moves between Rows and Columns only.
///   Moves arrive as [`FieldListEdit::Move`].
/// - Keyboard: each list is one Tab stop, then the buttons; AccessKit is the lists' (`ListBox`
///   named by the area's word) and the buttons'.
pub struct FieldListPane<'a> {
    id_salt: IdSalt,
    fields: &'a [PivotField],
    areas: &'a [(FieldArea, Vec<String>)],
    values_row: Option<ValuesRow>,
    labels: Option<FieldListLabels>,
}

impl<'a> FieldListPane<'a> {
    pub fn new(id_salt: impl AsIdSalt, fields: &'a [PivotField], areas: &'a [(FieldArea, Vec<String>)]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), fields, areas, values_row: None, labels: None }
    }

    pub fn labels(mut self, labels: FieldListLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the "Σ Values" row at `row`; the app positions it and passes it only while the pivot
    /// has two or more value fields.
    pub fn values_row(mut self, row: Option<ValuesRow>) -> Self {
        self.values_row = row;
        self
    }

    // gridcraft: crates/ui-egui/src/pivot_pane.rs:21 `show`.
    pub fn show(self, ui: &mut Ui) -> FieldListPaneOutput {
        let Self { id_salt, fields, areas, values_row, labels } = self;
        let labels = labels.unwrap_or_else(|| FieldListLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = SheetStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let cx = Cx { id, labels: &labels, areas, fields, values_row, style };
        let mut edits = Vec::new();
        let mut pass = Pass::default();

        let response = ui
            .with_layout(direction::column(ui), |ui| {
                ui.label(RichText::new(&labels.title).strong());
                let hint = tokens.font(style.hint_text);
                ui.label(RichText::new(&labels.choose).font(hint.clone()).color(tokens.color.text.muted.to_egui()));
                let button_height = Button::style(&tokens, craft_ui::controls::ButtonVariant::Secondary, ControlSize::Md).height;
                let footer = ui.spacing().item_spacing.y + button_height;
                checklist(&cx, ui, &mut edits);
                ui.separator();
                ui.label(RichText::new(&labels.areas_hint).font(hint).color(tokens.color.text.muted.to_egui()));
                let heading_font = tokens.font(style.heading_text);
                let heading = ui.fonts_mut(|fonts| fonts.row_height(&heading_font)) + ui.spacing().item_spacing.y;
                let framing = 2.0 * (f32::from(margin(tokens.space.pad.control).top) + ui.spacing().item_spacing.y);
                let room = (ui.available_height() - footer - 2.0 * (heading + framing)).max(0.0) / 2.0;
                let room = room.max(style.field_list_area_min);
                ui.columns(2, |columns| {
                    for (at, area) in FieldArea::ALL.into_iter().enumerate() {
                        let ui = &mut columns[at % 2];
                        area_list(&cx, ui, &tokens, area, room, &mut pass);
                    }
                });
                ui.add_space(ui.spacing().item_spacing.y);
                direction::horizontal(ui, |ui| {
                    if ui.add(Button::new(&labels.refresh)).clicked() {
                        edits.push(FieldListEdit::Refresh);
                    }
                    if ui.add(Button::new(&labels.calculated)).clicked() {
                        edits.push(FieldListEdit::CalculatedField);
                    }
                    let button = ui.add(Button::new(&labels.layout).trailing_icon(Icon::ChevronDown));
                    let entries: Vec<MenuEntry<ReportLayout>> = vec![
                        MenuItem::new(&labels.compact, ReportLayout::Compact).into(),
                        MenuItem::new(&labels.outline, ReportLayout::Outline).into(),
                        MenuItem::new(&labels.tabular, ReportLayout::Tabular).into(),
                    ];
                    if let Some(layout) = PressMenu::new(&entries).show(ui, &button).chosen {
                        edits.push(FieldListEdit::Layout(layout));
                    }
                });
            })
            .response;

        let menu_edits = field_menu(&cx, ui, &mut pass);
        edits.extend(std::mem::take(&mut pass.edits));
        edits.extend(menu_edits);
        FieldListPaneOutput { response, edits }
    }
}

/// What the pane did this pass.
#[must_use]
pub struct FieldListPaneOutput {
    pub response: Response,
    /// This pass's edits, in order.
    pub edits: Vec<FieldListEdit>,
}

/// A row of an area's list: a field, or the "Σ Values" pseudo-field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Slot {
    Field(String),
    Values,
}

impl Slot {
    fn field(&self) -> Option<&String> {
        match self {
            Slot::Field(name) => Some(name),
            Slot::Values => None,
        }
    }
}

/// What every part of a pass reads.
struct Cx<'a> {
    id: Id,
    labels: &'a FieldListLabels,
    areas: &'a [(FieldArea, Vec<String>)],
    fields: &'a [PivotField],
    values_row: Option<ValuesRow>,
    style: SheetStyle,
}

impl Cx<'_> {
    fn fields_in(&self, area: FieldArea) -> &[String] {
        self.areas.iter().find(|(a, _)| *a == area).map_or(&[], |(_, fields)| fields.as_slice())
    }

    /// The rows of an area's list: its fields, with the "Σ Values" row where the app put it.
    fn slots(&self, area: FieldArea) -> Vec<Slot> {
        let mut slots: Vec<Slot> = self.fields_in(area).iter().cloned().map(Slot::Field).collect();
        if let Some(row) = self.values_row.filter(|row| row.area == area) {
            slots.insert(row.at.min(slots.len()), Slot::Values);
        }
        slots
    }

    fn name(&self, area: FieldArea) -> &str {
        match area {
            FieldArea::Filters => &self.labels.filters,
            FieldArea::Columns => &self.labels.columns,
            FieldArea::Rows => &self.labels.rows,
            FieldArea::Values => &self.labels.values,
        }
    }

    fn ticked(&self, field: &str) -> bool {
        self.areas.iter().any(|(_, fields)| fields.iter().any(|f| f == field))
    }

    fn list_id(&self, ui: &Ui, area: FieldArea) -> Id {
        List::id_in(ui, self.id.with(("area", area)))
    }
}

/// What the areas collected this pass.
#[derive(Default)]
struct Pass {
    /// Moves, in the order the lists reported them.
    edits: Vec<FieldListEdit>,
    /// A field's menu asked for, and the areas' lists it opens on.
    menu: Option<(MenuTarget, OpenAt)>,
    lists: Vec<(FieldArea, Response)>,
}

#[derive(Clone)]
struct MenuTarget {
    area: FieldArea,
    slot: Slot,
}

/// A tick sweep of the checklist: the rows it has reported, with the state each was set to. Kept in
/// egui memory under the pane's id between passes of one gesture.
#[derive(Clone, Debug)]
struct Sweep {
    id: GestureId,
    reported: Vec<(String, bool)>,
}

/// The edit that sets `field` ticked or unticked.
fn tick_edit(field: String, on: bool) -> FieldListEdit {
    if on { FieldListEdit::Add { field } } else { FieldListEdit::Remove { field, from: None } }
}

/// Reports a row the sweep `id` set, once.
fn report_swept(sweep: &mut Option<Sweep>, id: GestureId, row: ToggleEdit<String>, edits: &mut Vec<FieldListEdit>) {
    let sweep = sweep.get_or_insert_with(|| Sweep { id, reported: Vec::new() });
    if !sweep.reported.iter().any(|(swept, _)| *swept == row.row) {
        sweep.reported.push((row.row.clone(), row.on));
        edits.push(tick_edit(row.row, row.on));
    }
}

/// The checklist of source fields. A sweep reports from its own set: each row once, as it is
/// swept, so an app that applies the edits late sees no repeats; Esc reports the opposite edits.
fn checklist(cx: &Cx<'_>, ui: &mut Ui, edits: &mut Vec<FieldListEdit>) {
    let order: Vec<RowInfo<String>> = cx.fields.iter().map(|field| RowInfo::leaf(field.name.clone())).collect();
    let key = cx.id.with("checklist-selection");
    let mut selection = ui.ctx().data(|d| d.get_temp::<Selection<String>>(key)).unwrap_or_default();
    let separator = &cx.labels.samples_separator;
    let output = List::new(cx.id.with("checklist"), &cx.labels.choose, order.as_slice())
        .selection(&mut selection)
        .max_height(cx.style.field_list_checklist_max)
        .show(ui, |_, name| {
            let state = if cx.ticked(name) { CheckState::Checked } else { CheckState::Unchecked };
            // The column's accessible name is the field's.
            let row = ListRow::new(name.clone(), name.clone()).toggle(ToggleColumn::check_box("{row}", state));
            match cx.fields.iter().find(|field| field.name == *name).filter(|field| !field.samples.is_empty()) {
                Some(field) => row.tooltip(field.samples.join(separator)),
                None => row,
            }
        });
    ui.ctx().data_mut(|d| d.insert_temp(key, selection));
    let sweep_key = cx.id.with("sweep");
    let mut sweep = ui.ctx().data(|d| d.get_temp::<Option<Sweep>>(sweep_key)).flatten();
    for event in output.events {
        let ListEvent::Row(RowEvent::Toggle(toggle)) = event else { continue };
        match toggle {
            EditEvent::Begin { id, .. } => sweep = Some(Sweep { id, reported: Vec::new() }),
            EditEvent::Update { id, value, .. } => report_swept(&mut sweep, id, value, edits),
            EditEvent::Commit { id, value, .. } => {
                report_swept(&mut sweep, id, value, edits);
                sweep = None;
            }
            EditEvent::Cancel { id, .. } => {
                if let Some(ended) = sweep.take_if(|sweep| sweep.id == id) {
                    edits.extend(ended.reported.into_iter().rev().map(|(row, on)| tick_edit(row, !on)));
                }
            }
            EditEvent::Set { value, .. } => edits.push(tick_edit(value.row, value.on)),
            _ => {}
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(sweep_key, sweep));
}
/// One area: its name over a list of its fields.
fn area_list(cx: &Cx<'_>, ui: &mut Ui, tokens: &craft_ui::theme::UiTokens, area: FieldArea, max_height: f32, pass: &mut Pass) {
    let ctx = ui.ctx().clone();
    ui.label(RichText::new(cx.name(area)).strong());
    let slots = cx.slots(area);
    let order: Vec<RowInfo<Slot>> = slots.iter().cloned().map(RowInfo::leaf).collect();
    let selection_key = cx.id.with(("selection", area));
    let frame =
        Frame::new().fill(tokens.color.surface.sunken.to_egui()).corner_radius(tokens.radius.control).inner_margin(margin(tokens.space.pad.control));
    let framed = frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.set_min_height(cx.style.field_list_area_min);
        let mut selection = ctx.data(|d| d.get_temp::<Selection<Slot>>(selection_key)).unwrap_or_default();
        // Every area's frame is a child of its column with the same scope, so the lists' ids differ only by their salts.
        let others: Vec<Id> = FieldArea::ALL.into_iter().filter(|other| *other != area).map(|other| cx.list_id(ui, other)).collect();
        // The "Σ Values" row goes only to Rows and Columns.
        let values_here = matches!(area, FieldArea::Rows | FieldArea::Columns);
        let reorder =
            Reorder::new(Axis::Vertical).accepts(move |mv: &Move<Slot>, _| values_here || mv.items.iter().any(|slot| *slot != Slot::Values));
        let output = List::new(cx.id.with(("area", area)), cx.name(area), order.as_slice())
            .selection(&mut selection)
            .max_height(max_height)
            .reorder(reorder)
            .accept_from(others)
            .show(ui, |_, slot| match slot {
                Slot::Field(name) => ListRow::new(slot.clone(), name.clone()),
                Slot::Values => ListRow::new(slot.clone(), cx.labels.values_field.clone()),
            });
        (output, selection)
    });
    let (output, mut selection) = framed.inner;
    let modifiers = ui.input(|input| input.modifiers);
    for event in output.events {
        match event {
            ListEvent::Reorder(change) => pass.edits.extend(reordered(&slots, area, change)),
            ListEvent::Dropped { from, target } => {
                // The source list ended its drag as dismissed; this drop is the move. An area
                // without rows reports no target: the fields go into it, beside the grabbed one.
                let mv = target.or_else(|| {
                    slots.is_empty().then(|| Move {
                        items: from.items.clone(),
                        target: from.grabbed.clone(),
                        position: DropPosition::After,
                        copy: false,
                    })
                });
                if let Some(mv) = mv {
                    pass.edits.extend(reordered(&slots, area, EditEvent::set(mv, modifiers)));
                }
            }
            ListEvent::ContextMenu(request) => {
                if let Some(slot) = request.row {
                    if !request.selected {
                        selection = Selection::only(slot.clone());
                    }
                    let at = request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer);
                    pass.menu = Some((MenuTarget { area, slot }, at));
                }
            }
            _ => {}
        }
    }
    ctx.data_mut(|d| d.insert_temp(selection_key, selection));
    pass.lists.push((area, output.response));
}

/// The edit a move of an area's rows stands for: a [`FieldListEdit::Move`] of its fields, or
/// [`FieldListEdit::MoveValues`] when only the "Σ Values" row moved (reported when the move ends).
fn reordered(slots: &[Slot], area: FieldArea, change: EditEvent<Move<Slot>>) -> Option<FieldListEdit> {
    let Some(mv) = change.value() else {
        // A `Begin` without a value and a `Cancel` carry nothing to convert.
        return Some(FieldListEdit::Move { to: area, change: change.map(|_| unreachable!("an event without a value converts nothing")) });
    };
    match fields_move(slots, mv) {
        Some(fields) => Some(FieldListEdit::Move { to: area, change: change.map(|_| fields) }),
        None => matches!(change, EditEvent::Commit { .. } | EditEvent::Set { .. }).then_some(FieldListEdit::MoveValues { to: area }),
    }
}

/// `mv` without the "Σ Values" row, its target a field: a drop beside the "Σ Values" row lands
/// beside the field next to it (the app places the row). `None` when only the row moved.
fn fields_move(slots: &[Slot], mv: &Move<Slot>) -> Option<Move<String>> {
    let items: Vec<String> = mv.items.iter().filter_map(|slot| slot.field().cloned()).collect();
    let first = items.first()?.clone();
    let (target, position) = match &mv.target {
        Slot::Field(name) => (name.clone(), mv.position),
        Slot::Values => {
            let at = slots.iter().position(|slot| *slot == Slot::Values).unwrap_or(slots.len());
            let before = slots[..at].iter().rev().find_map(Slot::field);
            let after = slots.get(at + 1..).unwrap_or_default().iter().find_map(Slot::field);
            match (mv.position, before, after) {
                (DropPosition::Before, Some(field), _) => (field.clone(), DropPosition::After),
                (DropPosition::Before, None, Some(field)) | (DropPosition::After, _, Some(field)) => (field.clone(), DropPosition::Before),
                (DropPosition::After, Some(field), None) => (field.clone(), DropPosition::After),
                _ => (first, DropPosition::After),
            }
        }
    };
    Some(Move { items, target, position, copy: mv.copy })
}

/// What a pick of a field's menu stands for.
#[derive(Clone, Copy)]
enum Pick {
    MoveTo(FieldArea),
    Up,
    Down,
    Summarize(Summary),
    ShowAs(ShowAs),
    Sort(FieldSort),
    Group(FieldGroup),
    Remove,
}

/// A field's menu (GridCraft `pivot_pane.rs:117-157`): Move to each other area, Move Up and Move
/// Down, then for a value field Summarize by and Show as, for any other Sort and Group, then Remove
/// Field. The "Σ Values" row's menu has only Move to Rows and Columns. It stays open over later
/// passes, its field kept in egui memory.
fn field_menu(cx: &Cx<'_>, ui: &mut Ui, pass: &mut Pass) -> Vec<FieldListEdit> {
    let ctx = ui.ctx().clone();
    let key = cx.id.with("menu");
    let mut at = None;
    if let Some((target, place)) = pass.menu.take() {
        ctx.data_mut(|d| d.insert_temp(key, target));
        at = Some(place);
    }
    let Some(target) = ctx.data(|d| d.get_temp::<MenuTarget>(key)) else { return Vec::new() };
    let Some((_, list)) = pass.lists.iter().find(|(area, _)| *area == target.area) else { return Vec::new() };
    let labels = cx.labels;
    let names = cx.fields_in(target.area);
    let entries: Vec<MenuEntry<Pick>> = match &target.slot {
        Slot::Values => [FieldArea::Rows, FieldArea::Columns]
            .into_iter()
            .filter(|area| *area != target.area)
            .map(|area| MenuItem::new(fmt(&labels.move_to, &[("area", cx.name(area))]), Pick::MoveTo(area)).into())
            .collect(),
        Slot::Field(field) => {
            let position = names.iter().position(|name| name == field);
            let item = |label: &str, pick| -> MenuEntry<Pick> { MenuItem::new(label, pick).into() };
            let mut entries: Vec<MenuEntry<Pick>> = FieldArea::ALL
                .into_iter()
                .filter(|area| *area != target.area)
                .map(|area| MenuItem::new(fmt(&labels.move_to, &[("area", cx.name(area))]), Pick::MoveTo(area)).into())
                .collect();
            entries.push(MenuEntry::Separator);
            entries.push(MenuItem::new(&labels.move_up, Pick::Up).enabled(position.is_some_and(|at| at > 0)).into());
            entries.push(MenuItem::new(&labels.move_down, Pick::Down).enabled(position.is_some_and(|at| at + 1 < names.len())).into());
            entries.push(MenuEntry::Separator);
            if target.area == FieldArea::Values {
                entries.extend([
                    item(&labels.summarize_sum, Pick::Summarize(Summary::Sum)),
                    item(&labels.summarize_count, Pick::Summarize(Summary::Count)),
                    item(&labels.summarize_average, Pick::Summarize(Summary::Average)),
                    item(&labels.summarize_max, Pick::Summarize(Summary::Max)),
                    item(&labels.summarize_min, Pick::Summarize(Summary::Min)),
                    item(&labels.show_as_normal, Pick::ShowAs(ShowAs::NoCalculation)),
                    item(&labels.show_as_percent_of_grand_total, Pick::ShowAs(ShowAs::PercentOfGrandTotal)),
                    item(&labels.show_as_running_total, Pick::ShowAs(ShowAs::RunningTotal)),
                    item(&labels.show_as_rank, Pick::ShowAs(ShowAs::Rank)),
                ]);
            } else {
                entries.extend([
                    item(&labels.sort_ascending, Pick::Sort(FieldSort::Ascending)),
                    item(&labels.sort_descending, Pick::Sort(FieldSort::Descending)),
                    item(&labels.group_years, Pick::Group(FieldGroup::Years)),
                    item(&labels.group_quarters, Pick::Group(FieldGroup::Quarters)),
                    item(&labels.group_months, Pick::Group(FieldGroup::Months)),
                    item(&labels.ungroup, Pick::Group(FieldGroup::Ungroup)),
                ]);
            }
            entries.push(MenuEntry::Separator);
            entries.push(item(&labels.remove, Pick::Remove));
            entries
        }
    };
    let shown = ContextMenu::new(&entries).own_gestures(false).open_at(at).show(ui, list);
    let alone = |mv: Move<String>| EditEvent::set(mv, Modifiers::NONE);
    let edit = shown.chosen.map(|pick| match &target.slot {
        Slot::Values => match pick {
            Pick::MoveTo(to) => Some(FieldListEdit::MoveValues { to }),
            _ => None,
        },
        Slot::Field(field) => {
            let field = field.clone();
            let position = names.iter().position(|name| *name == field);
            let step = |to: FieldArea, beside: &String, position| FieldListEdit::Move {
                to,
                change: alone(Move { items: vec![field.clone()], target: beside.clone(), position, copy: false }),
            };
            Some(match pick {
                Pick::MoveTo(to) => match cx.fields_in(to).last() {
                    Some(last) => step(to, last, DropPosition::After),
                    None => step(to, &field, DropPosition::After),
                },
                Pick::Up => step(target.area, &names[position.map_or(0, |at| at.saturating_sub(1))], DropPosition::Before),
                Pick::Down => step(target.area, &names[position.map_or(0, |at| (at + 1).min(names.len() - 1))], DropPosition::After),
                Pick::Summarize(summary) => FieldListEdit::ValueSettings { field, setting: ValueSetting::Summarize(summary) },
                Pick::ShowAs(show) => FieldListEdit::ValueSettings { field, setting: ValueSetting::ShowAs(show) },
                Pick::Sort(order) => FieldListEdit::Sort { field, order },
                Pick::Group(by) => FieldListEdit::Group { field, by },
                Pick::Remove => FieldListEdit::Remove { field, from: Some(target.area) },
            })
        }
    });
    if !(shown.open || at.is_some()) || edit.is_some() {
        ctx.data_mut(|d| d.remove::<MenuTarget>(key));
    }
    edit.flatten().into_iter().collect()
}
