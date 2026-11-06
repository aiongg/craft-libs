//! `ChannelsStyle`, read from the `channels` tokens by `ChannelsPanel::style`.

/// The Channels panel's own sizes, besides the list's (`list.*`) and the footer's.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ChannelsStyle {
    /// The thumbnail side when the app sets no `thumbnail_size`: the thumb (Compact 28,
    /// Comfortable 30); the row height follows the list's thumbnail-row rule.
    pub thumb: f32,
    /// A spot channel's ink swatch at the thumbnail's bottom trailing corner:
    /// the spot swatch (9).
    pub spot_swatch: f32,
}
