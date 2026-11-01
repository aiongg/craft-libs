//! The time ruler and the timeline scroll bar (N1.01).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditEvent;
use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::markers::MarkerEdit;
use crate::clips::SnapTargets;
use crate::markers::Marker;
use crate::time::{Tick, TimeRange, TimeView};
use crate::timecode::TimeFormat;

craft_ui::words! {
    /// The ruler's accessible names, English by default.
    #[non_exhaustive]
    pub struct TimelineRulerLabels {
        /// The ruler's AccessKit name; its value is the playhead's time.
        ruler: "timelineRuler.ruler" = "Time Ruler";
        work_area: "timelineRuler.workArea" = "Work Area";
        work_area_start: "timelineRuler.workAreaStart" = "Work Area Start";
        work_area_end: "timelineRuler.workAreaEnd" = "Work Area End";
        scroll_bar: "timelineRuler.scrollBar" = "Zoom and Scroll";
    }
}

/// One ruler tick.
#[derive(Clone, Debug, PartialEq)]
pub struct RulerTick {
    pub at: Tick,
    /// The label of a major tick ("00:00:10:00", "5|1"); minor ticks have none.
    pub label: Option<String>,
    pub major: bool,
}

/// Ticks for a time scale the library doesn't know: SoundCraft's Bars|Beats from its tempo map.
pub trait RulerLabels {
    /// The ticks within `visible`, at `points_per_second`.
    fn ticks(&self, visible: TimeRange, points_per_second: f64) -> Vec<RulerTick>;
}

/// What the ruler counts in.
#[derive(Clone, Copy)]
pub enum RulerScale<'a> {
    /// Frames, labelled in the format's display (Premiere's timecode; After Effects' frame
    /// numbers with [`TimeDisplay::Frames`](crate::TimeDisplay::Frames)).
    Format(TimeFormat),
    /// The app's ticks.
    Custom(&'a dyn RulerLabels),
}

/// The ruler's two strides in frames, `(tick, label)`. `label` is the first of
/// `[1, 2, 5, 10, base/2, base, 2·base, 5·base, 10·base, 15·base, 30·base, 60·base, 120·base,
/// 300·base, 600·base, 1800·base, 3600·base]` frames whose width at `frame_points` (points per
/// frame) is at least `label_gap` (`timeline.ruler_label_gap`), so timestamps never touch;
/// `tick` is the finest divisor `label / k` (k from 9 down to 1) that leaves ticks at least
/// `tick_gap` apart (`timeline.ruler_tick_gap`), so every label sits on a tick. Steps past
/// `base` saturate (rates are file data).
// filmcraft: crates/ui-egui/src/panels/timeline/ruler.rs:12 `ruler_steps`.
pub fn ruler_steps(frame_points: f64, base: i64, label_gap: f32, tick_gap: f32) -> (i64, i64) {
    todo!()
}

/// The ticks of a [`RulerScale::Format`] ruler over a view `width` points wide: one every
/// [`ruler_steps`] tick stride from the first one at or before the left edge, labelled every
/// label stride with [`format_time`](crate::format_time); the gaps from `style`.
// filmcraft: crates/ui-egui/src/panels/timeline/ruler.rs:24 `draw_top` (the tick loop).
pub fn ruler_ticks(format: &TimeFormat, view: &TimeView, width: f32, style: &super::TimelineStyle) -> Vec<RulerTick> {
    todo!()
}

/// The ruler across the top of a timeline: ticks and labels, the playhead's head, In and Out
/// shading, the work-area bar, cache bars and the sequence markers (through
/// [`MarkerStrip`](super::MarkerStrip)).
///
/// - Press and drag scrubs the playhead (snapped to `snap` targets when given; Shift snaps when
///   no targets are given, as After Effects), reported as an `EditEvent<Tick>` gesture; Esc
///   during a scrub cancels it back to where it began.
/// - The work-area bar drags by its middle (moves) or an end (moves that end), Shift snaps
///   (`style.work_area_grab`; EffectCraft `timeline.rs:538`), reported as an `EditEvent<TimeRange>` gesture;
///   double-clicking it sets it to the visible span.
/// - Wheel over the ruler zooms about the pointer (Alt+wheel in Premiere zooms anywhere; the
///   panel decides), changing `view` directly.
/// - Keyboard: one Tab stop. Left and Right step the playhead one frame (Shift: five), Home and
///   End go to the start and the end; each a `Set` event. Visible focus ring around the ruler.
/// - AccessKit: a `Slider` named by [`TimelineRulerLabels::ruler`], value the playhead's text.
/// - Sizes from [`TimelineStyle`](super::TimelineStyle): `style.ruler_height` unless
///   [`height`](Self::height) says otherwise.
pub struct TimelineRuler<'a> {
    id_salt: IdSalt,
    view: &'a mut TimeView,
    scale: RulerScale<'a>,
    format: TimeFormat,
    playhead: Tick,
    end: Tick,
    in_out: (Option<Tick>, Option<Tick>),
    work_area: Option<TimeRange>,
    cached: &'a [TimeRange],
    disk_cached: &'a [TimeRange],
    markers: &'a [Marker],
    snap: Option<&'a SnapTargets>,
    height: Option<f32>,
    labels: Option<TimelineRulerLabels>,
}

impl<'a> TimelineRuler<'a> {
    /// A ruler over `view` showing `format`'s times, for content ending at `end`.
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut TimeView, format: TimeFormat, end: Tick) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            view,
            scale: RulerScale::Format(format),
            format,
            playhead: Tick::ZERO,
            end,
            in_out: (None, None),
            work_area: None,
            cached: &[],
            disk_cached: &[],
            markers: &[],
            snap: None,
            height: None,
            labels: None,
        }
    }

    /// Custom ticks (Bars|Beats); scrubbing still reports ticks and shows `format`'s text.
    pub fn scale(mut self, scale: RulerScale<'a>) -> Self {
        self.scale = scale;
        self
    }

    pub fn playhead(mut self, t: Tick) -> Self {
        self.playhead = t;
        self
    }

    /// Shades outside Mark In … Mark Out.
    pub fn in_out(mut self, marks: (Option<Tick>, Option<Tick>)) -> Self {
        self.in_out = marks;
        self
    }

    pub fn work_area(mut self, range: Option<TimeRange>) -> Self {
        self.work_area = range;
        self
    }

    /// Ranges with frames in RAM (green bar) and on disk (blue bar), After Effects' cache bars
    /// (EffectCraft `timeline.rs:2379`); Premiere's render bar uses the same.
    pub fn cache_bars(mut self, ram: &'a [TimeRange], disk: &'a [TimeRange]) -> Self {
        self.cached = ram;
        self.disk_cached = disk;
        self
    }

    pub fn markers(mut self, markers: &'a [Marker]) -> Self {
        self.markers = markers;
        self
    }

    /// Scrubbing snaps to these targets (Premiere with Snap on).
    pub fn snap(mut self, targets: Option<&'a SnapTargets>) -> Self {
        self.snap = targets;
        self
    }

    /// Height in points, for a ruler stacked in a row of another height (SoundCraft's ruler
    /// rows); `style.ruler_height` by default.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn labels(mut self, labels: TimelineRulerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/timeline/ruler.rs:24 `draw_top` (ruler part), `:176`
    // `draw_markers`; effectcraft: crates/ui-egui/src/panels/timeline.rs:2308 `work_area_bar`,
    // `:2379` `cache_bars`, `:2483` `ruler_scrub`.
    pub fn show(self, ui: &mut Ui) -> TimelineRulerResponse {
        todo!()
    }
}

/// What the ruler did this pass.
pub struct TimelineRulerResponse {
    pub response: Response,
    /// The playhead scrub, a gesture or a key's `Set`.
    pub scrub: Option<EditEvent<Tick>>,
    pub work_area: Option<EditEvent<TimeRange>>,
    pub markers: Vec<MarkerEdit>,
    /// Where scrubbing snapped this pass (for a snap line across the tracks).
    pub snapped_at: Option<Tick>,
}

/// Premiere's timeline scroll bar: the thumb spans the visible time over the view's extent
/// ([`TimeView::max_start`]); dragging its middle scrolls, dragging an end zooms (that end moves,
/// the other stays); the thumb is at least `style.scroll_thumb_min` wide. Changes `view` directly (view changes
/// are not edits). Not a Tab stop: the panel's `=` / `-` and arrow keys zoom and scroll.
/// EffectCraft places it above the ruler as its time navigator (`timeline.rs:2245`).
pub struct TimelineScrollBar<'a> {
    id_salt: IdSalt,
    view: &'a mut TimeView,
    end: Tick,
}

impl<'a> TimelineScrollBar<'a> {
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut TimeView, end: Tick) -> Self {
        Self { id_salt: IdSalt::new(id_salt), view, end }
    }

    // filmcraft: crates/ui-egui/src/panels/timeline/scrollbars.rs:6 `MIN_THUMB_W`, `:12`
    // `zoom_thumb`, `:24` `zoom_scrollbar`.
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
