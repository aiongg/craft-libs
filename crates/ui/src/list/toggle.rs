//! Toggle columns of a [`ListRow`](super::ListRow): an eye or lock cell per row, and the sweep.

use craft_theme::Srgba;
use egui::accesskit::Role;
use egui::{Id, KeyboardShortcut, Rect, Response, Sense, Ui};

use super::row::{ListRowStyle, RowEvent};
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, Gesture, GestureId};
use crate::icons::{self, Icon, IconSize};
use crate::primitives::Interactive;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

/// One toggle column of a row: an icon when on, nothing when off (the off icon shows under the
/// pointer), clickable either way.
#[derive(Clone, Debug)]
pub struct ToggleColumn {
    pub(super) name: String,
    pub(super) on_icon: Icon,
    pub(super) off_icon: Icon,
    pub(super) on: bool,
    pub(super) key: Option<KeyboardShortcut>,
    pub(super) tint: Option<Srgba>,
}

impl ToggleColumn {
    /// A column whose state in this row is `on`, drawn with `on_icon`, or `off_icon` under the
    /// pointer while off. `name` is the cell's name for a screen reader, translated by the host as
    /// a whole pattern with `{row}` for the row's label: "Visibility {row}" (PhotoCraft).
    pub fn new(name: impl Into<String>, on_icon: Icon, off_icon: Icon, on: bool) -> Self {
        Self { name: name.into(), on_icon, off_icon, on, key: None, tint: None }
    }

    /// A key that toggles the column on the selected rows while a [`List`](super::List) has the
    /// focus, as one edit gesture; the host maps it to its own command where it has one
    /// (PhotoCraft's Layer › Hide Layers). The list reads it from its lead row.
    pub fn key(mut self, key: KeyboardShortcut) -> Self {
        self.key = Some(key);
        self
    }

    /// Fills the cell with a label colour, as Photoshop's Layers panel fills the eye cell of a
    /// layer that has one. The fill and the icon on it are the menu's tint tokens
    /// (`menu.tint.<hue>`) for PhotoCraft's seven label colours, else the same derivation for
    /// the colour, so the icon keeps the text contrast on the fill.
    pub fn tint(mut self, color: impl Into<Srgba>) -> Self {
        self.tint = Some(color.into());
        self
    }

    /// The eye column of PhotoCraft's and DesignCraft's layer rows, `name` as in
    /// [`new`](ToggleColumn::new).
    pub fn eye(name: impl Into<String>, on: bool) -> Self {
        Self::new(name, Icon::Eye, Icon::EyeOff, on)
    }

    /// The lock column, `name` as in [`new`](ToggleColumn::new).
    pub fn lock(name: impl Into<String>, on: bool) -> Self {
        Self::new(name, Icon::Lock, Icon::LockOpen, on)
    }
}

/// The value of a toggle column's edit: the row and the state it takes.
///
/// A press on a cell sets the cell to the opposite of its state and starts a sweep: every row
/// the pointer passes over with the button held takes the same state. The sweep is one edit
/// gesture ([`EditEvent`], run by an [`edit::Gesture`](Gesture), so
/// [`GestureHost`](crate::edit::GestureHost) takes it): `Begin` at the press, one `Update` per
/// row swept (the first one the pressed row), `Commit` on release (the host makes one undo step),
/// `Cancel` with Esc or [`cancel_all`](crate::edit::cancel_all). A screen reader's click is one
/// `Set`. Updates carry the modifiers held, so Alt-click reports "show only this layer". The
/// `Commit` carries the pressed row's edit.
///
/// The row that took the press keeps the gesture live and reports its end, so it must keep being
/// drawn while the button is held: a [`List`](super::List) keeps drawing it out of view, and
/// cancels the sweep with [`CancelReason::Removed`] when the row leaves the list. A pass that
/// doesn't draw it ends the sweep as [`edit::is_live`](crate::edit::is_live) says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToggleEdit<K> {
    /// Index of the column in the row's [`ListRow::toggle`](super::ListRow::toggle) order.
    pub column: usize,
    /// The row.
    pub row: K,
    /// The state the row's cell takes.
    pub on: bool,
}

/// The sweep under way in one column, kept in egui memory under the column's id. Its lifecycle
/// is the column's [`Gesture`] ([`sweep_gesture`]).
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

/// Senses and paints one cell in `rect`, reports its edits, and returns the cell's response.
pub(super) fn show<K: Clone>(
    ui: &mut Ui,
    cell: &Cell<'_, K>,
    column: &ToggleColumn,
    rect: Rect,
    style: &ListRowStyle,
    events: &mut Vec<RowEvent<K>>,
) -> Response {
    let id = cell.row_id.with(("toggle", cell.index));
    let name = super::tree::with_row(&column.name, cell.row_label);
    let sensed = Interactive::with_id(id, Role::Button, name)
        .sense(Sense::CLICK | Sense::DRAG)
        .checked(column.on)
        .state_layer(false)
        .focus_ring(false)
        .interact_at(ui, rect);
    let response = sensed.response().clone();
    let scope = ui.scope_id();
    let edit = |on| ToggleEdit { column: cell.index, row: cell.key.clone(), on };
    let (pressed, down, modifiers, pointer, delta) =
        ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down(), i.modifiers, i.pointer.latest_pos(), i.pointer.delta()));
    let sweep: Option<Sweep> = ui.data(|d| d.get_temp::<Option<Sweep>>(sweep_id(scope, cell.index))).flatten();
    let press = response.is_pointer_button_down_on() && pressed && ui.is_enabled();
    let mut report = |event: EditEvent<ToggleEdit<K>>| events.push(RowEvent::Toggle(event));
    let mut owned = false;
    if press || sweep.as_ref().is_some_and(|s| s.owner == id) {
        owned = true;
        let mut gesture = sweep_gesture(ui, scope, cell.index);
        let mut kept = None;
        let mut swept = None;
        let on = match &sweep {
            Some(sweep) if !press => sweep.on,
            _ => !column.on,
        };
        if press {
            gesture.begin();
            gesture.update(1, modifiers);
            if let Some(gesture_id) = gesture.id() {
                kept = Some(Sweep { id: gesture_id, owner: id, on, seen: vec![id] });
                swept = Some(EditEvent::update(gesture_id, edit(on), modifiers));
            }
        } else if gesture.is_active() {
            // Esc, `cancel_all` and a pass that didn't draw this row end it in `load`.
            if down {
                kept = sweep;
            } else {
                gesture.commit();
            }
        }
        if let Some(event) = sweep_event(gesture.finish(), edit(on)) {
            report(event);
        }
        if let Some(update) = swept {
            report(update);
        }
        ui.data_mut(|d| d.insert_temp(sweep_id(scope, cell.index), kept));
    } else if let Some(mut sweep) = sweep
        && down
        && !sweep.seen.contains(&id)
        && passed_over(rect, pointer, delta)
    {
        sweep.seen.push(id);
        if column.on != sweep.on {
            report(EditEvent::update(sweep.id, edit(sweep.on), modifiers));
        }
        ui.data_mut(|d| d.insert_temp(sweep_id(scope, cell.index), Some(sweep)));
    }
    // A pointer click takes part in a sweep; anything else is a screen reader's click.
    if response.clicked() && !owned {
        report(match edit::in_scope(ui) {
            true => EditEvent::update(GestureId::next(), edit(!column.on), modifiers),
            false => EditEvent::set(edit(!column.on), modifiers),
        });
    }
    let hovered = ui.rect_contains_pointer(rect);
    let response = sensed.finish(ui);
    paint(ui, response.rect, column, hovered, style);
    response
}

/// Where the sweep of `column` among rows shown in a `Ui` whose scope is `scope` is kept, and its
/// gesture's id: the rows of one list share it.
fn sweep_id(scope: Id, column: usize) -> Id {
    scope.with(("list-row-sweep", column))
}

/// The gesture of a sweep in `column`, which the pressed row loads every pass. Its value counts
/// the rows the sweep set, which no update reports: the rows' edits go out as updates of their
/// own with its id, several in a pass when the pointer crosses several rows.
fn sweep_gesture(ui: &Ui, scope: Id, column: usize) -> Gesture<u32> {
    let policy = EditPolicy::default().on_release_only(true);
    Gesture::load(ui, sweep_id(scope, column), &0, policy).focus_owners(&[]).enabled(ui.is_enabled())
}

/// The event of a sweep's gesture as the column reports it: the end carries `edit`, the pressed
/// row's edit.
fn sweep_event<K>(event: Option<EditEvent<u32>>, edit: ToggleEdit<K>) -> Option<EditEvent<ToggleEdit<K>>> {
    match event? {
        EditEvent::Begin { id } => Some(EditEvent::Begin { id }),
        EditEvent::Commit { id, changed, modifiers, .. } => Some(EditEvent::Commit { id, value: edit, changed, modifiers }),
        EditEvent::Cancel { id, reason } => Some(EditEvent::Cancel { id, reason }),
        // Inside an edit scope only the rows' updates go out.
        EditEvent::Update { .. } | EditEvent::Set { .. } => None,
    }
}

/// Whether the sweep in `column` of the rows shown in a `Ui` whose scope is `scope` is under way:
/// its row loaded its gesture in this pass or the last.
pub(super) fn sweeping(ui: &Ui, scope: Id, column: usize) -> bool {
    Gesture::<u32>::reported(ui, sweep_id(scope, column)).is_some()
}

/// Cancels the sweep under way in `column` of the rows shown in a `Ui` whose scope is `scope`:
/// the row that took the press left the list.
pub(super) fn cancel_sweep<K>(ui: &Ui, scope: Id, column: usize) -> Option<EditEvent<ToggleEdit<K>>> {
    let mut gesture = sweep_gesture(ui, scope, column);
    gesture.cancel(CancelReason::Removed);
    ui.data_mut(|d| d.insert_temp(sweep_id(scope, column), None::<Sweep>));
    match gesture.finish()? {
        EditEvent::Cancel { id, reason } => Some(EditEvent::Cancel { id, reason }),
        _ => None,
    }
}

/// Whether the pointer's path since the last frame crossed `rect`'s column at its rows.
fn passed_over(rect: Rect, pointer: Option<egui::Pos2>, delta: egui::Vec2) -> bool {
    let Some(now) = pointer else { return false };
    let before = now.y - delta.y;
    rect.x_range().contains(now.x) && before.min(now.y) <= rect.bottom() && before.max(now.y) >= rect.top()
}

fn paint(ui: &Ui, rect: Rect, column: &ToggleColumn, hovered: bool, style: &ListRowStyle) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let tint = column.tint.map(|color| crate::menu::row::tint_colors(&Tokens::of(ui), color));
    if let Some(tint) = tint {
        ui.painter().rect_filled(rect, 0.0, tint.fill.to_egui());
    }
    if !(column.on || hovered) {
        return;
    }
    let icon = if column.on { column.on_icon } else { column.off_icon };
    let color = match tint {
        Some(tint) => tint.text.to_egui(),
        None if hovered => style.icon_hover,
        None => style.icon,
    };
    icons::paint_in(ui, rect, icon, icons::active(ui.ctx()), style.icon_size, IconSize::Md.stroke(), color);
}
