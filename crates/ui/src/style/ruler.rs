//! Style of the `ruler` family.

const LABEL_PITCH: f32 = 8.5;
const STEP_MIN: f32 = 60.0;
const TICK_MID: f32 = 6.0;
const TICK_MINOR: f32 = 3.0;
const MARKER_HALF: f32 = 4.5;
const MARKER_COLUMN: f32 = 7.0;
const MARKER_TIP: f32 = 5.0;
const MARKER_BAR: f32 = 4.0;
const MARKER_STEM: f32 = 5.0;
const MARKER_FOOT: f32 = 4.0;
const MARKER_SELECTOR: f32 = 14.0;

/// Ruler values that are not semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RulerStyle {
    /// Between the characters of the left ruler's stacked labels.
    pub label_pitch: f32,
    /// Least distance between labelled ticks.
    pub step_min: f32,
    /// Length of the tick halfway between labelled ticks.
    pub tick_mid: f32,
    /// Length of the other minor ticks.
    pub tick_minor: f32,
    /// Half the width of a text-ruler marker.
    pub marker_half: f32,
    /// Width of a table-ruler column marker.
    pub marker_column: f32,
    /// Height of a text-ruler indent triangle.
    pub marker_tip: f32,
    /// Height of the left indent's box under its triangle.
    pub marker_bar: f32,
    /// Height of a tab stop's stem.
    pub marker_stem: f32,
    /// Length of a tab stop's foot, to either side of the stem.
    pub marker_foot: f32,
    /// Side of the tab selector's box.
    pub marker_selector: f32,
}

impl RulerStyle {
    /// The ruler style for `tokens`.
    pub fn new(_tokens: &craft_theme::Tokens) -> Self {
        Self {
            label_pitch: LABEL_PITCH,
            step_min: STEP_MIN,
            tick_mid: TICK_MID,
            tick_minor: TICK_MINOR,
            marker_half: MARKER_HALF,
            marker_column: MARKER_COLUMN,
            marker_tip: MARKER_TIP,
            marker_bar: MARKER_BAR,
            marker_stem: MARKER_STEM,
            marker_foot: MARKER_FOOT,
            marker_selector: MARKER_SELECTOR,
        }
    }
}
