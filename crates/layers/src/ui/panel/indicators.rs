//! The row indicators: fx, blend, lock, link and warning marks.

use craft_theme::{FontSlot, TypeRole};
use craft_ui::controls::{Kbd, TooltipExt as _};
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::list::{ListRow, ToggleEdit};
use craft_ui::primitives::Interactive;
use craft_ui::theme::{UiTokens, current, style_format};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Align2, Color32, Context, Key, KeyboardShortcut, Modifiers, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};

use super::context::PanelContext;
use super::events::{LayerColumn, LayerToggle, LayersEvent};
use super::style::LayersStyle;
use crate::ui::tree::{LayerInfo, LayerPart, LayerRowKey, LayerTree, MaskInfo};

/// The index of the lock column in a row's toggle columns when the panel has one; a lock
/// indicator reports the same edit.
const LOCK_COLUMN: usize = 1;

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
    let style = cx.style;
    let side = style.indicator_icon;
    let key = LayerRowKey::Layer(layer.clone());
    let tokens = current(&cx.ctx);
    let mut row = row;

    if let Some(warning) = info.warning.clone() {
        row = row.trailing_item(side, move |ui| {
            let at = cell(ui, side);
            icons::paint(ui, at, Icon::Warning, IconSize::Sm, style.warn);
            let response = ui.interact(at, ui.scope_id().with("warning"), Sense::hover());
            let _ = response.tooltip(warning);
        });
    }

    if info.locks.any() {
        let name = craft_i18n::fmt(&cx.labels.unlock, &[("row", info.name.as_str())]);
        let key = key.clone();
        row = row.trailing_item(side, move |ui| {
            let at = cell(ui, side);
            let response = Interactive::new(("layers.lock-indicator", &key), Role::Button, name.as_str()).interact_at(ui, at).finish(ui);
            icons::paint(ui, at, Icon::Lock, IconSize::Sm, style.lock);
            if response.clicked() {
                let edit = ToggleEdit { column: LOCK_COLUMN, row: key.clone(), on: false };
                let toggle = LayerToggle { row: key.clone(), column: LayerColumn::Lock, edit: EditEvent::set(edit, ui.input(|i| i.modifiers)) };
                cx.sink.borrow_mut().events.push(LayersEvent::Toggle(toggle));
            }
            let _ = response.tooltip(name.as_str());
        });
    }

    if info.has_effects || info.has_smart_filters {
        let verb = if effects_open { &cx.labels.collapse_effects } else { &cx.labels.expand_effects };
        let name = craft_i18n::fmt(verb, &[("layer", info.name.as_str())]);
        let tip = craft_i18n::fmt(&cx.labels.effects_tip, &[("key", alt_name(&cx.ctx).as_str())]);
        let (key, id) = (key.clone(), layer.clone());
        let width = style.indicator_triangle;
        row = row.trailing_item(width, move |ui| {
            let at = cell(ui, width);
            let response = Interactive::new(("layers.effects-triangle", &key), Role::Button, name.as_str())
                .expanded(effects_open)
                .interact_at(ui, at)
                .finish(ui);
            let color = if response.hovered() { cx.row_style.icon_hover } else { cx.row_style.icon };
            paint_triangle(ui.painter(), at, effects_open, direction::is_rtl(ui), color);
            if response.clicked() {
                let all = ui.input(|i| i.modifiers.alt);
                cx.sink.borrow_mut().events.push(LayersEvent::FxOpen { layer: id.clone(), open: !effects_open, all });
            }
            let _ = response.tooltip(tip.as_str());
        });
    }

    if info.has_smart_filters {
        let color = if info.smart_filters_enabled { cx.row_style.icon } else { cx.row_style.dimmed };
        let tip = cx.labels.smart_filters.clone();
        row = row.trailing_item(side, move |ui| {
            let at = cell(ui, side);
            paint_rings(ui.painter(), at, color);
            let response = ui.interact(at, ui.scope_id().with("smart-filters"), Sense::hover());
            let _ = response.tooltip(tip.as_str());
        });
    }

    if info.has_effects {
        let (job, width) = text_job("fx", &tokens, true, style.fx, &cx.ctx);
        row = row.trailing_item(width, move |ui| paint_text(ui, job));
    }

    if info.linked {
        row = row.trailing_item(side, move |ui| icons::paint(ui, cell(ui, side), Icon::Link, IconSize::Sm, style.link));
    }

    if let Some(label) = info.blend_label.as_deref() {
        let (job, width) = text_job(label, &tokens, false, style.blend, &cx.ctx);
        row = row.optional_item(width, move |ui| paint_text(ui, job));
    }
    row
}

/// Paints a mask thumbnail's marks over its picture in `rect`: the red cross of a disabled mask
/// (`layers.mask.disabled`, 1.5 pt diagonals inset 1 pt) and, with `badges`, the
/// `layers.mask.badge` box at the bottom trailing corner with a pen (vector mask) or a dot
/// (pixel mask) glyph.
pub(super) fn paint_mask_marks(painter: &Painter, rect: Rect, mask: &MaskInfo, badges: bool, style: &LayersStyle) {
    // pc: crates/ui-egui/src/panels/mask_thumbs_ui.rs:142-154 (red X, 9 pt badge, 8 pt glyph).
    if !mask.enabled {
        let stroke = Stroke::new(1.5, style.mask_disabled);
        let inner = rect.shrink(1.0);
        painter.line_segment([inner.left_top(), inner.right_bottom()], stroke);
        painter.line_segment([inner.right_top(), inner.left_bottom()], stroke);
    }
    if badges {
        let side = style.mask_badge;
        let rtl = direction::is_rtl(painter.ctx());
        let x = if rtl { rect.left() } else { rect.right() - side };
        let badge = Rect::from_min_size(pos2(x, rect.bottom() - side), vec2(side, side));
        painter.rect_filled(badge, 0.0, style.mask_badge_fill);
        let icon = if mask.part == LayerPart::VectorMask { Icon::PenTool } else { Icon::Circle };
        let set = icons::active(painter.ctx());
        icons::paint_on(painter, badge, icon, set, (side - 1.0).max(0.0), IconSize::Sm.stroke(), style.mask_badge_glyph);
    }
}

/// The square cell of `side` centred in an item's `Ui`.
fn cell(ui: &Ui, side: f32) -> Rect {
    Rect::from_center_size(ui.max_rect().center(), vec2(side, side))
}

/// The Alt key as the platform writes it.
fn alt_name(ctx: &Context) -> String {
    Kbd::parts(&KeyboardShortcut::new(Modifiers::ALT, Key::A), ctx.os()).into_iter().next().unwrap_or_default()
}

/// A single-line caption job and its width.
fn text_job(text: &str, tokens: &UiTokens, semibold: bool, color: Color32, ctx: &Context) -> (LayoutJob, f32) {
    let mut type_style = tokens.typography.get(TypeRole::Caption);
    if semibold {
        type_style.font = FontSlot::UiSemibold;
    }
    let job = LayoutJob::single_section(text.to_owned(), style_format(tokens.families(), type_style, color));
    let width = ctx.fonts_mut(|fonts| fonts.layout_job(job.clone()).size().x);
    (job, width)
}

/// Draws a [`text_job`] at the start of the item, centred vertically.
fn paint_text(ui: &mut Ui, job: LayoutJob) {
    let rect = ui.max_rect();
    let galley = ui.painter().layout_job(job);
    let anchor = if direction::is_rtl(ui) { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER };
    let at = anchor.pos_in_rect(&rect);
    ui.painter().galley(anchor.anchor_size(at, galley.size()).min, galley, Color32::PLACEHOLDER);
}

/// The effects disclosure triangle: down when open, towards the reading end when closed.
fn paint_triangle(painter: &Painter, rect: Rect, open: bool, rtl: bool, color: Color32) {
    let c = rect.center();
    let s = rect.width() * 0.3;
    let flip = if rtl { -1.0 } else { 1.0 };
    let points: Vec<Pos2> = if open {
        vec![pos2(c.x - s, c.y - s * 0.6), pos2(c.x + s, c.y - s * 0.6), pos2(c.x, c.y + s * 0.6)]
    } else {
        vec![pos2(c.x - flip * s * 0.6, c.y - s), pos2(c.x + flip * s * 0.6, c.y), pos2(c.x - flip * s * 0.6, c.y + s)]
    };
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

/// The Smart Filters badge: two rings side by side.
fn paint_rings(painter: &Painter, rect: Rect, color: Color32) {
    let (offset, radius) = (rect.width() * 0.18, rect.width() * 0.25);
    for dx in [-offset, offset] {
        painter.circle_stroke(rect.center() + vec2(dx, 0.0), radius, Stroke::new(1.0, color));
    }
}
