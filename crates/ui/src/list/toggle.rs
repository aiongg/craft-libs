//! Toggle columns of a [`ListRow`](super::ListRow): an eye or lock cell per row, and the sweep.

use egui::accesskit::Role;
use egui::{Context, Id, KeyboardShortcut, Rect, Sense, Ui};

use super::row::{ListRowStyle, RowEvent};
use crate::edit::{CancelReason, EditEvent, GestureId};
use crate::icons::{self, Icon, IconSize};
use crate::primitives::Interactive;

/// One toggle column of a row: an icon when on, nothing when off (the off icon shows under the
/// pointer), clickable either way.
#[derive(Clone, Debug)]
pub struct ToggleColumn {
    pub(super) label: String,
    pub(super) on_icon: Icon,
    pub(super) off_icon: Icon,
    pub(super) on: bool,
    pub(super) key: Option<KeyboardShortcut>,
}

impl ToggleColumn {
    /// A column named `label` ("Visibility"; the button's name is the label and the row's label,
    /// "Visibility Layer 1") whose state in this row is `on`, drawn with `on_icon`, or `off_icon`
    /// under the pointer while off.
    pub fn new(label: impl Into<String>, on_icon: Icon, off_icon: Icon, on: bool) -> Self {
        Self { label: label.into(), on_icon, off_icon, on, key: None }
    }

    /// A key that toggles the column on the selected rows while a [`List`](super::List) has the
    /// focus, as one edit gesture; the host maps it to its own command where it has one
    /// (PhotoCraft's Layer › Hide Layers). The list reads it from its lead row.
    pub fn key(mut self, key: KeyboardShortcut) -> Self {
        self.key = Some(key);
        self
    }

    /// The eye column of PhotoCraft's and DesignCraft's layer rows.
    pub fn eye(label: impl Into<String>, on: bool) -> Self {
        Self::new(label, Icon::Eye, Icon::EyeOff, on)
    }

    /// The lock column.
    pub fn lock(label: impl Into<String>, on: bool) -> Self {
        Self::new(label, Icon::Lock, Icon::LockOpen, on)
    }
}

/// The value of a toggle column's edit: the row and the state it takes.
///
/// A press on a cell sets the cell to the opposite of its state and starts a sweep: every row
/// the pointer passes over with the button held takes the same state. The sweep is one gesture
/// ([`EditEvent`]): `Begin` at the press, one `Update` per row swept (the first one the pressed
/// row), `Commit` on release (the host makes one undo step), `Cancel` with Esc. A screen
/// reader's click is one `Set`. Updates carry the modifiers held, so Alt-click reports "show only
/// this layer". The `Commit` carries the pressed row's edit.
///
/// The row that took the press reports the end of the gesture, so it must keep being drawn while
/// the button is held: a [`List`](super::List) keeps drawing it out of view, and cancels the sweep
/// with [`CancelReason::Removed`] when the row leaves the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToggleEdit<K> {
    /// Index of the column in the row's [`ListRow::toggle`](super::ListRow::toggle) order.
    pub column: usize,
    /// The row.
    pub row: K,
    /// The state the row's cell takes.
    pub on: bool,
}

/// The sweep under way in one column, kept in egui memory under the column's id.
#[derive(Clone, Debug)]
struct Sweep {
    id: GestureId,
    /// The cell the press started on.
    owner: Id,
    on: bool,
    /// Cells already visited.
    seen: Vec<Id>,
}

/// What a cell needs to know about its row.
pub(super) struct Cell<'a, K> {
    pub index: usize,
    pub row_id: Id,
    pub key: &'a K,
    pub row_label: &'a str,
}

/// Senses and paints one cell in `rect`, and reports its edits.
pub(super) fn show<K: Clone>(
    ui: &mut Ui,
    cell: &Cell<'_, K>,
    column: &ToggleColumn,
    rect: Rect,
    style: &ListRowStyle,
    events: &mut Vec<RowEvent<K>>,
) {
    let id = cell.row_id.with(("toggle", cell.index));
    let name = format!("{} {}", column.label, cell.row_label);
    let sensed = Interactive::with_id(id, Role::Button, name)
        .sense(Sense::CLICK | Sense::DRAG)
        .checked(column.on)
        .state_layer(false)
        .focus_ring(false)
        .interact_at(ui, rect);
    let response = sensed.response().clone();
    let sweep_id = sweep_id(ui.scope_id(), cell.index);
    let edit = |on| ToggleEdit { column: cell.index, row: cell.key.clone(), on };
    let (pressed, down, escape, modifiers, pointer, delta) = ui.input(|i| {
        (
            i.pointer.primary_pressed(),
            i.pointer.primary_down(),
            i.key_pressed(egui::Key::Escape),
            i.modifiers,
            i.pointer.latest_pos(),
            i.pointer.delta(),
        )
    });
    let sweep: Option<Sweep> = ui.data(|d| d.get_temp::<Option<Sweep>>(sweep_id)).flatten();
    let mut ended = false;
    let mut started = false;
    let mut report = |event: EditEvent<ToggleEdit<K>>| events.push(RowEvent::Toggle(event));
    if response.is_pointer_button_down_on() && pressed && ui.is_enabled() {
        let sweep = Sweep { id: GestureId::next(), owner: id, on: !column.on, seen: vec![id] };
        report(EditEvent::begin(sweep.id));
        report(EditEvent::update(sweep.id, edit(sweep.on), modifiers));
        ui.data_mut(|d| d.insert_temp(sweep_id, Some(sweep)));
        started = true;
    } else if let Some(mut sweep) = sweep {
        if sweep.owner == id {
            if escape || !down {
                if escape {
                    report(EditEvent::cancel(sweep.id, CancelReason::Escape));
                } else {
                    report(EditEvent::commit(sweep.id, edit(sweep.on), true, modifiers));
                }
                ui.data_mut(|d| d.insert_temp(sweep_id, None::<Sweep>));
                ended = true;
            }
        } else if down && !sweep.seen.contains(&id) && passed_over(rect, pointer, delta) {
            sweep.seen.push(id);
            if column.on != sweep.on {
                report(EditEvent::update(sweep.id, edit(sweep.on), modifiers));
            }
            ui.data_mut(|d| d.insert_temp(sweep_id, Some(sweep)));
        }
    }
    // A pointer click ends a sweep on this frame or began one; anything else is a screen
    // reader's click.
    if response.clicked() && !started && !ended {
        report(EditEvent::set(edit(!column.on), modifiers));
    }
    paint(ui, sensed.finish(ui).rect, column, ui.rect_contains_pointer(rect), style);
}

/// Where the sweep of `column` among rows shown in a `Ui` whose scope is `scope` is kept: the rows
/// of one list share it.
fn sweep_id(scope: Id, column: usize) -> Id {
    scope.with(("list-row-sweep", column))
}

/// Forgets the sweep under way in `column` of the rows shown in a `Ui` whose scope is `scope`:
/// the row that took the press left the list.
pub(super) fn clear_sweep(ctx: &Context, scope: Id, column: usize) {
    ctx.data_mut(|d| d.insert_temp(sweep_id(scope, column), None::<Sweep>));
}

/// Whether the pointer's path since the last frame crossed `rect`'s column at its rows.
fn passed_over(rect: Rect, pointer: Option<egui::Pos2>, delta: egui::Vec2) -> bool {
    let Some(now) = pointer else { return false };
    let before = now.y - delta.y;
    rect.x_range().contains(now.x) && before.min(now.y) <= rect.bottom() && before.max(now.y) >= rect.top()
}

fn paint(ui: &Ui, rect: Rect, column: &ToggleColumn, hovered: bool, style: &ListRowStyle) {
    if !ui.is_rect_visible(rect) || !(column.on || hovered) {
        return;
    }
    let icon = if column.on { column.on_icon } else { column.off_icon };
    let color = if hovered { style.icon_hover } else { style.icon };
    icons::paint_in(ui, rect, icon, icons::active(ui.ctx()), style.icon_size, IconSize::Md.stroke(), color);
}
