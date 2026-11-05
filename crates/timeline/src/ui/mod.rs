//! The timeline widgets, for egui. The look follows Premiere Pro (FilmCraft); After Effects'
//! (EffectCraft) and Pro Tools' (SoundCraft) differences are options.
//!
//! - [`panel`] has the [`Timeline`] panel over a [`TimelineModel`](crate::TimelineModel): track
//!   headers, rows, clips, the ruler and the playhead; edits out as [`TimelineEdit`]s.
//! - [`ruler`] has [`TimelineRuler`] (ticks, timecode labels, scrubbing, work area, cache bars) and
//!   [`TimelineScrollBar`] (scroll, with ends that zoom).
//! - [`clips`] has [`ClipLane`], the clips of one track with trims and moves.
//! - [`keyframes`] has [`KeyframeLane`], [`KeyframeNavigator`] and [`AnimationToggle`].
//! - [`graph`] has the [`GraphEditor`] (value and speed graphs).
//! - [`transport`] has [`TransportBar`] (a monitor's buttons) and [`MediaControls`] (a compact
//!   play / seek bar).
//! - [`timecode`] has [`TimecodeField`] and the [`TimecodePanel`].
//! - [`preview`] has [`FramePreview`], a frame fitted in a panel with a status line.
//! - [`markers`] has [`MarkerStrip`] and the [`MarkerDialog`].
//! - [`overview`] has [`TimelineOverview`], the whole timeline in miniature.
//! - [`render_queue`] has [`RenderQueueList`].
//! - [`style`] has [`TimelineStyle`], every size the widgets read (`timeline.*` tokens).

pub mod clips;
pub mod graph;
pub mod keyframes;
pub mod markers;
pub mod overview;
pub mod panel;
pub mod preview;
pub mod render_queue;
pub mod ruler;
pub mod style;
pub mod timecode;
pub mod transport;

mod stories;

#[cfg(test)]
mod clips_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod graph_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod keyframes_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod markers_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod overview_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_tests;
#[cfg(test)]
mod preview_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod render_queue_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod ruler_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod timecode_tests;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod transport_tests;

pub use clips::{ClipLane, ClipLaneResponse, ThumbnailMode, TimelineTool, thumbnail_tiles};
pub use graph::{
    GraphAction, GraphCurve, GraphEditor, GraphEditorLabels, GraphEditorOutput, GraphKey, GraphKeyMove, GraphMode, GraphRange, HandleMove, HandleSide,
};
pub use keyframes::{
    AnimationToggle, KeyframeId, KeyframeInterpolation, KeyframeLabels, KeyframeLane, KeyframeLaneResponse, KeyframeMark, KeyframeNavigator,
    KeyframeNavigatorOutput, paint_keyframe,
};
pub use markers::{
    MARKER_POINT, MarkerDialog, MarkerDialogAction, MarkerDialogLabels, MarkerDialogResult, MarkerDraft, MarkerEdit, MarkerStrip, MarkerStripResponse,
};
pub use overview::{OverviewRun, TimelineOverview, overview_runs};
pub use panel::{Timeline, TimelineEdit, TimelineLabels, TimelineOutput, TimelineToolbar, TrackHeader, TrackHeaderOutput, TrackSwitch};
pub use preview::{FrameFit, FramePreview, paint_frame};
pub use render_queue::{RenderItemId, RenderQueueEvent, RenderQueueItem, RenderQueueLabels, RenderQueueList, RenderQueueListOutput, RenderStatus};
pub use ruler::{
    RulerLabels, RulerScale, RulerTick, TimelineRuler, TimelineRulerLabels, TimelineRulerResponse, TimelineScrollBar, ruler_steps, ruler_ticks,
};
pub use style::TimelineStyle;
pub use timecode::{
    TimecodeField, TimecodePanel, TimecodePanelLabels, TimecodePanelOutput, TimecodePanelSettings, TimecodeRow, TimecodeRowKind, TimecodeSource,
};
pub use transport::{MediaControls, MediaControlsOutput, TransportBar, TransportBarOutput, TransportBarSettings, TransportButton, TransportLabels};

use craft_ui::localized::{Labels, Word};

/// The colours of the sixteen labels ([`MarkerColor`](crate::MarkerColor) indices), the app's
/// Label preferences. `Default` is Premiere's (FilmCraft `crates/project/src/label.rs:64`).
#[derive(Clone, Debug, PartialEq)]
pub struct LabelPalette(pub [egui::Color32; 16]);

impl Default for LabelPalette {
    fn default() -> Self {
        let rgb = [
            [0x38, 0x0e, 0xa7],
            [0x1d, 0x4a, 0x64],
            [0x35, 0x54, 0x18],
            [0x6b, 0x1c, 0x82],
            [0x23, 0x53, 0x5a],
            [0x40, 0x4a, 0x11],
            [0x80, 0x18, 0x36],
            [0x80, 0x3f, 0x17],
            [0x59, 0x0d, 0xb0],
            [0x19, 0x2d, 0x94],
            [0x1f, 0x4d, 0x45],
            [0x79, 0x1c, 0x56],
            [0x6c, 0x5b, 0x47],
            [0x29, 0x5c, 0x2d],
            [0x58, 0x3d, 0x14],
            [0x6e, 0x66, 0x28],
        ];
        Self(rgb.map(|[r, g, b]| egui::Color32::from_rgb(r, g, b)))
    }
}

impl LabelPalette {
    /// The colour of `label`; out-of-range indices wrap.
    pub fn color(&self, label: crate::MarkerColor) -> egui::Color32 {
        self.0[usize::from(label.0) % self.0.len()]
    }
}

/// The stories of the timeline widgets.
pub fn stories() -> Vec<craft_ui::story::Story> {
    stories::stories()
}

/// The labels tables of the timeline widgets, for the words registry.
pub const WORDS: &[&[Word]] = &[
    <TimelineLabels as Labels>::WORDS,
    <TimelineRulerLabels as Labels>::WORDS,
    <KeyframeLabels as Labels>::WORDS,
    <GraphEditorLabels as Labels>::WORDS,
    <TransportLabels as Labels>::WORDS,
    <TimecodePanelLabels as Labels>::WORDS,
    <MarkerDialogLabels as Labels>::WORDS,
    <RenderQueueLabels as Labels>::WORDS,
    <stories::TimelineStoryLabels as Labels>::WORDS,
];
