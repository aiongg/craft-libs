//! Drag and drop: `Reorder` rules, payloads, transfers, sub-row reorder and the drop preview.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::{Move, Reorder, RowInfo};
use craft_ui::edit::EditEvent;
use egui::{Modifiers, Ui};

use super::context::PanelContext;
use super::events::{LayerDrop, LayerMove, LayersEvent};
use crate::ui::options::LayersOptions;
use crate::ui::tree::{LayerPart, LayerRowKey, LayerTree};

/// The list's reorder rules for the panel's rows: `copy_with_alt(true)` (Alt copies, with the
/// copy cursor), [`into_rows`](Reorder::into_rows) accepting a layer row without children as an
/// `Into` target for effects and Smart Filter rows, and [`accepts`](Reorder::accepts) by
/// [`accepts_move`]. The list's own rules give the zones (a group row's 0.3 to 0.7 band is Into,
/// else the top half Before and the bottom half After; the gaps and the space under the last row
/// are drop zones), the 2 pt marker or outline, the ghost with the grabbed row's name, the edge
/// scroll, Esc and release-elsewhere cancels.
pub(super) fn reorder<'a, T: LayerTree>(cx: &'a PanelContext<'a, T>, rows: &'a [RowInfo<LayerRowKey<T::Id>>]) -> Reorder<'a, LayerRowKey<T::Id>> {
    // pc: crates/ui-egui/src/panels/layer_drag.rs:86-183 `layer_drag_and_drop` (0.3 to 0.7 Into on groups, Alt copies);
    // pc: crates/ui-egui/src/panels/layer_drag.rs:189-222 `layer_drop_zone` (gaps and the space below the rows).
    todo!()
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
    todo!()
}

/// What a move of rows means: [`LayerDrop::Layers`] for layer rows; [`LayerDrop::Transfer`] for
/// effects or filter rows `Into` a layer; [`LayerDrop::SubRows`] for a Smart Filter row placed
/// among its layer's filters (`to` counted after it is taken out). `None` for a move
/// [`accepts_move`] refuses.
pub(super) fn layer_drop<Id: Clone + PartialEq>(mv: &Move<LayerRowKey<Id>>) -> Option<LayerDrop<Id>> {
    // pc: crates/ui-egui/src/panels/layer_drag.rs:66-72 `layer_drop_payload`, :276-284 `fx_drop_action`;
    // pc: crates/ui-egui/src/panels/smart_ui.rs:368-390 `reorder_target`.
    todo!()
}

/// The panel's event for the list's reorder edit: the same gesture (`Begin`, `Commit`, `Cancel`,
/// or a stacking key's `Set`) carrying [`layer_drop`] of the move. A `Commit` or `Set` whose move
/// means nothing is dropped.
pub(super) fn drop_event<Id: Clone + PartialEq>(edit: EditEvent<Move<LayerRowKey<Id>>>) -> Option<LayersEvent<Id>> {
    // new: the list's `ListEvent::Reorder` gesture, mapped.
    todo!()
}

/// The drop preview the app may show on its canvas: the layer move the drag would make now,
/// while `options.preview_on_canvas_when_reordering` is on; `None` otherwise, over no target, and
/// for transfers and sub-row moves.
pub(super) fn drop_preview<Id: Clone + PartialEq>(target: Option<&Move<LayerRowKey<Id>>>, options: &LayersOptions) -> Option<LayerMove<Id>> {
    // pc: crates/ui-egui/src/panels/layers_panel_ui.rs:72-99 `preview_reorder` (only while the option is on).
    todo!()
}

/// Starts a mask drag for a `RowEvent::ThumbDrag` on a row's mask thumbnail: sets egui's drag
/// payload to a [`ListDrag`](craft_ui::behavior::ListDrag) of the panel's list `list` whose only
/// item is [`LayerRowKey::Mask`]`(layer, part)`, labelled `label` ("Layer Mask", "Vector Mask")
/// for the ghost. Only the footer's Delete takes it; a release elsewhere drops it.
pub(super) fn start_mask_drag<Id: Clone + Send + Sync + 'static>(ui: &Ui, list: egui::Id, layer: Id, part: LayerPart, label: &str) {
    // pc: crates/ui-egui/src/panels/layer_drag.rs:236-248 `starts_mask_drag`;
    // pc: crates/ui-egui/src/panels/layers.rs:357-363 (the mask's ghost label).
    todo!()
}
