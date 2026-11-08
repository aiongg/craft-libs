//! The item step's row work beyond selection by row: rows with focusable children (17.5), cell selection with a
//! lead column (17.7) and auto-height rows measured into `LayoutMemory` (17.3, I1).

use super::{ItemsHooks, key_hash};
use crate::contract::{Collection, Cx, SelectMode};
use crate::runtime::egui::memory as store;
use crate::runtime::memory::sub;
use crate::runtime::{focus, layout_memory};
use crate::style::Flags;
use craft_behavior::{CellAt, CellDirection, CellSpace, ItemLayout, MoveMods, NavMove, NavShape, RowAt, SelectionChange, SelectionModel};
use egui::{Context, Id, Vec2};
use std::sync::Arc;

/// The roving group of the children and parts of the item `item`'s row.
pub(super) fn row_group(item: Id) -> Id {
    sub(item, "row_children")
}

/// The stop focus is on when it is a child or part of the row group `group`.
pub(super) fn child_focus(ctx: &Context, group: Id) -> Option<focus::StopKey> {
    let key = focus::focused_stop(ctx)?;
    (focus::stop_rec(ctx, key)?.stop.group == Some(group)).then_some(key)
}

/// The `SELECTED` and `LEAD` flags of the visible cells of a row, by column position.
#[derive(Clone, Default)]
struct CellFlags(Arc<[(u64, Flags)]>);

pub(super) fn put_cell_flags(ctx: &Context, item: Id, flags: Vec<(u64, Flags)>) {
    store::put(ctx, sub(item, "cell_flags"), CellFlags(flags.into()));
}

/// The flags the item step wrote for column position `column` of the row `item`.
pub(in crate::contract) fn cell_flags(ctx: &Context, item: Id, column: u64) -> Flags {
    let Some(CellFlags(flags)) = store::peek_clone::<CellFlags>(ctx, sub(item, "cell_flags")) else { return Flags::empty() };
    flags.iter().find(|(at, _)| *at == column).map_or(Flags::empty(), |(_, flags)| *flags)
}

/// The id a row's measured height is kept under.
pub(super) fn measured_id<K: std::hash::Hash>(collection: Id, key: &K) -> Id {
    sub(collection, ("row_size", key_hash(key)))
}

/// Runs `f` with a reader of the heights the last pass measured for auto rows.
pub(super) fn measured_heights<R>(ctx: &Context, f: impl FnOnce(&dyn Fn(Id) -> Option<f32>) -> R) -> R {
    layout_memory::peeking(ctx, |peek| f(&|id| peek(id).map(|size| size.y)))
}

/// The id an auto-width item's measured widths (ideal, then minimum) are kept under.
pub(super) fn measured_width_id<K: std::hash::Hash>(collection: Id, key: &K) -> Id {
    sub(collection, ("item_width", key_hash(key)))
}

/// Runs `f` with a reader of the `(ideal, min)` widths the last pass measured for auto-width items.
pub(super) fn measured_widths<R>(ctx: &Context, f: impl FnOnce(&dyn Fn(Id) -> Option<(f32, f32)>) -> R) -> R {
    layout_memory::peeking(ctx, |peek| f(&|id| peek(id).map(|size| (size.x, size.y))))
}

/// Records the size an auto row measured this pass. The row is placed by it in the same pass, so the entry is
/// never read for a placement and a change asks for no discard.
pub(super) fn record_measure(ctx: &Context, id: Id, size: Vec2) {
    layout_memory::record(ctx, id, size);
}

/// The widest ideal row width the last pass measured for a content-sized collection, read as a placement: a change
/// at `record_widest` asks for the one discard pass.
pub(super) fn widest_row(ctx: &Context, collection: Id) -> Option<f32> {
    layout_memory::read(ctx, sub(collection, "row_width")).map(|size| size.x)
}

/// Records the widest ideal width of the rows this pass built.
pub(super) fn record_widest(ctx: &Context, collection: Id, widest: f32) {
    layout_memory::record(ctx, sub(collection, "row_width"), Vec2::new(widest, 0.0));
}

impl<C: Collection> ItemsHooks<'_, '_, C> {
    /// The row-child move of 17.5, tried before the navigation core: Forward on the lead row enters its first
    /// focusable child or part. A closed parent expands instead, a row with no children leaves the move to the
    /// core (an open parent then moves to its first child row), and so does every other move. `true` when focus
    /// moved in and the move is taken.
    pub(super) fn row_child_move(&self, cx: &Cx<'_, C>, layout: &C::ItemLayout, mv: NavMove, mods: MoveMods) -> bool {
        if !is_forward(mv, mods) || matches!(layout.nav(), NavShape::Cell) {
            return false;
        }
        let Some(at) = self.lead.key.as_ref().and_then(|key| self.index_of(key)) else { return false };
        if self.source.has_children(at) && !self.source.open(at) {
            return false;
        }
        let item = sub(cx.core.id, ("key", key_hash(&self.source.key(at))));
        let group = row_group(item);
        focus::row_has_stops(&cx.core.ctx, group) && focus::enter_row(&cx.core.ctx, group)
    }

    /// Whether the row at position `at` is selected.
    pub(super) fn holds(&self, at: usize) -> bool {
        self.holds_in(&self.selection, at)
    }

    /// Whether `key`, standing at position `at`, is selected. `key` need not be in the source: a selection of ranges
    /// answers for the position.
    pub(super) fn holds_key(&self, key: &C::Key, at: usize) -> bool {
        self.selection.contains(RowAt::new(key, at, self.source))
    }

    /// Whether the row at position `at` is selected in `selection`.
    pub(super) fn holds_in(&self, selection: &C::Selection, at: usize) -> bool {
        let key = self.source.key(at);
        selection.contains(RowAt::new(&key, at, self.source))
    }

    /// A move of the lead in a sheet (17.7): the lead cell moves to `row` and `column`. The selection follows
    /// unless the lead moves alone (Ctrl): a step of one cell runs the collection's cell rules as an arrow
    /// (`step`, or `extend` from the head with Shift), any other target as a click (`select`, or `extend_to` with
    /// Shift). The lead goes where the rules put the active cell, or the head while the range extends.
    pub(super) fn cell_move(&mut self, comp: &C, layout: &C::ItemLayout, row: usize, column: usize, mods: MoveMods) {
        let from = self.lead.key.clone();
        let Some(mut cells) = self.selection.cells_mut().map(|cells| cells.clone()) else {
            if self.lead.key.as_ref() != Some(&self.source.key(row)) {
                self.move_lead(row, super::LeadSource::Keys);
            }
            self.lead.column = column;
            if !mods.lead {
                let anchor = if mods.extend { self.anchor_at(row) } else { row };
                self.change(SelectionChange::Set(self.range(anchor, row)));
            }
            return;
        };
        let (mut at, mut across) = (row, column);
        let mut change = None;
        if !mods.lead {
            let source = self.source;
            let hidden = |row: u32| !layout.shown(row as usize);
            let space = comp.cell_space(CellSpace::new(source, layout.column_count() as u32).hidden_rows(&hidden));
            let target = CellAt::new(source.key(row), column as u32);
            let target_at = CellAt::new(row as u32, column as u32);
            let moved = if mods.extend {
                if cells.anchor_cell().is_none()
                    && let Some(from) = from.as_ref()
                {
                    cells.select(CellAt::new(from.clone(), self.lead.anchor_column as u32), &space);
                }
                match cells.head(&space).and_then(|head| one_step(head, target_at)) {
                    Some(direction) => cells.extend(direction, &space),
                    None => cells.extend_to(target, &space),
                }
            } else {
                match cells.active().and_then(|active| space.place(active)).and_then(|active| one_step(active, target_at)) {
                    Some(direction) => cells.step(direction, &space),
                    None => cells.select(target, &space),
                }
            };
            let end = if mods.extend { cells.head(&space) } else { cells.active().and_then(|active| space.place(active)) };
            if let Some(end) = end {
                (at, across) = (end.row as usize, end.column as usize);
            }
            change = moved.then(|| cells.change());
        }
        if self.lead.key.as_ref() != Some(&self.source.key(at)) {
            self.move_lead(at, super::LeadSource::Keys);
        }
        self.lead.column = across;
        if !mods.extend {
            self.lead.anchor_column = across;
        }
        if let Some(change) = change {
            self.change(change);
        }
    }

    /// A press on row `at` at `column`: the lead cell goes there and the selection follows the modifiers. A press
    /// selects the cell, Shift extends from the anchor cell, and Cmd/Ctrl adds a range at an unselected cell or
    /// takes a selected one out.
    pub(super) fn cell_press(&mut self, comp: &C, layout: &C::ItemLayout, at: usize, column: usize, extend: bool, toggle: bool) {
        self.lead.column = column;
        if !extend {
            self.lead.anchor_column = column;
        }
        let key = self.source.key(at);
        let Some(mut cells) = self.selection.cells_mut().map(|cells| cells.clone()) else {
            if toggle {
                self.change(SelectionChange::Toggle(key));
            } else {
                let anchor = if extend { self.anchor_at(self.lead.at) } else { at };
                self.change(SelectionChange::Set(self.range(anchor, at)));
            }
            return;
        };
        let target = CellAt::new(key, column as u32);
        if toggle && self.selection.contains_cell(RowAt::new(&target.row, at, self.source), target.column) {
            self.change(SelectionChange::ToggleCell(target));
            return;
        }
        let source = self.source;
        let hidden = |row: u32| !layout.shown(row as usize);
        let space = comp.cell_space(CellSpace::new(source, layout.column_count() as u32).hidden_rows(&hidden));
        let changed = if toggle {
            cells.add(target, &space)
        } else if extend {
            cells.extend_to(target, &space)
        } else {
            cells.select(target, &space)
        };
        if changed {
            self.change(cells.change());
        }
    }

    /// The flags of the visible cells of the row `at`: `SELECTED` for a selected cell and `LEAD` for the lead
    /// cell while the collection has focus. `columns` are the column positions of the row's cells in order.
    pub(super) fn cell_flag_list(&self, at: usize, columns: &[u64], focused: bool) -> Vec<(u64, Flags)> {
        let key = self.source.key(at);
        let lead = self.lead.key.as_ref() == Some(&key);
        let row = RowAt::new(&key, at, self.source);
        columns
            .iter()
            .filter_map(|&column| {
                let mut flags = Flags::empty();
                flags.set(Flags::SELECTED, self.selection.contains_cell(row, column as u32));
                flags.set(Flags::LEAD, lead && focused && column as usize == self.lead.column);
                (!flags.is_empty()).then_some((column, flags))
            })
            .collect()
    }

    /// Whether the collection selects cells.
    pub(super) fn selects_cells(comp: &C) -> bool {
        Self::select_mode(comp) == SelectMode::Cells
    }
}

/// Whether the move is the one that enters a row's children.
fn is_forward(mv: NavMove, mods: MoveMods) -> bool {
    mv == NavMove::Forward && !mods.extend && !mods.lead
}

/// The direction of `to` when it is the cell next to `from` along a row or a column.
fn one_step(from: CellAt<u32>, to: CellAt<u32>) -> Option<CellDirection> {
    match (i64::from(to.row) - i64::from(from.row), i64::from(to.column) - i64::from(from.column)) {
        (-1, 0) => Some(CellDirection::Up),
        (1, 0) => Some(CellDirection::Down),
        (0, -1) => Some(CellDirection::Left),
        (0, 1) => Some(CellDirection::Right),
        _ => None,
    }
}
