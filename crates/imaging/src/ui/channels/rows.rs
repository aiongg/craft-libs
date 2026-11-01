//! One channel row: eye, thumbnail, name, shortcut, rename.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::fmt::Debug;
use std::hash::Hash;

use craft_ui::list::ListRow;
use egui::{Painter, Rect};

use super::model::ChannelRow;
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
    todo!()
}

/// Paints a spot channel's ink as a `channels.spot_swatch` square at the bottom trailing corner
/// of the fitted thumbnail `thumb` (the ink's own colour, exempt from contrast checks).
pub(super) fn paint_spot_swatch(painter: &Painter, thumb: Rect, ink: craft_theme::Srgba, side: f32) {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:242-247 `paint_spot_swatch`.
    todo!()
}
