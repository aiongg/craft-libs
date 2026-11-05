//! The clips of one track row, with Premiere's trims and moves (N1.05).

use craft_ui::behavior::{ListSelection, RowInfo, Selection, SelectionChange};
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use egui::accesskit::Role;
use egui::{
    Align2, AsIdSalt, Color32, CursorIcon, IdSalt, Modifiers, Painter, PointerButton, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2,
};

use super::{LabelPalette, TimelineStyle};
use crate::clips::{ClipEdge, ClipEdit, EdgeZones, SnapTargets, TrimKind, edge_at, snap_time};
use crate::markers::MarkerColor;
use crate::model::{ClipId, ClipInfo, TrackId};
use crate::rates::FrameRate;
use crate::time::{Tick, TimeView};
use crate::timecode::{TimeFormat, format_time};

/// How much darker a disabled clip's label colour is drawn (FilmCraft `clips.rs` halves each
/// channel).
const DISABLED_DIVISOR: u8 = 2;
/// How strongly a track's colour label tints its lane (FilmCraft `timeline.rs:255`
/// `TRACK_LANE_TINT`, used at `:335`).
const LANE_LABEL_TINT: f32 = 0.10;
/// The lane's tint by its track's colour label, under the clips.
fn paint_lane_tint(painter: &Painter, rect: Rect, palette: &LabelPalette, label: Option<MarkerColor>) {
    if let Some(label) = label {
        painter.rect_filled(rect, 0.0, palette.color(label).gamma_multiply(LANE_LABEL_TINT));
    }
}
/// The locked hatch's line spacing, as a fraction of the caption line height.
const HATCH_STEP_DIVISOR: f32 = 4.0;

/// The timeline tool that decides what a press on a clip does (Premiere's Tools panel; the app
/// keeps the current one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimelineTool {
    /// Select, move, and trim at edges ([`selection_trim`](crate::clips::selection_trim)).
    #[default]
    Selection,
    /// Ripple Edit: trims move later clips along.
    Ripple,
    /// Rolling Edit: moves the cut between two touching clips.
    Rolling,
    /// Rate Stretch: changes a clip's speed to fill its new length.
    RateStretch,
    /// Razor: cuts a clip at the pointer.
    Razor,
    /// Slip: moves a clip's media inside the clip.
    Slip,
    /// Slide: moves a clip between its neighbours.
    Slide,
}

/// How a video clip's thumbnails are laid out (Premiere's Video Thumbnails display setting).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThumbnailMode {
    /// None.
    Off,
    /// One at the head.
    Head,
    /// One at the head and one at the tail.
    #[default]
    HeadAndTail,
    /// Tiles along the clip.
    Continuous,
}

/// At most this many thumbnail tiles per clip (FilmCraft `clips.rs:128`).
pub const MAX_THUMBNAIL_TILES: usize = 256;

/// Where the thumbnails of a clip from `x0` to `x1` go, `tile_width` wide, and the clip time each
/// shows: `(x, time)`. Only tiles within `visible` (an x range) are listed; `Continuous` tiles the
/// clip edge to edge, capped at [`MAX_THUMBNAIL_TILES`]; `frame` is one frame's duration (the
/// tail shows the last frame).
// filmcraft: crates/ui-egui/src/panels/timeline/clips.rs:135 `thumbnail_tiles`.
pub fn thumbnail_tiles(mode: ThumbnailMode, x0: f32, x1: f32, tile_width: f32, visible: (f32, f32), duration: Tick, frame: Tick) -> Vec<(f32, Tick)> {
    let width = x1 - x0;
    if !(x0.is_finite() && width.is_finite() && width > 0.0 && tile_width.is_finite() && tile_width >= 1.0) {
        return Vec::new();
    }
    let last = (duration - frame).max(Tick::ZERO);
    let offset_at = |x: f32| Tick(((x - x0) as f64 / width as f64 * duration.0 as f64).round() as i64).clamp(Tick::ZERO, last);
    let shown = |x: f32| x < visible.1 && x + tile_width > visible.0;
    let head = (x0, Tick::ZERO);
    let tiles = match mode {
        ThumbnailMode::Off => Vec::new(),
        ThumbnailMode::Head => vec![head],
        ThumbnailMode::HeadAndTail if width >= 2.0 * tile_width => vec![head, (x1 - tile_width, last)],
        ThumbnailMode::HeadAndTail => vec![head],
        ThumbnailMode::Continuous => {
            // A grid anchored at the clip's head: scrolling reveals tiles without changing the
            // others' frames.
            let first = ((visible.0 - x0).max(0.0) / tile_width).floor() as usize;
            let end = ((visible.1.min(x1) - x0) / tile_width).ceil().max(1.0) as usize;
            (first..end).take(MAX_THUMBNAIL_TILES).map(|k| x0 + k as f32 * tile_width).map(|x| (x, offset_at(x))).collect()
        }
    };
    tiles.into_iter().filter(|(x, _)| shown(*x)).collect()
}

/// Paints a clip's content (thumbnails, waveform, fades) into its body rect.
pub type ClipPainter<'a> = Box<dyn FnMut(&Painter, Rect, &ClipInfo<'_>) + 'a>;

/// The clips of one track row.
///
/// - Draws each [`ClipInfo`] as a block: label colour (from [`LabelPalette`]), name, a
///   selected outline, dimmed when disabled; the app paints the content (thumbnails, waveform,
///   fades) through [`content`](Self::content) before the outline.
/// - A press with the [`TimelineTool`] on an edge ([`edge_at`] with the
///   `timeline.edge_zone` and `timeline.roll_zone` sizes of [`TimelineStyle`]) starts a trim of the kind the tool and modifiers give; on a body it moves (Selection),
///   slips or slides; the Razor cuts at the pointer (snapped to `rate`'s frames). The drag is
///   reported as an `EditEvent<ClipEdit>` gesture; moves snap to `snap` targets within
///   `timeline.snap_distance` ([`snap_time`]). Esc during the
///   drag cancels it, and the release after an Esc is ignored (FilmCraft #580).
/// - A press acts on what it pressed, worked out on the press frame, not when egui's drag
///   threshold passes (FilmCraft `Pressed`, #259).
/// - A click picks a clip, or clears on empty space, over the [`selection`](Self::selection):
///   a click replaces, Shift toggles, Cmd/Ctrl toggles without linked partners (Premiere's),
///   reported as a [`SelectionChange`]; a double-click and a secondary click are reported.
/// - A locked track draws stripes over its clips and starts no gesture.
/// - Hover cursors are egui's system cursors, as FilmCraft's (`timeline/interact.rs:291-299`):
///   `ResizeColumn` on a trim, ripple, roll or rate-stretch edge, `ResizeHorizontal` on a clip
///   under Slip or Slide, `Crosshair` under the Razor. They are the kernel's `ToolCursor::System`
///   form. Premiere's own bracket and bar cursors are bitmaps; the icon sets have no glyphs for
///   them, so the lane doesn't draw them.
/// - Keyboard: none here; the lane is not a Tab stop. The panel's time area holds the focus
///   (playhead keys); clips are selected with the pointer and Cmd/Ctrl+A, as in Premiere.
/// - AccessKit: each visible clip a `Button` (named by its name, its start and duration as the
///   value, `selected`), with a Click action that selects it.
pub struct ClipLane<'a> {
    id_salt: IdSalt,
    track: TrackId,
    clips: &'a [ClipInfo<'a>],
    view: &'a TimeView,
    rate: FrameRate,
    tool: TimelineTool,
    no_modifier_trims: bool,
    snap: Option<&'a SnapTargets>,
    locked: bool,
    selection: &'a [ClipId],
    palette: Option<&'a LabelPalette>,
    track_label: Option<MarkerColor>,
    content: Option<ClipPainter<'a>>,
}

impl<'a> ClipLane<'a> {
    /// A lane for `track`'s `clips` (those that overlap the visible time) on `view`; times snap to the frames of `rate`.
    pub fn new(id_salt: impl AsIdSalt, track: TrackId, clips: &'a [ClipInfo<'a>], view: &'a TimeView, rate: FrameRate) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            track,
            clips,
            view,
            rate,
            tool: TimelineTool::Selection,
            no_modifier_trims: false,
            snap: None,
            locked: false,
            selection: &[],
            palette: None,
            track_label: None,
            content: None,
        }
    }

    /// The tool that decides what a press does; [`TimelineTool::Selection`] by default.
    pub fn tool(mut self, tool: TimelineTool) -> Self {
        self.tool = tool;
        self
    }

    /// Premiere's Trim setting "Allow Selection tool to choose Roll and Ripple trims without
    /// modifier key".
    pub fn no_modifier_trims(mut self, on: bool) -> Self {
        self.no_modifier_trims = on;
        self
    }

    /// The snap targets for drags; `None` turns snapping off.
    pub fn snap(mut self, targets: Option<&'a SnapTargets>) -> Self {
        self.snap = targets;
        self
    }

    /// Whether the track is locked: clips draw hatched and ignore presses.
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// The selected clips (the app's, [`TimelineModel::selection`](crate::TimelineModel::selection));
    /// none selected without it.
    pub fn selection(mut self, selection: &'a [ClipId]) -> Self {
        self.selection = selection;
        self
    }

    /// The palette that gives label colours; the default palette when not set.
    pub fn palette(mut self, palette: &'a LabelPalette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// The track's colour label: it tints the lane by `LANE_LABEL_TINT` of the
    /// [`palette`](Self::palette)'s colour (FilmCraft `timeline.rs:335`, #1186). The panel passes
    /// [`TrackInfo::label`](crate::TrackInfo::label).
    pub fn track_label(mut self, label: Option<MarkerColor>) -> Self {
        self.track_label = label;
        self
    }

    /// Paints a clip's content into its body rect (clipped to the visible part).
    pub fn content(mut self, paint: impl FnMut(&Painter, Rect, &ClipInfo<'_>) + 'a) -> Self {
        self.content = Some(Box::new(paint));
        self
    }

    /// Draws into `rect` (the row's time area; `rect.left()` is the view's left edge).
    // filmcraft: crates/ui-egui/src/panels/timeline.rs:564 `draw_clips`,
    // `timeline/clips.rs:183` `draw_clip`, `timeline/interact.rs:355` `start_drag`, `:558`
    // `drag_update`, `:713` `commit_drag`, `:526` `escape_cancel`, `:260` `hover_cursor`;
    // `timeline_hit.rs:283` `grab_at`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> ClipLaneResponse {
        let Self { id_salt, track, clips, view, rate, tool, no_modifier_trims, snap, locked, selection, palette, track_label, mut content } = self;
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let lane = Lane {
            clips,
            view,
            left: rect.left(),
            rate,
            zones: EdgeZones { edge: style.edge_zone, roll: style.roll_zone },
            no_modifier_trims,
            tool,
            snap,
            tolerance: style.snap_distance,
        };
        let id = ui.make_persistent_id(id_salt);
        let enabled = ui.is_enabled() && !locked;
        let format = TimeFormat::timecode(rate);

        // One reader-facing button per visible clip, below the lane's own widget: the pointer
        // goes to the lane, a reader's Click action reaches the clip.
        let mut reader_click = None;
        for clip in clips {
            let Some(body) = lane.body(clip, rect) else { continue };
            let node = ui.interact(body.intersect(rect), id.with((track.0, clip.id.0)), Sense::CLICK);
            let selected = selection.contains(&clip.id);
            node.widget_info(|| WidgetInfo {
                current_text_value: Some(format!("{} {}", format_time(clip.range.start, &format), format_time(clip.range.duration, &format))),
                ..WidgetInfo::selected(Role::Button, enabled, selected, clip.name)
            });
            if enabled && node.clicked() {
                reader_click = Some(clip.id);
            }
        }

        let sense = if enabled { Sense::CLICK | Sense::DRAG } else { Sense::HOVER };
        let response = ui.interact(rect, id, sense);
        let mods = ui.input(|i| i.modifiers);
        let pointer = response.interact_pointer_pos().or_else(|| response.hover_pos());

        if ui.is_rect_visible(rect) {
            let palette_owned;
            let palette = match palette {
                Some(palette) => palette,
                None => {
                    palette_owned = LabelPalette::default();
                    &palette_owned
                }
            };
            let painter = ui.painter_at(rect);
            paint_lane_tint(&painter, rect, palette, track_label);
            for clip in clips {
                if let Some(body) = lane.body(clip, rect) {
                    paint_clip(ui, &painter, &tokens, body, clip, palette, selection.contains(&clip.id), locked, content.as_mut());
                }
            }
        }

        let mut out =
            ClipLaneResponse { response, edit: None, select: None, double_clicked: None, context_menu: None, snapped_at: None, grabbed: None };
        if let Some(clip) = reader_click {
            out.select = lane.pick(selection, Some(clip), Modifiers::NONE);
        }
        if !enabled {
            ui.data_mut(|d| d.remove::<Pressed>(id));
            return out;
        }

        let primary_down = ui.input(|i| i.pointer.primary_down());
        let mut pressed = ui.data(|d| d.get_temp::<Pressed>(id));
        if let Some(pos) = pointer
            && out.response.contains_pointer()
            && ui.input(|i| i.pointer.primary_pressed())
            && tool != TimelineTool::Razor
        {
            let grab = lane.grab(pos.x, mods);
            let moving = match grab {
                Grab::Body(clip) if selection.contains(&clip) => selection.to_vec(),
                Grab::Body(clip) => vec![clip],
                _ => Vec::new(),
            };
            pressed = Some(Pressed { grab, from: view.time_at(pos.x, lane.left), moving, linked: !mods.alt, cancelled: false });
        }

        // The Razor cuts on the press, snapped to a frame.
        if tool == TimelineTool::Razor
            && (out.response.clicked() || out.response.drag_started_by(PointerButton::Primary))
            && let Some(pos) = pointer
            && let Grab::Body(clip) = lane.body_at(pos.x)
        {
            let (at, snapped) = lane.snapped(rate.snap_nearest(view.time_at(pos.x, lane.left)));
            out.snapped_at = snapped;
            out.edit = Some(EditEvent::set(ClipEdit::Cut { clip, at }, mods));
        }

        if let Some(pressed) = pressed.as_mut()
            && let Some(rest) = lane.rest(pressed)
        {
            let mut gesture = Gesture::load(ui, id, &rest, EditPolicy::default());
            if !pressed.cancelled && out.response.drag_started_by(PointerButton::Primary) {
                gesture.begin_with_value();
                out.grabbed = moved_by(&rest);
                if let Grab::Body(clip) = pressed.grab
                    && !selection.contains(&clip)
                {
                    out.select = lane.pick(selection, Some(clip), Modifiers::NONE);
                }
            }
            if !pressed.cancelled
                && gesture.is_active()
                && out.response.dragged_by(PointerButton::Primary)
                && let Some(pos) = pointer
                && let Some((edit, snapped)) = lane.edit(pressed, pos.x, mods)
            {
                gesture.update(edit, mods);
                out.snapped_at = snapped;
            }
            if out.response.drag_stopped() {
                gesture.commit();
            }
            out.edit = gesture.finish();
            if matches!(out.edit, Some(EditEvent::Cancel { .. })) {
                pressed.cancelled = true;
            }
        }

        if out.response.clicked()
            && !pressed.as_ref().is_some_and(|p| p.cancelled)
            && tool != TimelineTool::Razor
            && let Some(pos) = pointer
        {
            let target = pressed.as_ref().map_or_else(|| lane.body_at(pos.x), |p| p.grab);
            let clip = match target {
                Grab::Body(clip) | Grab::Edge { clip, .. } => Some(clip),
                Grab::Empty => None,
            };
            out.select = lane.pick(selection, clip, mods);
        }
        if let Some(pos) = pointer {
            if out.response.double_clicked() {
                out.double_clicked = lane.clip_at(pos.x);
            }
            if out.response.secondary_clicked() {
                out.context_menu = lane.clip_at(pos.x);
            }
        }

        let hovered = match &pressed {
            Some(pressed) => Some(pressed.grab),
            None => pointer.filter(|_| out.response.contains_pointer()).map(|pos| lane.grab(pos.x, mods)),
        };
        if let Some(cursor) = hovered.and_then(|grab| lane.cursor(grab)) {
            ui.ctx().set_cursor_icon(cursor);
        }

        match pressed {
            Some(pressed) if primary_down => {
                ui.data_mut(|d| d.insert_temp(id, pressed));
            }
            _ => {
                ui.data_mut(|d| d.remove::<Pressed>(id));
            }
        }
        out
    }
}

/// What a clip lane did this pass.
pub struct ClipLaneResponse {
    /// The lane's response, for hover and focus.
    pub response: Response,
    /// The drag the lane reports, one update per frame and a commit on release.
    pub edit: Option<EditEvent<ClipEdit>>,
    /// A click's selection change and whether Cmd/Ctrl left the linked partners out. The panel
    /// reports it as [`TimelineEdit::Select`](super::TimelineEdit::Select).
    pub select: Option<(SelectionChange<ClipId>, bool)>,
    /// The clip that was double-clicked.
    pub double_clicked: Option<ClipId>,
    /// A secondary click: the app opens its clip menu.
    pub context_menu: Option<ClipId>,
    /// Where a drag snapped this pass.
    pub snapped_at: Option<Tick>,
    /// On the pass a drag began that moves clips (a body grab with a move tool): the clips the
    /// move carries, the selection when it holds the grabbed clip, else that clip. The `Begin`
    /// carries no value when the pointer has not moved the clips yet, so the panel builds a zero
    /// [`ClipEdit::Move`] of this set for the preview.
    pub grabbed: Option<Vec<ClipId>>,
}

/// The clips `edit` moves, when it is a move.
fn moved_by(edit: &ClipEdit) -> Option<Vec<ClipId>> {
    match edit {
        ClipEdit::Move { clips, .. } => Some(clips.clone()),
        _ => None,
    }
}

/// What a press grabbed, worked out on the press frame (FilmCraft `Pressed`, #259).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Grab {
    /// A clip edge and the trim it starts; `partner` is the clip touching the cut on the other
    /// side.
    Edge {
        clip: ClipId,
        edge: ClipEdge,
        kind: TrimKind,
        partner: Option<ClipId>,
    },
    Body(ClipId),
    Empty,
}

/// A primary press under way: kept from the press to the release.
#[derive(Clone, Debug)]
struct Pressed {
    grab: Grab,
    /// The time under the pointer at the press.
    from: Tick,
    /// The clips a body drag moves: the selection when it holds the pressed clip, else the clip.
    moving: Vec<ClipId>,
    /// Linked partners trim along (false when Alt was held at the press).
    linked: bool,
    /// Esc cancelled the gesture: the rest of the drag and its release do nothing.
    cancelled: bool,
}

/// The lane's clips and the numbers its rules read.
struct Lane<'a> {
    clips: &'a [ClipInfo<'a>],
    view: &'a TimeView,
    left: f32,
    rate: FrameRate,
    zones: EdgeZones,
    no_modifier_trims: bool,
    tool: TimelineTool,
    snap: Option<&'a SnapTargets>,
    tolerance: f32,
}

impl Lane<'_> {
    fn info(&self, id: ClipId) -> Option<&ClipInfo<'_>> {
        self.clips.iter().find(|clip| clip.id == id)
    }

    /// A clip's block: the lane's full height from its start to its end, `None` outside `rect`.
    fn body(&self, clip: &ClipInfo<'_>, rect: Rect) -> Option<Rect> {
        let (x0, x1) = (self.view.x_of(clip.range.start, self.left), self.view.x_of(clip.range.end(), self.left));
        (x1 >= rect.left() && x0 <= rect.right()).then(|| Rect::from_min_max(pos2(x0, rect.top()), pos2(x1, rect.bottom())))
    }

    fn clip_at(&self, x: f32) -> Option<ClipId> {
        self.clips
            .iter()
            .find(|clip| (self.view.x_of(clip.range.start, self.left)..self.view.x_of(clip.range.end(), self.left)).contains(&x))
            .map(|clip| clip.id)
    }

    /// The clip body under `x`, whatever the tool.
    fn body_at(&self, x: f32) -> Grab {
        self.clip_at(x).map_or(Grab::Empty, Grab::Body)
    }

    /// What a press of the tool at `x` grabs (FilmCraft `timeline_hit.rs:283` `grab_at`).
    fn grab(&self, x: f32, mods: Modifiers) -> Grab {
        let ranges: Vec<_> = self.clips.iter().map(|clip| (clip.id, clip.range)).collect();
        let Some(hit) = edge_at(&ranges, self.view, self.left, x, &self.zones) else { return self.body_at(x) };
        let Some(info) = self.info(hit.clip) else { return self.body_at(x) };
        let cut = match hit.edge {
            ClipEdge::In => info.range.start,
            ClipEdge::Out => info.range.end(),
        };
        let partner = self
            .clips
            .iter()
            .find(|other| {
                other.id != hit.clip
                    && match hit.edge {
                        ClipEdge::In => other.range.end() == cut,
                        ClipEdge::Out => other.range.start == cut,
                    }
            })
            .map(|other| other.id);
        let kind = match self.tool {
            TimelineTool::Selection => {
                crate::clips::selection_trim(mods.command, mods.shift, self.no_modifier_trims, hit.distance, partner.is_some(), &self.zones)
            }
            TimelineTool::Ripple => TrimKind::Ripple,
            TimelineTool::Rolling => TrimKind::Roll,
            TimelineTool::RateStretch => TrimKind::Stretch,
            TimelineTool::Razor | TimelineTool::Slip | TimelineTool::Slide => return Grab::Body(hit.clip),
        };
        // Nothing to roll against: a regular trim.
        let kind = if kind == TrimKind::Roll && partner.is_none() { TrimKind::Trim } else { kind };
        Grab::Edge { clip: hit.clip, edge: hit.edge, kind, partner }
    }

    fn cursor(&self, grab: Grab) -> Option<CursorIcon> {
        match (grab, self.tool) {
            (Grab::Edge { .. }, _) => Some(CursorIcon::ResizeColumn),
            (Grab::Body(_), TimelineTool::Razor) => Some(CursorIcon::Crosshair),
            (Grab::Body(_), TimelineTool::Slip | TimelineTool::Slide) => Some(CursorIcon::ResizeHorizontal),
            _ => None,
        }
    }

    fn snapped(&self, t: Tick) -> (Tick, Option<Tick>) {
        match self.snap {
            Some(targets) => snap_time(targets, t, self.view, self.tolerance),
            None => (t, None),
        }
    }

    /// The selection change of a click on `clip` (none: empty space) and whether the linked
    /// partners stay out: Shift and Cmd/Ctrl toggle, Cmd/Ctrl without partners, else replace.
    fn pick(&self, selection: &[ClipId], clip: Option<ClipId>, mods: Modifiers) -> Option<(SelectionChange<ClipId>, bool)> {
        let last = selection.last().copied();
        let current = Selection { items: selection.to_vec(), lead: last, anchor: last };
        let rules = ListSelection::default();
        let Some(clip) = clip else { return rules.click_empty(&current).map(|change| (change, false)) };
        let order: Vec<RowInfo<ClipId>> = self.clips.iter().map(|info| RowInfo::leaf(info.id)).collect();
        let at = order.iter().position(|row| row.key == clip)?;
        let toggle = Modifiers { command: mods.shift || mods.command, ..Modifiers::NONE };
        rules.click(&current, order.as_slice(), at, toggle).map(|change| (change, mods.command))
    }

    /// The edit a drag reports before it moves: the gesture's start value.
    fn rest(&self, pressed: &Pressed) -> Option<ClipEdit> {
        match pressed.grab {
            Grab::Empty => None,
            Grab::Edge { clip, edge, kind: TrimKind::Roll, partner: Some(other) } => {
                let (left, right) = if edge == ClipEdge::Out { (clip, other) } else { (other, clip) };
                Some(ClipEdit::Roll { left, right, delta: Tick::ZERO })
            }
            Grab::Edge { clip, edge, kind, .. } => Some(ClipEdit::Trim { clip, edge, kind, delta: Tick::ZERO, linked: pressed.linked }),
            Grab::Body(clip) => match self.tool {
                TimelineTool::Razor => None,
                TimelineTool::Slip => Some(ClipEdit::Slip { clip, delta: Tick::ZERO }),
                TimelineTool::Slide => Some(ClipEdit::Slide { clip, delta: Tick::ZERO }),
                _ => Some(ClipEdit::Move { clips: pressed.moving.clone(), delta: Tick::ZERO, track_delta: 0, new_track: false, copy: false }),
            },
        }
    }

    /// The edit of the drag with the pointer at `x`, and the target it snapped to
    /// (FilmCraft `interact.rs:558` `drag_update`).
    fn edit(&self, pressed: &Pressed, x: f32, mods: Modifiers) -> Option<(ClipEdit, Option<Tick>)> {
        let here = self.view.time_at(x, self.left);
        let moved = here - pressed.from;
        let frame = self.rate.frame_duration();
        match pressed.grab {
            Grab::Empty => None,
            Grab::Body(clip) => match self.tool {
                TimelineTool::Razor => None,
                TimelineTool::Slip => {
                    let info = self.info(clip)?;
                    let mut delta = -self.rate.snap_nearest(moved);
                    if let Some(source) = info.source {
                        delta = delta.clamp(info.range.end() - source.end(), info.range.start - source.start);
                    }
                    Some((ClipEdit::Slip { clip, delta }, None))
                }
                TimelineTool::Slide => Some((ClipEdit::Slide { clip, delta: self.rate.snap_nearest(moved) }, None)),
                _ => {
                    let dragged: Vec<_> = self.clips.iter().filter(|info| pressed.moving.contains(&info.id)).collect();
                    let first = dragged.iter().map(|info| info.range.start).min()?;
                    let last = dragged.iter().map(|info| info.range.end()).max()?;
                    let raw = self.rate.snap_nearest(moved);
                    let (start, start_hit) = self.snapped(first + raw);
                    let (offset, hit) = if start != first + raw {
                        (start - first, start_hit)
                    } else {
                        let (end, end_hit) = self.snapped(last + raw);
                        (end - last, end_hit)
                    };
                    let delta = offset.max(-first);
                    let edit = ClipEdit::Move { clips: pressed.moving.clone(), delta, track_delta: 0, new_track: false, copy: mods.alt };
                    Some((edit, hit))
                }
            },
            Grab::Edge { clip, edge, kind: TrimKind::Roll, partner: Some(other) } => {
                let (left, right) = if edge == ClipEdge::Out { (self.info(clip)?, self.info(other)?) } else { (self.info(other)?, self.info(clip)?) };
                let cut = left.range.end();
                let (target, hit) = self.snapped(self.rate.snap_nearest(cut + moved));
                let mut lowest = -(left.range.duration - frame);
                let mut highest = right.range.duration - frame;
                if let Some(source) = left.source {
                    highest = highest.min(source.end() - cut);
                }
                if let Some(source) = right.source {
                    lowest = lowest.max(source.start - right.range.start);
                }
                let delta = (target - cut).clamp(lowest, highest);
                let hit = hit.filter(|_| delta == target - cut);
                Some((ClipEdit::Roll { left: left.id, right: right.id, delta }, hit))
            }
            Grab::Edge { clip, edge, kind, .. } => {
                let info = self.info(clip)?;
                let base = match edge {
                    ClipEdge::In => info.range.start,
                    ClipEdge::Out => info.range.end(),
                };
                let (target, hit) = self.snapped(self.rate.snap_nearest(base + moved));
                // A clip keeps one frame, and a trim stops at the media's ends (a stretch changes
                // the speed and has none).
                let shortest = info.range.duration - frame;
                let (mut lowest, mut highest) = match edge {
                    ClipEdge::In => (Tick::MIN, shortest),
                    ClipEdge::Out => (-shortest, Tick::MAX),
                };
                if kind != TrimKind::Stretch
                    && let Some(source) = info.source
                {
                    match edge {
                        ClipEdge::In => lowest = source.start - base,
                        ClipEdge::Out => highest = source.end() - base,
                    }
                }
                let delta = (target - base).clamp(lowest, highest);
                let hit = hit.filter(|_| delta == target - base);
                Some((ClipEdit::Trim { clip, edge, kind, delta, linked: pressed.linked }, hit))
            }
        }
    }
}

/// Paints one clip block: label colour (dimmed when disabled), the app's content, the name, the
/// left-edge separator, the locked hatch, the selected outline.
#[allow(clippy::too_many_arguments)]
fn paint_clip(
    ui: &Ui,
    painter: &Painter,
    tokens: &craft_ui::theme::UiTokens,
    body: Rect,
    clip: &ClipInfo<'_>,
    palette: &LabelPalette,
    selected: bool,
    locked: bool,
    content: Option<&mut ClipPainter<'_>>,
) {
    let base = palette.color(clip.label.unwrap_or_default());
    let fill =
        if clip.enabled { base } else { Color32::from_rgb(base.r() / DISABLED_DIVISOR, base.g() / DISABLED_DIVISOR, base.b() / DISABLED_DIVISOR) };
    let clipped = painter.with_clip_rect(body.intersect(painter.clip_rect()));
    clipped.rect_filled(body, 0.0, fill);
    if let Some(content) = content {
        content(&clipped, body, clip);
    }
    let text_color = if clip.enabled { tokens.color.text.on_strong } else { tokens.color.text.disabled };
    let caption = role_format(tokens, TimelineStyle::of(tokens).clip_text, text_color.to_egui());
    let line = tokens.typography.get(TimelineStyle::of(tokens).clip_text).line_height;
    let pad = ui.spacing().item_spacing.x;
    let name_at = pos2(body.left().max(clipped.clip_rect().left()) + pad, body.top() + pad + line / 2.0);
    clipped.text(name_at, Align2::LEFT_CENTER, clip.name, caption.font_id.clone(), caption.color);
    let hairline = ui.visuals().widgets.noninteractive.bg_stroke.width;
    painter.line_segment([body.left_top(), body.left_bottom()], Stroke::new(hairline, tokens.color.surface.canvas.to_egui()));
    if locked {
        let step = line / HATCH_STEP_DIVISOR;
        let stroke = Stroke::new(hairline, tokens.color.text.faint.to_egui());
        let visible = body.intersect(clipped.clip_rect());
        if step > 0.0 {
            let first = ((visible.left() - body.height() - body.left()) / step).floor().max(0.0);
            let mut x = body.left() + first * step;
            while x < visible.right() {
                clipped.line_segment([pos2(x, body.bottom()), pos2(x + body.height(), body.top())], stroke);
                x += step;
            }
        }
    }
    if selected {
        painter.rect_stroke(body, 0.0, ui.visuals().selection.stroke, StrokeKind::Inside);
    }
}
