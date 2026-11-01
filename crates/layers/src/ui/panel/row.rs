//! One layer row: `LayerInfo` to `ListRow` (eye, thumbnails, kind tiles, masks, brackets,
//! trailing slot, sub-rows).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::RowInfo;
use craft_ui::list::{ListRow, ThumbSize};
use egui::{Painter, Rect};

use super::context::PanelContext;
use super::labels::LayersLabels;
use super::style::LayersStyle;
use crate::ui::tree::{LayerInfo, LayerKind, LayerPart, LayerRowKey, LayerTree, SubRowGroup};

/// Builds the [`ListRow`] the list draws at display position `at` for `key`. Called by the
/// list's row builder for the rows it draws (in view, one above and below, and rows it keeps
/// drawn), so `cx.tree.info` runs for those only. Records `at` in `cx.sink.built`.
///
/// A **layer row**, in reading order (PhotoCraft's Pro layout in every theme and size):
/// - `.layer(true)`; the eye column ([`ToggleColumn::eye`](craft_ui::list::ToggleColumn::eye),
///   named by `LayersLabels::visibility` with the layer name, on = `info.visible`; empty when
///   off; filled with `info.label`'s colour; `.key(visibility_key)` when the app gives one), and
///   with `lock_column` a lock column after it ([`ToggleColumn::lock`](craft_ui::list::ToggleColumn::lock),
///   on = `info.locks.any()`). The list adds the indent and the group triangle (tree mode).
/// - `.clipped(info.clipped)`: the clip arrow and its indent.
/// - The layer thumbnail at `cx.options.thumbnail_size` (none at `ThumbSize::None`): before
///   building the row, `Thumbnails::state(&cx.ctx, source, &LayerThumb::pixels(id, contents),
///   &ThumbRequest::for_cell(side, cx.ctx.pixels_per_point()), Priority::VISIBLE)`, then
///   `.thumb(size, move |painter, rect| ..)` paints it with `Thumbnails::paint`, or, when that
///   draws nothing (`Empty`: groups, adjustments, type layers), the kind tile
///   ([`paint_kind_tile`]). The closure records `(key, LayerPart::Pixels, rect)` in
///   `cx.sink.thumbs`.
/// - Per mask in `info.masks` (pixel first): `.thumb_link(mask.linked, name)` (the chain, named
///   by `LayersLabels::link_mask` or `unlink_mask`) and `.extra_thumb(..)` painting the mask's
///   picture ([`LayerThumb::mask`](crate::ui::thumbs::LayerThumb::mask) or `vector_mask`), then
///   [`paint_mask_marks`](super::indicators::paint_mask_marks); `.thumb_drags(true)` so a drag
///   from a mask drags the mask. Thumbnails keep their size on a narrow panel; the name is cut
///   first.
/// - `.thumb_target(..)` on the active layer's row only: the thumbnail of
///   `cx.selection.target` gets the corner brackets.
/// - The name (`info.name`); `.dimmed(info.dimmed)`; `.highlight(..)` when `cx.tree.hovered()`
///   is this layer. The Background's name is upright.
/// - The indicators ([`add_indicators`](super::indicators::add_indicators)), then the app's
///   trailing furniture (`cx.trailing`) as a required trailing item.
/// - `.description(row_description(..))`, `.tooltip(warning)` when `info.warning` is set.
///
/// An **effects or Smart Filters row** ([`LayerRowKey::Effects`], [`LayerRowKey::Filters`]):
/// `.layer(true)`, `.indented_toggles(true)` (its eye under the layer's eye), the row's icon
/// (`SubRow::icon`), its label, an eye when `eye_shown` (on = `SubRow::on`), `.dimmed(!on)`; a
/// Smart Filters header with `mask` shows the shared filter-mask thumbnail
/// ([`LayerThumb::filter_mask`](crate::ui::thumbs::LayerThumb::filter_mask)) at the row's
/// thumbnail size (none at `ThumbSize::None`). Described by `effect_of` or `smart_filter_of`.
pub(super) fn build_row<'r, T: LayerTree>(cx: &'r PanelContext<'r, T>, at: usize, key: &LayerRowKey<T::Id>) -> ListRow<'r, LayerRowKey<T::Id>> {
    // pc: crates/ui-egui/src/panels/layer_row.rs:264-548 (row parts, eye, brackets, name, thumbnail);
    // pc: crates/ui-egui/src/panels/mask_thumbs_ui.rs:98-168 `paint` (chains, mask thumbnails, red X, badges);
    // pc: crates/ui-egui/src/panels/layer_drag.rs:303-367 `effect_rows` (sub-rows, eye under the layer's).
    todo!()
}

/// The height of the row at display position `at`, for `List::heights`: a layer row is the
/// list's height for `thumbnail_size` ([`ListRowStyle::height`](craft_ui::list::ListRowStyle::height):
/// `list.row_height` at None, `(thumb + list.thumb_pad).max(list.thumb_row_min)` otherwise); an
/// effects or Smart Filter row is `list.row_height_dense` (22), except a Smart Filters header
/// with its mask thumbnail, which follows the thumbnail row rule.
pub(super) fn row_height<T: LayerTree>(cx: &PanelContext<'_, T>, row: &RowInfo<LayerRowKey<T::Id>>, thumbnail_size: ThumbSize) -> f32 {
    // pc: crates/ui-egui/src/panels/layers_panel_ui.rs:178-192 `thumbnail_geometry`;
    // pc: crates/ui-egui/src/panels/smart_ui.rs:78-85 `smart_filters_header_geometry` (`max(22, thumb + 6)`, unified to the row rule).
    todo!()
}

/// Asks for the pixel and mask thumbnails of the rows one viewport beyond those the list built
/// this pass (`cx.sink.built`), above and below, at `Priority::NEAR`, so scrolling shows them at
/// once. Calls `Thumbnails::state` only: nothing is drawn.
pub(super) fn ask_near_thumbnails<T: LayerTree>(cx: &PanelContext<'_, T>, rows: &[RowInfo<LayerRowKey<T::Id>>]) {
    // new: the shared thumbnail rule (`step6a-plan.md`, "Who uses what": one row or viewport around at `NEAR`).
    todo!()
}

/// Paints the kind tile of a row whose thumbnail source answered `Blank`: a folder for a group,
/// the adjustment glyph for an adjustment layer, a "T" on a light tile for a type layer, the
/// `thumbnail.placeholder` fill for any other kind; the tile fills `rect` (the row frames it).
pub(super) fn paint_kind_tile(painter: &Painter, rect: Rect, kind: LayerKind, style: &LayersStyle) {
    // pc: crates/ui-egui/src/panels/layer_row.rs:507-517 (folder and `contrast` tiles on `t.field`,
    // the "T" tile); the Pro look's square corners (one layout).
    todo!()
}

/// The row's description for a screen reader: the kind word, then each flag that holds
/// ("clipped", "locked", "linked", "layer mask", "vector mask", "disabled mask", "effects",
/// "Smart Filters", "hidden"), then the target words when `target` is given (the active row),
/// then `info.description`, joined with ", ".
pub(super) fn row_description(labels: &LayersLabels, info: &LayerInfo, target: Option<LayerPart>) -> String {
    // new: the spec's AccessKit table (`step6a-layers.md`); PhotoCraft names painted rows only.
    todo!()
}

/// The description of a sub-row: `LayersLabels::effect_of` or `smart_filter_of` with the layer's
/// name.
pub(super) fn sub_row_description(labels: &LayersLabels, group: SubRowGroup, layer_name: &str) -> String {
    // new: the spec's AccessKit table.
    todo!()
}
