//! The row indicators: fx, blend, lock, link and warning marks.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::list::ListRow;
use egui::{Painter, Rect};

use super::context::PanelContext;
use super::style::LayersStyle;
use crate::ui::tree::{LayerInfo, LayerRowKey, LayerTree, MaskInfo};

/// Adds a layer row's indicators at its trailing end, from the reading end inward: the warning
/// (`info.warning`, `Icon::Warning` in `layers.indicator.warn` with the text as tooltip), the lock
/// (any lock; `layers.indicator.lock`), the effects triangle (`has_effects` or
/// `has_smart_filters`; open as `effects_open`), the Smart Filters badge, `fx` (type role
/// `caption` semibold, `layers.indicator.fx`), the link (`linked`), then the blend label
/// (`blend_label`, role `caption`, `layers.indicator.blend`). `layers.indicator.icon` boxes,
/// `list.gap` apart, `list.trailing_pad` from the edge (the row's trailing-item rules).
///
/// - Every indicator is a required trailing item except the blend label, an
///   [`optional_item`](ListRow::optional_item) that drops first when the name would get narrower
///   than `list.min_label`.
/// - A click on the lock reports [`LayersEvent::Toggle`](super::LayersEvent::Toggle) with
///   [`LayerColumn::Lock`](super::LayerColumn::Lock) and a `Set` to off for that row, through
///   `cx.sink`; the row is not selected.
/// - A click on the effects triangle reports [`LayersEvent::FxOpen`](super::LayersEvent::FxOpen)
///   (Alt: `all`), through `cx.sink`; it is a button named `expand_effects` or
///   `collapse_effects` with `expanded`, tooltip `effects_tip`.
/// - Right to left the order mirrors with the row.
pub(super) fn add_indicators<'r, T: LayerTree>(
    row: ListRow<'r, LayerRowKey<T::Id>>,
    cx: &'r PanelContext<'r, T>,
    layer: &T::Id,
    info: &LayerInfo,
    effects_open: bool,
) -> ListRow<'r, LayerRowKey<T::Id>> {
    // pc: crates/ui-egui/src/panels/layer_row_ui.rs:143-236 `indicators` (right to left: warning,
    // lock, effects triangle, Smart Filters badge, fx, link, blend label; `RIGHT_PAD` 10, `GAP` 4,
    // `ICON_W` 14, `TRIANGLE_W` 10);
    // pc: crates/ui-egui/src/panels/layer_row_ui.rs:58-74 `layout` (the blend label drops first under `MIN_NAME_W` 64);
    // pc: crates/ui-egui/src/panels/layer_row.rs:205-209 (a lock click unlocks and doesn't select).
    todo!()
}

/// Paints a mask thumbnail's marks over its picture in `rect`: the red cross of a disabled mask
/// (`layers.mask.disabled`, 1.5 pt diagonals inset 1 pt) and, with `badges`, the
/// `layers.mask.badge` box at the bottom trailing corner with a pen (vector mask) or a dot
/// (pixel mask) glyph.
pub(super) fn paint_mask_marks(painter: &Painter, rect: Rect, mask: &MaskInfo, badges: bool, style: &LayersStyle) {
    // pc: crates/ui-egui/src/panels/mask_thumbs_ui.rs:142-154 (red X, 9 pt badge, 8 pt glyph).
    todo!()
}
