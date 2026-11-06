//! Style of the `tile` family.

use craft_theme::Tokens;

/// Height of the range bar under a tile.
const RANGE_BAR_HEIGHT: f32 = 4.0;
/// Width of the caption column of a start-placed caption.
const START_CAPTION_WIDTH: f32 = 30.0;
/// Share of the grid's height that one wheel notch scrolls.
const WHEEL_SHARE: f32 = 0.33;
/// Room for a compared tile's name.
const COMPARE_NAME_ROOM: f32 = 110.0;

/// Sizes of tiles that the semantic tokens don't name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileStyle {
    /// Diameter of a status dot.
    pub dot: f32,
    /// Height of the range bar under a tile.
    pub range_bar: f32,
    /// Width of the caption column of a start-placed caption.
    pub start_caption_width: f32,
    /// Share of the grid's height that one wheel notch scrolls.
    pub wheel_share: f32,
    /// Room for a compared tile's name.
    pub compare_name_room: f32,
}

impl TileStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        Self {
            dot: tokens.size.handle.md + tokens.stroke.thick,
            range_bar: RANGE_BAR_HEIGHT,
            start_caption_width: START_CAPTION_WIDTH,
            wheel_share: WHEEL_SHARE,
            compare_name_room: COMPARE_NAME_ROOM,
        }
    }
}
