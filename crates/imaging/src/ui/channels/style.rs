//! `ChannelsStyle` from the `channels` tokens.

use craft_ui::theme::UiTokens;

/// The Channels panel's own sizes, besides the list's (`list.*`) and the footer's.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ChannelsStyle {
    /// The thumbnail side when the app sets no `thumbnail_size`: `channels.thumb` (Compact 28,
    /// Comfortable 30); the row height follows the list's thumbnail-row rule.
    pub thumb: f32,
    /// A spot channel's ink swatch at the thumbnail's bottom trailing corner:
    /// `channels.spot_swatch` (9).
    pub spot_swatch: f32,
}

impl ChannelsStyle {
    /// The style of `tokens`. Until the `channels` token family exists (task TK1), the values are
    /// PhotoCraft's Pro ones (`channels_panel.rs:375`, `:246`) at every Size; TK1 reads
    /// `channels.thumb` (28 at Compact, 30 at Comfortable) and `channels.spot_swatch`.
    pub fn of(tokens: &UiTokens) -> ChannelsStyle {
        let _ = tokens;
        ChannelsStyle { thumb: 28.0, spot_swatch: 9.0 }
    }
}
