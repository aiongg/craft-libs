//! The timeline panel: track headers, track rows with clips, the ruler, the playhead and the
//! scroll bars, over the app's [`TimelineModel`] (N1.07).

use craft_theme::ControlSize;
use craft_ui::behavior::{ClaimedKey, KeyClaim, Selection, SelectionChange, SelectionPick};
use craft_ui::controls::IconButton;
use craft_ui::edit::{EditEvent, EditPolicy, Gesture, GestureId};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, fmt};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format};
use egui::accesskit::Role;
use egui::containers::scroll_area::ScrollSource;
use egui::text::LayoutJob;
use egui::{
    Align, Align2, AsIdSalt, CornerRadius, CursorIcon, EventFilter, Id, IdSalt, Key, Layout, Modifiers, Painter, PointerButton, Pos2, Rect, Response,
    ScrollArea, Sense, Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::clips::{ClipLane, ClipPainter, TimelineTool};
use super::markers::MarkerEdit;
use super::ruler::{SHIFT_STEP_FRAMES, STEP_FRAMES, TimelineRuler, TimelineScrollBar, wheel_zoom_factor};
use super::{LabelPalette, TimecodeField, TimelineStyle};
use crate::clips::{ClipEdit, EdgeZones, SnapTargets, edge_at, marquee_clips};
use crate::clock::{Follow, FollowMode};
use crate::markers::MarkerColor;
use crate::model::{ClipId, ClipInfo, TimelineModel, TrackId, TrackInfo, TrackKind};
use crate::rates::FrameRate;
use crate::time::{Tick, TimeRange, TimeView};
use crate::timecode::TimeFormat;

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
        overview: "timeline.overview" = "Overview";
        mix: "timeline.mix" = "Mix";
        track_height: "timeline.trackHeight" = "Drag to change the height of {track}";
        track_height_top: "timeline.trackHeightTop" = "Drag the top edge to change the height of {track}";
        header_width: "timeline.headerWidth" = "Drag to change the width of the track headers";
        divider: "timeline.divider" = "Drag to change how the room is shared between video and audio tracks";
        scroll_captions: "timeline.scrollCaptions" = "Scroll bar of the caption tracks";
        scroll_video: "timeline.scrollVideo" = "Scroll bar of the video tracks";
        scroll_audio: "timeline.scrollAudio" = "Scroll bar of the audio tracks";
        label_violet: "timeline.labelViolet" = "Violet";
        label_iris: "timeline.labelIris" = "Iris";
        label_caribbean: "timeline.labelCaribbean" = "Caribbean";
        label_lavender: "timeline.labelLavender" = "Lavender";
        label_cerulean: "timeline.labelCerulean" = "Cerulean";
        label_forest: "timeline.labelForest" = "Forest";
        label_rose: "timeline.labelRose" = "Rose";
        label_mango: "timeline.labelMango" = "Mango";
        label_purple: "timeline.labelPurple" = "Purple";
        label_blue: "timeline.labelBlue" = "Blue";
        label_teal: "timeline.labelTeal" = "Teal";
        label_magenta: "timeline.labelMagenta" = "Magenta";
        label_tan: "timeline.labelTan" = "Tan";
        label_green: "timeline.labelGreen" = "Green";
        label_brown: "timeline.labelBrown" = "Brown";
        label_yellow: "timeline.labelYellow" = "Yellow";
    }
}

impl TimelineLabels {
    /// The name of the track colour label `label` (Premiere's sixteen; a number past them wraps
    /// to the first).
    pub fn label_name(&self, label: MarkerColor) -> &str {
        let names = [
            &self.label_violet,
            &self.label_iris,
            &self.label_caribbean,
            &self.label_lavender,
            &self.label_cerulean,
            &self.label_forest,
            &self.label_rose,
            &self.label_mango,
            &self.label_purple,
            &self.label_blue,
            &self.label_teal,
            &self.label_magenta,
            &self.label_tan,
            &self.label_green,
            &self.label_brown,
            &self.label_yellow,
        ];
        names[usize::from(label.0) % names.len()]
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
#[non_exhaustive]
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
    /// A row's edge dragged, or a double-click on its header (a one-step `Set` between 64 and
    /// 30 points, FilmCraft `timeline/headers.rs:179-186`): the new height in points, clamped to
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
    /// The divider between the video and audio tracks dragged: the video side's new share of the
    /// room below the captions, from 0.1 to 0.9 (FilmCraft `timeline.rs:385`), as one gesture. A
    /// view setting the app keeps and hands back through [`Timeline::divider`], never in undo.
    Divider(EditEvent<f32>),
    /// A secondary click, or the Menu key or Shift+F10 with the time area focused: the app opens
    /// its menu. `anchor` is the clicked clip's or track's rect (the focused clip's with a key),
    /// `pointer` the click position, `None` for a key.
    ContextMenu {
        track: Option<TrackId>,
        clip: Option<ClipId>,
        anchor: Rect,
        pointer: Option<Pos2>,
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
/// buttons (left, over the headers, `style.top_height` high) beside the
/// [`TimelineRuler`](super::TimelineRuler); then the track rows: the caption tracks in their own
/// area (FilmCraft's), a divider, video tracks stacked upward from the next divider (V1 nearest
/// it), the divider (`style.divider_height`), audio tracks downward, and the Mix row
/// (`style.mix_row_height`) when the model has a [master volume](TimelineModel::master_volume_db);
/// each side scrolls vertically on its own; the [`TimelineScrollBar`](super::TimelineScrollBar)
/// at the bottom. Headers are [`TrackHeader`]s, `style.header_width` wide unless
/// [`header_width`](Self::header_width) says otherwise, resizable at their right edge
/// ([`TimelineEdit::HeaderWidth`], within `style.header_width_min`…`style.header_width_max`);
/// rows are [`ClipLane`](super::ClipLane)s, which paint their clips' content through
/// [`content`](Self::content). The playhead line runs over every row; the snap line shows while
/// a gesture snaps. `style` is [`TimelineStyle`](super::TimelineStyle) (`timeline.*` tokens). The
/// toolbar buttons are [`IconButton`]s the app owns the state of (Snap, Linked Selection),
/// reported through their responses on [`TimelineOutput::toolbar`]; the app adds a marker when
/// the Add Marker response clicks (FilmCraft `ruler.rs:70`).
///
/// Input:
/// - FilmCraft's wheel mapping (Premiere Pro on macOS), read from the wheel events and their own
///   modifiers: the wheel scrolls the side of the divider under the pointer, Cmd or Ctrl and the
///   wheel scroll the time sideways (so does Shift, and any mostly sideways gesture), Alt and
///   the wheel zoom about the pointer by `(1 + dy * 0.01).clamp(0.5, 2.0)`, a pinch zooms. A sideways
///   scroll is not eased; zooms ease to their target as FilmCraft's do.
/// - `=` and `-` zoom in and out by 1.6 about the playhead (FilmCraft), `\` fits the sequence.
/// - Each side of the divider has a vertical scroll bar (a kernel scroll area) right of the time
///   area, and the divider drags ([`TimelineEdit::Divider`], [`divider`](Self::divider)).
/// - A clip drag keeps its source row shown (clipped) when a wheel scroll takes the row out of
///   view, so it ends when the button is released. A move onto a locked track is refused (the
///   Not Allowed cursor); a new track is offered only when every source track is unlocked.
///   Esc drops the marquee.
/// - While playing, the view follows the playhead by the [`FollowMode`]; a jump while stopped
///   reveals it ([`TimeView::scroll_to_reveal`](crate::TimeView::scroll_to_reveal)).
/// - A drop of media from the app is the app's (it reads the pointer over
///   [`TimelineOutput::row_at_pointer`]).
/// - Keyboard: the panel is a focus region (F6 reaches it); the time area is one Tab stop
///   whose Left / Right step the playhead a frame (Shift: five frames, Premiere's), Home / End
///   go to the start and end, Up / Down to the previous and next edit point
///   ([`TimelineModel::edit_points`]), each a [`TimelineEdit::Playhead`] `Set`; every key is an
///   exact chord (Alt+Left is not Left). The Menu key and Shift+F10 report a
///   [`TimelineEdit::ContextMenu`] for the selection's last clip, else the first targeted track,
///   with the clip's or the row's rect as `anchor` and no `pointer`. The header switches are Tab
///   stops in row order; Delete on selected clips is the app's shortcut (the panel claims no keys
///   beyond these).
/// - AccessKit: the time area is a `Group` named by [`TimelineLabels::panel`]; each track a
///   `Row`; each height edge and the headers' width edge a `Splitter`; each clip a `Button` named by its name with its start and duration as the value
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
    linked_selection: bool,
    no_modifier_trims: bool,
    follow: FollowMode,
    playing: bool,
    header_width: Option<f32>,
    divider: Option<f32>,
    palette: Option<&'a LabelPalette>,
    header_extra: Option<HeaderExtra<'a>>,
    content: Option<ClipPainter<'a>>,
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
            linked_selection: true,
            no_modifier_trims: false,
            follow: FollowMode::Page,
            playing: false,
            header_width: None,
            divider: None,
            palette: None,
            header_extra: None,
            content: None,
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

    /// Whether Linked Selection is on, for its toolbar toggle's look; the app owns the setting.
    pub fn linked_selection(mut self, on: bool) -> Self {
        self.linked_selection = on;
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

    /// The video side's share of the room below the captions that the user dragged the divider to
    /// ([`TimelineEdit::Divider`]), kept by the app. Without it each side gets what its tracks
    /// need, else the sides share the room.
    pub fn divider(mut self, share: f32) -> Self {
        self.divider = Some(share);
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

    /// Paints each clip's content into its body rect (waveforms, thumbnails), forwarded to every
    /// lane's [`ClipLane::content`](super::ClipLane::content).
    pub fn content(mut self, paint: impl FnMut(&Painter, Rect, &ClipInfo<'_>) + 'a) -> Self {
        self.content = Some(Box::new(paint));
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
        let Self {
            id_salt,
            model,
            tool,
            snap,
            linked_selection,
            no_modifier_trims,
            follow,
            playing,
            header_width,
            divider,
            palette,
            mut header_extra,
            mut content,
            labels,
        } = self;
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let color = &tokens.color;
        let labels = labels.unwrap_or_else(|| TimelineLabels::of(ui));
        let default_palette = LabelPalette::default();
        let palette = palette.unwrap_or(&default_palette);
        let id = ui.make_persistent_id(id_salt);
        let enabled = ui.is_enabled();
        let mods = ui.input(|i| i.modifiers);
        let mut view = *model.view();
        let mut out = Pass::default();
        let mut menu_key = false;

        let output = {
            let model: &M = &*model;
            let tracks = model.tracks();
            let format = model.format();
            let (rate, duration, playhead) = (format.rate, model.duration(), model.playhead());
            let heights: Vec<f32> = tracks.iter().map(|track| resolved_height(&style, track)).collect();
            let side_of = |track: &TrackInfo<'_>| match track.kind {
                TrackKind::Caption => Side::Caption,
                TrackKind::Video => Side::Upper,
                _ => Side::Lower,
            };
            let on_side = |side: Side| -> Vec<usize> { (0..tracks.len()).filter(|&i| side_of(&tracks[i]) == side).collect() };
            let (captions, upper, lower) = (on_side(Side::Caption), on_side(Side::Upper), on_side(Side::Lower));
            let total = |indices: &[usize]| indices.iter().map(|&i| heights[i]).sum::<f32>();
            let (caption_need, upper_sum, lower_sum) = (total(&captions), total(&upper), total(&lower));
            // The video and audio sides keep one empty track's height past their last track: where
            // a drop adds one.
            let (upper_need, lower_need) = (upper_sum + style.video_track_height, lower_sum + style.audio_track_height);
            let mix = model.master_volume_db();

            let avail = ui.available_size();
            let wanted = style.top_height
                + caption_need
                + upper_need
                + style.divider_height
                + lower_need
                + if mix.is_some() { style.mix_row_height } else { 0.0 }
                + style.scrollbar_height;
            let (outer, _) = ui.allocate_exact_size(vec2(avail.x, if avail.y.is_finite() { avail.y } else { wanted }), Sense::hover());
            let panel_response = ui.interact(outer, id.with("panel"), Sense::hover());
            let header_w = header_width.unwrap_or(style.header_width).clamp(style.header_width_min, style.header_width_max).min(outer.width());
            let geo = Geo::new(
                &style,
                outer,
                header_w,
                Needs { caption: caption_need, upper: upper_need, lower: lower_need, mix: mix.is_some(), share: divider },
            );
            let (left, width) = (geo.time_left, geo.time_area.width());

            let painter = ui.painter_at(outer);
            painter.rect_filled(outer, CornerRadius::ZERO, color.surface.panel.to_egui());
            painter.rect_filled(geo.top, CornerRadius::ZERO, color.surface.chrome.to_egui());
            painter.rect_filled(geo.time_area, CornerRadius::ZERO, color.surface.sunken.to_egui());
            painter.rect_filled(geo.divider, CornerRadius::ZERO, color.surface.chrome.to_egui());
            painter.rect_filled(geo.caption_divider, CornerRadius::ZERO, color.surface.chrome.to_egui());

            // The time area is the panel's one Tab stop, below the lanes: a lane gets the pointer
            // and hands the focus over.
            let area_id = id.with("area");
            let area_response = ui.interact(geo.time_area, area_id, Sense::focusable_noninteractive());
            area_response.widget_info(|| WidgetInfo::labeled(Role::Group, enabled, &labels.panel));

            // Wheel, keys and the view's easing.
            let anim_id = id.with("view");
            let mut anim = ui.data(|d| d.get_temp::<ViewAnim>(anim_id)).unwrap_or(ViewAnim { target: view, last: view, anchor: None, seen: None });
            if view != anim.last {
                anim.target = view;
                anim.anchor = None;
            }
            let scrolls_id = id.with("vscroll");
            let mut scrolls = ui.data(|d| d.get_temp::<Scrolls>(scrolls_id)).unwrap_or_default();
            let valid = view.points_per_second > 0.0 && view.points_per_second.is_finite() && width > 0.0;
            if enabled && valid {
                wheel(ui, &geo, &mut anim, &mut view, &mut scrolls);
                if area_response.has_focus() {
                    let keys = Keys { id: area_id, rate, playhead, duration, points: model.edit_points(), width, fit_margin: style.fit_margin };
                    match keys.handle(ui, &mut anim) {
                        Some(KeyOutcome::Playhead(to)) => out.edits.push(TimelineEdit::Playhead(EditEvent::set(to, mods))),
                        Some(KeyOutcome::ContextMenu) => menu_key = true,
                        None => {}
                    }
                }
            }
            scrolls.caption = scrolls.caption.clamp(0.0, geo.max_caption);
            scrolls.upper = scrolls.upper.clamp(0.0, geo.max_upper);
            scrolls.lower = scrolls.lower.clamp(0.0, geo.max_lower);
            if enabled {
                scrolls.caption = side_scroll_bar(
                    ui,
                    (id, "vscroll", 0u8),
                    geo.caption.with_min_x(geo.content_right),
                    caption_need,
                    scrolls.caption,
                    false,
                    &labels.scroll_captions,
                );
                scrolls.upper = side_scroll_bar(
                    ui,
                    (id, "vscroll", 1u8),
                    geo.upper.with_min_x(geo.content_right),
                    upper_need,
                    scrolls.upper,
                    true,
                    &labels.scroll_video,
                );
                scrolls.lower = side_scroll_bar(
                    ui,
                    (id, "vscroll", 2u8),
                    geo.lower.with_min_x(geo.content_right),
                    lower_need,
                    scrolls.lower,
                    false,
                    &labels.scroll_audio,
                );
            }
            ui.data_mut(|d| d.insert_temp(scrolls_id, scrolls));
            if valid {
                follow_view(ui, anim_id, &mut anim, &mut view, FollowStep { mode: follow, playing, width, playhead, duration });
            }
            let eased = view;

            // The clips in view, and what snapping reaches.
            let visible = TimeRange::from_bounds(view.start, view.time_at(geo.time_area.right(), left));
            let clips: Vec<Vec<ClipInfo<'_>>> = tracks.iter().map(|track| model.clips(track.id, visible)).collect();
            let targets = snap.then(|| {
                let mut targets = SnapTargets::new();
                targets.ranges(clips.iter().flatten().map(|clip| clip.range));
                targets.push(playhead);
                targets.extend(model.markers().iter().map(|marker| marker.start));
                let (mark_in, mark_out) = model.in_out();
                targets.extend(mark_in.into_iter().chain(mark_out));
                targets.ranges(model.work_area());
                targets
            });

            // Top left: the large timecode and the toolbar toggles.
            let head = Head { id, rect: Rect::from_min_max(geo.top.min, pos2(left, geo.top.bottom())), playhead, format, snap, linked_selection };
            let (timecode, toolbar) = head.show(ui, &labels);
            out.edits.extend(timecode.map(TimelineEdit::Playhead));

            // The ruler.
            let mut ruler_ui = ui.new_child(UiBuilder::new().max_rect(geo.ruler));
            ruler_ui.set_clip_rect(geo.ruler.intersect(ui.clip_rect()));
            let ruler = TimelineRuler::new((id, "ruler"), &mut view, format, duration)
                .playhead(playhead)
                .in_out(model.in_out())
                .work_area(model.work_area())
                .markers(model.markers())
                .snap(targets.as_ref())
                .show(&mut ruler_ui);
            out.edits.extend(ruler.scrub.map(TimelineEdit::Playhead));
            out.edits.extend(ruler.work_area.map(TimelineEdit::WorkArea));
            out.edits.extend(ruler.markers.into_iter().map(TimelineEdit::Marker));
            out.snapped = ruler.snapped_at.or(out.snapped);
            if ruler.response.double_clicked()
                && let Some(pos) = ruler.response.interact_pointer_pos()
                && pos.y < geo.ruler.top() + style.ruler_marker_band
                && !model.markers().iter().any(|marker| (view.x_of(marker.start, left) - pos.x).abs() <= style.marker_hit / 2.0)
            {
                out.edits.push(TimelineEdit::Marker(MarkerEdit::Add { at: view.time_at(pos.x, left).max(Tick::ZERO) }));
            }

            // The rows.
            let mut rows = Vec::with_capacity(tracks.len());
            for (indices, top, side) in [
                (&captions, geo.caption.top() - scrolls.caption, Side::Caption),
                (&upper, geo.upper.bottom() - upper_sum + scrolls.upper, Side::Upper),
                (&lower, geo.lower.top() - scrolls.lower, Side::Lower),
            ] {
                let mut y = top;
                for (position, &index) in indices.iter().enumerate() {
                    let kind = tracks[index].kind;
                    let before = indices[..position].iter().filter(|&&i| tracks[i].kind == kind).count();
                    let of_kind = indices.iter().filter(|&&i| tracks[i].kind == kind).count();
                    let kind_index = if side == Side::Upper { of_kind - 1 - before } else { before } as i32;
                    let top_video = side == Side::Upper && position == 0;
                    rows.push(Row {
                        index,
                        kind,
                        side,
                        kind_index,
                        top_video,
                        locked: tracks[index].locked.unwrap_or(false),
                        top: y,
                        bottom: y + heights[index],
                    });
                    y += heights[index];
                }
            }
            let drag_key = id.with("clip drag");
            out.drag = ui.data(|d| d.get_temp::<ClipDrag>(drag_key));
            let marquee_key = id.with("marquee");
            out.marquee = ui.data(|d| d.get_temp::<Marquee>(marquee_key));
            let selection = model.selection();
            let track_ids: Vec<TrackId> = tracks.iter().map(|track| track.id).collect();
            if tracks.is_empty() {
                let text = role_format(&tokens, style.empty_text, color.text.muted.to_egui());
                let galley = ui.fonts_mut(|fonts| fonts.layout_job(LayoutJob::single_section(labels.empty.clone(), text.clone())));
                painter.galley(geo.time_area.center() - galley.size() / 2.0, galley, text.color);
            }
            let cx = RowCx {
                id,
                area_id,
                geo,
                rows: &rows,
                clips: &clips,
                zones: EdgeZones { edge: style.edge_zone, roll: style.roll_zone },
                track_ids: &track_ids,
                view: &view,
                rate,
                tool,
                no_modifier_trims,
                targets: targets.as_ref(),
                selection,
                palette,
                labels: &labels,
                mods,
                enabled,
                pointer_y: ui.input(|i| i.pointer.latest_pos()).map(|pos| pos.y),
                hover: ui.input(|i| i.pointer.hover_pos()),
            };
            for (area, side) in [(geo.caption, Side::Caption), (geo.upper, Side::Upper), (geo.lower, Side::Lower)] {
                let mut child = ui.new_child(UiBuilder::new().max_rect(area));
                child.set_clip_rect(area.intersect(ui.clip_rect()));
                for row in rows.iter().filter(|row| row.side == side) {
                    let (track, clips) = (&tracks[row.index], &clips[row.index]);
                    // The source row of a clip drag stays shown (clipped) after it scrolls out of view,
                    // so its lane sees the release.
                    let drags = out.drag.as_ref().is_some_and(|drag| drag.source == track.id);
                    if drags || row_rect(&geo, row).intersects(area) {
                        let hooks = RowHooks { extra: header_extra.as_mut(), content: content.as_mut() };
                        cx.show_row(&mut child, area, row, (track, heights[row.index], clips), hooks, &mut out);
                    }
                }
            }
            if let Some(db) = mix {
                paint_mix_row(ui, &geo, &tokens, &labels, db);
            }
            out.settle_drag(ui, &cx, drag_key);
            out.marquee_pass(ui, &cx, marquee_key, tracks.iter().zip(&clips).map(|(track, list)| (track.id, list.as_slice())));

            // The playhead line and the snap line over every row.
            let lines = ui.painter_at(geo.time_area);
            let hairline = craft_theme::Stroke::W1.width(ui.pixels_per_point());
            if let Some(at) = out.snapped {
                let x = view.x_of(at, left);
                lines.line_segment(
                    [pos2(x, geo.time_area.top()), pos2(x, geo.time_area.bottom())],
                    Stroke::new(hairline, color.overlay.line.to_egui()),
                );
            }
            let x = view.x_of(playhead, left);
            lines.line_segment([pos2(x, geo.time_area.top()), pos2(x, geo.time_area.bottom())], Stroke::new(hairline, color.accent.solid.to_egui()));
            let separator = Stroke::new(craft_theme::Stroke::Hairline.width(ui.pixels_per_point()), color.line.divider.to_egui());
            painter.line_segment([pos2(left, geo.top.top()), pos2(left, geo.rows.bottom())], separator);
            out.edits.extend(resize_headers(ui, &geo, header_w, id, &labels));
            out.edits.extend(resize_divider(ui, &geo, id, &labels));
            if menu_key {
                out.edits.extend(menu_key_edit(&cx, model.selection(), &tracks));
            }

            // The scroll bar.
            let mut bar_ui = ui.new_child(UiBuilder::new().max_rect(geo.bar));
            bar_ui.set_clip_rect(geo.bar.intersect(ui.clip_rect()));
            TimelineScrollBar::new((id, "scroll"), &mut view, duration).show(&mut bar_ui);

            // Whatever moved the view directly (the ruler, the scroll bar) is the new target.
            if view != eased {
                anim.target = view;
                anim.anchor = None;
            }
            anim.last = view;
            ui.data_mut(|d| d.insert_temp(anim_id, anim));
            if view != anim.target || anim.anchor.is_some() {
                ui.ctx().request_repaint();
            }

            TimelineOutput {
                response: panel_response,
                edits: std::mem::take(&mut out.edits),
                toolbar,
                row_at_pointer: out.row_at_pointer,
                time_area: geo.time_area,
            }
        };
        *model.view() = view;
        output
    }
}

/// What the panel did this pass.
#[must_use]
pub struct TimelineOutput {
    pub response: Response,
    /// This pass's edits, in the order the user made them.
    pub edits: Vec<TimelineEdit>,
    /// The toolbar buttons' responses; the app reacts to their clicks.
    pub toolbar: TimelineToolbar,
    /// The track row under the pointer and the time there (drops, the app's own overlays).
    pub row_at_pointer: Option<(TrackId, Tick)>,
    /// The time area's rect this pass (the app's overlays: captions, effect drop previews).
    pub time_area: Rect,
}

/// The toolbar buttons' responses: the Snap and Linked Selection toggles (the app flips its
/// settings on a click), Add Marker (the app adds the marker at the playhead; the panel reports
/// no `TimelineEdit` for it, FilmCraft `ruler.rs:70`) and the display settings.
pub struct TimelineToolbar {
    pub snap: Response,
    pub linked_selection: Response,
    pub add_marker: Response,
    pub settings: Response,
}

/// One track's header (Premiere's): the colour label strip (`style.label_strip` wide; a click
/// opens a [`PressMenu`](craft_ui::menu::PressMenu) of the labels and None), the name (indented
/// by the track's `depth`), then the switches the [`TrackInfo`] has (`Some`): lock, target, sync
/// lock, output, mute, solo, as icon [`IconButton`]s with the tooltips of [`TimelineLabels`]
/// (FilmCraft puts the switches on the header's upper line and the name on the lower one); and a
/// resize edge at the bottom (vertical resize cursor;
/// `style.track_height_min`…`style.track_height_max`), which starts from the row's
/// [`height`](Self::height). Rows above the divider grow upward, so
/// [`above_divider`](Self::above_divider) negates their travel, and the top video row also has
/// a top edge ([`top_edge`](Self::top_edge)), as FilmCraft's. A double-click on the header's
/// body toggles the height between 64 and 30 points. The label tints the header by
/// `HEADER_LABEL_TINT`, a share of the app's colour (FilmCraft `timeline.rs:254`). The colour
/// strip and the edges are pointer-only; each switch is a Tab stop; Space toggles; Alt-click on
/// Mute or Solo is exclusive. AccessKit: each switch a toggle `Button` named by its tooltip and
/// the track's name; each edge a `Splitter` named by the track.
pub struct TrackHeader<'a> {
    id_salt: IdSalt,
    track: &'a TrackInfo<'a>,
    height: Option<f32>,
    above_divider: bool,
    top_edge: bool,
    palette: Option<&'a LabelPalette>,
    labels: Option<&'a TimelineLabels>,
}

impl<'a> TrackHeader<'a> {
    pub fn new(id_salt: impl AsIdSalt, track: &'a TrackInfo<'a>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), track, height: None, above_divider: false, top_edge: false, palette: None, labels: None }
    }

    /// The row's height this pass, which a resize starts from. Without it the header resolves
    /// the track's height itself (the style's default for its kind when `None`).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The row is above the divider: dragging an edge up grows it (video rows stack upward).
    pub fn above_divider(mut self, above: bool) -> Self {
        self.above_divider = above;
        self
    }

    /// Adds a resize edge at the header's top (the top video row's outer edge).
    pub fn top_edge(mut self, on: bool) -> Self {
        self.top_edge = on;
        self
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
        let Self { id_salt, track, height, above_divider, top_edge, palette, labels } = self;
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let color = &tokens.color;
        let labels_owned;
        let labels = match labels {
            Some(labels) => labels,
            None => {
                labels_owned = TimelineLabels::of(ui);
                &labels_owned
            }
        };
        let palette_owned;
        let palette = match palette {
            Some(palette) => palette,
            None => {
                palette_owned = LabelPalette::default();
                &palette_owned
            }
        };
        let id = ui.make_persistent_id(id_salt);
        let enabled = ui.is_enabled();
        let mods = ui.input(|i| i.modifiers);
        let height = height.unwrap_or_else(|| resolved_height(&style, track));
        let response = ui.interact(rect, id, Sense::hover());
        let mut edits = Vec::new();
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, color.surface.panel.to_egui());
        let strip = Rect::from_min_size(rect.min, vec2(style.label_strip, rect.height()));
        if let Some(label) = track.label {
            let tint = palette.color(label);
            painter.rect_filled(rect, CornerRadius::ZERO, tint.gamma_multiply(HEADER_LABEL_TINT));
            painter.rect_filled(strip, CornerRadius::ZERO, tint);
        }
        let divider = Stroke::new(craft_theme::Stroke::Hairline.width(ui.pixels_per_point()), color.line.divider.to_egui());
        painter.line_segment([pos2(rect.left(), rect.bottom()), pos2(rect.right(), rect.bottom())], divider);

        // The colour strip opens the label menu.
        let strip_response = ui.interact(strip, id.with("strip"), Sense::CLICK);
        strip_response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, &labels.track_color));
        let strip_response = strip_response.on_hover_text(&labels.track_color);
        if strip_response.hovered() && track.label.is_none() {
            painter.rect_filled(strip, CornerRadius::ZERO, color.state.hover.to_egui());
        }
        let mut entries: Vec<MenuEntry<Option<MarkerColor>>> =
            vec![MenuItem::new(labels.no_label.as_str(), None).radio(track.label.is_none()).into()];
        for (n, tint) in palette.0.iter().enumerate() {
            let label = MarkerColor(u8::try_from(n).unwrap_or(u8::MAX));
            entries.push(MenuItem::new(labels.label_name(label), Some(label)).radio(track.label == Some(label)).tint(*tint).into());
        }
        if enabled && let Some(label) = PressMenu::new(&entries).show(ui, &strip_response).chosen {
            edits.push(TimelineEdit::Label { track: track.id, label });
        }

        // A double-click on the header's body toggles the height (below the switches in z order).
        let body = Rect::from_min_max(pos2(strip.right(), rect.top()), rect.max);
        if ui.interact(body, id.with("body"), Sense::CLICK).double_clicked() && enabled {
            let toggled = if height < HEIGHT_TOGGLE_BELOW { HEIGHT_TOGGLE_TALL } else { HEIGHT_TOGGLE_SHORT };
            edits.push(TimelineEdit::TrackHeight {
                track: track.id,
                height: EditEvent::set(toggled.clamp(style.track_height_min, style.track_height_max), mods),
            });
        }

        // The name (indented by the track's depth), then the switches the track has.
        let text = role_format(&tokens, style.header_text, color.text.normal.to_egui());
        let gap = ui.spacing().item_spacing.x;
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(LayoutJob::single_section(track.name.to_owned(), text.clone())));
        let mut x = strip.right() + gap + f32::from(track.depth) * header_indent(&style);
        painter.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley.clone(), text.color);
        x += galley.size().x + gap;
        let side = IconButton::style(&tokens, ControlSize::Sm).side;
        let switches = [
            (TrackSwitch::Locked, track.locked, Icon::LockOpen, Icon::Lock, labels.lock.as_str()),
            (TrackSwitch::Targeted, track.targeted, Icon::Target, Icon::Target, labels.target.as_str()),
            (TrackSwitch::SyncLock, track.sync_lock, Icon::SyncLock, Icon::SyncLock, labels.sync_lock.as_str()),
            (TrackSwitch::Enabled, track.enabled, Icon::EyeOff, Icon::Eye, labels.output.as_str()),
            (TrackSwitch::Muted, track.muted, Icon::TrackMute, Icon::TrackMute, labels.mute.as_str()),
            (TrackSwitch::Solo, track.solo, Icon::Solo, Icon::Solo, labels.solo.as_str()),
        ];
        let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        for (switch, value, off_icon, on_icon, tip) in switches {
            let Some(value) = value else { continue };
            let at = Rect::from_center_size(pos2(x + side / 2.0, rect.center().y), vec2(side, side));
            x += side + gap;
            let name = format!("{tip} {}", track.name);
            let mut on = value;
            let button = IconButton::new(off_icon, name.as_str())
                .when_on(on_icon, name.as_str())
                .toggle(&mut on)
                .size(ControlSize::Sm)
                .with_id(id.with(("switch", switch as u8)))
                .enabled(enabled)
                .show_at(&mut child, at);
            if button.clicked() {
                let exclusive = mods.alt && matches!(switch, TrackSwitch::Muted | TrackSwitch::Solo);
                edits.push(TimelineEdit::Switch { track: track.id, switch, value: on, exclusive });
            }
        }

        // The edges resize the row: the bottom one, and the top one of the top video row.
        let sign = if above_divider { -1.0 } else { 1.0 };
        let half = style.edge_zone / 2.0;
        let bottom = Rect::from_min_max(pos2(rect.left(), rect.bottom() - half), pos2(rect.right(), rect.bottom() + half));
        let top = Rect::from_min_max(pos2(rect.left(), rect.top() - half), pos2(rect.right(), rect.top() + half));
        let name = |word: &str| fmt(word, &[("track", track.name)]);
        let mut edges = vec![(HeaderEdge::Bottom, bottom, name(&labels.track_height))];
        if top_edge {
            edges.push((HeaderEdge::Top, top, name(&labels.track_height_top)));
        }
        for (edge_kind, grab, label) in edges {
            let edge = ui.interact(grab, id.with(("height", edge_kind as u8)), Sense::DRAG);
            edge.widget_info(|| WidgetInfo::labeled(Role::Splitter, enabled, &label));
            if edge.hovered() || edge.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
            }
            let mut resize = Gesture::load(ui, edge.id, &height, EditPolicy::default()).enabled(enabled);
            if edge.drag_started_by(PointerButton::Primary) {
                resize.begin();
            }
            if resize.is_active()
                && edge.dragged()
                && let (Some(origin), Some(pos)) = (ui.input(|i| i.pointer.press_origin()), ui.input(|i| i.pointer.latest_pos()))
            {
                let wanted = *resize.start().unwrap_or(&height) + sign * (pos.y - origin.y);
                resize.update(wanted.clamp(style.track_height_min, style.track_height_max), mods);
            }
            if edge.drag_stopped() {
                resize.commit();
            }
            edits.extend(resize.finish().map(|height| TimelineEdit::TrackHeight { track: track.id, height }));
        }
        TrackHeaderOutput { response, edits }
    }
}

/// What a header did this pass (the panel passes its edits on as [`TimelineEdit`]s).
#[must_use]
pub struct TrackHeaderOutput {
    pub response: Response,
    pub edits: Vec<TimelineEdit>,
}

/// How strongly a track's colour label tints its header: a share of the app's label colour
/// (FilmCraft `timeline.rs:254` `TRACK_HEADER_TINT`).
const HEADER_LABEL_TINT: f32 = 0.18;
/// A double-click on a header sets a row below this height to the tall one, else to the short
/// one (FilmCraft `timeline/headers.rs:179-186`).
const HEIGHT_TOGGLE_BELOW: f32 = 50.0;
const HEIGHT_TOGGLE_TALL: f32 = 64.0;
const HEIGHT_TOGGLE_SHORT: f32 = 30.0;

/// Which edge of a header a resize came from.
#[derive(Clone, Copy)]
enum HeaderEdge {
    Bottom,
    Top,
}

/// A track's height in points: its own, or the style's default for its kind, within the style's
/// least and most.
fn resolved_height(style: &TimelineStyle, track: &TrackInfo<'_>) -> f32 {
    let default = match track.kind {
        TrackKind::Video => style.video_track_height,
        TrackKind::Caption => style.caption_row_height,
        TrackKind::Audio | TrackKind::Other => style.audio_track_height,
    };
    track.height.filter(|height| height.is_finite() && *height > 0.0).unwrap_or(default).clamp(style.track_height_min, style.track_height_max)
}

/// How far each level of a track's `depth` indents its name.
fn header_indent(style: &TimelineStyle) -> f32 {
    style.header_indent
}
/// The factor one `=` or `-` press zooms by, about the playhead (FilmCraft `menus.rs:421`).
const KEY_ZOOM: f64 = 1.6;
/// The view's ease rate per second (FilmCraft `timeline.rs:468`) and the longest frame it counts.
const EASE_RATE: f64 = 20.0;
const EASE_MAX_DT: f32 = 0.05;
/// A zoom within this share of its target has arrived; a scroll within this many points has.
const ZOOM_SETTLE: f64 = 0.0005;
const SCROLL_SETTLE: f64 = 0.05;
/// The least and most of the video side's share of the room below the captions (FilmCraft
/// `timeline.rs:385`).
/// How far a pinch's factor differs from 1 before it counts (FilmCraft `interact.rs:233`).
const WHEEL_PINCH_EPSILON: f64 = 1e-4;
const DIVIDER_SHARE_MIN: f32 = 0.1;
const DIVIDER_SHARE_MAX: f32 = 0.9;

/// What a key of the time area does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyAction {
    Step { forward: bool, five: bool },
    Start,
    End,
    PreviousEdit,
    NextEdit,
    ZoomIn,
    ZoomOut,
    Fit,
    ContextMenu,
}

/// The keys the time area claims while focused, each an exact chord (Alt+Left is Premiere's
/// nudge, Shift+Home is not Home).
const KEYS: [(ClaimedKey, KeyAction); 16] = [
    (ClaimedKey::exact(Modifiers::NONE, Key::ArrowLeft), KeyAction::Step { forward: false, five: false }),
    (ClaimedKey::exact(Modifiers::NONE, Key::ArrowRight), KeyAction::Step { forward: true, five: false }),
    (ClaimedKey::exact(Modifiers::SHIFT, Key::ArrowLeft), KeyAction::Step { forward: false, five: true }),
    (ClaimedKey::exact(Modifiers::SHIFT, Key::ArrowRight), KeyAction::Step { forward: true, five: true }),
    (ClaimedKey::exact(Modifiers::NONE, Key::Home), KeyAction::Start),
    (ClaimedKey::exact(Modifiers::NONE, Key::End), KeyAction::End),
    (ClaimedKey::exact(Modifiers::NONE, Key::ArrowUp), KeyAction::PreviousEdit),
    (ClaimedKey::exact(Modifiers::NONE, Key::ArrowDown), KeyAction::NextEdit),
    (ClaimedKey::exact(Modifiers::NONE, Key::Equals), KeyAction::ZoomIn),
    (ClaimedKey::exact(Modifiers::SHIFT, Key::Equals), KeyAction::ZoomIn),
    (ClaimedKey::exact(Modifiers::NONE, Key::Plus), KeyAction::ZoomIn),
    (ClaimedKey::exact(Modifiers::SHIFT, Key::Plus), KeyAction::ZoomIn),
    (ClaimedKey::exact(Modifiers::NONE, Key::Minus), KeyAction::ZoomOut),
    (ClaimedKey::exact(Modifiers::NONE, Key::Backslash), KeyAction::Fit),
    (ClaimedKey::exact(Modifiers::NONE, Key::Menu), KeyAction::ContextMenu),
    (ClaimedKey::exact(Modifiers::SHIFT, Key::F10), KeyAction::ContextMenu),
];

/// The view's eased state: where it is heading, what it was last pass, the time kept under one
/// x while a zoom eases, and the playhead seen last pass.
#[derive(Clone, Copy, Debug)]
struct ViewAnim {
    target: TimeView,
    last: TimeView,
    anchor: Option<(Tick, f32)>,
    seen: Option<Tick>,
}

/// How far each side of the divider is scrolled, in points.
#[derive(Clone, Copy, Debug, Default)]
struct Scrolls {
    caption: f32,
    upper: f32,
    lower: f32,
}

/// The three areas of rows: the captions at the top, video above the divider, the rest below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Caption,
    Upper,
    Lower,
}

/// A track row's place this pass. `kind_index` counts away from the divider within the kind
/// (V1 is 0, V2 is 1; A1 is 0; captions count from the top). `top_video` marks the top video
/// row, which has a top edge.
#[derive(Clone, Copy, Debug)]
struct Row {
    index: usize,
    kind: TrackKind,
    side: Side,
    kind_index: i32,
    top_video: bool,
    locked: bool,
    top: f32,
    bottom: f32,
}

/// A clip drag in progress: its gesture, the last value reported and the lane it started in.
#[derive(Clone, Debug)]
struct ClipDrag {
    id: GestureId,
    value: ClipEdit,
    source: TrackId,
}

/// A marquee started on empty track space.
#[derive(Clone, Copy, Debug)]
struct Marquee {
    origin: Pos2,
}

/// Zooms the view's target by `factor` about `anchor`, which stays under the same x while the view
/// eases.
fn zoom_target(anim: &mut ViewAnim, factor: f64, anchor: Tick, width: f32) {
    let target = &mut anim.target;
    let x = ((anchor - target.start).seconds() * target.points_per_second) as f32;
    let x = if (0.0..=width).contains(&x) { x } else { width / 2.0 };
    target.zoom_about(factor, anchor, width);
    anim.anchor = Some((anchor, x));
}

/// Moves `view` one step toward the target (in one step with animations off).
// filmcraft: crates/ui-egui/src/panels/timeline.rs:468 `animate_view`.
fn ease(ui: &Ui, anim: &mut ViewAnim, view: &mut TimeView) {
    let target = anim.target;
    if !(target.points_per_second > 0.0 && target.points_per_second.is_finite()) {
        return;
    }
    let dt = ui.input(|i| i.stable_dt).min(EASE_MAX_DT);
    let k = if ui.style().animation_time > 0.0 { 1.0 - (-f64::from(dt) * EASE_RATE).exp() } else { 1.0 };
    let zooming = (view.points_per_second - target.points_per_second).abs() / target.points_per_second > ZOOM_SETTLE;
    view.points_per_second += (target.points_per_second - view.points_per_second) * k;
    if (view.points_per_second - target.points_per_second).abs() / target.points_per_second < ZOOM_SETTLE {
        view.points_per_second = target.points_per_second;
    }
    match anim.anchor.filter(|_| zooming) {
        Some((time, x)) => view.start = (time - Tick::from_seconds_f64(f64::from(x) / view.points_per_second)).max(Tick::ZERO),
        None => {
            anim.anchor = None;
            let (from, to) = (view.start.seconds(), target.start.seconds());
            view.start = if (to - from).abs() * view.points_per_second < SCROLL_SETTLE {
                target.start
            } else {
                Tick::from_seconds_f64(from + (to - from) * k)
            };
        }
    }
    if *view != target {
        ui.ctx().request_repaint();
    }
}

/// Where a clip move is headed: how many tracks it changes, whether it adds one, and whether the
/// panel refuses it (a locked track is involved).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Derived {
    track_delta: i32,
    new_track: bool,
    refused: bool,
}

impl Derived {
    const STAY: Self = Self { track_delta: 0, new_track: false, refused: false };
}

/// What a drag at `y` does to a move from `sources` (the lane's own row first, then the rows of
/// the other moved clips): `track_delta` counts the rows of the pointer's row away from the
/// lane's when it is of the same kind and 0 off every row; `new_track` is a separate flag for
/// the empty space past the last track of the kind (above the top video track, below the last
/// audio track), where the drop adds a track. A move that starts on or reaches a locked track is
/// refused and stays where it was. A lane knows only its own row, so the panel works this out
/// from all rows.
// filmcraft: crates/ui-egui/src/panels/timeline/layout.rs:40 `new_track_at`, `interact.rs:597-611`.
fn resolve_move(rows: &[Row], sources: &[&Row], y: f32, bounds: (f32, f32)) -> Derived {
    let Some(source) = sources.first() else { return Derived::STAY };
    if !(bounds.0..=bounds.1).contains(&y) {
        return Derived::STAY;
    }
    let (track_delta, new_track) = match rows.iter().find(|row| row.top <= y && y < row.bottom) {
        Some(row) if row.kind == source.kind => (row.kind_index - source.kind_index, false),
        Some(_) => (0, false),
        None if source.side == Side::Caption => (0, false),
        None => {
            let same = || rows.iter().filter(|row| row.kind == source.kind);
            let past = if source.kind == TrackKind::Video { same().all(|row| y < row.top) } else { same().all(|row| y >= row.bottom) };
            (0, past)
        }
    };
    let reaches_locked = |from: &Row| {
        let count = rows.iter().filter(|row| row.kind == from.kind).count() as i32;
        let target = (from.kind_index + track_delta).clamp(0, (count - 1).max(0));
        rows.iter().any(|row| row.kind == from.kind && row.kind_index == target && row.locked)
    };
    if sources.iter().any(|from| from.locked || (track_delta != 0 && reaches_locked(from))) {
        return Derived { track_delta: 0, new_track: false, refused: true };
    }
    Derived { track_delta, new_track, refused: false }
}

/// The clips a move event carries.
fn moved_clips(event: &EditEvent<ClipEdit>) -> Option<&[ClipId]> {
    match event {
        EditEvent::Begin { value, .. } => value.as_ref().and_then(moved_by),
        EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } => moved_by(value),
        _ => None,
    }
}

/// The clips `edit` moves, when it is a move.
fn moved_by(edit: &ClipEdit) -> Option<&[ClipId]> {
    match edit {
        ClipEdit::Move { clips, .. } => Some(clips),
        _ => None,
    }
}

/// `event` with the track fields of a move taken from the panel, and the value it carries.
fn with_tracks(event: EditEvent<ClipEdit>, (track_delta, new_track): (i32, bool)) -> (EditEvent<ClipEdit>, Option<ClipEdit>) {
    let fix = |mut edit: ClipEdit| {
        if let ClipEdit::Move { track_delta: delta, new_track: new, .. } = &mut edit {
            (*delta, *new) = (track_delta, new_track);
        }
        edit
    };
    match event {
        EditEvent::Begin { id, value, .. } => {
            let value = value.map(fix);
            (value.clone().map_or_else(|| EditEvent::begin(id), |value| EditEvent::begin_with(id, value)), value)
        }
        EditEvent::Update { id, value, modifiers, .. } => {
            let value = fix(value);
            (EditEvent::update(id, value.clone(), modifiers), Some(value))
        }
        EditEvent::Commit { id, value, changed, modifiers, .. } => {
            let value = fix(value);
            let across = (track_delta != 0 || new_track) && matches!(value, ClipEdit::Move { .. });
            (EditEvent::commit(id, value.clone(), changed || across, modifiers), Some(value))
        }
        other => (other, None),
    }
}

/// The selection a marquee over `hit` gives: it replaces `current`, or with Shift adds to it; an
/// empty marquee clears.
fn marquee_change(current: &[ClipId], hit: Vec<ClipId>, add: bool) -> Option<SelectionChange<ClipId>> {
    if hit.is_empty() {
        return (!add && !current.is_empty()).then(|| SelectionChange { pick: SelectionPick::Clear, row: None, selection: Selection::new() });
    }
    let mut items: Vec<ClipId> = if add { current.to_vec() } else { Vec::new() };
    for clip in hit {
        if !items.contains(&clip) {
            items.push(clip);
        }
    }
    let last = items.last().copied();
    let pick = if add { SelectionPick::Toggle } else { SelectionPick::Replace };
    Some(SelectionChange { pick, row: last, selection: Selection { items, lead: last, anchor: last } })
}

/// The panel's rectangles this pass.
#[derive(Clone, Copy, Debug)]
struct Geo {
    outer: Rect,
    top: Rect,
    /// The headers and the time area below the top strip, above the scroll bar.
    rows: Rect,
    time_area: Rect,
    ruler: Rect,
    bar: Rect,
    caption: Rect,
    caption_divider: Rect,
    upper: Rect,
    divider: Rect,
    lower: Rect,
    mix: Rect,
    time_left: f32,
    /// Where the time area ends; the vertical scroll bars sit right of it.
    content_right: f32,
    /// The room the divider splits between the video and audio sides.
    split_room: f32,
    max_caption: f32,
    max_upper: f32,
    max_lower: f32,
}

/// What each side of the rows needs, in points.
#[derive(Clone, Copy, Debug)]
struct Needs {
    caption: f32,
    upper: f32,
    lower: f32,
    mix: bool,
    /// The video side's share of the room below the captions, when the app keeps one.
    share: Option<f32>,
}

impl Geo {
    /// Every side needs its tracks (video and audio plus one empty track); the captions take what
    /// they need up to half the room, then the divider sits where each side gets what it needs,
    /// else the sides share the room.
    fn new(style: &TimelineStyle, outer: Rect, header_w: f32, needs: Needs) -> Self {
        let Needs { caption: caption_need, upper: upper_need, lower: lower_need, mix, share } = needs;
        let time_left = outer.left() + header_w;
        let content_right = (outer.right() - style.scrollbar_height).max(time_left);
        let top = Rect::from_min_size(outer.min, vec2(outer.width(), style.top_height.min(outer.height())));
        let rows =
            Rect::from_min_max(pos2(outer.left(), top.bottom()), pos2(outer.right(), (outer.bottom() - style.scrollbar_height).max(top.bottom())));
        let time_area = Rect::from_min_max(pos2(time_left, rows.top()), pos2(content_right, rows.bottom()));
        let ruler = Rect::from_min_max(pos2(time_left, (top.bottom() - style.ruler_height).max(top.top())), pos2(content_right, top.bottom()));
        let bar = Rect::from_min_max(pos2(time_left, rows.bottom()), pos2(content_right, outer.bottom()));
        let mix_h = if mix { style.mix_row_height.min(rows.height()) } else { 0.0 };
        let caption_divider_h = if caption_need > 0.0 { style.divider_height } else { 0.0 };
        let room = (rows.height() - style.divider_height - caption_divider_h - mix_h).max(0.0);
        let caption_h = caption_need.min(room / 2.0);
        let room = room - caption_h;
        let split_room = room;
        let upper_h = match share {
            Some(share) => room * share.clamp(DIVIDER_SHARE_MIN, DIVIDER_SHARE_MAX),
            None if upper_need + lower_need <= room => upper_need,
            None => upper_need.min(room / 2.0).max(room - lower_need),
        }
        .clamp(0.0, room);
        let caption = Rect::from_min_size(rows.min, vec2(rows.width(), caption_h));
        let caption_divider = Rect::from_min_size(pos2(rows.left(), caption.bottom()), vec2(rows.width(), caption_divider_h));
        let upper = Rect::from_min_size(pos2(rows.left(), caption_divider.bottom()), vec2(rows.width(), upper_h));
        let divider = Rect::from_min_size(pos2(rows.left(), upper.bottom()), vec2(rows.width(), style.divider_height));
        let lower = Rect::from_min_max(pos2(rows.left(), divider.bottom()), pos2(rows.right(), (rows.bottom() - mix_h).max(divider.bottom())));
        let mix = Rect::from_min_max(pos2(rows.left(), lower.bottom()), rows.max);
        let (max_caption, max_upper, max_lower) =
            ((caption_need - caption.height()).max(0.0), (upper_need - upper.height()).max(0.0), (lower_need - lower.height()).max(0.0));
        Self {
            outer,
            top,
            rows,
            time_area,
            ruler,
            bar,
            caption,
            caption_divider,
            upper,
            divider,
            lower,
            mix,
            time_left,
            content_right,
            split_room,
            max_caption,
            max_upper,
            max_lower,
        }
    }
}

/// The Mix row (FilmCraft `timeline.rs:387-396`): its name over the headers' left, the master
/// volume in dB at the headers' right.
fn paint_mix_row(ui: &Ui, geo: &Geo, tokens: &UiTokens, labels: &TimelineLabels, db: f32) {
    if geo.mix.height() <= 0.0 {
        return;
    }
    let style = TimelineStyle::of(tokens);
    let color = &tokens.color;
    let painter = ui.painter_at(geo.mix);
    painter.rect_filled(geo.mix, CornerRadius::ZERO, color.surface.panel.to_egui());
    let gap = ui.spacing().item_spacing.x;
    let numbers = LocaleNumbers::of(ui);
    let name = role_format(tokens, style.header_text, color.text.normal.to_egui());
    painter.text(
        pos2(geo.mix.left() + style.label_strip + gap, geo.mix.center().y),
        Align2::LEFT_CENTER,
        &labels.mix,
        name.font_id.clone(),
        name.color,
    );
    let value = role_format(tokens, style.header_text, color.accent.text.to_egui());
    painter.text(
        pos2(geo.time_left - gap, geo.mix.center().y),
        Align2::RIGHT_CENTER,
        numbers.format(f64::from(db), 1..=1),
        value.font_id.clone(),
        value.color,
    );
}

/// A row's full width, headers included, up to the scroll bars.
fn row_rect(geo: &Geo, row: &Row) -> Rect {
    Rect::from_min_max(pos2(geo.outer.left(), row.top), pos2(geo.content_right, row.bottom))
}

/// What the wheel and the trackpad did this pass, read from the events themselves: egui turns
/// Cmd or Ctrl and the wheel into a zoom and Shift and the wheel into a sideways scroll before
/// anyone asks, and the Timeline has its own meaning for both (FilmCraft `interact.rs:173`
/// `wheel_input`). The modifiers are those of the events.
#[derive(Clone, Copy, Debug, PartialEq)]
struct WheelInput {
    /// In points; wheel lines and pages are converted.
    delta: egui::Vec2,
    /// A pinch's factor (1 for none).
    pinch: f64,
    command: bool,
    alt: bool,
    shift: bool,
}

/// Points a wheel line and a wheel page scroll (FilmCraft `interact.rs:178-182`).
const WHEEL_LINE_POINTS: f32 = 40.0;
const WHEEL_PAGE_POINTS: f32 = 400.0;

impl WheelInput {
    fn read(ui: &Ui) -> Self {
        ui.input(|i| {
            let mut wheel = Self { delta: egui::Vec2::ZERO, pinch: 1.0, command: false, alt: false, shift: false };
            for event in &i.events {
                match event {
                    egui::Event::MouseWheel { unit, delta, modifiers, .. } => {
                        let points = match unit {
                            egui::MouseWheelUnit::Point => 1.0,
                            egui::MouseWheelUnit::Line => WHEEL_LINE_POINTS,
                            egui::MouseWheelUnit::Page => WHEEL_PAGE_POINTS,
                        };
                        if delta.x.is_finite() && delta.y.is_finite() {
                            wheel.delta += *delta * points;
                        }
                        wheel.command |= modifiers.command || modifiers.mac_cmd || modifiers.ctrl;
                        wheel.alt |= modifiers.alt;
                        wheel.shift |= modifiers.shift;
                    }
                    egui::Event::Zoom(factor) if factor.is_finite() && *factor > 0.0 => wheel.pinch *= f64::from(*factor),
                    _ => {}
                }
            }
            wheel
        })
    }
}

/// Wheel and pinch over the rows, as FilmCraft's (Premiere Pro on macOS): the wheel scrolls the
/// tracks under the pointer, Cmd or Ctrl and the wheel scrolls the timeline sideways, Alt and the
/// wheel zooms about the pointer, and a pinch zooms. A sideways gesture (a trackpad swipe, a tilt
/// wheel, Shift and the wheel) always scrolls sideways. A sideways scroll is not eased.
// filmcraft: crates/ui-egui/src/panels/timeline/interact.rs:217 `wheel_scroll`.
fn wheel(ui: &Ui, geo: &Geo, anim: &mut ViewAnim, view: &mut TimeView, scrolls: &mut Scrolls) {
    let Some(pos) = ui.input(|i| i.pointer.hover_pos()).filter(|pos| geo.rows.contains(*pos) && ui.rect_contains_pointer(geo.rows)) else {
        return;
    };
    let wheel = WheelInput::read(ui);
    let pinched = (wheel.pinch - 1.0).abs() > WHEEL_PINCH_EPSILON;
    let zooms = wheel.alt && wheel.delta.y != 0.0;
    if pinched || zooms {
        let factor = if pinched { wheel.pinch } else { wheel_zoom_factor(wheel.delta.y) };
        if factor.is_finite() {
            let anchor = anim.target.time_at(pos.x.max(geo.time_left), geo.time_left);
            zoom_target(anim, factor, anchor, geo.time_area.width());
        }
    } else if wheel.delta != egui::Vec2::ZERO {
        let sideways = if wheel.delta.x.abs() > wheel.delta.y.abs() {
            Some(wheel.delta.x)
        } else if wheel.command || wheel.shift {
            Some(wheel.delta.y)
        } else {
            None
        };
        match sideways {
            Some(points) => {
                let moved = Tick::from_seconds_f64(f64::from(points) / anim.target.points_per_second);
                anim.target.start = (anim.target.start - moved).max(Tick::ZERO);
                anim.anchor = None;
                view.start = anim.target.start;
            }
            None if pos.y < geo.caption.bottom() => scrolls.caption -= wheel.delta.y,
            None if pos.y < geo.upper.bottom() => scrolls.upper += wheel.delta.y,
            None if pos.y > geo.divider.bottom() => scrolls.lower -= wheel.delta.y,
            None => {}
        }
    }
    if pinched || wheel.delta != egui::Vec2::ZERO {
        ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    }
}

/// What the time area's keys need.
struct Keys {
    id: Id,
    rate: FrameRate,
    playhead: Tick,
    duration: Tick,
    points: Vec<Tick>,
    width: f32,
    fit_margin: f32,
}

impl Keys {
    /// Runs the key pressed this pass: the playhead it asks for, or the menu key; `=`, `-` and `\`
    /// change the view's target.
    fn handle(&self, ui: &Ui, anim: &mut ViewAnim) -> Option<KeyOutcome> {
        ui.memory_mut(|m| m.set_focus_lock_filter(self.id, EventFilter { horizontal_arrows: true, vertical_arrows: true, ..EventFilter::default() }));
        let claims = KEYS.map(|(key, _)| key);
        KeyClaim::claim(ui.ctx(), self.id, &claims);
        let action = ui.input_mut(|i| KEYS.iter().find_map(|(key, action)| key.consume(i).then_some(*action)))?;
        let from = self.rate.frame_at(self.playhead);
        let to = match action {
            KeyAction::Step { forward, five } => {
                let frames = if five { SHIFT_STEP_FRAMES } else { STEP_FRAMES };
                Some(self.rate.tick_of(if forward { from.saturating_add(frames) } else { from.saturating_sub(frames) }))
            }
            KeyAction::Start => Some(Tick::ZERO),
            KeyAction::End => Some(self.duration),
            KeyAction::PreviousEdit => self.points.iter().copied().filter(|p| *p < self.playhead).max(),
            KeyAction::NextEdit => self.points.iter().copied().filter(|p| *p > self.playhead).min(),
            KeyAction::ZoomIn => {
                zoom_target(anim, KEY_ZOOM, self.playhead, self.width);
                None
            }
            KeyAction::ZoomOut => {
                zoom_target(anim, 1.0 / KEY_ZOOM, self.playhead, self.width);
                None
            }
            KeyAction::Fit => {
                anim.target = TimeView::fit(TimeRange::new(Tick::ZERO, self.duration), self.width, self.fit_margin);
                anim.anchor = None;
                None
            }
            KeyAction::ContextMenu => return Some(KeyOutcome::ContextMenu),
        };
        to.map(|to| KeyOutcome::Playhead(to.max(Tick::ZERO)))
    }
}

/// What a key of the time area asked of the panel.
enum KeyOutcome {
    Playhead(Tick),
    ContextMenu,
}

/// What the follow and the reveal need.
struct FollowStep {
    mode: FollowMode,
    playing: bool,
    width: f32,
    playhead: Tick,
    duration: Tick,
}

/// Eases `view` toward its target, follows the playhead while playing, brings it into view after
/// a jump while stopped, and keeps the view inside the sequence's extent.
fn follow_view(ui: &Ui, anim_id: Id, anim: &mut ViewAnim, view: &mut TimeView, step: FollowStep) {
    let FollowStep { mode, playing, width, playhead, duration } = step;
    ease(ui, anim, view);
    let held_id = anim_id.with("follow");
    let mut held = ui.data(|d| d.get_temp::<Follow>(held_id)).unwrap_or_default();
    let mut moved = held.update(mode, playing, view, width, playhead);
    ui.data_mut(|d| d.insert_temp(held_id, held));
    if moved.is_none() && !playing && anim.seen.is_some_and(|seen| seen != playhead) {
        moved = view.scroll_to_reveal(playhead, width);
    }
    if let Some(start) = moved {
        view.start = start;
        anim.target.start = start;
        anim.anchor = None;
    }
    anim.seen = Some(playhead);
    view.start = view.start.clamp(Tick::ZERO, view.max_start(duration, width));
    anim.target.start = anim.target.start.clamp(Tick::ZERO, anim.target.max_start(duration, width));
}

/// The top strip's left part: the large timecode over the toolbar toggles.
struct Head {
    id: Id,
    rect: Rect,
    playhead: Tick,
    format: TimeFormat,
    snap: bool,
    linked_selection: bool,
}

impl Head {
    fn show(self, ui: &mut Ui, labels: &TimelineLabels) -> (Option<EditEvent<Tick>>, TimelineToolbar) {
        let Self { id, rect, playhead, format, snap, linked_selection } = self;
        let mut head = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)));
        head.set_clip_rect(rect.intersect(ui.clip_rect()));
        let timecode = TimecodeField::new(id.with("timecode"), playhead, &format).large(true).show(&mut head);
        let toolbar = head
            .horizontal(|ui| {
                let mut button = |icon: Icon, name: &str, key: &str, on: Option<bool>| {
                    let mut button = IconButton::new(icon, name).size(ControlSize::Sm).with_id(id.with(key));
                    if let Some(on) = on {
                        button = button.selected(on);
                    }
                    ui.add(button)
                };
                TimelineToolbar {
                    snap: button(Icon::Snap, &labels.snap, "snap", Some(snap)),
                    linked_selection: button(Icon::LinkedSelection, &labels.linked_selection, "linked", Some(linked_selection)),
                    add_marker: button(Icon::Flag, &labels.add_marker, "marker", None),
                    settings: button(Icon::Settings, &labels.settings, "settings", None),
                }
            })
            .inner;
        (timecode.event, toolbar)
    }
}

/// What one pass collects.
#[derive(Default)]
struct Pass {
    edits: Vec<TimelineEdit>,
    snapped: Option<Tick>,
    drag: Option<ClipDrag>,
    /// A lane reported a clip event this pass.
    drag_seen: bool,
    marquee: Option<Marquee>,
    row_at_pointer: Option<(TrackId, Tick)>,
}

/// What every row of the pass shares.
struct RowCx<'a> {
    id: Id,
    area_id: Id,
    geo: Geo,
    rows: &'a [Row],
    clips: &'a [Vec<ClipInfo<'a>>],
    zones: EdgeZones,
    track_ids: &'a [TrackId],
    view: &'a TimeView,
    rate: FrameRate,
    tool: TimelineTool,
    no_modifier_trims: bool,
    targets: Option<&'a SnapTargets>,
    selection: &'a [ClipId],
    palette: &'a LabelPalette,
    labels: &'a TimelineLabels,
    mods: Modifiers,
    enabled: bool,
    pointer_y: Option<f32>,
    hover: Option<Pos2>,
}

/// The app's hooks into a row.
struct RowHooks<'h, 'a> {
    extra: Option<&'h mut HeaderExtra<'a>>,
    content: Option<&'h mut ClipPainter<'a>>,
}

impl RowCx<'_> {
    /// A clip's rect on screen in its row.
    fn clip_rect(&self, row: &Row, clip: &ClipInfo<'_>) -> Rect {
        let left = self.geo.time_left;
        Rect::from_min_max(pos2(self.view.x_of(clip.range.start, left), row.top), pos2(self.view.x_of(clip.range.end(), left), row.bottom))
    }

    /// The zero move of the clips a press on this lane's body grabbed, for a `Begin` that carries
    /// no value (the pointer has not moved the clips yet): the app previews it, and the panel
    /// reports the track change of a drag that only crosses rows. `None` when the press is not a
    /// move (an edge, a slip, a slide, a cut or empty space).
    fn zero_move(&self, ui: &Ui, clips: &[ClipInfo<'_>]) -> Option<ClipEdit> {
        if matches!(self.tool, TimelineTool::Razor | TimelineTool::Slip | TimelineTool::Slide) {
            return None;
        }
        let (view, left) = (self.view, self.geo.time_left);
        let origin = ui.input(|i| i.pointer.press_origin())?;
        let ranges: Vec<_> = clips.iter().map(|clip| (clip.id, clip.range)).collect();
        if edge_at(&ranges, view, left, origin.x, &self.zones).is_some() {
            return None;
        }
        let clip = clips.iter().find(|clip| (view.x_of(clip.range.start, left)..view.x_of(clip.range.end(), left)).contains(&origin.x))?.id;
        let moving = if self.selection.contains(&clip) { self.selection.to_vec() } else { vec![clip] };
        Some(ClipEdit::Move { clips: moving, delta: Tick::ZERO, track_delta: 0, new_track: false, copy: false })
    }

    /// Where moving `moving` from `source`'s row heads with the pointer where it is.
    fn derive(&self, source: &Row, moving: &[ClipId]) -> Derived {
        let Some(y) = self.pointer_y else { return Derived::STAY };
        let mut sources = vec![source];
        for row in self.rows {
            if row.index != source.index && self.clips[row.index].iter().any(|clip| moving.contains(&clip.id)) {
                sources.push(row);
            }
        }
        let bounds = if source.side == Side::Caption {
            (self.geo.rows.top(), self.geo.rows.bottom())
        } else {
            (self.geo.upper.top(), self.geo.lower.bottom())
        };
        resolve_move(self.rows, &sources, y, bounds)
    }

    /// One track row: its header, its lane, and what the lane reported. `track` comes with its
    /// resolved height and its clips.
    fn show_row(
        &self,
        child: &mut Ui,
        area: Rect,
        row: &Row,
        (track, height, clips): (&TrackInfo<'_>, f32, &[ClipInfo<'_>]),
        hooks: RowHooks<'_, '_>,
        out: &mut Pass,
    ) {
        let RowHooks { extra, content } = hooks;
        let (id, left) = (self.id, self.geo.time_left);
        let full = row_rect(&self.geo, row);
        let header_rect = Rect::from_min_max(full.min, pos2(left, full.bottom()));
        let lane_rect = Rect::from_min_max(pos2(left, full.top()), full.max);
        let node = child.interact(full.intersect(area), id.with(("row", track.id.0)), Sense::HOVER);
        node.widget_info(|| WidgetInfo::labeled(Role::Row, self.enabled, track.name));

        let header = TrackHeader::new((id, "header", track.id.0), track)
            .height(height)
            .above_divider(row.side == Side::Upper)
            .top_edge(row.top_video)
            .palette(self.palette)
            .labels(self.labels)
            .show(child, header_rect);
        out.edits.extend(header.edits);
        if let Some(extra) = extra {
            let mut slot = child.new_child(UiBuilder::new().max_rect(header_rect).layout(Layout::right_to_left(Align::Center)));
            slot.set_clip_rect(header_rect.intersect(child.clip_rect()));
            extra(&mut slot, track);
        }

        let locked = track.locked.unwrap_or(false);
        let mut lane = ClipLane::new((id, "lane", track.id.0), track.id, clips, self.view, self.rate)
            .tool(self.tool)
            .no_modifier_trims(self.no_modifier_trims)
            .snap(self.targets)
            .locked(locked)
            .selection(self.selection)
            .palette(self.palette)
            .track_label(track.label);
        if let Some(paint) = content {
            lane = lane.content(move |painter, rect, clip| paint(painter, rect, clip));
        }
        let lane = lane.show(child, lane_rect);
        if lane.response.drag_started() || lane.response.clicked() {
            child.memory_mut(|m| m.request_focus(self.area_id));
        }
        out.snapped = lane.snapped_at.or(out.snapped);
        if let Some(event) = lane.edit {
            out.drag_seen = true;
            let event = match event {
                EditEvent::Begin { id, value: None, .. } => {
                    self.zero_move(child, clips).map_or(EditEvent::begin(id), |zero| EditEvent::begin_with(id, zero))
                }
                other => other,
            };
            let moving = moved_clips(&event).or_else(|| out.drag.as_ref().and_then(|drag| moved_by(&drag.value))).unwrap_or_default().to_vec();
            let derived = self.derive(row, &moving);
            if derived.refused {
                child.ctx().set_cursor_icon(CursorIcon::NotAllowed);
            }
            let (event, value) = with_tracks(event, (derived.track_delta, derived.new_track));
            out.drag = match (&event, value) {
                (EditEvent::Cancel { .. } | EditEvent::Commit { .. }, _) => None,
                (_, Some(value)) => event.id().map(|id| ClipDrag { id, value, source: track.id }),
                (_, None) => out.drag.clone(),
            };
            out.edits.push(TimelineEdit::Clip(event));
        }
        if let Some((change, unlinked)) = lane.select {
            out.edits.push(TimelineEdit::Select { change, unlinked });
        }
        if let Some(clip) = lane.double_clicked {
            out.edits.push(TimelineEdit::OpenClip(clip));
        }
        if lane.response.secondary_clicked() {
            let anchor =
                lane.context_menu.and_then(|clip| clips.iter().find(|info| info.id == clip)).map_or(lane_rect, |info| self.clip_rect(row, info));
            let pointer = lane.response.interact_pointer_pos();
            out.edits.push(TimelineEdit::ContextMenu { track: Some(track.id), clip: lane.context_menu, anchor, pointer });
        }
        if self.tool != TimelineTool::Razor
            && !locked
            && lane.response.drag_started_by(PointerButton::Primary)
            && let Some(origin) = child.input(|i| i.pointer.press_origin())
            && !clips.iter().any(|clip| self.view.x_of(clip.range.start, left) <= origin.x && origin.x < self.view.x_of(clip.range.end(), left))
        {
            out.marquee = Some(Marquee { origin });
        }
        if let Some(pos) = self.hover.filter(|pos| lane_rect.intersect(area).contains(*pos)) {
            out.row_at_pointer = Some((track.id, self.view.time_at(pos.x, left)));
        }
    }
}

impl Pass {
    /// A drag that only moved across rows changes no lane value, so the panel reports its update
    /// itself; the drag state is kept for the next pass.
    fn settle_drag(&mut self, ui: &Ui, cx: &RowCx<'_>, key: Id) {
        if let Some(active) = self.drag.clone().filter(|_| !self.drag_seen)
            && let Some(row) = cx.rows.iter().find(|row| cx.track_ids[row.index] == active.source)
            && cx.pointer_y.is_some()
        {
            let moving = moved_by(&active.value).unwrap_or_default();
            let derived = cx.derive(row, moving);
            if derived.refused {
                ui.ctx().set_cursor_icon(CursorIcon::NotAllowed);
            }
            let derived = (derived.track_delta, derived.new_track);
            let mut value = active.value.clone();
            if let ClipEdit::Move { track_delta, new_track, .. } = &mut value
                && (*track_delta, *new_track) != derived
            {
                (*track_delta, *new_track) = derived;
                self.edits.push(TimelineEdit::Clip(EditEvent::update(active.id, value.clone(), cx.mods)));
                self.drag = Some(ClipDrag { value, ..active });
            }
        }
        ui.data_mut(|d| match self.drag.take() {
            Some(active) => {
                d.insert_temp(key, active);
            }
            None => {
                d.remove::<ClipDrag>(key);
            }
        });
    }

    /// Draws the marquee while the button is down and reports the selection when it is released.
    fn marquee_pass<'c, 'a: 'c>(&mut self, ui: &Ui, cx: &RowCx<'_>, key: Id, lists: impl Iterator<Item = (TrackId, &'c [ClipInfo<'a>])>) {
        let Some(active) = self.marquee else {
            ui.data_mut(|d| d.remove::<Marquee>(key));
            return;
        };
        if ui.input_mut(|i| ClaimedKey::exact(Modifiers::NONE, Key::Escape).consume(i)) {
            ui.data_mut(|d| d.remove::<Marquee>(key));
            return;
        }
        let tokens = Tokens::of(ui);
        let color = &tokens.color;
        let now = ui.input(|i| i.pointer.latest_pos()).unwrap_or(active.origin);
        if ui.input(|i| i.pointer.primary_down()) {
            let band = Rect::from_two_pos(active.origin, now).intersect(cx.geo.time_area);
            let paint = ui.painter_at(cx.geo.time_area);
            paint.rect_filled(band, CornerRadius::ZERO, color.state.selected.to_egui());
            let stroke = Stroke::new(craft_theme::Stroke::Hairline.width(ui.pixels_per_point()), color.overlay.marquee.to_egui());
            paint.rect_stroke(band, CornerRadius::ZERO, stroke, StrokeKind::Inside);
            ui.data_mut(|d| d.insert_temp(key, active));
            return;
        }
        let spans: Vec<(ClipId, TrackId, TimeRange)> =
            lists.flat_map(|(track, list)| list.iter().map(move |clip| (clip.id, track, clip.range))).collect();
        let row_spans: Vec<(TrackId, f32, f32)> = cx.rows.iter().map(|row| (cx.track_ids[row.index], row.top, row.bottom)).collect();
        let hit = marquee_clips(&spans, &row_spans, cx.view, cx.geo.time_left, active.origin.into(), now.into());
        if let Some(change) = marquee_change(cx.selection, hit, cx.mods.shift) {
            self.edits.push(TimelineEdit::Select { change, unlinked: false });
        }
        ui.data_mut(|d| d.remove::<Marquee>(key));
    }
}

/// The headers' right edge: drag to resize all headers. Reports the new width as one gesture.
fn resize_headers(ui: &Ui, geo: &Geo, header_w: f32, id: Id, labels: &TimelineLabels) -> Option<TimelineEdit> {
    let tokens = Tokens::of(ui);
    let style = TimelineStyle::of(&tokens);
    let mods = ui.input(|i| i.modifiers);
    let grab = Rect::from_min_max(
        pos2(geo.time_left - style.edge_zone / 2.0, geo.rows.top()),
        pos2(geo.time_left + style.edge_zone / 2.0, geo.rows.bottom()),
    );
    let edge = ui.interact(grab, id.with("header width"), Sense::DRAG);
    edge.widget_info(|| WidgetInfo::labeled(Role::Splitter, ui.is_enabled(), &labels.header_width));
    if edge.hovered() || edge.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    let mut resize = Gesture::load(ui, edge.id, &header_w, EditPolicy::default()).enabled(ui.is_enabled());
    if edge.drag_started_by(PointerButton::Primary) {
        resize.begin();
    }
    if resize.is_active()
        && edge.dragged()
        && let (Some(origin), Some(pos)) = (ui.input(|i| i.pointer.press_origin()), ui.input(|i| i.pointer.latest_pos()))
    {
        let wanted = *resize.start().unwrap_or(&header_w) + pos.x - origin.x;
        resize.update(wanted.clamp(style.header_width_min, style.header_width_max), mods);
    }
    if edge.drag_stopped() {
        resize.commit();
    }
    resize.finish().map(TimelineEdit::HeaderWidth)
}

/// One side's vertical scroll bar, a kernel scroll area over `strip` (right of the time area, as
/// tall as the side) whose content is as tall as the side's tracks need. Only the bar scrolls it:
/// the panel's wheel handling owns the wheel. `offset` is how far the side is scrolled in points
/// (`inverted` for the video side, whose tracks stack upward, so its bar's top is the most
/// scrolled); the new offset comes back in the same pass. FilmCraft `timeline.rs:435`,
/// `timeline/scrollbars.rs:116` `vertical_scrollbar`.
fn side_scroll_bar(ui: &mut Ui, salt: impl AsIdSalt, strip: Rect, need: f32, offset: f32, inverted: bool, name: &str) -> f32 {
    let max = (need - strip.height()).max(0.0);
    if max <= 0.0 || strip.width() <= 0.0 {
        return offset;
    }
    let shown = if inverted { max - offset } else { offset };
    let mut child = ui.new_child(UiBuilder::new().max_rect(strip));
    let out = ScrollArea::vertical()
        .id_salt(salt)
        .scroll_source(ScrollSource::SCROLL_BAR)
        .vertical_scroll_offset(shown)
        .vertical_scroll_bar_label(name)
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            ui.allocate_space(vec2(0.0, need));
        });
    let moved = out.state.clamped_offset().y.clamp(0.0, max);
    if inverted { max - moved } else { moved }
}

/// The divider between the video and audio tracks (right of the headers): drag to share the room between them. Reports
/// the video side's share as one gesture (FilmCraft `timeline.rs:380-385`).
fn resize_divider(ui: &Ui, geo: &Geo, id: Id, labels: &TimelineLabels) -> Option<TimelineEdit> {
    if geo.split_room <= 0.0 {
        return None;
    }
    let mods = ui.input(|i| i.modifiers);
    let enabled = ui.is_enabled();
    let share = geo.upper.height() / geo.split_room;
    // Over the time area only: the headers' bottom edges resize their rows there.
    let grab = geo.divider.with_min_x(geo.time_left);
    let edge = ui.interact(grab, id.with("divider"), Sense::DRAG);
    edge.widget_info(|| WidgetInfo::labeled(Role::Splitter, enabled, &labels.divider));
    if edge.hovered() || edge.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
    }
    let mut resize = Gesture::load(ui, edge.id, &share, EditPolicy::default()).enabled(enabled);
    if edge.drag_started_by(PointerButton::Primary) {
        resize.begin();
    }
    if resize.is_active()
        && edge.dragged()
        && let (Some(origin), Some(pos)) = (ui.input(|i| i.pointer.press_origin()), ui.input(|i| i.pointer.latest_pos()))
    {
        let wanted = *resize.start().unwrap_or(&share) + (pos.y - origin.y) / geo.split_room;
        resize.update(wanted.clamp(DIVIDER_SHARE_MIN, DIVIDER_SHARE_MAX), mods);
    }
    if edge.drag_stopped() {
        resize.commit();
    }
    resize.finish().map(TimelineEdit::Divider)
}

/// The context menu the Menu key or Shift+F10 opens: on the selection's last clip when it is in
/// view, else on the first targeted track (the first track without targets), anchored at the
/// clip's or the row's rect.
fn menu_key_edit(cx: &RowCx<'_>, selection: &[ClipId], tracks: &[TrackInfo<'_>]) -> Option<TimelineEdit> {
    let focused = selection
        .last()
        .and_then(|&clip| cx.rows.iter().find_map(|row| cx.clips[row.index].iter().find(|info| info.id == clip).map(|info| (row, Some(info)))));
    let (row, clip) = focused.or_else(|| {
        let targeted = cx.rows.iter().find(|row| tracks[row.index].targeted == Some(true));
        targeted.or_else(|| cx.rows.first()).map(|row| (row, None))
    })?;
    let anchor = match clip {
        Some(info) => cx.clip_rect(row, info),
        None => Rect::from_min_max(pos2(cx.geo.time_left, row.top), pos2(cx.geo.content_right, row.bottom)),
    };
    Some(TimelineEdit::ContextMenu { track: Some(tracks[row.index].id), clip: clip.map(|info| info.id), anchor, pointer: None })
}
