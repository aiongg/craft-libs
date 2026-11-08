//! The timeline panel: the large timecode and the toolbar, the marker strip and the ruler over the time area, and the
//! caption, video and audio panes as three [`TrackStack`]s in one [`TimelineView`], over the app's [`TimelineModel`].
//! Every change the user makes comes out as a [`TimelineEdit`].

use super::clips::{ClipContent, ClipLane, TimelineTool};
use super::markers::{MarkerEdit, MarkerSpan, MarkerStrip};
use super::ruler::TimelineRuler;
use super::stack::{LOCK, MUTE, NAME, NAME_DEFAULT, OUTPUT, SOLO, SYNC_LOCK, Side, TARGET, TrackEvent, TrackRow, TrackStack, Tracks, set_event};
use super::view::{TimelineView, TimelineWords};
use super::{DragEvent, TimecodeField};
use crate::clips::{ClipEdit, SnapTargets};
use crate::clock::FollowMode;
use crate::markers::{Marker, MarkerColor};
use crate::model::{ClipId, ClipInfo, TimelineModel, TrackId, TrackInfo, TrackKind};
use crate::time::{Tick, TimeRange};
use craft_behavior::{ColumnWidth, EditEvent, IconId, Metrics, PressPhase, SelectionChange, Sense};
use craft_ui::buttons::{IconButton, ToggleButton};
use craft_ui::contract::{
    Bind, BindMut, BodyLayout, CollectionEvent, CollectionResponse, ColumnEvent, Common, Component, Container, ContainerProps, Cx, Edit as _,
    EditResponse, NoValue, Outcome, Pattern, Pointer as _, Props, Remember as _, Role, Show as _, ShowCollection as _, ShowContainer as _, Sizing,
};
use craft_ui::display::Text;
use craft_ui::layout_components::{Row, Stack};
use craft_ui::menu::{Menu, MenuItem};
use craft_ui::primitives::IconSpec;
use craft_ui::runtime::env::{Env, Label};
use craft_ui::style::{ComponentStyle, StyleCx};
use egui::{Pos2, Rect, Response, Ui, Vec2};

/// A track header switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TrackSwitch {
    /// The track is locked against edits.
    Locked,
    /// The track is targeted by paste and insert.
    Targeted,
    /// The track moves with ripple edits on other tracks.
    SyncLock,
    /// The track's output is on (the eye or the speaker).
    Enabled,
    /// The track is muted.
    Muted,
    /// The track is soloed.
    Solo,
}

/// What the panel asks of the app. Clip and playhead edits come as `EditEvent` gestures: the app previews updates and
/// applies the commit as one undo step.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum TimelineEdit {
    /// A clip drag; see [`ClipEdit`].
    Clip(EditEvent<ClipEdit>),
    /// A playhead drag or click, as a new time.
    Playhead(EditEvent<Tick>),
    /// A drag of the work area's range, or the span a double click sets.
    WorkArea(DragEvent<TimeRange>),
    /// A marker was dragged: where it is, as the strip's gesture.
    MarkerSpan(EditEvent<MarkerSpan>),
    /// A marker gesture.
    Marker(MarkerEdit),
    /// A header switch was clicked; `value` is the new state. Alt-click on Solo or Mute (exclusive) arrives with
    /// `exclusive`.
    #[non_exhaustive]
    Switch {
        /// The track whose switch was clicked.
        track: TrackId,
        /// Which switch was clicked.
        switch: TrackSwitch,
        /// The switch's new state.
        value: bool,
        /// Alt was held: apply to every other track too.
        exclusive: bool,
    },
    /// The colour strip's menu.
    #[non_exhaustive]
    Label {
        /// The track whose label changed.
        track: TrackId,
        /// The new label, or `None` to clear it.
        label: Option<MarkerColor>,
    },
    /// A row's edge dragged, or a double click on its header: the new height in points, within the least and most
    /// of the track sizes, as one gesture (Esc cancels).
    #[non_exhaustive]
    TrackHeight {
        /// The track that is resized.
        track: TrackId,
        /// The new height, in points.
        height: DragEvent<f32>,
    },
    /// The name column dragged: its new width. A view setting the app keeps beside the `TimeView`, never in
    /// undo.
    HeaderWidth(EditEvent<ColumnWidth>),
    /// A click on a clip or empty track space, or a marquee: Premiere's picks (a click replaces, Shift+click toggles
    /// the clip, a Shift marquee adds; a click on empty space clears). `unlinked` with Cmd or Ctrl: the app leaves the
    /// clip's linked partners out. The app expands linked partners when Linked Selection is on, and keeps the
    /// selection ([`TimelineModel::selection`]).
    #[non_exhaustive]
    Select {
        /// The change of the clip selection.
        change: SelectionChange<ClipId>,
        /// Cmd or Ctrl was held: leave linked partners out.
        unlinked: bool,
    },
    /// The divider between the video and audio tracks dragged: the video side's new share of the room below the
    /// captions, from 0.1 to 0.9, as one gesture. A view setting the app keeps and hands back through
    /// [`Timeline::divider`], never in undo.
    Divider(EditEvent<f32>),
    /// A secondary click, or the Menu key or Shift+F10 with the time area focused: the app opens its menu. `anchor` is
    /// the clicked clip's or track's rect, `pointer` the click position, `None` for a key.
    #[non_exhaustive]
    ContextMenu {
        /// The track under the pointer, if any.
        track: Option<TrackId>,
        /// The clip under the pointer or focused, if any.
        clip: Option<ClipId>,
        /// The clicked clip's or track's rect, to anchor the menu.
        anchor: Rect,
        /// The click position; `None` for a key.
        pointer: Option<Pos2>,
    },
    /// A double click on a clip (Premiere opens it in the Source monitor).
    OpenClip(ClipId),
}

/// The least and most of the video side's share of the room below the captions (FilmCraft `timeline.rs:385`).
const DIVIDER_SHARE_MIN: f32 = 0.1;
const DIVIDER_SHARE_MAX: f32 = 0.9;
/// The share the video side gets when the app keeps none.
const DIVIDER_SHARE_DEFAULT: f32 = 0.5;

/// What the panel did this pass.
#[must_use]
#[non_exhaustive]
pub struct TimelineOutput {
    /// The panel's response, for hover and focus.
    pub response: Response,
    /// This pass's edits, in the order the user made them.
    pub edits: Vec<TimelineEdit>,
    /// The toolbar buttons' outputs; the app reacts to their clicks. `None` in a pass that did not run the body.
    pub toolbar: Option<TimelineToolbar>,
}

impl Outcome for TimelineOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The toolbar buttons' outputs: the Snap and Linked Selection toggles (the app flips its settings on a change), Add
/// Marker (the app adds the marker at the playhead; the panel reports no edit for it) and the display settings.
#[non_exhaustive]
pub struct TimelineToolbar {
    /// The Snap toggle button.
    pub snap: EditResponse<bool>,
    /// The Linked Selection toggle button.
    pub linked_selection: EditResponse<bool>,
    /// The Add Marker button; it is disabled while a marker sits at the playhead.
    pub add_marker: Response,
    /// The display settings button.
    pub settings: Response,
}

/// The Timeline panel (Premiere's Timeline; EffectCraft's layer timeline and SoundCraft's Edit window as options).
///
/// Layout, top to bottom: the large [`TimecodeField`] and the toolbar over the headers beside the [`MarkerStrip`] and the
/// [`TimelineRuler`]; then the caption tracks, the video tracks (V1 nearest the divider), the divider, the audio tracks
/// and the Mix row when the model has a master volume; the bar of the [`TimelineView`] at the bottom. The headers are the
/// header columns of each [`TrackStack`], `header_width` wide at the name column and resizable
/// ([`TimelineEdit::HeaderWidth`]); the lanes paint their clips' content through [`content`](Self::content).
///
/// Input: the wheel, the pinch and the keys of [`TimelineView`]; the colour strip of a header opens the label menu; the
/// Menu key and Shift+F10 report [`TimelineEdit::ContextMenu`] for the selection's last clip, else the first targeted
/// track. AccessKit: the time area is a `Group` named by [`TimelineWords::panel`], each track a `Row`, each height edge a
/// `Splitter`.
#[must_use = "show the panel with `show`"]
#[derive(Pattern, Props)]
#[pattern(probe(Timeline<'static, EmptySequence>))]
pub struct Timeline<'a, M: TimelineModel> {
    label: Label,
    model: &'a mut M,
    /// The tool that decides what a press on a clip does.
    #[prop]
    tool: TimelineTool,
    /// Snapping (Premiere's S); targets are clip edges, the playhead, markers, In and Out.
    #[prop]
    snap: bool,
    /// Whether Linked Selection is on, for its toggle's look; the app owns the setting.
    #[prop]
    linked_selection: bool,
    /// Allow the Selection tool to choose Roll and Ripple trims without a modifier key.
    #[prop]
    no_modifier_trims: bool,
    follow: FollowMode,
    playing: bool,
    /// The name column's width the user dragged to ([`TimelineEdit::HeaderWidth`]), kept by the app.
    #[prop(into)]
    header_width: Option<ColumnWidth>,
    /// The video side's share of the room below the captions that the user dragged the divider to
    /// ([`TimelineEdit::Divider`]), kept by the app.
    #[prop(into, quantity = "fraction")]
    divider: Option<f32>,
    /// Paints each clip's content into its body rect (waveforms, thumbnails).
    #[prop(func = "Fn(&craft_ui::primitives::ContentPainterCx<'_>, egui::Rect, egui::Rect, &ClipInfo<'_>)")]
    content: Option<ClipContent<'a>>,
}

impl<'a, M: TimelineModel> Timeline<'a, M> {
    /// The panel named `label` over `model`.
    pub fn new(label: impl Into<Label>, model: &'a mut M) -> Self {
        Timeline {
            label: label.into(),
            model,
            tool: TimelineTool::Selection,
            snap: true,
            linked_selection: true,
            no_modifier_trims: false,
            follow: FollowMode::Page,
            playing: false,
            header_width: None,
            divider: None,
            content: None,
        }
    }

    /// How the view follows the playhead while `playing`.
    pub fn follow(mut self, mode: FollowMode, playing: bool) -> Self {
        self.follow = mode;
        self.playing = playing;
        self
    }
}

/// What the panel keeps between passes, in its root container's slot.
#[derive(Clone, Debug, Default)]
struct Memory {
    /// The track whose label menu is open.
    label_menu: Option<TrackId>,
}

/// What the panes share in a pass.
struct Panes<'p, 'a> {
    words: &'p TimelineWords,
    name: Option<ColumnWidth>,
    playhead: Tick,
    selection: &'p [ClipId],
    targets: Option<&'p SnapTargets>,
    tool: TimelineTool,
    no_modifier_trims: bool,
    content: Option<&'p ClipContent<'a>>,
    clips: &'p [Vec<ClipInfo<'p>>],
}

impl Panes<'_, '_> {
    /// Shows one pane: the tracks `indices` of the model, as `pane`.
    fn show(&self, ui: &mut Ui, side: Side, pane: &[TrackInfo<'_>], indices: &[usize]) -> CollectionResponse<TrackId, TrackEvent> {
        let label = match side {
            Side::Caption => "Captions",
            Side::Upper => "Video",
            Side::Lower => "Audio",
        };
        let mut stack = TrackStack::new(label, self.name, self.words).tracks(pane).playhead(self.playhead);
        if side == Side::Upper {
            stack = stack.anchored_at_end();
        }
        let source = Tracks::new(pane);
        stack.show_items(ui, &source, &mut |key, icx| {
            let at = pane.iter().position(|track| track.id == *key).unwrap_or(0);
            let track = &pane[at];
            let lane = ClipLane::new(&self.clips[indices[at]])
                .tool(self.tool)
                .no_modifier_trims(self.no_modifier_trims)
                .snap(self.targets)
                .locked(track.locked.unwrap_or(false))
                .selection(self.selection)
                .track_label(track.label);
            let lane = match self.content {
                Some(content) => lane.content(content.clone()),
                None => lane,
            };
            TrackRow::new(track, icx.cells().to_vec(), lane, side == Side::Upper, side == Side::Upper && at == 0)
        })
    }
}

/// What turns a pane's events into edits.
struct Mapping<'p> {
    pane: &'p [TrackInfo<'p>],
    words: &'p TimelineWords,
    name: Option<ColumnWidth>,
}

impl Mapping<'_> {
    /// Maps what a pane reported to edits; a track whose colour strip was clicked opens the label menu.
    fn collect(&self, out: &CollectionResponse<TrackId, TrackEvent>, edits: &mut Vec<TimelineEdit>, memory: &mut Memory) {
        let mut with_clip: Vec<TrackId> = Vec::new();
        for event in &out.events {
            match event {
                CollectionEvent::Toggle(edit) => {
                    let Some(toggled) = edit.value() else { continue };
                    let switch = match &toggled.column {
                        column if *column == LOCK => TrackSwitch::Locked,
                        column if *column == TARGET => TrackSwitch::Targeted,
                        column if *column == SYNC_LOCK => TrackSwitch::SyncLock,
                        column if *column == OUTPUT => TrackSwitch::Enabled,
                        column if *column == MUTE => TrackSwitch::Muted,
                        column if *column == SOLO => TrackSwitch::Solo,
                        _ => continue,
                    };
                    let exclusive = toggled.mods.alt && matches!(switch, TrackSwitch::Muted | TrackSwitch::Solo);
                    edits.push(TimelineEdit::Switch { track: toggled.key, switch, value: toggled.on, exclusive });
                }
                CollectionEvent::Column(ColumnEvent::Resized { key, width, .. }) if *key == NAME => {
                    edits.push(TimelineEdit::HeaderWidth(set_event(
                        self.name.unwrap_or(NAME_DEFAULT),
                        ColumnWidth::Fixed(*width),
                        self.words.header_width.as_str(),
                    )));
                }
                CollectionEvent::Item(track, row) => {
                    edits.extend(row.clip.clone().map(TimelineEdit::Clip));
                    if let Some(change) = row.select.clone() {
                        edits.push(TimelineEdit::Select { change, unlinked: row.unlinked });
                    }
                    edits.extend(row.open.map(TimelineEdit::OpenClip));
                    if let Some((clip, anchor)) = row.context {
                        with_clip.push(*track);
                        edits.push(TimelineEdit::ContextMenu { track: Some(*track), clip: Some(clip), anchor, pointer: None });
                    }
                    edits.extend(row.height.clone().map(|height| TimelineEdit::TrackHeight { track: *track, height }));
                    if row.label_menu {
                        memory.label_menu = Some(*track);
                    }
                }
                _ => {}
            }
        }
        for event in &out.events {
            let CollectionEvent::ContextMenu(request) = event else { continue };
            if !with_clip.contains(&request.key) && self.pane.iter().any(|track| track.id == request.key) {
                let anchor = Rect::from_min_size(request.at, Vec2::ZERO);
                edits.push(TimelineEdit::ContextMenu { track: Some(request.key), clip: None, anchor, pointer: Some(request.at) });
            }
        }
    }

    /// The label menu of the track in this pane whose strip was clicked: None, then the sixteen names.
    fn label_menu(&self, ui: &mut Ui, anchor: &Response, memory: &mut Memory, edits: &mut Vec<TimelineEdit>) {
        let Some(track) = memory.label_menu.and_then(|id| self.pane.iter().find(|track| track.id == id)) else { return };
        let current = track.label;
        let words = self.words;
        let menu = Menu::context(words.track_color.clone(), anchor).show(ui, |ui, _| {
            let mut chosen = None;
            if MenuItem::new(words.no_label.clone()).check(current.is_none()).show(ui).ran {
                chosen = Some(None);
            }
            for n in 0..=u8::MAX.min(15) {
                let label = MarkerColor(n);
                if MenuItem::new(words.label_name(label).to_owned()).check(current == Some(label)).show(ui).ran {
                    chosen = Some(Some(label));
                }
            }
            chosen
        });
        if let Some(Some(label)) = menu.inner {
            edits.push(TimelineEdit::Label { track: track.id, label });
        }
        if menu.output.closed.is_some() {
            memory.label_menu = None;
        }
    }
}

impl<M: TimelineModel> Timeline<'_, M> {
    /// Shows the panel and reports what the user did.
    pub fn show(self, ui: &mut Ui) -> TimelineOutput {
        let Timeline { label, model, tool, snap, linked_selection, no_modifier_trims, follow, playing, header_width, divider, content } = self;
        let env = Env::of(ui);
        let words = env.words::<TimelineWords>();
        let mut edits: Vec<TimelineEdit> = Vec::new();
        let mut toolbar: Option<TimelineToolbar> = None;
        let view = *model.view();

        let output = {
            let model: &M = &*model;
            let tracks = model.tracks();
            let format = model.format();
            let (duration, playhead) = (model.duration(), model.playhead());
            let selection = model.selection().to_vec();
            let points = model.edit_points();
            let markers = model.markers();
            let (mark_in, mark_out) = model.in_out();
            let work_area = model.work_area();
            let visible = TimeRange::from_bounds(view.start, view.time_at(ui.available_width(), 0.0));
            let clips: Vec<Vec<ClipInfo<'_>>> = tracks.iter().map(|track| model.clips(track.id, visible)).collect();
            let targets = snap.then(|| {
                let mut targets = SnapTargets::new();
                targets.ranges(clips.iter().flatten().map(|clip| clip.range));
                targets.push(playhead);
                targets.extend(markers.iter().map(|marker| marker.start));
                targets.extend(mark_in.into_iter().chain(mark_out));
                targets.ranges(work_area);
                targets
            });
            let of_kind = |keep: fn(TrackKind) -> bool| -> Vec<usize> { (0..tracks.len()).filter(|&i| keep(tracks[i].kind)).collect() };
            let panes = [
                (Side::Caption, of_kind(|kind| kind == TrackKind::Caption)),
                (Side::Upper, of_kind(|kind| kind == TrackKind::Video)),
                (Side::Lower, of_kind(|kind| !matches!(kind, TrackKind::Caption | TrackKind::Video))),
            ];
            let shared = Panes {
                words: &words,
                name: header_width,
                playhead,
                selection: &selection,
                targets: targets.as_ref(),
                tool,
                no_modifier_trims,
                content: content.as_ref(),
                clips: &clips,
            };
            let mut timecode_edit: Option<EditEvent<Tick>> = None;
            let mut ruler_edits: Vec<TimelineEdit> = Vec::new();

            let area =
                TimelineView::new(label, Bind::given(view), format, duration).headers(true).follow(follow, playing, playhead).edit_points(&points);
            let area = match header_width {
                Some(width) => area.header_width(width),
                None => area,
            };
            let shown = area.show(ui, |ui, cx| {
                TopStrip::new().show(ui, |ui, _| {
                    Stack::new().show(ui, |ui, _| {
                        let field = TimecodeField::new(words.panel.clone(), Bind::given(playhead), format).large().show(ui);
                        timecode_edit = field.edit.event;
                        toolbar = toolbar_row(ui, &words, snap, linked_selection, markers, playhead);
                    });
                    Stack::new().show(ui, |ui, _| {
                        let strip = MarkerStrip::new(words.overview.clone(), markers);
                        let strip = match shared.targets {
                            Some(targets) => strip.snap(targets),
                            None => strip,
                        };
                        let strip = strip.show(ui);
                        ruler_edits.extend(strip.span.event.clone().map(TimelineEdit::MarkerSpan));
                        ruler_edits.extend(strip.edits.into_iter().map(TimelineEdit::Marker));
                        let ruler = TimelineRuler::new(words.overview.clone(), Bind::given(playhead)).in_out((mark_in, mark_out));
                        let ruler = match work_area {
                            Some(range) => ruler.work_area(range),
                            None => ruler,
                        };
                        let ruler = ruler.show(ui);
                        ruler_edits.extend(ruler.playhead.event.map(TimelineEdit::Playhead));
                        ruler_edits.extend(ruler.work_area.map(TimelineEdit::WorkArea));
                    });
                });
                let mut memory = std::mem::take(cx.keep::<Memory>());
                for (side, indices) in &panes {
                    if indices.is_empty() {
                        continue;
                    }
                    let pane: Vec<TrackInfo<'_>> = indices.iter().map(|&i| tracks[i].clone()).collect();
                    let out = shared.show(ui, *side, &pane, indices);
                    let mapping = Mapping { pane: &pane, words: &words, name: header_width };
                    mapping.collect(&out, &mut edits, &mut memory);
                    mapping.label_menu(ui, &out.response, &mut memory, &mut edits);
                    if *side == Side::Upper {
                        let share = divider.unwrap_or(DIVIDER_SHARE_DEFAULT);
                        let divided = Divider::new(words.divider.clone(), Bind::given(share)).show(ui);
                        edits.extend(divided.event.map(TimelineEdit::Divider));
                    }
                }
                if let Some(db) = model.master_volume_db() {
                    mix_row(ui, &words, &env.view().format().number(f64::from(db), 1));
                }
                if tracks.is_empty() {
                    Text::new(words.empty.clone()).show(ui);
                }
                *cx.keep::<Memory>() = memory;
            });
            edits.extend(timecode_edit.map(TimelineEdit::Playhead));
            edits.extend(ruler_edits);
            shown.output
        };
        edits.extend(output.playhead.clone().map(TimelineEdit::Playhead));
        if let Some(next) = output.view.event.as_ref().and_then(EditEvent::value) {
            *model.view() = *next;
        }
        if output.context_menu {
            let tracks = model.tracks();
            let track = tracks.iter().find(|track| track.targeted == Some(true)).or_else(|| tracks.first()).map(|track| track.id);
            let clip = model.selection().last().copied();
            edits.push(TimelineEdit::ContextMenu { track, clip, anchor: output.view.response.rect, pointer: None });
        }
        TimelineOutput { response: output.view.response, edits, toolbar }
    }
}

/// The toolbar under the timecode: Snap, Linked Selection, Add Marker and the display settings.
fn toolbar_row(
    ui: &mut Ui,
    words: &TimelineWords,
    snap: bool,
    linked_selection: bool,
    markers: &[Marker],
    playhead: Tick,
) -> Option<TimelineToolbar> {
    let shown = Row::new().show(ui, |ui, _| {
        let snap = ToggleButton::new(words.snap.clone(), Bind::given(snap)).icon(IconSpec::svg(IconId::new("snap"))).icon_only().show(ui);
        let linked = ToggleButton::new(words.linked_selection.clone(), Bind::given(linked_selection))
            .icon(IconSpec::svg(IconId::new("linked_selection")))
            .icon_only()
            .show(ui);
        let refused = markers.iter().any(|marker| marker.start == playhead);
        let add_marker = IconButton::new(IconSpec::svg(IconId::new("flag"))).label(words.add_marker.clone()).enabled(!refused).show(ui);
        let settings = IconButton::new(IconSpec::svg(IconId::new("settings"))).label(words.settings.clone()).show(ui);
        (snap, linked, add_marker, settings)
    });
    shown.inner.map(|(snap, linked_selection, add_marker, settings)| TimelineToolbar { snap, linked_selection, add_marker, settings })
}

/// The Mix row: its name and the master volume in dB.
fn mix_row(ui: &mut Ui, words: &TimelineWords, value: &str) {
    Row::new().show(ui, |ui, _| {
        Text::new(words.mix.clone()).show(ui);
        Text::new(value.to_owned()).show(ui);
    });
}

/// The top strip: the head block over the headers, then the marker strip and the ruler over the time area.
#[derive(Component)]
#[component(container)]
struct TopStrip {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl TopStrip {
    fn new() -> Self {
        const COLUMNS: &[ColumnWidth] = &[ColumnWidth::Content, ColumnWidth::fill(1.0)];
        TopStrip { common: Common::new(), props: ContainerProps::new().sizing(Sizing::FillWidth).layout(BodyLayout::grid(COLUMNS)) }
    }
}

impl Component for TopStrip {
    const ROLE: Role = Role::None;
    const OVERLAYS: bool = false;
    type Style = TopStripStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: TopStripStyleKey) -> TopStripStyle {
        TopStripStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for TopStrip {}

/// The top strip has no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct TopStripStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The line between the video and audio panes: dragging it changes the video side's share of the room.
#[derive(Component)]
struct Divider<'a> {
    #[common]
    common: Common,
    share: Bind<'a, f32>,
}

impl<'a> Divider<'a> {
    fn new(label: impl Into<Label>, share: Bind<'a, f32>) -> Self {
        Divider { common: Common::new(), share }.label(label)
    }
}

impl Component for Divider<'_> {
    const ROLE: Role = Role::SPLITTER;
    const PARTS: bool = false;
    const OVERLAYS: bool = false;
    type Style = DividerStyle;
    type Output = EditResponse<f32>;

    fn style(cx: &StyleCx<'_>, _: DividerStyleKey) -> DividerStyle {
        DividerStyle { metrics: Metrics::new().min_size(Vec2::new(0.0, cx.tokens.size.hit.target)) }
    }

    fn sense(&self) -> Sense {
        Sense::DRAG
    }

    fn bind(&mut self) -> Option<BindMut<'_, f32>> {
        Some(self.share.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(share) = cx.value().copied() else { return };
        let room = cx.visible.height().max(f32::EPSILON);
        match cx.press().phase {
            PressPhase::DragStart { .. } => cx.begin(),
            PressPhase::Drag { start, pos, .. } => cx.propose((share + (pos.y - start.origin.y) / room).clamp(DIVIDER_SHARE_MIN, DIVIDER_SHARE_MAX)),
            PressPhase::Release { click: false, .. } => cx.commit(),
            _ => {}
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<f32> {
        cx.edit_response()
    }
}

/// The divider has only its minimum size.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DividerStyle {
    #[style(metrics)]
    metrics: Metrics,
}

#[cfg(feature = "screen-testing")]
mod stories;

/// The model of the pattern's registration probe: no tracks.
struct EmptySequence(crate::time::TimeView);

impl TimelineModel for EmptySequence {
    fn duration(&self) -> Tick {
        Tick::ZERO
    }

    fn format(&self) -> crate::timecode::TimeFormat {
        crate::timecode::TimeFormat::timecode(crate::rates::FrameRate::FPS_29_97)
    }

    fn playhead(&self) -> Tick {
        Tick::ZERO
    }

    fn view(&mut self) -> &mut crate::time::TimeView {
        &mut self.0
    }

    fn tracks(&self) -> Vec<TrackInfo<'_>> {
        Vec::new()
    }

    fn clips(&self, _: TrackId, _: TimeRange) -> Vec<ClipInfo<'_>> {
        Vec::new()
    }

    fn selection(&self) -> &[ClipId] {
        &[]
    }
}
