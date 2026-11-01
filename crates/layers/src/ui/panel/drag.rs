//! Drag and drop: `Reorder` rules, payloads, transfers, sub-row reorder and the drop preview.

use craft_ui::behavior::{DropPosition, ListDrag, Move, Reorder, RowInfo};
use craft_ui::edit::EditEvent;
use egui::{DragAndDrop, Modifiers, Ui};

use super::context::PanelContext;
use super::events::{LayerDrop, LayerMove, LayersEvent, RowTransfer, SubRowMove};
use crate::ui::options::LayersOptions;
use crate::ui::tree::{LayerKind, LayerPart, LayerRowKey, LayerTree, SubRowGroup};

/// The list's reorder rules for the panel's rows: `copy_with_alt(true)` (Alt copies, with the
/// copy cursor), [`into_rows`](Reorder::into_rows) accepting a layer row without children as an
/// `Into` target for effects and Smart Filter rows, and [`accepts`](Reorder::accepts) by
/// [`accepts_move`]. The list's own rules give the zones (a group row's 0.3 to 0.7 band is Into,
/// else the top half Before and the bottom half After; the gaps and the space under the last row
/// are drop zones), the 2 pt marker or outline, the ghost with the grabbed row's name, the edge
/// scroll, Esc and release-elsewhere cancels.
pub(super) fn reorder<'a, T: LayerTree>(cx: &'a PanelContext<'a, T>, _rows: &'a [RowInfo<LayerRowKey<T::Id>>]) -> Reorder<'a, LayerRowKey<T::Id>> {
    // pc: crates/ui-egui/src/panels/layer_drag.rs:86-183 `layer_drag_and_drop` (0.3 to 0.7 Into on groups, Alt copies);
    // pc: crates/ui-egui/src/panels/layer_drag.rs:189-222 `layer_drop_zone` (gaps and the space below the rows).
    Reorder::default()
        .copy_with_alt(true)
        .into_rows(|dragged: &[LayerRowKey<T::Id>], row: &LayerRowKey<T::Id>| {
            row.is_layer() && !dragged.is_empty() && dragged.iter().all(|key| !key.is_layer())
        })
        .accepts(|mv, modifiers| accepts_move(cx.tree, mv, modifiers))
}

/// Whether the panel takes the move `mv` (the marker shows only where it does):
///
/// - layer rows onto a layer row, Before, After or Into a group: when
///   [`LayerTree::accepts_drop`] does;
/// - effects rows or Smart Filter rows `Into` another layer's row: a transfer (the whole row is
///   outlined); onto their own layer: refused;
/// - a Smart Filter row Before or After another Smart Filter row of the same layer: a reorder of
///   the filters;
/// - anything else (sub-rows between layers, layers onto sub-rows, a mixed drag): refused.
pub(super) fn accepts_move<T: LayerTree>(tree: &T, mv: &Move<LayerRowKey<T::Id>>, modifiers: Modifiers) -> bool {
    // pc: crates/ui-egui/src/panels/layer_drag.rs:76-81 `background_drop_refused` (through the app's `accepts_drop`);
    // pc: crates/ui-egui/src/panels/layer_drag.rs:288-299 `fx_drop` (effects onto a layer, outlined);
    // pc: crates/ui-egui/src/panels/smart_ui.rs:352-410 `drop_on_row`, `reorder_target`, `reorder_drop`.
    match layer_drop(mv) {
        Some(LayerDrop::Layers(_)) => tree.accepts_drop(mv, modifiers),
        // Smart Filters go only onto a smart object; Photoshop and PhotoCraft take no others.
        Some(LayerDrop::Transfer(transfer)) => {
            transfer.rows.iter().all(|row| !matches!(row, LayerRowKey::Filters(..))) || tree.info(&transfer.to).kind == LayerKind::SmartObject
        }
        Some(LayerDrop::SubRows(_)) => true,
        None => false,
    }
}

/// What a move of rows means: [`LayerDrop::Layers`] for layer rows; [`LayerDrop::Transfer`] for
/// effects or filter rows `Into` a layer; [`LayerDrop::SubRows`] for a Smart Filter row placed
/// among its layer's filters (`to` counted after it is taken out). `None` for a move
/// [`accepts_move`] refuses.
pub(super) fn layer_drop<Id: Clone + PartialEq>(mv: &Move<LayerRowKey<Id>>) -> Option<LayerDrop<Id>> {
    // pc: crates/ui-egui/src/panels/layer_drag.rs:66-72 `layer_drop_payload`, :276-284 `fx_drop_action`;
    // pc: crates/ui-egui/src/panels/smart_ui.rs:368-390 `reorder_target`.
    if mv.items.is_empty() {
        return None;
    }
    if mv.items.iter().all(LayerRowKey::is_layer) {
        let LayerRowKey::Layer(target) = &mv.target else { return None };
        let layers = mv.items.iter().map(|key| key.layer().clone()).collect();
        return Some(LayerDrop::Layers(LayerMove { layers, target: target.clone(), position: mv.position, copy: mv.copy }));
    }
    // Sub-rows: all of one layer and one group.
    let layer = mv.items[0].layer();
    let group = sub_row_group(&mv.items[0])?;
    if mv.items.iter().any(|key| key.layer() != layer || sub_row_group(key) != Some(group)) {
        return None;
    }
    match (&mv.target, mv.position) {
        (LayerRowKey::Layer(to), DropPosition::Into) if to != layer => {
            Some(LayerDrop::Transfer(RowTransfer { rows: mv.items.clone(), to: to.clone(), copy: mv.copy }))
        }
        (LayerRowKey::Filters(on, row), DropPosition::Before | DropPosition::After) if on == layer && group == SubRowGroup::SmartFilters => {
            let [LayerRowKey::Filters(_, Some(from))] = mv.items.as_slice() else { return None };
            let slot = match (row, mv.position) {
                // The header's bottom edge is the top of the stack; nothing goes above the header.
                (None, DropPosition::After) => 0,
                (Some(row), DropPosition::Before) => *row,
                (Some(row), DropPosition::After) => row + 1,
                _ => return None,
            };
            let to = if slot > *from { slot - 1 } else { slot };
            (to != *from || mv.copy).then(|| LayerDrop::SubRows(SubRowMove { layer: layer.clone(), group, from: *from, to, copy: mv.copy }))
        }
        _ => None,
    }
}

/// The group a sub-row key belongs to; `None` for a layer row or a mask.
fn sub_row_group<Id>(key: &LayerRowKey<Id>) -> Option<SubRowGroup> {
    match key {
        LayerRowKey::Effects(..) => Some(SubRowGroup::Effects),
        LayerRowKey::Filters(..) => Some(SubRowGroup::SmartFilters),
        LayerRowKey::Layer(_) | LayerRowKey::Mask(..) => None,
    }
}

/// The panel's event for the list's reorder edit: the same gesture (`Begin`, `Commit`, `Cancel`,
/// or a stacking key's `Set`) carrying [`layer_drop`] of the move. A `Commit` or `Set` whose move
/// means nothing is dropped.
pub(super) fn drop_event<Id: Clone + PartialEq>(edit: EditEvent<Move<LayerRowKey<Id>>>) -> Option<LayersEvent<Id>> {
    // new: the list's `ListEvent::Reorder` gesture, mapped.
    let drop = match edit {
        EditEvent::Begin { id, value, .. } => match value.and_then(|mv| layer_drop(&mv)) {
            Some(drop) => EditEvent::begin_with(id, drop),
            None => EditEvent::begin(id),
        },
        EditEvent::Commit { id, value, changed, modifiers, .. } => EditEvent::commit(id, layer_drop(&value)?, changed, modifiers),
        EditEvent::Cancel { id, reason, .. } => EditEvent::cancel(id, reason),
        EditEvent::Set { value, modifiers, .. } => EditEvent::set(layer_drop(&value)?, modifiers),
        // The panel reports no per-frame updates of a drag; the preview is `drop_preview`.
        _ => return None,
    };
    Some(LayersEvent::Drop(drop))
}

/// The drop preview the app may show on its canvas: the layer move the drag would make now,
/// while `options.preview_on_canvas_when_reordering` is on; `None` otherwise, over no target, and
/// for transfers and sub-row moves.
pub(super) fn drop_preview<Id: Clone + PartialEq>(target: Option<&Move<LayerRowKey<Id>>>, options: &LayersOptions) -> Option<LayerMove<Id>> {
    // pc: crates/ui-egui/src/panels/layers_panel_ui.rs:72-99 `preview_reorder` (only while the option is on).
    if !options.preview_on_canvas_when_reordering {
        return None;
    }
    match layer_drop(target?)? {
        LayerDrop::Layers(mv) => Some(mv),
        LayerDrop::SubRows(_) | LayerDrop::Transfer(_) => None,
    }
}

/// Starts a mask drag for a `RowEvent::ThumbDrag` on a row's mask thumbnail: sets egui's drag
/// payload to a [`ListDrag`](craft_ui::behavior::ListDrag) of the panel's list `list` whose only
/// item is [`LayerRowKey::Mask`]`(layer, part)`, labelled `label` ("Layer Mask", "Vector Mask")
/// for the ghost. Only the footer's Delete takes it; a release elsewhere drops it.
pub(super) fn start_mask_drag<Id: Clone + Send + Sync + 'static>(ui: &Ui, list: egui::Id, layer: Id, part: LayerPart, label: &str) {
    // pc: crates/ui-egui/src/panels/layer_drag.rs:236-248 `starts_mask_drag`;
    // pc: crates/ui-egui/src/panels/layers.rs:357-363 (the mask's ghost label).
    let mask = LayerRowKey::Mask(layer, part);
    let payload = ListDrag { list, items: vec![mask.clone()], grabbed: mask, label: label.to_owned() };
    DragAndDrop::set_payload(ui.ctx(), payload);
}
