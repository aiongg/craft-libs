//! The timeline panel: track headers, track rows with clips, the ruler, the playhead and the
//! scroll bars, over the app's [`TimelineModel`] (N1.07).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::SelectionChange;
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
    /// A row's bottom edge dragged: the new height in points, clamped to
    /// `style.track_height_min`…`style.track_height_max`, as one gesture (Esc cancels).
    TrackHeight {
        track: TrackId,
        height: EditEvent<f32>,
    },
    /// The headers' right edge dragged: their new width in points, as one gesture. A view
    /// setting the app keeps beside [`TimeView`](crate::TimeView), never in undo.
    HeaderWidth(EditEvent<f32>),
    /// A click on a clip or empty track space, or a marquee: the selection change the
    /// kernel's list rules give, with Premiere's picks (a click replaces, Shift+click toggles the
    /// clip, a Shift marquee adds; a click on empty space clears). `unlinked` with Cmd/Ctrl (Alt
    /// on Windows, Premiere's): the app leaves the clip's linked partners out. The app expands
    /// linked partners when Linked Selection is on, and keeps the selection
    /// ([`TimelineModel::selection`]).
    Select {
        change: SelectionChange<ClipId>,
        unlinked: bool,
    },
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
/// toggles (left, over the headers, `style.top_height` high) beside the
/// [`TimelineRuler`](super::TimelineRuler); then the track rows: video tracks stacked upward
/// from the divider (V1 nearest it), the divider (`style.divider_height`), audio tracks
/// downward; each side scrolls vertically on its own; the
/// [`TimelineScrollBar`](super::TimelineScrollBar) at the bottom. Headers are [`TrackHeader`]s,
/// `style.header_width` wide unless [`header_width`](Self::header_width) says otherwise and
/// resizable at their right edge ([`TimelineEdit::HeaderWidth`]); rows are
/// [`ClipLane`](super::ClipLane)s. The playhead line runs over every row; the snap line shows
/// while a gesture snaps. `style` is [`TimelineStyle`](super::TimelineStyle) (`timeline.*` tokens). The toolbar
/// toggles are [`ToggleButton`](craft_ui::controls::ToggleButton)s the app owns the state of
/// (Snap, Linked Selection), reported through their responses on
/// [`TimelineOutput::toolbar`].
///
/// Input:
/// - Wheel scrolls vertically, Shift+wheel horizontally, Alt+wheel (Cmd+wheel on macOS) zooms
///   about the pointer; a pinch zooms; the view animates to its target as FilmCraft's does.
/// - `=` and `-` zoom in and out about the playhead, `\` fits the sequence.
/// - While playing, the view follows the playhead by the [`FollowMode`]; a jump while stopped
///   reveals it ([`TimeView::scroll_to_reveal`](crate::TimeView::scroll_to_reveal)).
/// - A drop of media from the app is the app's (it reads the pointer over
///   [`TimelineResponse::row_at_pointer`]).
/// - Keyboard: the panel is a focus region (F6 reaches it); the time area is one Tab stop
///   whose Left / Right step the playhead a frame (Shift: five frames, Premiere's), Home / End
///   go to the start and end, Up / Down to the previous and next edit point (the app's, through
///   [`TimelineModel`]); the header switches are Tab stops in row order; Delete on selected
///   clips is the app's shortcut (the panel claims no keys beyond these).
/// - AccessKit: the time area is a `Group` named by [`TimelineLabels::panel`]; each track a
///   `Row`; each clip a `Button` named by its name with its start and duration as the value
///   (the [`TimeFormat`](crate::TimeFormat) of the model), `selected` set.
///
/// ```ignore
/// let output = Timeline::new(("timeline", seq_id), &mut model).tool(app.tool).snap(app.snapping).show(ui);
/// for edit in output.edits { app.apply(edit) }
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
    /// `id_salt` keeps the panel's scroll, focus and gestures apart from another timeline in the
    /// window (Premiere's sequence tabs share one panel; the Source monitor's mini timeline is
    /// another): `("timeline", sequence_id)`.
    pub fn new(id_salt: impl AsIdSalt, model: &'a mut M) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
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

    /// The headers' width the user dragged to ([`TimelineEdit::HeaderWidth`]), kept by the app.
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
    pub fn show(self, ui: &mut Ui) -> TimelineOutput {
        todo!()
    }
}

/// What the panel did this pass.
#[must_use]
pub struct TimelineOutput {
    pub response: Response,
    /// This pass's edits, in the order the user made them.
    pub edits: Vec<TimelineEdit>,
    /// The toolbar toggles' responses, Snap and Linked Selection (the app flips its settings on
    /// a click).
    pub toolbar: TimelineToolbar,
    /// The track row under the pointer and the time there (drops, the app's own overlays).
    pub row_at_pointer: Option<(TrackId, Tick)>,
    /// The time area's rect this pass (the app's overlays: captions, effect drop previews).
    pub time_area: Rect,
}

/// The Snap and Linked Selection toggles' responses.
pub struct TimelineToolbar {
    pub snap: Response,
    pub linked_selection: Response,
    pub add_marker: Response,
    pub settings: Response,
}

/// One track's header (Premiere's): the colour label strip (`style.label_strip` wide; a click
/// opens a [`PressMenu`](craft_ui::menu::PressMenu) of the labels and None), then the switches
/// the [`TrackInfo`] has (`Some`): lock, target, sync lock, output, mute, solo, as icon
/// [`ToggleButton`](craft_ui::controls::ToggleButton)s with the tooltips of
/// [`TimelineLabels`]; the name; and a resize edge at the bottom (vertical resize cursor;
/// `style.track_height_min`…`style.track_height_max`). The label tints the header by
/// `style.header_label_tint` (a proportion). Each switch is a Tab stop; Space toggles;
/// Alt-click on Mute or Solo is exclusive. AccessKit: each switch a toggle `Button` named by its
/// tooltip and the track's name.
pub struct TrackHeader<'a> {
    id_salt: IdSalt,
    track: &'a TrackInfo<'a>,
    palette: Option<&'a LabelPalette>,
    labels: Option<&'a TimelineLabels>,
}

impl<'a> TrackHeader<'a> {
    pub fn new(id_salt: impl AsIdSalt, track: &'a TrackInfo<'a>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), track, palette: None, labels: None }
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
    pub fn show(self, ui: &mut Ui, rect: Rect) -> TrackHeaderOutput {
        todo!()
    }
}

/// What a header did this pass (the panel passes its edits on as [`TimelineEdit`]s).
#[must_use]
pub struct TrackHeaderOutput {
    pub response: Response,
    pub edits: Vec<TimelineEdit>,
}
