//! Stories of the timeline widgets. Not registered yet: registering them would change the registry
//! dump and run their bodies. The registration task adds them to the gallery, the words of
//! [`TimelineStoryLabels`] to the words registry, and regenerates the dump.
//!
//! Each story keeps its own demo state in egui memory and applies the widget's events to it as
//! an app would. The line under a story names the last event. Sample clip, marker and queue
//! names are words of [`TimelineStoryLabels`]; track names (`V1`, `A2`) are identifiers.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/timeline/panel` | a 30 s sequence at 29.97 drop-frame: V1–V2, A1–A2, a marker, In and Out, the playhead at 5 s |
//! | `composites/timeline-ruler/ruler` | the ruler at frame, second and minute zoom, with work area, cache bars and markers |
//! | `composites/keyframes/lane` | a lane of Linear, Hold and Bezier keys (two selected) with the toggle and navigator |
//! | `patterns/graph-editor/editor` | Position X and Y curves, one key selected with its handles |
//! | `composites/transport/bar` | the Program row, playing and looping |
//! | `composites/transport/media-controls` | the media controls at 0:12.50 of 1:05.00 with trim marks |
//! | `controls/timecode-field/field` | a large and a small field at 29.97 drop-frame |
//! | `patterns/timecode-panel/panel` | Current Time, Duration and Remaining rows |
//! | `composites/frame-preview/preview` | a 4:3 test card letterboxed in a 16:9 box, and a cropped one |
//! | `patterns/marker-dialog/dialog` | the Marker dialog on a range marker |
//! | `composites/timeline-overview/overview` | eight tracks of runs and the visible box |
//! | `patterns/render-queue/list` | Ready, Encoding 42 %, Done, Failed and Cancelled items |

use craft_ui::behavior::{DropPosition, Move, Selection};
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, NoArgs, Story, StoryArgs, StoryLabels, drive, lock, shared};
use craft_ui::theme::{Tokens, TokensOf as _};
use craft_ui::typed_story;
use egui::{Color32, ColorImage, Sense, TextureHandle, TextureOptions, Ui, vec2};
use std::time::Duration;

use super::graph::{GraphAction, GraphCurve, GraphEditor, GraphKey, GraphRange, HandleSide};
use super::keyframes::{AnimationToggle, KeyframeId, KeyframeInterpolation, KeyframeLane, KeyframeMark, KeyframeNavigator};
use super::markers::{MarkerDialog, MarkerEdit};
use super::overview::{TimelineOverview, overview_runs};
use super::panel::{Timeline, TimelineEdit, TrackSwitch};
use super::preview::{FrameFit, FramePreview};
use super::render_queue::{RenderItemId, RenderQueueEvent, RenderQueueItem, RenderQueueList, RenderStatus};
use super::ruler::TimelineRuler;
use super::timecode::{TimecodeField, TimecodePanel, TimecodePanelSettings, TimecodeRow, TimecodeRowKind, TimecodeSource};
use super::transport::{MediaControls, TransportBar, TransportBarSettings, TransportButton};
use super::{LabelPalette, TimelineStyle};
use crate::clips::{ClipEdge, ClipEdit, TrimKind};
use crate::markers::{Marker, MarkerColor, MarkerId};
use crate::model::{ClipId, ClipInfo, TimelineModel, TrackId, TrackInfo, TrackKind};
use crate::rates::FrameRate;
use crate::time::{Tick, TimeRange, TimeView};
use crate::timecode::{TimeFormat, format_time};

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/timeline/panel", Level::Patterns, "Timeline", "Panel", NoArgs {}, timeline_panel),
        typed_story!("composites/timeline-ruler/ruler", Level::Composites, "Timeline ruler", "Ruler", NoArgs {}, ruler),
        typed_story!("composites/keyframes/lane", Level::Composites, "Keyframes", "Lane", NoArgs {}, keyframe_lane),
        typed_story!("patterns/graph-editor/editor", Level::Patterns, "Graph editor", "Editor", NoArgs {}, graph_editor),
        typed_story!("composites/transport/bar", Level::Composites, "Transport", "Bar", NoArgs {}, transport_bar),
        typed_story!("composites/transport/media-controls", Level::Composites, "Transport", "Media controls", NoArgs {}, media_controls),
        typed_story!("controls/timecode-field/field", Level::Controls, "Timecode field", "Field", NoArgs {}, timecode_field),
        typed_story!("patterns/timecode-panel/panel", Level::Patterns, "Timecode panel", "Panel", NoArgs {}, timecode_panel),
        typed_story!("composites/frame-preview/preview", Level::Composites, "Frame preview", "Preview", NoArgs {}, frame_preview),
        typed_story!("patterns/marker-dialog/dialog", Level::Patterns, "Marker dialog", "Dialog", DialogArgs { open: true }, marker_dialog),
        typed_story!("composites/timeline-overview/overview", Level::Composites, "Timeline overview", "Overview", NoArgs {}, overview),
        typed_story!("patterns/render-queue/list", Level::Patterns, "Render queue", "List", NoArgs {}, render_queue),
    ]
}

craft_ui::words! {
    /// The words the timeline stories add: sample clips, markers, curves, previews and render
    /// items.
    #[non_exhaustive]
    pub(crate) struct TimelineStoryLabels {
        /// Clip names of the sample sequence.
        clip_interview: "timelineStory.clipInterview" = "Interview A-cam";
        clip_city: "timelineStory.clipCity" = "City b-roll";
        clip_title: "timelineStory.clipTitle" = "Title card";
        clip_logo: "timelineStory.clipLogo" = "Logo sting";
        clip_voice: "timelineStory.clipVoice" = "Voice-over";
        clip_music: "timelineStory.clipMusic" = "Music bed";
        /// Marker names and the range marker's comment.
        marker_check: "timelineStory.markerCheck" = "Check this cut";
        marker_swell: "timelineStory.markerSwell" = "Music swell";
        marker_swell_comment: "timelineStory.markerSwellComment" = "Let the music rise under the title.";
        /// The three rulers' captions.
        zoom_frames: "timelineStory.zoomFrames" = "Frames";
        zoom_seconds: "timelineStory.zoomSeconds" = "Seconds";
        zoom_minutes: "timelineStory.zoomMinutes" = "Minutes";
        /// The graph's curves.
        curve_x: "timelineStory.curveX" = "Position X";
        curve_y: "timelineStory.curveY" = "Position Y";
        /// The preview's frames and status line.
        preview_card: "timelineStory.previewCard" = "Test card";
        preview_cropped: "timelineStory.previewCropped" = "Test card, cropped";
        preview_status: "timelineStory.previewStatus" = "{name} · {time}";
        /// The render queue's sequences and the failure message.
        render_interview: "timelineStory.renderInterview" = "Interview cut";
        render_trailer: "timelineStory.renderTrailer" = "Trailer";
        render_highlights: "timelineStory.renderHighlights" = "Highlights";
        render_behind: "timelineStory.renderBehind" = "Behind the scenes";
        render_teaser: "timelineStory.renderTeaser" = "Teaser";
        render_error: "timelineStory.renderError" = "Not enough disk space";
    }
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct DialogArgs {
    open: bool,
}

const RATE: FrameRate = FrameRate::FPS_29_97;

/// 29.97 drop-frame timecode, the sample sequences' format.
fn format() -> TimeFormat {
    TimeFormat::timecode(RATE).drop_frame(true)
}

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

fn span(start: f64, end: f64) -> TimeRange {
    TimeRange::from_bounds(secs(start), secs(end))
}

fn last_event(ui: &mut Ui, event: Option<&str>) {
    if let Some(event) = event {
        let line = fmt(&StoryLabels::of(ui).last_event, &[("event", event)]);
        ui.add(Text::new(line).content(Content::Muted));
    }
}

/// The value a gesture or a one-shot change has settled on.
fn settled<T>(event: &EditEvent<T>) -> Option<&T> {
    match event {
        EditEvent::Commit { value, changed: true, .. } | EditEvent::Set { value, .. } => Some(value),
        _ => None,
    }
}

/// The value a gesture has now, for a control the app previews live (a scrub).
fn live<T>(event: &EditEvent<T>) -> Option<&T> {
    match event {
        EditEvent::Begin { value, .. } => value.as_ref(),
        EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } => Some(value),
        _ => None,
    }
}

fn apply_marker(markers: &mut Vec<Marker>, edit: &MarkerEdit) {
    match edit {
        MarkerEdit::Move { id, to } => {
            if let (Some(&time), Some(marker)) = (live(to), markers.iter_mut().find(|m| m.id == *id)) {
                marker.start = time;
            }
        }
        MarkerEdit::SetDuration { id, duration } => {
            if let (Some(&time), Some(marker)) = (live(duration), markers.iter_mut().find(|m| m.id == *id)) {
                marker.duration = time.max(Tick::ZERO);
            }
        }
        MarkerEdit::Delete(id) => markers.retain(|m| m.id != *id),
        MarkerEdit::Add { at } => {
            let id = MarkerId(markers.iter().map(|m| m.id.0).max().unwrap_or(0) + 1);
            markers.push(Marker { id, start: *at, ..Marker::default() });
        }
        MarkerEdit::Open(_) => {}
    }
}

// ---- The panel ----------------------------------------------------------------------------

struct TrackData {
    id: TrackId,
    kind: TrackKind,
    name: &'static str,
    height: Option<f32>,
    label: Option<MarkerColor>,
    locked: bool,
    targeted: bool,
    enabled: bool,
    muted: bool,
    solo: bool,
}

struct ClipData {
    id: ClipId,
    track: TrackId,
    range: TimeRange,
    name: String,
    label: MarkerColor,
    enabled: bool,
    source: TimeRange,
}

/// The sample sequence: the model the panel reads and the edits are applied to.
struct Sequence {
    tracks: Vec<TrackData>,
    clips: Vec<ClipData>,
    markers: Vec<Marker>,
    selection: Selection<ClipId>,
    view: TimeView,
    playhead: Tick,
    marks: (Option<Tick>, Option<Tick>),
    work_area: Option<TimeRange>,
    header_width: Option<f32>,
}

const SEQUENCE_SECONDS: f64 = 30.0;

impl TimelineModel for Sequence {
    fn duration(&self) -> Tick {
        secs(SEQUENCE_SECONDS)
    }

    fn format(&self) -> TimeFormat {
        format()
    }

    fn playhead(&self) -> Tick {
        self.playhead
    }

    fn view(&mut self) -> &mut TimeView {
        &mut self.view
    }

    fn tracks(&self) -> Vec<TrackInfo<'_>> {
        let audio = |t: &TrackData| t.kind == TrackKind::Audio;
        self.tracks
            .iter()
            .map(|t| TrackInfo {
                id: t.id,
                kind: t.kind,
                name: t.name,
                height: t.height,
                depth: 0,
                label: t.label,
                locked: Some(t.locked),
                targeted: Some(t.targeted),
                sync_lock: None,
                enabled: Some(t.enabled),
                muted: audio(t).then_some(t.muted),
                solo: audio(t).then_some(t.solo),
            })
            .collect()
    }

    fn clips(&self, track: TrackId, visible: TimeRange) -> Vec<ClipInfo<'_>> {
        self.clips
            .iter()
            .filter(|c| c.track == track && c.range.overlaps(&visible))
            .map(|c| ClipInfo {
                id: c.id,
                track: c.track,
                range: c.range,
                name: &c.name,
                label: Some(c.label),
                enabled: c.enabled,
                source: Some(c.source),
            })
            .collect()
    }

    fn selection(&self) -> &[ClipId] {
        &self.selection.items
    }

    fn markers(&self) -> &[Marker] {
        &self.markers
    }

    fn edit_points(&self) -> Vec<Tick> {
        let mut points: Vec<Tick> = self.clips.iter().flat_map(|c| [c.range.start, c.range.end()]).collect();
        points.sort();
        points.dedup();
        points
    }

    fn in_out(&self) -> (Option<Tick>, Option<Tick>) {
        self.marks
    }

    fn work_area(&self) -> Option<TimeRange> {
        self.work_area
    }
}

const V1: TrackId = TrackId(1);
const V2: TrackId = TrackId(2);
const A1: TrackId = TrackId(3);
const A2: TrackId = TrackId(4);

/// The seconds the panel shows when the story starts.
const SHOWN_SECONDS: f32 = 32.0;
/// Rows shown above and below the divider.
const ROWS_PER_SIDE: f32 = 3.0;

fn sample_sequence(labels: &TimelineStoryLabels, points_per_second: f32) -> Sequence {
    let track = |id: TrackId, kind: TrackKind, name: &'static str| TrackData {
        id,
        kind,
        name,
        height: None,
        label: None,
        locked: false,
        targeted: true,
        enabled: true,
        muted: false,
        solo: false,
    };
    let clip = |id: u64, track: TrackId, name: &str, from: f64, to: f64, label: u8| ClipData {
        id: ClipId(id),
        track,
        range: span(from, to),
        name: name.into(),
        label: MarkerColor(label),
        enabled: true,
        source: span((from - 3.0).max(0.0), to + 3.0),
    };
    let mut logo = clip(4, V2, &labels.clip_logo, 14.0, 20.0, 3);
    logo.enabled = false;
    let marker = |id: u64, at: f64, length: f64, name: &str, comment: &str, color: u8| Marker {
        id: MarkerId(id),
        start: secs(at),
        duration: secs(length),
        name: name.into(),
        comment: comment.into(),
        color: MarkerColor(color),
    };
    Sequence {
        tracks: vec![
            track(V2, TrackKind::Video, "V2"),
            track(V1, TrackKind::Video, "V1"),
            track(A1, TrackKind::Audio, "A1"),
            track(A2, TrackKind::Audio, "A2"),
        ],
        clips: vec![
            clip(1, V1, &labels.clip_interview, 0.0, 12.0, 1),
            clip(2, V1, &labels.clip_city, 12.0, 26.0, 5),
            clip(3, V2, &labels.clip_title, 2.0, 9.0, 7),
            logo,
            clip(5, A1, &labels.clip_voice, 0.0, 22.0, 2),
            clip(6, A2, &labels.clip_music, 1.0, 29.0, 10),
        ],
        markers: vec![marker(1, 12.0, 0.0, &labels.marker_check, "", 0), marker(2, 20.0, 4.0, &labels.marker_swell, "", 8)],
        selection: Selection::only(ClipId(2)),
        view: TimeView::new(Tick::ZERO, f64::from(points_per_second)),
        playhead: secs(5.0),
        marks: (Some(secs(2.0)), Some(secs(22.0))),
        work_area: None,
        header_width: None,
    }
}

impl Sequence {
    fn clip_mut(&mut self, id: ClipId) -> Option<&mut ClipData> {
        self.clips.iter_mut().find(|c| c.id == id)
    }

    /// The tracks of `kind`, counted away from the divider (V1 and A1 first).
    fn lanes(&self, kind: TrackKind) -> Vec<TrackId> {
        let mut ids: Vec<TrackId> = self.tracks.iter().filter(|t| t.kind == kind).map(|t| t.id).collect();
        if kind == TrackKind::Video {
            ids.reverse();
        }
        ids
    }

    fn apply_clip(&mut self, edit: &ClipEdit) {
        let frame = RATE.tick_of(1);
        match edit {
            ClipEdit::Move { clips, delta, track_delta, .. } => {
                for id in clips {
                    let Some(track) = self.clips.iter().find(|c| c.id == *id).map(|c| c.track) else { continue };
                    let target = self.tracks.iter().find(|t| t.id == track).and_then(|t| {
                        let lanes = self.lanes(t.kind);
                        let at = lanes.iter().position(|l| *l == track)? as i32 + track_delta;
                        usize::try_from(at).ok().and_then(|at| lanes.get(at).copied())
                    });
                    if let Some(clip) = self.clip_mut(*id) {
                        clip.range.start = (clip.range.start + *delta).max(Tick::ZERO);
                        if let Some(target) = target {
                            clip.track = target;
                        }
                    }
                }
            }
            ClipEdit::Trim { clip, edge, kind, delta, .. } => {
                let Some(old) = self.clips.iter().find(|c| c.id == *clip).map(|c| (c.range, c.track)) else { return };
                let (old, track) = old;
                let new = match (edge, kind) {
                    (ClipEdge::In, TrimKind::Ripple) => TimeRange::from_bounds(old.start, old.end() - *delta),
                    (ClipEdge::In, _) => TimeRange::from_bounds(old.start + *delta, old.end()),
                    (ClipEdge::Out, _) => TimeRange::from_bounds(old.start, old.end() + *delta),
                };
                if new.duration < frame {
                    return;
                }
                if *kind == TrimKind::Ripple {
                    let shift = new.duration - old.duration;
                    for later in self.clips.iter_mut().filter(|c| c.track == track && c.id != *clip && c.range.start >= old.end()) {
                        later.range.start += shift;
                    }
                }
                if let Some(c) = self.clip_mut(*clip) {
                    c.range = new;
                }
            }
            ClipEdit::Roll { left, right, delta } => {
                let (Some(l), Some(r)) =
                    (self.clips.iter().find(|c| c.id == *left).map(|c| c.range), self.clips.iter().find(|c| c.id == *right).map(|c| c.range))
                else {
                    return;
                };
                if l.duration + *delta < frame || r.duration - *delta < frame {
                    return;
                }
                if let Some(c) = self.clip_mut(*left) {
                    c.range = TimeRange::new(l.start, l.duration + *delta);
                }
                if let Some(c) = self.clip_mut(*right) {
                    c.range = TimeRange::from_bounds(r.start + *delta, r.end());
                }
            }
            ClipEdit::Slide { clip, delta } => {
                let Some((old, track)) = self.clips.iter().find(|c| c.id == *clip).map(|c| (c.range, c.track)) else { return };
                for other in self.clips.iter_mut().filter(|c| c.track == track && c.id != *clip) {
                    if other.range.end() == old.start {
                        other.range = TimeRange::from_bounds(other.range.start, other.range.end() + *delta);
                    } else if other.range.start == old.end() {
                        other.range = TimeRange::from_bounds(other.range.start + *delta, other.range.end());
                    }
                }
                if let Some(c) = self.clip_mut(*clip) {
                    c.range.start += *delta;
                }
            }
            ClipEdit::Cut { clip, at } => {
                let Some(old) = self.clips.iter().find(|c| c.id == *clip) else { return };
                if !old.range.contains(*at) || *at - old.range.start < frame {
                    return;
                }
                let id = ClipId(self.clips.iter().map(|c| c.id.0).max().unwrap_or(0) + 1);
                let second = ClipData {
                    id,
                    track: old.track,
                    range: TimeRange::from_bounds(*at, old.range.end()),
                    name: old.name.clone(),
                    label: old.label,
                    enabled: old.enabled,
                    source: old.source,
                };
                if let Some(c) = self.clip_mut(*clip) {
                    c.range = TimeRange::from_bounds(c.range.start, *at);
                }
                self.clips.push(second);
            }
            // The media inside a clip is not modelled; the panel draws the slip's ghost itself.
            ClipEdit::Slip { .. } | ClipEdit::Select { .. } => {}
        }
    }

    fn apply(&mut self, edit: &TimelineEdit) {
        match edit {
            TimelineEdit::Clip(event) => {
                if let Some(edit) = settled(event) {
                    self.apply_clip(edit);
                }
            }
            TimelineEdit::Playhead(event) => {
                if let Some(&t) = live(event) {
                    self.playhead = t.clamp(Tick::ZERO, secs(SEQUENCE_SECONDS));
                }
            }
            TimelineEdit::WorkArea(event) => {
                if let Some(&range) = live(event) {
                    self.work_area = Some(range);
                }
            }
            TimelineEdit::Marker(edit) => apply_marker(&mut self.markers, edit),
            TimelineEdit::Switch { track, switch, value, exclusive } => {
                for t in &mut self.tracks {
                    let this = t.id == *track;
                    if !this && !(*exclusive && *value) {
                        continue;
                    }
                    let state = match switch {
                        TrackSwitch::Locked => &mut t.locked,
                        TrackSwitch::Targeted => &mut t.targeted,
                        TrackSwitch::Enabled => &mut t.enabled,
                        TrackSwitch::Muted => &mut t.muted,
                        TrackSwitch::Solo => &mut t.solo,
                        TrackSwitch::SyncLock => continue,
                    };
                    *state = if this { *value } else { false };
                }
            }
            TimelineEdit::Label { track, label } => {
                if let Some(t) = self.tracks.iter_mut().find(|t| t.id == *track) {
                    t.label = *label;
                }
            }
            TimelineEdit::TrackHeight { track, height } => {
                if let (Some(&h), Some(t)) = (live(height), self.tracks.iter_mut().find(|t| t.id == *track)) {
                    t.height = Some(h);
                }
            }
            TimelineEdit::HeaderWidth(event) => {
                if let Some(&w) = live(event) {
                    self.header_width = Some(w);
                }
            }
            TimelineEdit::Select { change, .. } => self.selection = change.selection.clone(),
            TimelineEdit::ContextMenu { .. } | TimelineEdit::OpenClip(_) => {}
        }
    }
}

struct PanelDemo {
    seq: Sequence,
    snap: bool,
    linked: bool,
    last: Option<String>,
}

// new: a 30 s sequence at 29.97 drop-frame over an in-memory model: V1–V2 with four clips (one
// selected, one disabled), A1–A2 with two, a marker, In and Out set, the playhead at 00;00;05;00.
// The keyframe lane has its own story: the panel's rows hold clips.
fn timeline_panel(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TimelineStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let style = TimelineStyle::of(&tokens);
    let width = ui.available_width();
    let demo = shared(ui, "panel", || PanelDemo {
        seq: sample_sequence(&labels, (width - style.header_width) / SHOWN_SECONDS),
        snap: true,
        linked: true,
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let height =
        style.top_height + style.divider_height + style.scrollbar_height + ROWS_PER_SIDE * (style.video_track_height + style.audio_track_height);
    let (snap, linked, header_width) = (demo.snap, demo.linked, demo.seq.header_width);
    let out = ui
        .allocate_ui(vec2(width, height), |ui| {
            let mut timeline = Timeline::new("timeline", &mut demo.seq).snap(snap).linked_selection(linked);
            if let Some(width) = header_width {
                timeline = timeline.header_width(width);
            }
            timeline.show(ui)
        })
        .inner;
    if out.toolbar.snap.clicked() {
        demo.snap = !demo.snap;
    }
    if out.toolbar.linked_selection.clicked() {
        demo.linked = !demo.linked;
    }
    if out.toolbar.add_marker.clicked() {
        let at = demo.seq.playhead;
        apply_marker(&mut demo.seq.markers, &MarkerEdit::Add { at });
        demo.last = Some(format!("{:?}", MarkerEdit::Add { at }));
    }
    for edit in &out.edits {
        demo.last = Some(format!("{edit:?}"));
        demo.seq.apply(edit);
    }
    last_event(ui, demo.last.as_deref());
}

// ---- The ruler ----------------------------------------------------------------------------

struct RulerDemo {
    views: [TimeView; 3],
    playhead: Tick,
    marks: (Option<Tick>, Option<Tick>),
    work_area: Option<TimeRange>,
    ram: Vec<TimeRange>,
    disk: Vec<TimeRange>,
    markers: Vec<Marker>,
    last: Option<String>,
}

/// The end of the ruler story's content.
const RULER_SECONDS: f64 = 90.0;

// new: the ruler at three zooms (frames, seconds, minutes), with a work area and cache bars.
fn ruler(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TimelineStoryLabels::of(ui);
    let demo = shared(ui, "ruler", || RulerDemo {
        // Points per second: about 12 per frame, 40 (FilmCraft's default) and 4 per second.
        views: [TimeView::new(secs(4.0), 360.0), TimeView::new(Tick::ZERO, 40.0), TimeView::new(Tick::ZERO, 4.0)],
        playhead: secs(5.0),
        marks: (Some(secs(3.0)), Some(secs(40.0))),
        work_area: Some(span(2.0, 60.0)),
        ram: vec![span(1.0, 8.0), span(20.0, 31.0)],
        disk: vec![span(8.0, 20.0)],
        markers: vec![
            Marker { id: MarkerId(1), start: secs(6.0), name: labels.marker_check.clone(), color: MarkerColor(0), ..Marker::default() },
            Marker {
                id: MarkerId(2),
                start: secs(30.0),
                duration: secs(8.0),
                name: labels.marker_swell.clone(),
                color: MarkerColor(8),
                ..Marker::default()
            },
        ],
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let captions = [&labels.zoom_frames, &labels.zoom_seconds, &labels.zoom_minutes];
    for (index, caption) in captions.into_iter().enumerate() {
        ui.add(Text::new(caption.as_str()).content(Content::Muted));
        let out = TimelineRuler::new(("ruler", index), &mut demo.views[index], format(), secs(RULER_SECONDS))
            .playhead(demo.playhead)
            .in_out(demo.marks)
            .work_area(demo.work_area)
            .cache_bars(&demo.ram, &demo.disk)
            .markers(&demo.markers)
            .show(ui);
        if let Some(event) = &out.scrub {
            if let Some(&t) = live(event) {
                demo.playhead = t;
            }
            demo.last = Some(format!("{event:?}"));
        }
        if let Some(event) = &out.work_area {
            if let Some(&range) = live(event) {
                demo.work_area = Some(range);
            }
            demo.last = Some(format!("{event:?}"));
        }
        for edit in &out.markers {
            apply_marker(&mut demo.markers, edit);
            demo.last = Some(format!("{edit:?}"));
        }
    }
    last_event(ui, demo.last.as_deref());
}

// ---- Keyframes ----------------------------------------------------------------------------

struct LaneDemo {
    keys: Vec<KeyframeMark>,
    selection: Selection<KeyframeId>,
    view: TimeView,
    playhead: Tick,
    animated: bool,
    last: Option<String>,
}

// new: a lane with Linear, Hold and Bezier keys, two selected, and the navigator beside it.
fn keyframe_lane(ui: &mut Ui, _: &mut NoArgs) {
    let tokens = Tokens::of(ui);
    let style = TimelineStyle::of(&tokens);
    let demo = shared(ui, "lane", || {
        let key = |id: u64, at: f64, interpolation| KeyframeMark { id: KeyframeId(id), time: secs(at), interpolation };
        LaneDemo {
            keys: vec![
                key(1, 1.0, KeyframeInterpolation::Linear),
                key(2, 3.0, KeyframeInterpolation::Hold),
                key(3, 5.0, KeyframeInterpolation::Bezier),
                key(4, 7.5, KeyframeInterpolation::Linear),
                key(5, 10.0, KeyframeInterpolation::Bezier),
            ],
            selection: Selection { items: vec![KeyframeId(2), KeyframeId(3)], lead: Some(KeyframeId(3)), anchor: Some(KeyframeId(2)) },
            // Points per second.
            view: TimeView::new(Tick::ZERO, 50.0),
            playhead: secs(4.0),
            animated: true,
            last: None,
        }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let (before, after, at) = (
        demo.keys.iter().any(|k| k.time < demo.playhead),
        demo.keys.iter().any(|k| k.time > demo.playhead),
        demo.keys.iter().any(|k| k.time == demo.playhead),
    );
    let (toggle, navigator, lane) = ui
        .horizontal(|ui| {
            let toggle = AnimationToggle::new("animate", &demo.animated).show(ui);
            let navigator = KeyframeNavigator::new("navigator", before, after, at).animated(demo.animated).show(ui);
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), style.keyframe_arrow_height), Sense::hover());
            let lane = KeyframeLane::new("lane", &demo.keys, &demo.selection, &demo.view, RATE).playhead(demo.playhead).show(ui, rect);
            (toggle, navigator, lane)
        })
        .inner;
    if let Some(event) = &toggle.event {
        if let Some(&on) = settled(event) {
            demo.animated = on;
        }
        demo.last = Some(format!("{event:?}"));
    }
    if navigator.previous {
        if let Some(key) = demo.keys.iter().rev().find(|k| k.time < demo.playhead) {
            demo.playhead = key.time;
        }
        demo.last = Some("Previous".into());
    }
    if navigator.next {
        if let Some(key) = demo.keys.iter().find(|k| k.time > demo.playhead) {
            demo.playhead = key.time;
        }
        demo.last = Some("Next".into());
    }
    if navigator.toggle {
        let playhead = demo.playhead;
        if at {
            demo.keys.retain(|k| k.time != playhead);
        } else {
            let id = KeyframeId(demo.keys.iter().map(|k| k.id.0).max().unwrap_or(0) + 1);
            demo.keys.push(KeyframeMark { id, time: playhead, interpolation: KeyframeInterpolation::Linear });
            demo.keys.sort_by_key(|k| k.time);
        }
        demo.last = Some("Toggle".into());
    }
    if let Some(change) = lane.selection {
        demo.last = Some(format!("{:?}", change.pick));
        demo.selection = change.selection;
    }
    if let Some(event) = &lane.moved {
        if let Some(&delta) = settled(event) {
            let selection = &demo.selection;
            for key in demo.keys.iter_mut().filter(|k| selection.contains(&k.id)) {
                key.time = (key.time + delta).max(Tick::ZERO);
            }
            demo.keys.sort_by_key(|k| k.time);
        }
        demo.last = Some(format!("{event:?}"));
    }
    if lane.delete {
        let selection = std::mem::take(&mut demo.selection);
        demo.keys.retain(|k| !selection.contains(&k.id));
        demo.last = Some("Delete".into());
    }
    if let Some(id) = lane.double_clicked {
        demo.last = Some(format!("DoubleClicked({id:?})"));
    }
    if let Some(id) = lane.context_menu {
        demo.last = Some(format!("ContextMenu({id:?})"));
    }
    last_event(ui, demo.last.as_deref());
}

// ---- The graph ----------------------------------------------------------------------------

struct GraphDemo {
    curves: Vec<GraphCurve>,
    selection: Selection<KeyframeId>,
    view: TimeView,
    range: GraphRange,
    playhead: Tick,
    last: Option<String>,
}

/// Seconds between a curve's samples.
const SAMPLE_STEP: f64 = 0.05;
/// The seconds a Bezier key's handles reach to each side.
const HANDLE_REACH: f64 = 0.8;
/// The graph's height over its width.
const GRAPH_ASPECT: f32 = 0.45;

fn bezier_like(interpolation: KeyframeInterpolation) -> bool {
    !matches!(interpolation, KeyframeInterpolation::Linear | KeyframeInterpolation::Hold)
}

fn graph_key(id: u64, at: f64, value: f64, interpolation: KeyframeInterpolation) -> GraphKey {
    let mut key = GraphKey { id: KeyframeId(id), time: secs(at), value, interpolation, in_handle: None, out_handle: None };
    flatten_handles(&mut key);
    key
}

/// Horizontal handles for a Bezier-like key, none for the others.
fn flatten_handles(key: &mut GraphKey) {
    if bezier_like(key.interpolation) {
        key.in_handle = Some((key.time - secs(HANDLE_REACH), key.value));
        key.out_handle = Some((key.time + secs(HANDLE_REACH), key.value));
    } else {
        key.in_handle = None;
        key.out_handle = None;
    }
}

/// The curve's samples from its keys: Hold steps, Linear lines, the rest an ease.
fn resample(curve: &mut GraphCurve) {
    curve.keys.sort_by_key(|k| k.time);
    curve.samples.clear();
    let (Some(first), Some(last)) = (curve.keys.first(), curve.keys.last()) else { return };
    let (from, to) = (first.time.seconds(), last.time.seconds());
    let steps = ((to - from) / SAMPLE_STEP).ceil() as usize;
    for step in 0..=steps {
        let t = (from + step as f64 * SAMPLE_STEP).min(to);
        let at = curve.keys.partition_point(|k| k.time.seconds() <= t).saturating_sub(1);
        let (a, b) = (&curve.keys[at], curve.keys.get(at + 1).unwrap_or(&curve.keys[at]));
        let span = (b.time - a.time).seconds();
        let x = if span > 0.0 { ((t - a.time.seconds()) / span).clamp(0.0, 1.0) } else { 0.0 };
        let eased = match a.interpolation {
            KeyframeInterpolation::Hold => 0.0,
            KeyframeInterpolation::Linear => x,
            _ => x * x * (3.0 - 2.0 * x),
        };
        curve.samples.push((secs(t), a.value + (b.value - a.value) * eased));
    }
}

fn sample_curve(id: u64, name: &str, color: Color32, keys: Vec<GraphKey>) -> GraphCurve {
    let mut curve = GraphCurve { id, name: name.into(), color: Some(color), samples: Vec::new(), keys };
    resample(&mut curve);
    curve
}

// new: Position X and Y curves with four keys, one selected with handles, value graph.
fn graph_editor(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TimelineStoryLabels::of(ui);
    let demo = shared(ui, "graph", || {
        use KeyframeInterpolation::{AutoBezier, Bezier, Linear};
        GraphDemo {
            curves: vec![
                sample_curve(
                    1,
                    &labels.curve_x,
                    Color32::from_rgb(0xE5, 0x4B, 0x4B),
                    vec![
                        graph_key(1, 0.0, 120.0, Bezier),
                        graph_key(2, 2.0, 340.0, Bezier),
                        graph_key(3, 4.0, 180.0, AutoBezier),
                        graph_key(4, 6.0, 420.0, Bezier),
                    ],
                ),
                sample_curve(
                    2,
                    &labels.curve_y,
                    Color32::from_rgb(0x4B, 0xB5, 0x6E),
                    vec![
                        graph_key(5, 0.0, 60.0, Linear),
                        graph_key(6, 2.0, 140.0, Bezier),
                        graph_key(7, 4.0, 300.0, Bezier),
                        graph_key(8, 6.0, 220.0, Linear),
                    ],
                ),
            ],
            selection: Selection::only(KeyframeId(2)),
            // Points per second.
            view: TimeView::new(Tick::ZERO, 90.0),
            range: GraphRange::default(),
            playhead: secs(3.0),
            last: None,
        }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, width * GRAPH_ASPECT), Sense::hover());
    let out =
        GraphEditor::new("graph", &demo.curves, &demo.selection, &demo.view, &mut demo.range).playhead(demo.playhead).format(format()).show(ui, rect);
    if let Some(change) = out.selection {
        demo.last = Some(format!("{:?}", change.pick));
        demo.selection = change.selection;
    }
    if let Some(event) = &out.keys_moved {
        if let Some(delta) = settled(event) {
            let selection = &demo.selection;
            for curve in &mut demo.curves {
                for key in curve.keys.iter_mut().filter(|k| selection.contains(&k.id)) {
                    key.time = (key.time + delta.dt).max(Tick::ZERO);
                    key.value += delta.dv;
                    flatten_handles(key);
                }
                resample(curve);
            }
        }
        demo.last = Some(format!("{event:?}"));
    }
    if let Some(event) = &out.handle_moved {
        if let Some(moved) = live(event) {
            for curve in &mut demo.curves {
                if let Some(key) = curve.keys.iter_mut().find(|k| k.id == moved.key) {
                    let (this, other) = match moved.side {
                        HandleSide::In => (&mut key.in_handle, &mut key.out_handle),
                        HandleSide::Out => (&mut key.out_handle, &mut key.in_handle),
                    };
                    *this = Some((moved.time, moved.value));
                    if !moved.broken {
                        *other = Some((key.time + (key.time - moved.time), 2.0 * key.value - moved.value));
                    }
                }
                resample(curve);
            }
        }
        demo.last = Some(format!("{event:?}"));
    }
    if let Some(action) = out.action {
        demo.last = Some(format!("{action:?}"));
        let interpolation = match action {
            GraphAction::Hold => Some(KeyframeInterpolation::Hold),
            GraphAction::Linear => Some(KeyframeInterpolation::Linear),
            GraphAction::AutoBezier => Some(KeyframeInterpolation::AutoBezier),
            GraphAction::EasyEase => Some(KeyframeInterpolation::Bezier),
            GraphAction::EasyEaseIn => Some(KeyframeInterpolation::EaseIn),
            GraphAction::EasyEaseOut => Some(KeyframeInterpolation::EaseOut),
            GraphAction::Delete | GraphAction::FitSelection | GraphAction::FitAll => None,
        };
        let selection = std::mem::take(&mut demo.selection);
        for curve in &mut demo.curves {
            match (action, interpolation) {
                (GraphAction::Delete, _) => curve.keys.retain(|k| !selection.contains(&k.id)),
                (_, Some(interpolation)) => {
                    for key in curve.keys.iter_mut().filter(|k| selection.contains(&k.id)) {
                        key.interpolation = interpolation;
                        flatten_handles(key);
                    }
                }
                _ => {}
            }
            resample(curve);
        }
        demo.selection = if action == GraphAction::Delete { Selection::new() } else { selection };
    }
    last_event(ui, demo.last.as_deref());
}

// ---- Transport ----------------------------------------------------------------------------

struct BarDemo {
    settings: TransportBarSettings,
    playing: bool,
    looping: bool,
    last: Option<String>,
}

// new: the Program row, playing and looping.
fn transport_bar(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "bar", || BarDemo { settings: TransportBarSettings::default(), playing: true, looping: true, last: None });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let out = TransportBar::new("bar", &TransportButton::PROGRAM).playing(demo.playing).looping(demo.looping).settings(&mut demo.settings).show(ui);
    if let Some(button) = out.pressed {
        match button {
            TransportButton::PlayStop => demo.playing = !demo.playing,
            TransportButton::Loop => demo.looping = !demo.looping,
            _ => {}
        }
        demo.last = Some(format!("{button:?}"));
    }
    if out.settings_changed {
        demo.last = Some(format!("{:?}", demo.settings));
    }
    last_event(ui, demo.last.as_deref());
}

struct MediaDemo {
    position: f64,
    playing: bool,
    last: Option<String>,
}

/// The media's length, and its trim marks, in seconds.
const MEDIA_SECONDS: f64 = 65.0;
const TRIM: (f64, f64) = (4.0, 52.0);

// new: the media controls at 0:12.50 of 1:05.00 with trim marks.
fn media_controls(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "media", || MediaDemo { position: 12.5, playing: false, last: None });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let out = MediaControls::new("media", demo.position, MEDIA_SECONDS).playing(demo.playing).trim(TRIM.0, TRIM.1).show(ui);
    if out.toggle {
        demo.playing = !demo.playing;
        demo.last = Some("Toggle".into());
    }
    if let Some(step) = out.nudge {
        demo.position = (demo.position + step).clamp(0.0, MEDIA_SECONDS);
        demo.last = Some(format!("Nudge({step})"));
    }
    if let Some(event) = &out.seek {
        if let Some(&seconds) = live(event) {
            demo.position = seconds.clamp(0.0, MEDIA_SECONDS);
        }
        demo.last = Some(format!("{event:?}"));
    }
    last_event(ui, demo.last.as_deref());
}

// ---- Timecode -----------------------------------------------------------------------------

struct FieldDemo {
    large: Tick,
    small: Tick,
    last: Option<String>,
}

// new: a large and a small field at 29.97 drop-frame; a click starts typing in either.
fn timecode_field(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "field", || FieldDemo { large: secs(5.0), small: secs(64.5), last: None });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let format = format();
    let large = TimecodeField::new("large", demo.large, &format).large(true).show(ui);
    let small = TimecodeField::new("small", demo.small, &format).show(ui);
    for (out, time) in [(large, &mut demo.large), (small, &mut demo.small)] {
        if let Some(event) = &out.event {
            if let Some(&t) = live(event) {
                *time = t;
            }
            demo.last = Some(format!("{event:?}"));
        }
    }
    last_event(ui, demo.last.as_deref());
}

struct TimecodeDemo {
    settings: TimecodePanelSettings,
    last: Option<String>,
}

// new: Current Time, Duration and Remaining rows.
fn timecode_panel(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "timecode", || {
        let row = |kind| TimecodeRow { kind, source: TimecodeSource::Program, display: None };
        TimecodeDemo {
            settings: TimecodePanelSettings {
                rows: vec![row(TimecodeRowKind::CurrentTime), row(TimecodeRowKind::Duration), row(TimecodeRowKind::Remaining)],
            },
            last: None,
        }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let (playhead, duration) = (secs(5.0), secs(SEQUENCE_SECONDS));
    let text = |row: &TimecodeRow| {
        let base = format();
        let format = row.display.map_or(base, |display| base.display(display));
        let time = match row.kind {
            TimecodeRowKind::Duration | TimecodeRowKind::InOutDuration => duration,
            TimecodeRowKind::Remaining => duration - playhead,
            TimecodeRowKind::CurrentTime | TimecodeRowKind::MediaTime => playhead,
        };
        format_time(time, &format)
    };
    let out = TimecodePanel::new("timecode", &mut demo.settings, &text).show(ui);
    if out.settings_changed {
        demo.last = Some(format!("{:?}", demo.settings.rows));
    }
    last_event(ui, demo.last.as_deref());
}

// ---- Preview ------------------------------------------------------------------------------

/// The test card's size in pixels, and its eight bars.
const CARD: [usize; 2] = [160, 120];
const BARS: [Color32; 8] = [
    Color32::from_rgb(0xC0, 0xC0, 0xC0),
    Color32::from_rgb(0xC0, 0xC0, 0x00),
    Color32::from_rgb(0x00, 0xC0, 0xC0),
    Color32::from_rgb(0x00, 0xC0, 0x00),
    Color32::from_rgb(0xC0, 0x00, 0xC0),
    Color32::from_rgb(0xC0, 0x00, 0x00),
    Color32::from_rgb(0x00, 0x00, 0xC0),
    Color32::from_rgb(0x10, 0x10, 0x10),
];

/// Colour bars over a grey ramp: a frame whose fit and crop show.
fn test_card() -> ColorImage {
    let [width, height] = CARD;
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            pixels.push(if y < height * 2 / 3 { BARS[x * BARS.len() / width] } else { Color32::from_gray((x * 255 / (width - 1)) as u8) });
        }
    }
    ColorImage::new(CARD, pixels)
}

/// The preview box's height over its width: 16:9.
const PREVIEW_ASPECT: f32 = 9.0 / 16.0;
/// The crop of the second preview: left, top, right, bottom fractions.
const CROP: [f32; 4] = [0.25, 0.1, 0.25, 0.1];

// new: a 4:3 test card letterboxed in a 16:9 box with a status line, and a cropped one.
fn frame_preview(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TimelineStoryLabels::of(ui);
    // The texture is made outside `shared`: its init runs under the context's data lock.
    let card = shared(ui, "card", || None::<TextureHandle>);
    let card: TextureHandle =
        lock(&card).get_or_insert_with(|| ui.ctx().load_texture("timeline-story-card", test_card(), TextureOptions::LINEAR)).clone();
    let time = format_time(secs(5.0), &format());
    let status = |name: &str| fmt(&labels.preview_status, &[("name", name), ("time", &time)]);
    let letterbox = status(&labels.preview_card);
    let cropped = status(&labels.preview_cropped);
    ui.columns(2, |columns| {
        for (column, (name, status, fit)) in
            columns.iter_mut().zip([("letterbox", &letterbox, FrameFit::Letterbox), ("crop", &cropped, FrameFit::Crop(CROP))])
        {
            let width = column.available_width();
            column.allocate_ui(vec2(width, width * PREVIEW_ASPECT), |ui| {
                FramePreview::new(name, Some((card.id(), card.size_vec2()))).fit(fit).status(status).show(ui);
            });
        }
    });
}

// new: the Marker dialog on a range marker.
fn marker_dialog(ui: &mut Ui, args: &mut DialogArgs) {
    let labels = TimelineStoryLabels::of(ui);
    drive(
        ui,
        "marker-dialog",
        &mut args.open,
        (),
        || {
            let marker = Marker {
                id: MarkerId(1),
                start: secs(20.0),
                duration: secs(4.0),
                name: labels.marker_swell.clone(),
                comment: labels.marker_swell_comment.clone(),
                color: MarkerColor(8),
            };
            MarkerDialog::new(&marker, format())
        },
        |_| {},
        |result| format!("{result:?}"),
    );
}

// ---- Overview -----------------------------------------------------------------------------

struct OverviewDemo {
    tracks: Vec<Vec<(TimeRange, Color32)>>,
    view: TimeView,
    end: Tick,
}

const OVERVIEW_TRACKS: u8 = 8;
const OVERVIEW_CLIPS: u8 = 6;

// new: eight tracks of runs and the visible box.
fn overview(ui: &mut Ui, _: &mut NoArgs) {
    let tokens = Tokens::of(ui);
    let style = TimelineStyle::of(&tokens);
    let demo = shared(ui, "overview", || {
        let palette = LabelPalette::default();
        let mut end = Tick::ZERO;
        let tracks = (0..OVERVIEW_TRACKS)
            .map(|track| {
                let mut at = f64::from(track % 3) * 2.0;
                (0..OVERVIEW_CLIPS)
                    .map(|n| {
                        let length = 6.0 + f64::from((track + n) % 4) * 3.0;
                        let range = TimeRange::new(secs(at), secs(length));
                        at += length + 1.0 + f64::from((track + 2 * n) % 3) * 2.0;
                        end = end.max(range.end());
                        (range, palette.color(MarkerColor((track * 2 + n) % 16)))
                    })
                    .collect()
            })
            .collect();
        // Points per second.
        OverviewDemo { tracks, view: TimeView::new(secs(20.0), 10.0), end }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let width = ui.available_width();
    let height = f32::from(OVERVIEW_TRACKS) * style.track_height_min + 2.0 * style.overview_inset_y;
    ui.allocate_ui(vec2(width, height), |ui| {
        let area = ui.available_rect_before_wrap().shrink2(vec2(style.overview_inset_x, style.overview_inset_y));
        let runs = overview_runs(&demo.tracks, demo.end + secs(TAIL_SECONDS), area);
        TimelineOverview::new("overview", &runs, &mut demo.view, width, demo.end).playhead(Some(secs(30.0))).show(ui);
    });
}

/// The seconds the overview adds after the content's end (`TimelineOverview`'s total).
const TAIL_SECONDS: f64 = 10.0;

// ---- Render queue -------------------------------------------------------------------------

struct QueueDemo {
    items: Vec<RenderQueueItem>,
    selection: Selection<RenderItemId>,
    running: bool,
    last: Option<String>,
}

/// The rows an app takes out and puts back where a drop says.
fn reorder(items: &mut Vec<RenderQueueItem>, change: &Move<RenderItemId>) {
    let Some(target) = items.iter().position(|i| i.id == change.target) else { return };
    let kept_before = |upto: usize| items[..upto].iter().filter(|i| !change.items.contains(&i.id)).count();
    let at = match change.position {
        DropPosition::After => kept_before(target + 1),
        _ => kept_before(target),
    };
    let (picked, mut rest): (Vec<_>, Vec<_>) = std::mem::take(items).into_iter().partition(|i| change.items.contains(&i.id));
    rest.splice(at..at, picked);
    *items = rest;
}

fn set_status(items: &mut [RenderQueueItem], id: RenderItemId, status: RenderStatus) {
    if let Some(item) = items.iter_mut().find(|i| i.id == id) {
        item.status = status;
    }
}

// new: Ready, Encoding 42 %, Done, Failed and Cancelled items.
fn render_queue(ui: &mut Ui, _: &mut NoArgs) {
    let labels = TimelineStoryLabels::of(ui);
    let demo = shared(ui, "queue", || {
        let item = |id: u64, source: &str, file: &str, preset: &str, status| RenderQueueItem {
            id: RenderItemId(id),
            source: source.into(),
            file: file.into(),
            preset: preset.into(),
            status,
        };
        QueueDemo {
            items: vec![
                item(1, &labels.render_interview, "interview.mp4", "H.264 1080p", RenderStatus::Ready),
                item(
                    2,
                    &labels.render_trailer,
                    "trailer.mov",
                    "ProRes 422",
                    RenderStatus::Encoding { progress: 0.42, remaining: Some(Duration::from_secs(95)) },
                ),
                item(3, &labels.render_highlights, "highlights.mp4", "H.264 1080p", RenderStatus::Done),
                item(4, &labels.render_behind, "behind.mp4", "H.265 4K", RenderStatus::Failed(labels.render_error.clone())),
                item(5, &labels.render_teaser, "teaser.mp4", "H.264 720p", RenderStatus::Cancelled),
            ],
            selection: Selection::only(RenderItemId(1)),
            running: true,
            last: None,
        }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let out = RenderQueueList::new("queue", &demo.items, &demo.selection, demo.running).show(ui);
    for event in out.events {
        demo.last = Some(format!("{event:?}"));
        match event {
            RenderQueueEvent::Reorder(gesture) => {
                if let Some(change) = settled(&gesture) {
                    reorder(&mut demo.items, change);
                }
            }
            RenderQueueEvent::Selection(change) => demo.selection = change.selection,
            RenderQueueEvent::Cancel(id) => set_status(&mut demo.items, id, RenderStatus::Cancelled),
            RenderQueueEvent::Retry(id) => set_status(&mut demo.items, id, RenderStatus::Ready),
            RenderQueueEvent::Remove(id) => {
                demo.items.retain(|i| i.id != id);
                demo.selection.items.retain(|i| *i != id);
            }
            RenderQueueEvent::Start => {
                demo.running = true;
                if let Some(first) = demo.items.iter_mut().find(|i| i.status == RenderStatus::Ready) {
                    first.status = RenderStatus::Encoding { progress: 0.0, remaining: None };
                }
            }
            RenderQueueEvent::Stop => {
                demo.running = false;
                for item in &mut demo.items {
                    if matches!(item.status, RenderStatus::Encoding { .. }) {
                        item.status = RenderStatus::Ready;
                    }
                }
            }
            RenderQueueEvent::ClearFinished => {
                demo.items.retain(|i| !matches!(i.status, RenderStatus::Done | RenderStatus::Failed(_) | RenderStatus::Cancelled));
            }
        }
    }
    last_event(ui, demo.last.as_deref());
}

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests {
    use craft_ui::story::Args;
    use craft_ui::testing::symbols;
    use craft_ui::theme::{Base, install};
    use egui_kittest::Harness;

    // every story draws its widget over its sample data without panicking.
    #[test]
    fn every_story_draws() {
        for story in super::stories() {
            let mut harness = Harness::<Args>::builder()
                .with_size([720.0, 520.0])
                .with_context_setup(|ctx| {
                    install(ctx, &Base::suite_default());
                    symbols::install(ctx);
                })
                .build_ui_state(move |ui, args| (story.show)(ui, args), story.defaults());
            harness.run_steps(4);
        }
    }
}
