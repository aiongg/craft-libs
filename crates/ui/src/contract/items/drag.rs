//! Drags in a collection (17.3 step 9, 17.4): toggle-column sweeps, row drags over the drag session (8.4) and the
//! item move of a `FreeLayout`.

use super::ItemsHooks;
use crate::contract::collection::{CollectionEvent, CollectionProps, ColumnKind, ListDrag, ReorderMove, ToggleEdit};
use crate::contract::{Collection, Cx, DragPreview, DropMark, DropTarget, Pointer as _};
use crate::runtime::env::Label;
use craft_behavior::{
    CancelReason, ColumnKey, DropOperation, EditEvent, Flow, GestureId, ItemLayout, Mixed, NavShape, PointerButton, PressCancel, PressPhase,
    ScrollPos, SelectionModel,
};
use egui::{Pos2, Rect, Vec2};

/// A toggle sweep: the value it sets and the rows it has crossed.
#[derive(Clone)]
struct Sweep {
    column: ColumnKey,
    on: bool,
    crossed: Vec<usize>,
}

/// What a collection keeps of the drags it runs, in session memory.
#[derive(Clone)]
pub(super) struct Drags<K> {
    /// The row a row drag started on: it stays built while scrolled out of view.
    pub(super) grabbed: Option<K>,
    sweep: Option<Sweep>,
    moving: Option<GestureId>,
    /// The row a drag from another collection would drop on this pass.
    pub(super) over: Option<K>,
}

impl<K> Default for Drags<K> {
    fn default() -> Self {
        Drags { grabbed: None, sweep: None, moving: None, over: None }
    }
}

/// Where a drop would land.
#[derive(Clone, Copy)]
struct Spot {
    over: usize,
    insert: usize,
    mark: DropMark,
}

/// A drop the collection took.
#[derive(Clone, Copy)]
struct Landed {
    spot: Spot,
    op: DropOperation,
}

struct RowDrop {
    spot: Option<Spot>,
    /// Whether the held Alt may turn the drop into a copy.
    copy_ok: bool,
}

impl<K> DropTarget<ListDrag<K>> for RowDrop {
    type Drop = Landed;

    fn accept(&self, _: &ListDrag<K>, _: Pos2, op: DropOperation) -> Option<Landed> {
        if matches!(op, DropOperation::Copy) && !self.copy_ok {
            return None;
        }
        self.spot.map(|spot| Landed { spot, op })
    }

    fn mark(&self, drop: &Landed) -> DropMark {
        drop.spot.mark
    }
}

fn cancel_reason(cancel: PressCancel) -> CancelReason {
    match cancel {
        PressCancel::Escape => CancelReason::Escape,
        PressCancel::Removed => CancelReason::Removed,
        _ => CancelReason::Blur,
    }
}

impl<C: Collection> ItemsHooks<'_, '_, C> {
    /// `at` in the layout's logical coordinates, relative to the rows' rect.
    fn logical(layout: &C::ItemLayout, rect: Rect, rtl: bool, at: Pos2) -> Pos2 {
        let at = at - rect.min.to_vec2();
        if layout.mirrors() && rtl {
            Flow::new(Rect::from_min_size(Pos2::ZERO, rect.size()), true).to_physical(Rect::from_min_size(at, Vec2::ZERO)).min
        } else {
            at
        }
    }

    /// The toggle column under `point` (logical) in row `at`, and whether it sweeps.
    pub(super) fn toggle_cell(
        &self,
        props: &CollectionProps,
        layout: &C::ItemLayout,
        at: usize,
        offset: ScrollPos,
        point: Pos2,
    ) -> Option<(ColumnKey, bool)> {
        let mut cells = Vec::new();
        layout.cells(at, offset, &mut cells);
        let (key, _) = cells.into_iter().find(|(_, cell)| cell.contains(point))?;
        props.columns.iter().find(|spec| spec.key == key).and_then(|spec| match spec.kind {
            ColumnKind::Toggle(toggle) => Some((key.clone(), toggle.sweepable)),
            _ => None,
        })
    }

    fn emit_toggle(&mut self, cx: &mut Cx<'_, C>, at: usize, column: &ColumnKey, on: bool) {
        let key = self.source.key(at);
        let gesture = crate::runtime::edit::next_gesture_id(&cx.core.ctx);
        let before = ToggleEdit { key: key.clone(), column: column.clone(), on: !on };
        let after = ToggleEdit { key, column: column.clone(), on };
        self.events.push(CollectionEvent::Toggle(EditEvent::set(gesture, Mixed::Same(before), after, cx.core.label_arc(), None)));
    }

    /// A press on a toggle cell flips it; on a sweepable column it also starts a sweep of the new value.
    pub(super) fn begin_toggle(&mut self, cx: &mut Cx<'_, C>, at: usize, column: ColumnKey, sweepable: bool) {
        let on = !self.source.toggled(at, &column);
        self.emit_toggle(cx, at, &column, on);
        if sweepable {
            self.drags.sweep = Some(Sweep { column, on, crossed: vec![at] });
        }
    }

    fn sweep_to(&mut self, cx: &mut Cx<'_, C>, layout: &C::ItemLayout, rect: Rect, offset: ScrollPos, rtl: bool, pos: Pos2) {
        let Some(sweep) = self.drags.sweep.clone() else { return };
        if pos.y < rect.min.y {
            return;
        }
        let Some(now) = layout.hit(Self::logical(layout, rect, rtl, pos), offset) else { return };
        let Some(&last) = sweep.crossed.last() else { return };
        let rows: Vec<usize> = if now >= last { (last + 1..=now).collect() } else { (now..last).rev().collect() };
        for at in rows {
            if let Some(held) = &mut self.drags.sweep {
                held.crossed.push(at);
            }
            if self.source.enabled(at) && self.source.toggled(at, &sweep.column) != sweep.on {
                self.emit_toggle(cx, at, &sweep.column, sweep.on);
            }
        }
    }

    fn free_move(&mut self, cx: &mut Cx<'_, C>, origin: Pos2, pos: Pos2) {
        let gesture = match self.drags.moving {
            Some(gesture) => gesture,
            None => {
                let gesture = crate::runtime::edit::next_gesture_id(&cx.core.ctx);
                self.drags.moving = Some(gesture);
                self.events.push(CollectionEvent::Move(EditEvent::begin(gesture, Mixed::Same(Vec2::ZERO), cx.core.label_arc(), None)));
                gesture
            }
        };
        self.events.push(CollectionEvent::Move(EditEvent::update(gesture, pos - origin)));
    }

    fn begin_row_drag(&mut self, cx: &mut Cx<'_, C>, at: usize) {
        let id = cx.core.id;
        let key = self.source.key(at);
        let keys: Vec<C::Key> = if self.selection.contains(&key) {
            (0..self.source.len()).map(|at| self.source.key(at)).filter(|held| self.selection.contains(held)).collect()
        } else {
            vec![key.clone()]
        };
        let preview = if keys.len() > 1 {
            DragPreview::Badge(Label::from(keys.len().to_string()))
        } else {
            self.source.text(at).map_or(DragPreview::None, |text| DragPreview::Badge(Label::from(text.into_owned())))
        };
        cx.start_drag(ListDrag::new(id, keys, key.clone()), preview);
        self.drags.grabbed = Some(key);
    }

    /// Where a drop at `at` (the collection's layer space) lands, `None` outside the rows.
    fn spot(&self, layout: &C::ItemLayout, rect: Rect, offset: ScrollPos, rtl: bool, at: Pos2) -> Option<Spot> {
        if at.y < rect.min.y || self.source.is_empty() {
            return None;
        }
        let nav = layout.nav();
        let hit = layout.hit(Self::logical(layout, rect, rtl, at), offset);
        let across = !matches!(nav, NavShape::Flat | NavShape::Tree);
        let past = hit.is_none() && !across;
        let over = hit.or_else(|| past.then(|| self.source.len() - 1))?;
        let row = self.screen(layout, rect, over, offset, rtl);
        if matches!(nav, NavShape::Spatial) {
            return Some(Spot { over, insert: over, mark: DropMark::Into(row) });
        }
        let after = past || if across { (at.x > row.center().x) != rtl } else { at.y > row.center().y };
        let mark = if across {
            let x = if after != rtl { row.right() } else { row.left() };
            DropMark::line(Pos2::new(x, row.top()), Pos2::new(x, row.bottom()))
        } else {
            let y = if after { row.bottom() } else { row.top() };
            DropMark::line(Pos2::new(row.left(), y), Pos2::new(row.right(), y))
        };
        Some(Spot { over, insert: over + usize::from(after), mark })
    }

    fn land(&mut self, cx: &mut Cx<'_, C>, payload: &ListDrag<C::Key>, landed: Landed) {
        if payload.list != cx.core.id {
            self.events.push(CollectionEvent::Dropped { key: self.source.key(landed.spot.over) });
            return;
        }
        let Some(from) = self.source.index_of(&payload.grabbed) else { return };
        let insert = landed.spot.insert;
        let to = if insert > from { insert - 1 } else { insert };
        let copy = matches!(landed.op, DropOperation::Copy);
        if to == from && !copy {
            return;
        }
        let gesture = crate::runtime::edit::next_gesture_id(&cx.core.ctx);
        let key = payload.grabbed.clone();
        let before = ReorderMove { key: key.clone(), to: from, copy: false };
        self.events.push(CollectionEvent::Reorder(EditEvent::set(
            gesture,
            Mixed::Same(before),
            ReorderMove { key, to, copy },
            cx.core.label_arc(),
            None,
        )));
    }

    /// Step 9: sweeps, row drags, drops and the free-layout move, from the collection's own press and the drag session.
    pub(super) fn drag_step(
        &mut self,
        cx: &mut Cx<'_, C>,
        layout: &C::ItemLayout,
        rect: Rect,
        offset: ScrollPos,
        rtl: bool,
        props: &CollectionProps,
    ) {
        self.drags.over = None;
        let free = matches!(layout.nav(), NavShape::Spatial);
        let phase = cx.core.press.phase;
        match phase {
            PressPhase::DragStart { start, pos, .. } | PressPhase::Drag { start, pos, .. } if start.button == PointerButton::Primary => {
                let origin = Self::logical(layout, rect, rtl, start.origin);
                let hit = (start.origin.y >= rect.min.y).then(|| layout.hit(origin, offset)).flatten().filter(|&at| self.source.enabled(at));
                if self.drags.sweep.is_some() {
                    self.sweep_to(cx, layout, rect, offset, rtl, pos);
                } else if let Some(at) = hit {
                    if free {
                        self.free_move(cx, start.origin, pos);
                    } else if props.reorder.is_some() && self.drags.grabbed.is_none() {
                        self.begin_row_drag(cx, at);
                    }
                }
            }
            PressPhase::Release { start, pos, .. } => {
                if let Some(gesture) = self.drags.moving.take() {
                    self.events.push(CollectionEvent::Move(EditEvent::commit(gesture, Mixed::Same(Vec2::ZERO), pos - start.origin)));
                }
                self.drags.sweep = None;
            }
            PressPhase::Cancel { reason, .. } => {
                if let Some(gesture) = self.drags.moving.take() {
                    self.events.push(CollectionEvent::Move(EditEvent::cancel(gesture, cancel_reason(reason), Some(Mixed::Same(Vec2::ZERO)))));
                }
                self.drags.sweep = None;
            }
            _ => {}
        }
        if let Some((payload, at)) = cx.dragging::<ListDrag<C::Key>>() {
            let own = payload.list == cx.core.id;
            if !own || props.reorder.is_some() {
                let spot = self.spot(layout, rect, offset, rtl, at);
                if let Some(spot) = spot
                    && !own
                {
                    self.drags.over = Some(self.source.key(spot.over));
                }
                let copy_ok = !own || props.reorder.is_some_and(|rule| rule.copy_with_alt);
                if let Some(landed) = cx.drop_target::<ListDrag<C::Key>, _>(&RowDrop { spot, copy_ok }) {
                    self.land(cx, &payload, landed);
                }
            }
        } else {
            self.drags.grabbed = None;
        }
    }
}
