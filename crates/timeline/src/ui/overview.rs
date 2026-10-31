//! The whole timeline in miniature (N1.13).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Color32, IdSalt, Rect, Response, Ui};

use crate::time::{Tick, TimeRange, TimeView};

/// A run of touching clips of one colour on one track, in overview coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverviewRun {
    pub rect: Rect,
    pub color: Color32,
}

/// The overview's blocks for `tracks` (each a list of clip ranges and colours, in time order)
/// over `total` time in `rect`: one row per track, each clip at least one point wide, and
/// neighbours of one colour that touch or overlap merged into one run, which covers exactly the
/// same pixels in the same order (a drum edit holds tens of thousands of clips under a point
/// wide). The app caches the result per document revision and rect.
// soundcraft: crates/ui-egui/src/edit_window/universe.rs:15 `universe_runs`.
pub fn overview_runs(tracks: &[Vec<(TimeRange, Color32)>], total: Tick, rect: Rect) -> Vec<OverviewRun> {
    todo!()
}

/// Pro Tools' Universe (SoundCraft): the runs, the visible window as a box (1.5-point outline
/// in the `timeline.overview_view` colour), the playhead while playing; a click or drag centres
/// the window at the pointer, changing `view.start` directly. Total time is the content end
/// plus 10 seconds. Not a Tab stop (the panel scrolls by keys); AccessKit: an image named
/// "Overview".
pub struct TimelineOverview<'a> {
    id_salt: IdSalt,
    runs: &'a [OverviewRun],
    view: &'a mut TimeView,
    view_width: f32,
    end: Tick,
    playhead: Option<Tick>,
}

impl<'a> TimelineOverview<'a> {
    /// `view_width`: the timeline's time area in points (for the box's width).
    pub fn new(id_salt: impl AsIdSalt, runs: &'a [OverviewRun], view: &'a mut TimeView, view_width: f32, end: Tick) -> Self {
        Self { id_salt: IdSalt::new(id_salt), runs, view, view_width, end, playhead: None }
    }

    pub fn playhead(mut self, t: Option<Tick>) -> Self {
        self.playhead = t;
        self
    }

    // soundcraft: crates/ui-egui/src/edit_window/universe.rs:60 `universe`.
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
