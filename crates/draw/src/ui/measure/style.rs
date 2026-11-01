//! [`MeasureStyle`]: the Ruler tool's look and reach, from tokens (task TK2).

use craft_ui::canvas::straight_line::LineGuideStyle;
use craft_ui::theme::UiTokens;

/// What the Ruler tool draws and how far its ends reach. Every size is the same at every Size:
/// they sit on the document.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MeasureStyle {
    /// The line and the protractor arm: the line guide's look (`line_guide.width` 1 in
    /// `overlay.stroke` over `line_guide.halo` 3 in `overlay.halo`), shared with the Shift-click
    /// line.
    pub line: LineGuideStyle,
    /// A press within this many screen points of an end grabs it: `measure.hit` (7).
    pub hit: f32,
    /// Each arm of an end's cross, in screen points: `measure.handle_arm` (5), drawn as the line
    /// is (halo, then line).
    pub handle_arm: f32,
    /// The gap after each readout in the options bar: `measure.readout_gap` (6).
    pub readout_gap: f32,
}

impl MeasureStyle {
    /// The Ruler's look in `tokens`.
    pub fn of(tokens: &UiTokens) -> Self {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:210 (7 / zoom), :408 `handle` (5 pt arms,
        // 3 pt black under 1 pt white), :489 `readout` (6 pt gap); the line from
        // `LineGuideStyle::of` (task TK1).
        let m = &tokens.component.measure;
        Self { line: LineGuideStyle::of(tokens), hit: m.hit, handle_arm: m.handle_arm, readout_gap: m.readout_gap }
    }
}
