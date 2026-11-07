//! `TrackStack` and `TrackRow`: the timeline's track list on the collection contract (20.6).
//!
//! One collection with four columns: the track name (`Fixed`), the mute and lock toggles (sweepable toggle
//! columns the collection flips) and the lane (`Fill`). Each row paints its name and toggles in their cells and
//! registers its clips as parts in the lane cell, so a track of any number of clips costs one pipeline run. A clip is
//! a focusable part: a click or Enter on it reports [`TrackEvent::Selected`]. The time axis here is the lane's
//! width over `visible` seconds; the timeline track binds one `TimeView` for every stack.
//!
//! The example has no clip drag, trim or playhead: arrows on a focused part are row-child moves in a collection, so
//! the clip nudge of 20.6 has no route to a part yet.

use crate::contract::{
    Access, Collection, CollectionProps, CollectionResponse, ColumnKind, ColumnSpec, Common, Component, ContainerProps, Cx, Focus as _, ItemOutput,
    Name, NoValue, Outcome, PaintUi, PartsCx, Pointer as _, Props as _, Role, SelectMode, Styled as _, ToggleColumn,
};
use crate::controls::select::row_look;
use crate::primitives::{PaintShape, TextRun};
use crate::style::{ComponentStyle, Flags, Ink, Stateful, StyleColor, StyleCx, SurfaceLook};
use craft_behavior::{
    ColumnKey, ColumnWidth, DisplayOrder, Flow, Insets, Intent, ItemSource, Metrics, PartKey, PressPhase, Selection, Sense, SpanLayout,
};
use egui::{CornerRadius, Pos2, Rect, Response};

const NAME: ColumnKey = ColumnKey::new("name");
const MUTE: ColumnKey = ColumnKey::new("mute");
const LOCK: ColumnKey = ColumnKey::new("lock");
const LANE: ColumnKey = ColumnKey::new("lane");
const CLIP: PartKey = PartKey::new("clip");
/// A track row is this many control heights tall.
const TRACK_IN_HEIGHTS: f32 = 2.0;
/// The name column is this many control heights wide.
const NAME_IN_HEIGHTS: f32 = 6.0;

/// A clip: a stretch of a track's lane.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Clip {
    /// The clip's identity.
    pub id: u64,
    /// Where it starts, in seconds.
    pub start: f32,
    /// How long it runs, in seconds.
    pub length: f32,
}

impl Clip {
    /// A clip `id` from `start` for `length` seconds.
    pub const fn new(id: u64, start: f32, length: f32) -> Self {
        Clip { id, start, length }
    }
}

/// A track of the timeline.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Track {
    /// The track's identity; rows take their ids from it.
    pub id: usize,
    /// The track's name.
    pub name: String,
    /// The track is muted.
    pub muted: bool,
    /// The track is locked.
    pub locked: bool,
    /// The clips on the track.
    pub clips: Vec<Clip>,
}

impl Track {
    /// A track `id` named `name`, with no clips, on and unlocked.
    pub fn new(id: usize, name: impl Into<String>) -> Self {
        Track { id, name: name.into(), muted: false, locked: false, clips: Vec::new() }
    }

    /// The clips on the track.
    #[must_use]
    pub fn clips(mut self, clips: Vec<Clip>) -> Self {
        self.clips = clips;
        self
    }
}

/// The tracks as a collection source, keyed by track id.
#[non_exhaustive]
pub struct Tracks<'a>(pub &'a [Track]);

impl<'a> Tracks<'a> {
    /// The source over `tracks`.
    pub const fn new(tracks: &'a [Track]) -> Self {
        Tracks(tracks)
    }
}

impl DisplayOrder<usize> for Tracks<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> usize {
        self.0[at].id
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        Some(std::borrow::Cow::Borrowed(self.0[at].name.as_str()))
    }
}

impl ItemSource<usize> for Tracks<'_> {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        let track = &self.0[at];
        (*column == MUTE && track.muted) || (*column == LOCK && track.locked)
    }
}

/// What a track row reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrackEvent {
    /// A clip was clicked or activated from the keyboard.
    Selected(u64),
}

/// The style of the stack: no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct TrackStackStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A list of tracks over four columns: name, mute, lock and lane.
#[derive(crate::contract::Component)]
#[component(collection)]
pub struct TrackStack {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
}

impl TrackStack {
    /// A track stack named `label`.
    pub fn new(label: impl Into<crate::runtime::env::Label>) -> Self {
        let columns = vec![
            ColumnSpec::new(NAME, "Track", ColumnWidth::Controls(NAME_IN_HEIGHTS)),
            ColumnSpec::new(MUTE, "Mute", ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())),
            ColumnSpec::new(LOCK, "Lock", ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())),
            ColumnSpec::new(LANE, "Clips", ColumnWidth::fill(1.0)),
        ];
        TrackStack { common: Common::new(), props: ContainerProps::new(), items: CollectionProps::new().select(SelectMode::Single).columns(columns) }
            .label(label)
    }
}

impl Component for TrackStack {
    const ROLE: Role = Role::LIST;
    type Style = TrackStackStyle;
    type Output = CollectionResponse<usize, TrackEvent>;

    fn style(_: &StyleCx<'_>, _: TrackStackStyleKey) -> TrackStackStyle {
        TrackStackStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for TrackStack {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = TrackRow<'r>;
    type ItemEvent = TrackEvent;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    /// The column widths come from the specs; a row is `TRACK_IN_HEIGHTS` control heights tall.
    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, cx: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::new(source, width, cx.control().height * TRACK_IN_HEIGHTS)
    }
}

/// The style of a track row: the colours of name, toggles and clips.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct TrackRowStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    ink: StyleColor,
    dim: StyleColor,
    clip: Stateful<StyleColor>,
    pad: f32,
}

/// What a track row returns.
#[non_exhaustive]
pub struct TrackOut {
    /// The row's response.
    pub response: Response,
    /// What the row reported this pass.
    pub event: Option<TrackEvent>,
}

impl Outcome for TrackOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl ItemOutput<TrackEvent> for TrackOut {
    fn into_event(self) -> Option<TrackEvent> {
        self.event
    }
}

/// One track: its name and toggles in their cells, its clips as parts of the lane cell.
#[derive(crate::contract::Component)]
pub struct TrackRow<'r> {
    #[common]
    common: Common,
    track: &'r Track,
    cells: Vec<(ColumnKey, Rect)>,
    visible: f32,
    event: Option<TrackEvent>,
}

impl<'r> TrackRow<'r> {
    /// A row of `track` in `cells`, the rects the item context lists, with a lane showing `visible` seconds.
    pub fn new(track: &'r Track, cells: Vec<(ColumnKey, Rect)>, visible: f32) -> Self {
        TrackRow { common: Common::new(), track, cells, visible, event: None }
    }

    fn cell(&self, column: &ColumnKey) -> Option<Rect> {
        self.cells.iter().find(|(key, _)| key == column).map(|(_, rect)| *rect)
    }

    /// The rect of `clip` in the lane cell `lane`.
    fn clip_rect(&self, lane: Rect, clip: &Clip, rtl: bool) -> Rect {
        let per_second = lane.width() / self.visible.max(f32::EPSILON);
        let flow = Flow::new(lane, rtl);
        let span = flow.span(clip.start * per_second, clip.length * per_second);
        Rect::from_min_max(Pos2::new(span.left(), lane.top()), Pos2::new(span.right(), lane.bottom()))
    }
}

impl Component for TrackRow<'_> {
    const ROLE: Role = Role::LIST_ITEM;
    type Style = TrackRowStyle;
    type Output = TrackOut;

    fn style(cx: &StyleCx<'_>, _: TrackRowStyleKey) -> TrackRowStyle {
        let clip = Stateful::new(cx.color(cx.tokens.color.surface.raised)).when(Flags::SELECTED, cx.color(cx.tokens.color.state.selected_hover));
        TrackRowStyle {
            surface: row_look(cx),
            metrics: Metrics::new(),
            ink: cx.ink(Ink::OnPanel),
            dim: cx.color(cx.tokens.color.text.faint),
            clip,
            pad: cx.control().pad_x,
        }
    }

    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        let Some(lane) = self.cell(&LANE) else { return };
        for clip in &self.track.clips {
            cx.part(CLIP.at(clip.id), self.clip_rect(lane, clip, cx.flow.is_rtl())).sense(Sense::CLICK).focusable();
        }
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        for clip in &self.track.clips {
            access.node(CLIP.at(clip.id), Role::BUTTON).name = Name::Text(format!("{}, clip {}", self.track.name, clip.id));
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let keyed = cx.intents().take(Intent::Activate);
        for clip in &self.track.clips {
            let part = CLIP.at(clip.id);
            let clicked = matches!(cx.part(part).press.phase, PressPhase::Release { click: true, .. });
            if clicked || (keyed && cx.active_part() == Some(part)) {
                self.event = Some(TrackEvent::Selected(clip.id));
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rtl = cx.env.direction().is_rtl();
        let draw = ui.draw();
        if let Some(cell) = self.cell(&NAME) {
            draw.text(Flow::new(cell, rtl).inset(Insets::horizontal(cx.style.pad)), &TextRun::new(self.track.name.as_str()).elide());
        }
        for (column, label, on) in [(&MUTE, "M", self.track.muted), (&LOCK, "L", self.track.locked)] {
            if let Some(cell) = self.cell(column) {
                let color = if on { cx.style.ink } else { cx.style.dim };
                draw.text(Flow::new(cell, rtl).inset(Insets::horizontal(cx.style.pad)), &TextRun::new(label).color(color));
            }
        }
        if let Some(lane) = self.cell(&LANE) {
            for clip in &self.track.clips {
                let rect = self.clip_rect(lane, clip, rtl);
                draw.fill(&PaintShape::Rect(rect, CornerRadius::ZERO), cx.resolve_part(CLIP.at(clip.id), &cx.style.clip));
            }
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> TrackOut {
        TrackOut { response: cx.response.clone(), event: self.event }
    }
}
