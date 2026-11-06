//! Style of the `segmented` family.

use craft_theme::{BySize, Rgba, Tokens};

/// Gap between segments beyond `space.gap.inline`.
const GAP_OVER_INLINE: f32 = 4.0;
/// Opacity of an opaque hover fill under a segment.
const HOVER_OPACITY: f32 = 0.6;

/// Spacing and hover fill of a segmented control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentedStyle {
    /// Padding at the start and end of a segment.
    pub pad_x: BySize<f32>,
    /// Between a segment's icon and label.
    pub gap: f32,
    /// Fill of a segment under the pointer.
    pub hover_fill: Rgba,
}

impl SegmentedStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        let space = &tokens.space;
        let hover = tokens.color.state.hover;
        SegmentedStyle {
            pad_x: BySize { sm: space.pad.control, md: space.pad.control + space.gap.inline },
            gap: space.gap.inline + GAP_OVER_INLINE,
            hover_fill: if hover.a() == 255 { hover.faded(HOVER_OPACITY) } else { hover },
        }
    }
}
