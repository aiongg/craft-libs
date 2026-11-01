//! One channel row: eye, thumbnail, name, shortcut, rename.

use std::fmt::Debug;
use std::hash::Hash;

use craft_ui::canvas::textures::{Priority, TextureState, ThumbPaint, ThumbRequest, Thumbnails, fit_rect};
use craft_ui::controls::Kbd;
use craft_ui::list::{ListRow, ToggleColumn};
use craft_ui::theme::role_format;
use egui::{Color32, Painter, Rect, vec2};

use super::labels::chord_text;
use super::model::{ChannelKind, ChannelRow};
use super::panel::ChannelsContext;

/// Builds the [`ListRow`] of `row` for the panel's list, called by its row builder for the rows
/// it draws:
///
/// - `.layer(true)`; the eye column ([`ToggleColumn::eye`](craft_ui::list::ToggleColumn::eye)
///   named `ChannelsLabels::visibility` with the channel name, on = `row.visible`, empty when off;
///   tooltip `toggle_visibility`, or `mask_overlay` on a layer-mask row).
/// - The thumbnail at the panel's side ([`ChannelsContext::thumb_side`]): before building the
///   row, `Thumbnails::state(&cx.ctx, source, &row.key, &ThumbRequest::for_cell(side,
///   pixels_per_point), Priority::VISIBLE)`; the `.thumb_sized(..)` closure paints it with
///   `Thumbnails::paint` (the checkerboard shows through transparent channels) and, for a spot
///   row, [`paint_spot_swatch`].
/// - The name, `.italic(row.kind.italic())`; `.description(hidden)` when not visible.
/// - The shortcut chord at the trailing end (`labels::shortcut_numbers`, `labels::chord_text`,
///   the library's plain `Kbd` look in `text.muted`), an optional item that drops before the
///   name gets narrower than `list.min_label`.
/// - Selected (the list sets it) for `row.targeted`.
pub(super) fn channel_row<'r, K: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
    cx: &'r ChannelsContext<'r, K>,
    row: &'r ChannelRow<K>,
    shortcut: Option<u8>,
) -> ListRow<'r, K> {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:346-411 (eye 22 at 6 pt, thumbnail at 36 pt,
    // name 10 pt after it, italic rows, the chord 8 pt from the right); the list's layer form
    // replaces the row painter, the 30 pt rule and the 36/40 heights.
    let labels = cx.labels;
    let tooltip = if matches!(row.kind, ChannelKind::LayerMask { .. }) { &labels.mask_overlay } else { &labels.toggle_visibility };
    let eye = ToggleColumn::eye(labels.visibility.clone(), row.visible).tooltip(tooltip.clone());
    let mut list_row = ListRow::new(row.key.clone(), row.name.clone()).layer(true).toggle(eye).italic(row.kind.italic());
    if !row.visible {
        list_row = list_row.description(labels.hidden.clone());
    }
    if cx.thumb_side > 0.0 {
        list_row = with_thumbnail(list_row, cx, row);
    }
    match shortcut {
        Some(number) => with_chord(list_row, cx, number),
        None => list_row,
    }
}

/// `list_row` with the channel's thumbnail: the cache's picture, or a placeholder without a
/// cache, and the ink swatch of a spot channel.
fn with_thumbnail<'r, K: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
    mut list_row: ListRow<'r, K>,
    cx: &'r ChannelsContext<'r, K>,
    row: &'r ChannelRow<K>,
) -> ListRow<'r, K> {
    let side = cx.thumb_side;
    let request = ThumbRequest::for_cell(side, cx.ctx.pixels_per_point());
    let picture = cx.thumbnails.map(|(thumbnails, source)| (thumbnails, thumbnails.state(&cx.ctx, source, &row.key, &request, Priority::VISIBLE)));
    if let Some((_, TextureState::Failed { message })) = &picture {
        list_row = list_row.tooltip(message.clone());
    }
    let ink = match row.kind {
        ChannelKind::Spot { ink } => Some(ink),
        _ => None,
    };
    let swatch = cx.style.spot_swatch;
    list_row.thumb_sized(vec2(side, side), move |painter, rect| {
        let mut fitted = rect;
        let drawn = picture.as_ref().is_some_and(|(thumbnails, state)| {
            if let Some(texture) = state.shown() {
                fitted = fit_rect(rect, texture.size_vec2());
            }
            thumbnails.paint(painter, rect, state, &ThumbPaint::default())
        });
        if !drawn {
            let placeholder = Thumbnails::<K>::style(&craft_ui::theme::current(painter.ctx())).placeholder;
            painter.rect_filled(rect, 0.0, placeholder);
        }
        if let Some(ink) = ink {
            paint_spot_swatch(painter, fitted, ink, swatch);
        }
    })
}

/// `list_row` with the shortcut chord of `number` at the trailing end, which drops out before the
/// name gets narrower than `list.min_label`.
fn with_chord<'r, K: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
    list_row: ListRow<'r, K>,
    cx: &ChannelsContext<'_, K>,
    number: u8,
) -> ListRow<'r, K> {
    let text = chord_text(&cx.ctx, number);
    let tokens = craft_ui::theme::current(&cx.ctx);
    let format = role_format(&tokens, Kbd::style(&tokens).label, Color32::PLACEHOLDER);
    let width = cx.ctx.fonts_mut(|fonts| fonts.layout_no_wrap(text.clone(), format.font_id, Color32::PLACEHOLDER).size().x);
    list_row.optional_item(width.ceil(), move |ui| {
        ui.add(Kbd::keys(&[text]));
    })
}

/// Paints a spot channel's ink as a `channels.spot_swatch` square at the bottom trailing corner
/// of the fitted thumbnail `thumb` (the ink's own colour, exempt from contrast checks).
pub(super) fn paint_spot_swatch(painter: &Painter, thumb: Rect, ink: craft_theme::Srgba, side: f32) {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:242-247 `paint_spot_swatch`.
    let swatch = Rect::from_min_size(thumb.right_bottom() - vec2(side, side), vec2(side, side));
    painter.rect_filled(swatch, 0.0, Color32::from(ink));
}
