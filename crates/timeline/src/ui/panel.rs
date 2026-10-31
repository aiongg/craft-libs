//! The timeline panel: track headers, track rows with clips, the ruler, the playhead and the
//! scroll bars, over the app's [`TimelineModel`] (N1.07).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditEvent;
use egui::{AsIdSalt, IdSalt, Rect, Response, Ui};

use super::LabelPalette;
use super::clips::TimelineTool;
use crate::clips::ClipEdit;
use crate::clock::FollowMode;
use crate::markers::{MarkerColor, MarkerEdit};
use crate::model::{ClipId, TimelineModel, TrackId, TrackInfo};
use crate::time::{Tick, TimeRange};

craft_ui::words! {
    /// The panel's tooltips and names, English by default (FilmCraft's).
    #[non_exhaustive]
    pub struct TimelineLabels {
        panel: "timeline.panel" = "Timeline";
        snap: "timeline.snap" = "Snap in Timeline (S)";
        linked_selection: "timeline.linkedSelection" = "Linked Selection";
        add_marker: "timeline.addMarker" = "Add Marker (M)";
        settings: "timeline.settings" = "Timeline Display Settings";
        track_color: "timeline.trackColor" = "Track Color";
        lock: "timeline.lock" = "Toggle Track Lock";
        target: "timeline.target" = "Toggle track targeting";
        sync_lock: "timeline.syncLock" = "Toggle Sync Lock";
        output: "timeline.output" = "Toggle Track Output";
        mute: "timeline.mute" = "Mute Track";
        solo: "timeline.solo" = "Solo Track";
        no_label: "timeline.noLabel" = "None";
        empty: "timeline.empty" = "Drop media here to create sequence.";
    }
}

/// A track header switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrackSwitch {
    Locked,
    Targeted,
    SyncLock,
    Enabled,
    Muted,
    Solo,
}

/// What the panel asks of the app. Clip and playhead edits come as `EditEvent` gestures: the
/// app previews updates and applies the commit as one undo step.
#[derive(Clone, Debug, PartialEq)]
pub enum TimelineEdit {
    Clip(EditEvent<ClipEdit>),
    Playhead(EditEvent<Tick>),
    WorkArea(EditEvent<TimeRange>),
    Marker(MarkerEdit),
    /// A header switch was clicked; `value` is the new state. Alt-click on Solo or Mute
    /// (exclusive) arrives with `exclusive`.
    Switch {
        track: TrackId,
        switch: TrackSwitch,
        value: bool,
        exclusive: bool,
    },
    /// The colour strip's menu.
    Label {
        track: TrackId,
        label: Option<MarkerColor>,
    },
    /// A row's bottom edge was dragged; points.
    TrackHeight {
        track: TrackId,
        height: f32,
    },
    /// A click on a clip; `add` with Shift or Cmd.
    SelectClip {
        clip: ClipId,
        add: bool,
    },
    /// A click on empty track space.
    ClearSelection,
    /// A secondary click; the app opens its menu at the pointer.
    ContextMenu {
        track: Option<TrackId>,
        clip: Option<ClipId>,
    },
    /// A double-click on a clip (Premiere opens it in the Source monitor).
    OpenClip(ClipId),
}

/// Adds a track's extra header content.
pub type HeaderExtra<'a> = Box<dyn FnMut(&mut Ui, &TrackInfo<'_>) + 'a>;

/// The Timeline panel (Premiere's Timeline; EffectCraft's layer timeline and SoundCraft's Edit
/// window as options).
///
/// Layout, top to bottom: the large [`TimecodeField`](super::TimecodeField) and the toolbar
/// toggles (left, over the headers) beside the [`TimelineRuler`](super::TimelineRuler); then the
/// track rows: video tracks stacked upward from the divider (V1 nearest it), the divider
/// (5 points), audio tracks downward; each side scrolls vertically on its own; the
/// [`TimelineScrollBar`](super::TimelineScrollBar) at the bottom. Headers are
/// [`TrackHeader`]s, 204 points wide by default and resizable at their right edge; rows are
/// [`ClipLane`](super::ClipLane)s. The playhead line runs over every row; the snap line shows
/// while a gesture snaps.
///
/// Input:
/// - Wheel scrolls vertically, Shift+wheel horizontally, Alt+wheel (Cmd+wheel on macOS) zooms
///   about the pointer; a pinch zooms; the view animates to its target as FilmCraft's does.
/// - `=` and `-` zoom in and out about the playhead, `\` fits the sequence.
/// - While playing, the view follows the playhead by the [`FollowMode`]; a jump while stopped
///   reveals it ([`TimeView::scroll_to_reveal`](crate::TimeView::scroll_to_reveal)).
/// - A drop of media from the app is the app's (it reads the pointer over
///   [`TimelineResponse::row_at_pointer`]).
/// - Keyboard: the panel is one Tab stop region (focus region); Up / Down move between edit
///   points via the app, arrows step the playhead (through the ruler); the header buttons are
///   Tab stops in row order.
///
/// ```ignore
/// let response = Timeline::new(&mut doc.timeline).tool(app.tool).snap(app.snapping).show(ui);
/// for edit in response.edits { doc.apply(edit) }
/// ```
pub struct Timeline<'a, M: TimelineModel> {
    id_salt: IdSalt,
    model: &'a mut M,
    tool: TimelineTool,
    snap: bool,
    no_modifier_trims: bool,
    follow: FollowMode,
    playing: bool,
    header_width: Option<f32>,
    palette: Option<&'a LabelPalette>,
    header_extra: Option<HeaderExtra<'a>>,
    labels: Option<TimelineLabels>,
}

impl<'a, M: TimelineModel> Timeline<'a, M> {
    pub fn new(model: &'a mut M) -> Self {
        Self {
            id_salt: IdSalt::new("craft-timeline"),
            model,
            tool: TimelineTool::Selection,
            snap: true,
            no_modifier_trims: false,
            follow: FollowMode::Page,
            playing: false,
            header_width: None,
            palette: None,
            header_extra: None,
            labels: None,
        }
    }

    /// For two timelines in one window (Premiere's sequence tabs share one panel; the Source
    /// monitor's mini timeline is another).
    pub fn id_salt(mut self, salt: impl AsIdSalt) -> Self {
        self.id_salt = IdSalt::new(salt);
        self
    }

    pub fn tool(mut self, tool: TimelineTool) -> Self {
        self.tool = tool;
        self
    }

    /// Snapping (Premiere's S); targets are clip edges, the playhead, markers, In and Out.
    pub fn snap(mut self, on: bool) -> Self {
        self.snap = on;
        self
    }

    pub fn no_modifier_trims(mut self, on: bool) -> Self {
        self.no_modifier_trims = on;
        self
    }

    /// How the view follows the playhead while `playing`.
    pub fn follow(mut self, mode: FollowMode, playing: bool) -> Self {
        self.follow = mode;
        self.playing = playing;
        self
    }

    pub fn header_width(mut self, width: f32) -> Self {
        self.header_width = Some(width);
        self
    }

    pub fn palette(mut self, palette: &'a LabelPalette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// Extra header content per track, right of the switches (EffectCraft's columns, SoundCraft's
    /// strip buttons).
    pub fn header_extra(mut self, add: impl FnMut(&mut Ui, &TrackInfo<'_>) + 'a) -> Self {
        self.header_extra = Some(Box::new(add));
        self
    }

    pub fn labels(mut self, labels: TimelineLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/timeline.rs:263 `show`, `:468` `animate_view`, `:513`
    // (follow), `:757` `empty_state`; `timeline/layout.rs:61` `video_row_tops`;
    // `timeline/interact.rs:982` `interact`, `:215` `wheel_scroll`; `timeline/snap.rs:15`.
    pub fn show(self, ui: &mut Ui) -> TimelineResponse {
        todo!()
    }
}

/// What the panel did this pass.
pub struct TimelineResponse {
    pub response: Response,
    pub edits: Vec<TimelineEdit>,
    /// The track row under the pointer and the time there (drops, the app's own overlays).
    pub row_at_pointer: Option<(TrackId, Tick)>,
    /// The time area's rect this pass (the app's overlays: captions, effect drop previews).
    pub time_area: Rect,
}

/// One track's header: the colour label strip (5 points, its menu lists the labels and None),
/// then the switches the [`TrackInfo`] has (`Some`): lock, target, sync lock, output, mute,
/// solo, as icon toggles with the tooltips of [`TimelineLabels`]; the name; and a resize edge at
/// the bottom (vertical resize cursor; 8–600 points). The label tints the header by 18 %.
/// Each switch is a Tab stop; Space toggles; Alt-click on Mute or Solo is exclusive.
pub struct TrackHeader<'a> {
    track: &'a TrackInfo<'a>,
    palette: Option<&'a LabelPalette>,
    labels: Option<&'a TimelineLabels>,
}

impl<'a> TrackHeader<'a> {
    pub fn new(track: &'a TrackInfo<'a>) -> Self {
        Self { track, palette: None, labels: None }
    }

    pub fn palette(mut self, palette: &'a LabelPalette) -> Self {
        self.palette = Some(palette);
        self
    }

    pub fn labels(mut self, labels: &'a TimelineLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/timeline/headers.rs:38 `draw_headers`, `:202`
    // `header_line_buttons`; `timeline.rs:254` `TRACK_HEADER_TINT`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> TrackHeaderResponse {
        todo!()
    }
}

/// What a header did this pass (the panel turns it into [`TimelineEdit`]s).
pub struct TrackHeaderResponse {
    pub response: Response,
    pub edits: Vec<TimelineEdit>,
}
