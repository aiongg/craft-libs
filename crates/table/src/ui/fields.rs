//! The field list pane of a pivot table (N6.04).

use craft_ui::behavior::{Axis, DropPosition, ListDrag, Move, Reorder, RowInfo, Selection};
use craft_ui::controls::{Button, CheckState, ControlSize};
use craft_ui::direction;
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::icons::Icon;
use craft_ui::list::{DropMark, List, ListEvent, ListRow, RowEvent, ThumbSize, ToggleColumn, paint_drop_mark};
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt, PressMenu};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, margin};
use egui::{AsIdSalt, DragAndDrop, Frame, Id, IdSalt, Modifiers, Pos2, Response, RichText, TextStyle, Ui};

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
}

/// What the pane asks of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldListEdit {
    /// Ticked: add it to its default area.
    Add {
        field: String,
    },
    /// Unticked or Remove Field.
    Remove {
        field: String,
    },
    /// A field's menu (Move to …, Move Up, Move Down), or a drag between or inside the areas,
    /// as one gesture of the kernel's [`Move`] (the list's reorder):
    /// the app previews `Update`s and applies the `Commit`.
    Move {
        to: FieldArea,
        change: EditEvent<Move<String>>,
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
///   [`FieldListEdit::Remove`]).
/// - Each area is a `List` with [`List::reorder`](craft_ui::list::List) and the
///   [`ListDrag`](craft_ui::behavior::ListDrag) payload, so fields drag inside an area and between
///   areas with the list's drop marker and ghost; each row's context menu (and the Menu key) has
///   Move to … each other area, Move Up, Move Down and Remove Field; Cmd/Ctrl+] and [ move the
///   selected field (decisions §4.2, lists). Moves arrive as [`FieldListEdit::Move`].
/// - Keyboard: each list is one Tab stop, then the buttons; AccessKit is the lists' (`ListBox`
///   named by the area's word) and the buttons'.
pub struct FieldListPane<'a> {
    id_salt: IdSalt,
    fields: &'a [PivotField],
    areas: &'a [(FieldArea, Vec<String>)],
    labels: Option<FieldListLabels>,
}

impl<'a> FieldListPane<'a> {
    pub fn new(id_salt: impl AsIdSalt, fields: &'a [PivotField], areas: &'a [(FieldArea, Vec<String>)]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), fields, areas, labels: None }
    }

    pub fn labels(mut self, labels: FieldListLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // gridcraft: crates/ui-egui/src/pivot_pane.rs:21 `show`.
    pub fn show(self, ui: &mut Ui) -> FieldListPaneOutput {
        let Self { id_salt, fields, areas, labels } = self;
        let labels = labels.unwrap_or_else(|| FieldListLabels::of(ui));
        let tokens = Tokens::of(ui);
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(id_salt);
        let cx = Cx {
            id,
            labels: &labels,
            areas,
            // Read before any list draws: the list a drag started in clears the payload when it ends.
            drag: DragAndDrop::payload::<ListDrag<String>>(&ctx),
            released: ctx.input(|input| input.pointer.any_released()),
            pointer: ctx.pointer_latest_pos(),
            row_height: ListRow::<String>::style(&tokens).height(ThumbSize::None, false),
        };
        let mut edits = Vec::new();
        let mut pass = Pass::default();

        let response = ui
            .with_layout(direction::column(ui), |ui| {
                ui.label(RichText::new(&labels.title).strong());
                ui.label(RichText::new(&labels.choose).text_style(TextStyle::Small).color(tokens.color.text.muted.to_egui()));
                let button_height = Button::style(&tokens, craft_ui::controls::ButtonVariant::Secondary, ControlSize::Md).height;
                let footer = ui.spacing().item_spacing.y + button_height;
                checklist(&cx, ui, fields, ui.available_height() * CHECKLIST_SHARE, &mut edits);
                ui.separator();
                ui.label(RichText::new(&labels.areas_hint).text_style(TextStyle::Small).color(tokens.color.text.muted.to_egui()));
                let heading = ui.text_style_height(&TextStyle::Body) + ui.spacing().item_spacing.y;
                let framing = 2.0 * (f32::from(margin(tokens.space.pad.control).top) + ui.spacing().item_spacing.y);
                let room = (ui.available_height() - footer - 2.0 * (heading + framing)).max(0.0) / 2.0;
                let room = room.max(cx.row_height * AREA_MIN_ROWS);
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
        let mut moves = std::mem::take(&mut pass.moves);
        if let Some((to, mv, modifiers)) = pass.dropped.take() {
            // The list the drag started in ended it as dismissed; the drop makes it a move into `to`.
            let ended = moves.iter().position(|(_, event)| matches!(event, EditEvent::Cancel { reason: CancelReason::Dismissed, .. }));
            let event = match ended.and_then(|at| moves[at].1.id()) {
                Some(gesture) => EditEvent::commit(gesture, mv, true, modifiers),
                None => EditEvent::set(mv, modifiers),
            };
            match ended {
                Some(at) => moves[at] = (to, event),
                None => moves.push((to, event)),
            }
        }
        edits.extend(moves.into_iter().map(|(to, change)| FieldListEdit::Move { to, change }));
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

/// The share of the pane's height the checklist takes at most.
const CHECKLIST_SHARE: f32 = 0.4;

/// The fewest rows an area keeps room for, so an empty area can take a drop.
const AREA_MIN_ROWS: f32 = 2.0;

/// What every part of a pass reads.
struct Cx<'a> {
    id: Id,
    labels: &'a FieldListLabels,
    areas: &'a [(FieldArea, Vec<String>)],
    /// The drag of rows under way when the pass began.
    drag: Option<std::sync::Arc<ListDrag<String>>>,
    released: bool,
    pointer: Option<Pos2>,
    row_height: f32,
}

impl Cx<'_> {
    fn fields_in(&self, area: FieldArea) -> &[String] {
        self.areas.iter().find(|(a, _)| *a == area).map_or(&[], |(_, fields)| fields.as_slice())
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
    moves: Vec<(FieldArea, EditEvent<Move<String>>)>,
    /// A field's menu asked for, and the areas' lists it opens on.
    menu: Option<(MenuTarget, OpenAt)>,
    lists: Vec<(FieldArea, Response)>,
    /// A drag of fields released over an area other than the one it started in.
    dropped: Option<(FieldArea, Move<String>, Modifiers)>,
}

#[derive(Clone)]
struct MenuTarget {
    area: FieldArea,
    field: String,
}

/// The checklist of source fields.
fn checklist(cx: &Cx<'_>, ui: &mut Ui, fields: &[PivotField], max_height: f32, edits: &mut Vec<FieldListEdit>) {
    let order: Vec<RowInfo<String>> = fields.iter().map(|field| RowInfo::leaf(field.name.clone())).collect();
    let key = cx.id.with("checklist-selection");
    let mut selection = ui.ctx().data(|d| d.get_temp::<Selection<String>>(key)).unwrap_or_default();
    let output = List::new(cx.id.with("checklist"), &cx.labels.choose, order.as_slice())
        .selection(&mut selection)
        .max_height(max_height.max(cx.row_height))
        .show(ui, |_, name| {
            let state = if cx.ticked(name) { CheckState::Checked } else { CheckState::Unchecked };
            // The column's accessible name is the field's.
            ListRow::new(name.clone(), name.clone()).toggle(ToggleColumn::check_box("{row}", state))
        });
    ui.ctx().data_mut(|d| d.insert_temp(key, selection));
    for event in output.events {
        let ListEvent::Row(RowEvent::Toggle(toggle)) = event else { continue };
        let Some(edit) = toggle.value() else { continue };
        if edit.on == cx.ticked(&edit.row) {
            continue;
        }
        let edit = if edit.on { FieldListEdit::Add { field: edit.row.clone() } } else { FieldListEdit::Remove { field: edit.row.clone() } };
        if !edits.contains(&edit) {
            edits.push(edit);
        }
    }
}

/// One area: its name over a list of its fields.
fn area_list(cx: &Cx<'_>, ui: &mut Ui, tokens: &craft_ui::theme::UiTokens, area: FieldArea, max_height: f32, pass: &mut Pass) {
    let ctx = ui.ctx().clone();
    ui.label(RichText::new(cx.name(area)).strong());
    let names = cx.fields_in(area);
    let order: Vec<RowInfo<String>> = names.iter().map(|name| RowInfo::leaf(name.clone())).collect();
    let selection_key = cx.id.with(("selection", area));
    let list_id = cx.list_id(ui, area);
    let frame =
        Frame::new().fill(tokens.color.surface.sunken.to_egui()).corner_radius(tokens.radius.control).inner_margin(margin(tokens.space.pad.control));
    let framed = frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.set_min_height(cx.row_height * AREA_MIN_ROWS);
        let mut selection = ctx.data(|d| d.get_temp::<Selection<String>>(selection_key)).unwrap_or_default();
        let output = List::new(cx.id.with(("area", area)), cx.name(area), order.as_slice())
            .selection(&mut selection)
            .max_height(max_height)
            .reorder(Reorder::new(Axis::Vertical))
            .show(ui, |_, name| ListRow::new(name.clone(), name.clone()));
        (output, selection)
    });
    let (output, mut selection) = framed.inner;
    for event in output.events {
        match event {
            ListEvent::Reorder(change) => pass.moves.push((area, change)),
            ListEvent::ContextMenu(request) => {
                if let Some(field) = request.row {
                    if !request.selected {
                        selection = Selection::only(field.clone());
                    }
                    let at = request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer);
                    pass.menu = Some((MenuTarget { area, field }, at));
                }
            }
            _ => {}
        }
    }
    ctx.data_mut(|d| d.insert_temp(selection_key, selection));
    drop_from_other_area(cx, ui, area, list_id, framed.response.rect, &output.rows, pass);
    pass.lists.push((area, output.response));
}

/// A drag of fields from another area over this one: the drop marker, and the move when released.
fn drop_from_other_area(cx: &Cx<'_>, ui: &Ui, area: FieldArea, list_id: Id, bounds: egui::Rect, rows: &[(String, egui::Rect)], pass: &mut Pass) {
    let (Some(drag), Some(pointer)) = (&cx.drag, cx.pointer) else { return };
    if drag.list == list_id || !bounds.contains(pointer) {
        return;
    }
    // Between rows the drop goes before the lower one; below the last row after it; into an empty area onto itself.
    let (target, position, mark_rect, mark) = match rows.iter().find(|(_, rect)| pointer.y < rect.bottom()) {
        Some((name, rect)) if pointer.y < rect.center().y => (name.clone(), DropPosition::Before, *rect, DropMark::Before),
        Some((name, rect)) => (name.clone(), DropPosition::After, *rect, DropMark::After),
        None => match rows.last() {
            Some((name, rect)) => (name.clone(), DropPosition::After, *rect, DropMark::After),
            None => (drag.grabbed.clone(), DropPosition::After, bounds, DropMark::Into),
        },
    };
    paint_drop_mark(ui, mark_rect, mark);
    if cx.released {
        let modifiers = ui.input(|input| input.modifiers);
        pass.dropped = Some((area, Move { items: drag.items.clone(), target, position, copy: false }, modifiers));
    }
}

/// What a pick of a field's menu stands for.
#[derive(Clone, Copy)]
enum Pick {
    MoveTo(FieldArea),
    Up,
    Down,
    Remove,
}

/// A field's menu: Move to each other area, Move Up, Move Down and Remove Field. It stays open
/// over later passes, its field kept in egui memory.
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
    let names = cx.fields_in(target.area);
    let position = names.iter().position(|name| *name == target.field);
    let mut entries: Vec<MenuEntry<Pick>> = FieldArea::ALL
        .into_iter()
        .filter(|area| *area != target.area)
        .map(|area| MenuItem::new(fmt(&cx.labels.move_to, &[("area", cx.name(area))]), Pick::MoveTo(area)).into())
        .collect();
    entries.push(MenuEntry::Separator);
    entries.push(MenuItem::new(&cx.labels.move_up, Pick::Up).enabled(position.is_some_and(|at| at > 0)).into());
    entries.push(MenuItem::new(&cx.labels.move_down, Pick::Down).enabled(position.is_some_and(|at| at + 1 < names.len())).into());
    entries.push(MenuEntry::Separator);
    entries.push(MenuItem::new(&cx.labels.remove, Pick::Remove).into());
    let shown = ContextMenu::new(&entries).own_gestures(false).open_at(at).show(ui, list);
    let alone = |mv: Move<String>| EditEvent::set(mv, Modifiers::NONE);
    let edit = shown.chosen.map(|pick| {
        let field = target.field.clone();
        let step = |to: FieldArea, beside: &String, position| FieldListEdit::Move {
            to,
            change: alone(Move { items: vec![field.clone()], target: beside.clone(), position, copy: false }),
        };
        match pick {
            Pick::MoveTo(to) => match cx.fields_in(to).last() {
                Some(last) => step(to, last, DropPosition::After),
                None => step(to, &field, DropPosition::After),
            },
            Pick::Up => step(target.area, &names[position.map_or(0, |at| at.saturating_sub(1))], DropPosition::Before),
            Pick::Down => step(target.area, &names[position.map_or(0, |at| (at + 1).min(names.len() - 1))], DropPosition::After),
            Pick::Remove => FieldListEdit::Remove { field },
        }
    });
    if !(shown.open || at.is_some()) || edit.is_some() {
        ctx.data_mut(|d| d.remove::<MenuTarget>(key));
    }
    edit.into_iter().collect()
}
