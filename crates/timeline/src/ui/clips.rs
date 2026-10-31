//! The clips of one track row, with Premiere's trims and moves (N1.05).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditEvent;
use egui::{IdSalt, Painter, Rect, Response, Ui};

use super::LabelPalette;
use crate::clips::{ClipEdit, SnapTargets};
use crate::model::{ClipId, ClipInfo, TrackId};
use crate::rates::FrameRate;
use crate::time::{Tick, TimeView};

/// The timeline tool that decides what a press on a clip does (Premiere's Tools panel; the app
/// keeps the current one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimelineTool {
    /// Select, move, and trim at edges ([`selection_trim`](crate::clips::selection_trim)).
    #[default]
    Selection,
    Ripple,
    Rolling,
    RateStretch,
    Razor,
    Slip,
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

/// Where the thumbnails of a clip from `x0` to `x1` go, `tile_w` wide, and the clip time each
/// shows: `(x, time)`. Only tiles within `visible` (an x range) are listed; `Continuous` tiles the
/// clip edge to edge, capped at [`MAX_THUMBNAIL_TILES`]; `frame` is one frame's duration (the
/// tail shows the last frame).
// filmcraft: crates/ui-egui/src/panels/timeline/clips.rs:135 `thumbnail_tiles`.
pub fn thumbnail_tiles(mode: ThumbnailMode, x0: f32, x1: f32, tile_w: f32, visible: (f32, f32), duration: Tick, frame: Tick) -> Vec<(f32, Tick)> {
    todo!()
}

/// Paints a clip's content (thumbnails, waveform, fades) into its body rect.
pub type ClipPainter<'a> = Box<dyn FnMut(&Painter, Rect, &ClipInfo<'_>) + 'a>;

/// The clips of one track row.
///
/// - Draws each [`ClipInfo`] as a block: label colour (from [`LabelPalette`]), name, a
///   selected outline, dimmed when disabled; the app paints the content (thumbnails, waveform,
///   fades) through [`content`](Self::content) before the outline.
/// - A press with the [`TimelineTool`] on an edge ([`edge_at`](crate::edge_at), FilmCraft's
///   zones) starts a trim of the kind the tool and modifiers give; on a body it moves (Selection),
///   slips or slides; the Razor cuts at the pointer (snapped to `rate`'s frames). The drag is
///   reported as an `EditEvent<ClipEdit>` gesture; moves snap to `snap` targets. Esc during the
///   drag cancels it, and the release after an Esc is ignored (FilmCraft #580).
/// - A press acts on what it pressed, worked out on the press frame, not when egui's drag
///   threshold passes (FilmCraft `Pressed`, #259).
/// - A click selects (Shift or Cmd adds), a double-click and a secondary click are reported.
/// - A locked track draws stripes over its clips and starts no gesture.
/// - Hover cursors: trim, roll, slip, slide, razor (craft-ui's canvas cursors).
/// - Keyboard: none here; clips are reached through the panel (arrows move the selection).
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
    palette: Option<&'a LabelPalette>,
    content: Option<ClipPainter<'a>>,
}

impl<'a> ClipLane<'a> {
    pub fn new(track: TrackId, clips: &'a [ClipInfo<'a>], view: &'a TimeView, rate: FrameRate) -> Self {
        Self {
            id_salt: IdSalt::new(("clip-lane", track)),
            track,
            clips,
            view,
            rate,
            tool: TimelineTool::Selection,
            no_modifier_trims: false,
            snap: None,
            locked: false,
            palette: None,
            content: None,
        }
    }

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

    pub fn snap(mut self, targets: Option<&'a SnapTargets>) -> Self {
        self.snap = targets;
        self
    }

    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    pub fn palette(mut self, palette: &'a LabelPalette) -> Self {
        self.palette = Some(palette);
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
        todo!()
    }
}

/// What a clip lane did this pass.
pub struct ClipLaneResponse {
    pub response: Response,
    pub edit: Option<EditEvent<ClipEdit>>,
    /// A click on a clip; `add` with Shift or Cmd.
    pub clicked: Option<(ClipId, bool)>,
    pub double_clicked: Option<ClipId>,
    /// A secondary click: the app opens its clip menu.
    pub context_menu: Option<ClipId>,
    /// Where a drag snapped this pass.
    pub snapped_at: Option<Tick>,
}
