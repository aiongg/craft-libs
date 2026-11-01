//! One layer row: `LayerInfo` to `ListRow` (eye, thumbnails, kind tiles, masks, brackets,
//! trailing slot, sub-rows).

use craft_ui::behavior::RowInfo;
use craft_ui::canvas::textures::{Priority, TextureState, ThumbPaint, ThumbRequest, Thumbnails};
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::list::{ListRow, ThumbSize, ThumbTarget, ToggleColumn};
use craft_ui::theme::current;
use egui::{Color32, Painter, Rect};

use super::context::{PanelContext, ThumbHost};
use super::indicators::{add_indicators, paint_mask_marks};
use super::labels::LayersLabels;
use super::style::LayersStyle;
use crate::ui::thumbs::LayerThumb;
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
    {
        let mut sink = cx.sink.borrow_mut();
        sink.built = Some(sink.built.map_or((at, at), |(first, last)| (first.min(at), last.max(at))));
    }
    match key {
        LayerRowKey::Layer(id) => layer_row(cx, key, id),
        LayerRowKey::Effects(id, index) => sub_row(cx, key, id, SubRowGroup::Effects, *index),
        LayerRowKey::Filters(id, index) => sub_row(cx, key, id, SubRowGroup::SmartFilters, *index),
        LayerRowKey::Mask(id, _) => ListRow::new(key.clone(), cx.tree.info(id).name).layer(true),
    }
}

/// The height of the row at display position `at`, for `List::heights`: a layer row is the
/// list's height for `thumbnail_size` ([`ListRowStyle::height`](craft_ui::list::ListRowStyle::height):
/// `list.row_height` at None, `(thumb + list.thumb_pad).max(list.thumb_row_min)` otherwise); an
/// effects or Smart Filter row is `list.row_height_dense` (22), except a Smart Filters header
/// with its mask thumbnail, which follows the thumbnail row rule.
pub(super) fn row_height<T: LayerTree>(cx: &PanelContext<'_, T>, row: &RowInfo<LayerRowKey<T::Id>>, thumbnail_size: ThumbSize) -> f32 {
    // pc: crates/ui-egui/src/panels/layers_panel_ui.rs:178-192 `thumbnail_geometry`;
    // pc: crates/ui-egui/src/panels/smart_ui.rs:78-85 `smart_filters_header_geometry` (`max(22, thumb + 6)`, unified to the row rule).
    let dense = current(&cx.ctx).component.list.row_height_dense;
    match &row.key {
        LayerRowKey::Layer(_) | LayerRowKey::Mask(..) => cx.row_style.height(thumbnail_size, false),
        LayerRowKey::Filters(id, None)
            if thumbnail_size != ThumbSize::None && cx.tree.sub_rows(id, SubRowGroup::SmartFilters).is_some_and(|rows| rows.header.mask) =>
        {
            cx.row_style.height(thumbnail_size, false)
        }
        LayerRowKey::Effects(..) | LayerRowKey::Filters(..) => dense,
    }
}

/// Asks for the pixel and mask thumbnails of the rows one viewport beyond those the list built
/// this pass (`cx.sink.built`), above and below, at `Priority::NEAR`, so scrolling shows them at
/// once. Calls `Thumbnails::state` only: nothing is drawn.
pub(super) fn ask_near_thumbnails<T: LayerTree>(cx: &PanelContext<'_, T>, rows: &[RowInfo<LayerRowKey<T::Id>>]) {
    // new: the shared thumbnail rule (`step6a-plan.md`, "Who uses what": one row or viewport around at `NEAR`).
    let Some(host) = cx.thumbs else { return };
    let side = cx.row_style.thumb_side(cx.options.thumbnail_size);
    let Some((first, last)) = cx.sink.borrow().built else { return };
    if side <= 0.0 || rows.is_empty() {
        return;
    }
    let span = last - first + 1;
    let above = first.saturating_sub(span)..first.min(rows.len());
    let below = (last + 1).min(rows.len())..(last + 1 + span).min(rows.len());
    for row in above.chain(below).filter_map(|at| rows.get(at)) {
        let LayerRowKey::Layer(id) = &row.key else { continue };
        let info = cx.tree.info(id);
        let contents = cx.options.thumbnail_contents;
        let parts = std::iter::once(LayerThumb::pixels(id.clone(), contents))
            .chain(info.masks.iter().map(|mask| LayerThumb::of_part(id.clone(), mask.part, contents)));
        for thumb in parts {
            let _ = ask(cx, host, &thumb, side, Priority::NEAR);
        }
    }
}

/// Paints the kind tile of a row whose thumbnail source answered `Blank`: a folder for a group,
/// the adjustment glyph for an adjustment layer, a "T" on a light tile for a type layer, the
/// `thumbnail.placeholder` fill for any other kind; the tile fills `rect` (the row frames it).
pub(super) fn paint_kind_tile(painter: &Painter, rect: Rect, kind: LayerKind, _style: &LayersStyle) {
    // pc: crates/ui-egui/src/panels/layer_row.rs:507-517 (folder and `contrast` tiles on `t.field`,
    // the "T" tile); the Pro look's square corners (one layout).
    let ctx = painter.ctx();
    let tokens = current(ctx);
    let thumb = Thumbnails::<()>::style(&tokens);
    let glyph = ListRow::<()>::style(&tokens).icon;
    let set = icons::active(ctx);
    let points = (rect.width().min(rect.height()) * 0.64).max(0.0);
    let (fill, icon, color) = match kind {
        LayerKind::Group => (thumb.placeholder, Some(Icon::Folder), glyph),
        LayerKind::Adjustment => (thumb.placeholder, Some(Icon::Contrast), glyph),
        // The type tile is paper with ink: the same in every theme, like the checkerboard.
        LayerKind::Text => (thumb.checker[0], Some(Icon::Text), Color32::from_gray(40)), // design: allow(type tile ink)
        _ => (thumb.placeholder, None, glyph),
    };
    painter.rect_filled(rect, 0.0, fill);
    if let Some(icon) = icon {
        icons::paint_on(painter, rect, icon, set, points, IconSize::Md.stroke(), color);
    }
}

/// The row's description for a screen reader: the kind word, then each flag that holds
/// ("clipped", "locked", "linked", "layer mask", "vector mask", "disabled mask", "effects",
/// "Smart Filters", "hidden"), then the target words when `target` is given (the active row),
/// then `info.description`, joined with ", ".
pub(super) fn row_description(labels: &LayersLabels, info: &LayerInfo, target: Option<LayerPart>) -> String {
    // new: the spec's AccessKit table (`step6a-layers.md`); PhotoCraft names painted rows only.
    let kind = match info.kind {
        LayerKind::Pixel => &labels.kind_pixel,
        LayerKind::Group => &labels.kind_group,
        LayerKind::Adjustment => &labels.kind_adjustment,
        LayerKind::Fill => &labels.kind_fill,
        LayerKind::Text => &labels.kind_text,
        LayerKind::Shape => &labels.kind_shape,
        LayerKind::SmartObject => &labels.kind_smart_object,
        LayerKind::Artboard => &labels.kind_artboard,
        LayerKind::Path => &labels.kind_path,
        LayerKind::Image => &labels.kind_image,
        LayerKind::Object => &labels.kind_object,
    };
    let has = |part: LayerPart| info.masks.iter().any(|mask| mask.part == part);
    let flags = [
        (info.clipped, &labels.clipped),
        (info.locks.any(), &labels.locked),
        (info.linked, &labels.linked),
        (has(LayerPart::LayerMask), &labels.layer_mask),
        (has(LayerPart::VectorMask), &labels.vector_mask),
        (info.masks.iter().any(|mask| !mask.enabled), &labels.disabled_mask),
        (info.has_effects, &labels.has_effects),
        (info.has_smart_filters, &labels.has_smart_filters),
        (!info.visible, &labels.hidden),
    ];
    let target = target.map(|part| match part {
        LayerPart::Pixels => &labels.pixels_targeted,
        LayerPart::LayerMask => &labels.layer_mask_targeted,
        LayerPart::VectorMask => &labels.vector_mask_targeted,
        LayerPart::FilterMask => &labels.filter_mask_targeted,
    });
    let mut words: Vec<&str> = std::iter::once(kind.as_str()).chain(flags.iter().filter(|(on, _)| *on).map(|(_, word)| word.as_str())).collect();
    words.extend(target.map(String::as_str));
    words.extend(info.description.as_deref());
    words.join(", ")
}

/// The description of a sub-row: `LayersLabels::effect_of` or `smart_filter_of` with the layer's
/// name.
pub(super) fn sub_row_description(labels: &LayersLabels, group: SubRowGroup, layer_name: &str) -> String {
    // new: the spec's AccessKit table.
    let pattern = match group {
        SubRowGroup::Effects => &labels.effect_of,
        SubRowGroup::SmartFilters => &labels.smart_filter_of,
    };
    craft_i18n::fmt(pattern, &[("layer", layer_name)])
}

/// The state of one thumbnail of the row, asked now: the paint closure only gets a painter.
fn ask<T: LayerTree>(cx: &PanelContext<'_, T>, host: ThumbHost<'_, T::Id>, thumb: &LayerThumb<T::Id>, side: f32, priority: Priority) -> TextureState {
    let request = ThumbRequest::for_cell(side, cx.ctx.pixels_per_point());
    host.thumbnails.state(&cx.ctx, host.source, thumb, &request, priority)
}

/// Paints `state` through the thumbnail cache; false when there is nothing to show (no cache,
/// or the source answered `Blank`).
fn paint_picture<Id: Clone + Eq + std::hash::Hash + Send + Sync + 'static>(
    host: Option<ThumbHost<'_, Id>>,
    state: Option<&TextureState>,
    painter: &Painter,
    rect: Rect,
    paint: &ThumbPaint,
) -> bool {
    match (host, state) {
        (Some(host), Some(state)) => host.thumbnails.paint(painter, rect, state, paint),
        _ => false,
    }
}

fn layer_row<'r, T: LayerTree>(cx: &'r PanelContext<'r, T>, key: &LayerRowKey<T::Id>, id: &T::Id) -> ListRow<'r, LayerRowKey<T::Id>> {
    let info = cx.tree.info(id);
    let size = cx.options.thumbnail_size;
    let side = cx.row_style.thumb_side(size);
    let contents = cx.options.thumbnail_contents;
    let active = cx.selection.active.as_ref() == Some(id);

    let mut eye = ToggleColumn::eye(cx.labels.visibility.clone(), info.visible);
    if let Some(color) = info.label {
        eye = eye.label(color);
    }
    if let Some(shortcut) = cx.visibility_key {
        eye = eye.key(shortcut);
    }
    let mut row = ListRow::new(key.clone(), info.name.as_str()).layer(true).toggle(eye);
    if cx.lock_column {
        row = row.toggle(ToggleColumn::lock(cx.labels.lock_cell.clone(), info.locks.any()));
    }
    row = row.clipped(info.clipped).dimmed(info.dimmed).highlight(cx.tree.hovered().as_ref() == Some(id));

    if side > 0.0 {
        let host = cx.thumbs;
        let pixels = host.map(|host| ask(cx, host, &LayerThumb::pixels(id.clone(), contents), side, Priority::VISIBLE));
        let (row_key, kind, style) = (key.clone(), info.kind, cx.style);
        let sink = &cx.sink;
        row = row.thumb(size, move |painter, rect| {
            sink.borrow_mut().thumbs.push((row_key, LayerPart::Pixels, rect));
            if !paint_picture(host, pixels.as_ref(), painter, rect, &ThumbPaint::default()) {
                paint_kind_tile(painter, rect, kind, &style);
            }
        });
        for mask in &info.masks {
            let part = mask.part;
            let state = host.map(|host| ask(cx, host, &LayerThumb::of_part(id.clone(), part, contents), side, Priority::VISIBLE));
            let (row_key, mask, badges, style) = (key.clone(), *mask, cx.options.show_layer_mask_badges, cx.style);
            row = row.extra_thumb(move |painter, rect| {
                sink.borrow_mut().thumbs.push((row_key, part, rect));
                paint_picture(host, state.as_ref(), painter, rect, &ThumbPaint::default().checker(false));
                paint_mask_marks(painter, rect, &mask, badges, &style);
            });
            let word = if part == LayerPart::VectorMask { &cx.labels.vector_mask } else { &cx.labels.layer_mask };
            let pattern = if mask.linked { &cx.labels.unlink_mask } else { &cx.labels.link_mask };
            row = row.thumb_link(mask.linked, craft_i18n::fmt(pattern, &[("mask", word.as_str()), ("layer", info.name.as_str())]));
        }
        if !info.masks.is_empty() {
            row = row.thumb_drags(true);
        }
        if active {
            let target = match cx.selection.target {
                LayerPart::Pixels => Some(ThumbTarget::First),
                LayerPart::FilterMask => None,
                part => Some(info.masks.iter().position(|mask| mask.part == part).map_or(ThumbTarget::First, ThumbTarget::Extra)),
            };
            if let Some(target) = target {
                row = row.thumb_target(target);
            }
        }
    }

    row = add_indicators(row, cx, id, &info, cx.tree.effects_open(id));
    if let Some(trailing) = cx.trailing {
        let (id, info) = (id.clone(), info.clone());
        row = row.trailing_item(trailing.width, move |ui| (trailing.add)(ui, &id, &info));
    }
    row = row.description(row_description(cx.labels, &info, active.then_some(cx.selection.target)));
    if let Some(warning) = &info.warning {
        row = row.tooltip(warning.as_str());
    }
    row
}

fn sub_row<'r, T: LayerTree>(
    cx: &'r PanelContext<'r, T>,
    key: &LayerRowKey<T::Id>,
    id: &T::Id,
    group: SubRowGroup,
    index: Option<usize>,
) -> ListRow<'r, LayerRowKey<T::Id>> {
    let rows = cx.tree.sub_rows(id, group);
    let sub = rows.and_then(|rows| match index {
        None => Some(rows.header),
        Some(at) => rows.items.into_iter().nth(at),
    });
    let Some(sub) = sub else { return ListRow::new(key.clone(), "").layer(true) };
    let layer_name = cx.tree.info(id).name;
    let mut row = ListRow::new(key.clone(), sub.label.as_str()).layer(true).indented_toggles(true).dimmed(!sub.on).description(sub_row_description(
        cx.labels,
        group,
        &layer_name,
    ));
    if sub.eye_shown {
        row = row.toggle(ToggleColumn::eye(cx.labels.visibility.clone(), sub.on));
    }
    if let Some(icon) = sub.icon {
        row = row.icon(icon);
    }
    let size = cx.options.thumbnail_size;
    let side = cx.row_style.thumb_side(size);
    if group == SubRowGroup::SmartFilters && index.is_none() && sub.mask && side > 0.0 {
        let host = cx.thumbs;
        let state = host.map(|host| ask(cx, host, &LayerThumb::filter_mask(id.clone()), side, Priority::VISIBLE));
        let (row_key, sink) = (key.clone(), &cx.sink);
        row = row.thumb(size, move |painter, rect| {
            sink.borrow_mut().thumbs.push((row_key, LayerPart::FilterMask, rect));
            paint_picture(host, state.as_ref(), painter, rect, &ThumbPaint::default().checker(false));
        });
        if cx.selection.active.as_ref() == Some(id) && cx.selection.target == LayerPart::FilterMask {
            row = row.thumb_target(ThumbTarget::First);
        }
    }
    row
}
