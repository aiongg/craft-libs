//! The Channels panel: the thumbnail side of a channel row and the ink swatch on a spot channel's
//! thumbnail. Rows, fills and the footer are the list and panel footer families'.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Channels tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ChannelsTokens {
    /// Side of a row's thumbnail when the app sets none: `channels.thumb` (Compact 28,
    /// Comfortable 30).
    pub thumb: f32,
    /// Side of a spot channel's ink swatch at its thumbnail's bottom trailing corner:
    /// [`values::CHANNELS_SPOT_SWATCH`].
    pub spot_swatch: f32,
}

/// Channels have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChannelsField {}

impl Family for ChannelsTokens {
    type Field = ChannelsField;
    const FIELDS: &'static [ChannelsField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ChannelsTokens;
        "thumb" 12.0..=96.0 => thumb;
        "spot_swatch" 4.0..=32.0 => spot_swatch;
    };

    fn defaults(s: &Semantic<'_>) -> ChannelsTokens {
        ChannelsTokens { thumb: s.set.channels_thumb, spot_swatch: values::CHANNELS_SPOT_SWATCH }
    }

    fn path(f: ChannelsField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ChannelsField> {
        None
    }

    fn kind(f: ChannelsField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ChannelsField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ChannelsField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
