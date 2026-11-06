//! [`TimelineStyle`]: the timeline widgets' sizes, from the semantic tokens.

use craft_theme::TypeRole;
use craft_ui::theme::UiTokens;

const SCROLLBAR_HEIGHT: f32 = 17.0;
const SCROLL_THUMB_MIN: f32 = 34.0;
const DIVIDER_HEIGHT: f32 = 5.0;
const TRACK_HEIGHT_MIN: f32 = 8.0;
const TRACK_HEIGHT_MAX: f32 = 600.0;
const LABEL_STRIP_HEIGHT: f32 = 5.0;
const SNAP_DISTANCE: f32 = 9.0;
const EDGE_ZONE: f32 = 7.0;
const ROLL_ZONE: f32 = 2.5;
const KEYFRAME_GLYPH_SIZE: f32 = 15.0;
const KEYFRAME_SNAP: f32 = 8.0;
const KEYFRAME_ARROW_WIDTH: f32 = 14.0;
const KEYFRAME_ARROW_HEIGHT: f32 = 20.0;
const KEYFRAME_NAV_STEP: f32 = 16.0;
const MARKER_WIDTH: f32 = 8.0;
const WORK_AREA_GRAB: f32 = 7.0;
const RULER_WORK_HIT_PAD: f32 = 1.0;
const RULER_RENDER_HIT_PAD: f32 = 2.0;
const RULER_LABEL_GAP: f32 = 110.0;
const RULER_TICK_GAP: f32 = 12.0;
const SCRUB_STEP: f32 = 1.0;
const MARKER_HEIGHT: f32 = 10.0;
const MARKER_HIT: f32 = 10.0;
const GRAPH_KEY_SIZE: f32 = 7.0;
const GRAPH_KEY_HIT: f32 = 11.0;
const GRAPH_HANDLE_HIT: f32 = 10.0;
const GRAPH_REACH: f32 = 6.0;
const OVERVIEW_VIEW_MIN: f32 = 100.0;
const RULER_MARKER_BAND: f32 = 14.0;
const RULER_WORK_HEIGHT: f32 = 6.0;
const RULER_LABEL_ROW: f32 = 14.0;
const RULER_LABEL_Y: f32 = 24.0;
const RULER_TICK_INSET: f32 = 3.0;
const RULER_MAJOR_TICK: f32 = 10.0;
const RULER_MINOR_TICK: f32 = 4.0;
const RULER_RENDER_BAR: f32 = 2.0;
const RULER_HEAD_HALF_WIDTH: f32 = 5.0;
const RULER_HEAD_HEIGHT: f32 = 13.0;
const RULER_CAP_WIDTH: f32 = 3.0;
const SCROLL_END_ZONE: f32 = 9.0;
const HEADER_WIDTH_MIN: f32 = 204.0;
const HEADER_WIDTH_MAX: f32 = 480.0;

/// The sizes every timeline widget reads, computed from the semantic tokens, the widget's content
/// and named constants. No widget of this crate takes a size from anywhere else.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TimelineStyle {
    /// The timecode and toolbar strip over the headers, beside the ruler.
    pub top_height: f32,
    /// The ruler's height.
    pub ruler_height: f32,
    /// The scroll bar's height.
    pub scrollbar_height: f32,
    /// The least thumb length of the scroll bar.
    pub scroll_thumb_min: f32,
    /// The divider between video and audio tracks.
    pub divider_height: f32,
    /// The Mix row at the bottom.
    pub mix_row_height: f32,
    /// The track headers' default width.
    pub header_width: f32,
    /// A new video track's height.
    pub video_track_height: f32,
    /// A new audio track's height.
    pub audio_track_height: f32,
    /// The least and most a track's height can be dragged to.
    pub track_height_min: f32,
    #[allow(missing_docs)]
    pub track_height_max: f32,
    /// The colour label strip at a header's leading edge.
    pub label_strip: f32,
    /// How near a snap target must be.
    pub snap_distance: f32,
    /// The widest a clip's edge zone gets on each side of an edge.
    pub edge_zone: f32,
    /// Within this distance of a cut the Selection tool rolls (no-modifier trims).
    pub roll_zone: f32,
    /// The room [`TimeView::fit`](crate::TimeView::fit) leaves.
    pub fit_margin: f32,
    /// A transport button's square.
    pub transport_button: f32,
    /// A keyframe glyph's box.
    pub keyframe_glyph: f32,
    /// How near the playhead or another key a Shift-dragged key snaps.
    pub keyframe_snap: f32,
    /// The keyframe navigator's arrow box and the room between its parts.
    pub keyframe_arrow_width: f32,
    #[allow(missing_docs)]
    pub keyframe_arrow_height: f32,
    #[allow(missing_docs)]
    pub keyframe_nav_step: f32,
    /// A marker flag's width.
    pub marker_width: f32,
    /// How near a work-area end must be to grab it.
    pub work_area_grab: f32,
    /// How far the work-area bar's hit area reaches above and below the bar.
    pub ruler_work_hit_pad: f32,
    /// How far a render-bar span's hover area reaches above and below the bar.
    pub ruler_render_hit_pad: f32,
    /// The least room between two ruler labels and between two ticks.
    pub ruler_label_gap: f32,
    #[allow(missing_docs)]
    pub ruler_tick_gap: f32,
    /// A drag of this length moves a timecode field's value one frame.
    pub scrub_step: f32,
    /// A marker flag's height at most.
    pub marker_height: f32,
    /// A marker's hit width.
    pub marker_hit: f32,
    /// The room either side of a marker's name inside its box.
    pub marker_name_pad: f32,
    /// The room between a marker's flag and its name box.
    pub marker_name_gap: f32,
    /// The room between a range marker's end and a name box after it.
    pub marker_end_gap: f32,
    /// A graph key's square.
    pub graph_key: f32,
    /// A graph key's hit square.
    pub graph_key_hit: f32,
    /// A graph handle's dot radius.
    pub graph_handle: f32,
    /// A graph handle's hit square.
    pub graph_handle_hit: f32,
    /// The room over the graph's plot.
    pub graph_top_inset: f32,
    /// The room between the plot's edge and its value labels.
    pub graph_label_inset: f32,
    /// How far past the plot a key still counts as shown.
    pub graph_reach: f32,
    /// The graph's bottom bar.
    pub graph_bar_height: f32,
    /// The room either side of the overview's runs.
    pub overview_inset_x: f32,
    /// The room over and under the overview's runs.
    pub overview_inset_y: f32,
    /// How far the overview's view box reaches past the runs.
    pub overview_box_outset: f32,
    /// The least width the overview takes the time area to have.
    pub overview_view_min: f32,
    /// The ruler's marker band at its top.
    pub ruler_marker_band: f32,
    /// The work-area bar's height under the marker band.
    pub ruler_work_height: f32,
    /// How far below the ruler's top the time labels' centre line is.
    pub ruler_label_y: f32,
    /// How far above the ruler's bottom the ticks start.
    pub ruler_tick_inset: f32,
    /// A labelled tick's length.
    pub ruler_major_tick: f32,
    /// An unlabelled tick's length.
    pub ruler_minor_tick: f32,
    /// The render and cache bar's height at the ruler's bottom.
    pub ruler_render_bar: f32,
    /// Half the playhead head's width.
    pub ruler_head_half_width: f32,
    /// The playhead head's height.
    pub ruler_head_height: f32,
    /// A work-area end cap's width.
    pub ruler_cap_width: f32,
    /// The radius of the knobs at the scroll thumb's ends.
    pub scroll_knob_radius: f32,
    /// How near a scroll thumb's end a press must be to resize the view.
    pub scroll_end_zone: f32,
    /// The least the track headers' width can be dragged to.
    pub header_width_min: f32,
    /// The most the track headers' width can be dragged to.
    pub header_width_max: f32,
    /// A caption row's height when the app gives none.
    pub caption_row_height: f32,
    /// How far each level of a track's depth indents its header.
    pub header_indent: f32,
    /// The type role of The ruler's time labels: the theme's caption role.
    pub ruler_text: TypeRole,
    /// The type role of A marker's name: the theme's caption role.
    pub marker_text: TypeRole,
    /// The type role of The graph's value labels: the theme's caption role.
    pub graph_text: TypeRole,
    /// The type role of A track header's name: the theme's caption role.
    pub header_text: TypeRole,
    /// The type role of The empty panel's hint: the theme's body role.
    pub empty_text: TypeRole,
    /// The type role of A clip's name: the theme's caption role.
    pub clip_text: TypeRole,
    /// The type role of A render queue row's caption: the theme's caption role.
    pub render_queue_text: TypeRole,
    /// The type role of The preview's caption: the theme's caption role.
    pub preview_text: TypeRole,
    /// The type role of A timecode field's and a timecode panel row's value: the theme's mono role.
    pub timecode_text: TypeRole,
    /// The type role of The large timecode and the timecode panel's first row: the theme's mono role (FilmCraft's timecode face).
    pub timecode_large_text: TypeRole,
    /// The type role of A timecode panel row's caption: the theme's caption role.
    pub timecode_caption: TypeRole,
    /// The type role of The transport's time readout: the theme's mono role.
    pub transport_text: TypeRole,
}

impl TimelineStyle {
    /// The style from the theme's tokens: `TimelineStyle::of(&Tokens::of(ui))`.
    pub fn of(tokens: &UiTokens) -> TimelineStyle {
        let control = tokens.size.control.md;
        let space = &tokens.space;
        let top_height = control + 2.0 * space.pad.panel;
        TimelineStyle {
            top_height,
            ruler_height: RULER_MARKER_BAND + RULER_WORK_HEIGHT + RULER_LABEL_ROW + RULER_MAJOR_TICK,
            scrollbar_height: SCROLLBAR_HEIGHT,
            scroll_thumb_min: SCROLL_THUMB_MIN,
            divider_height: DIVIDER_HEIGHT,
            mix_row_height: control + 2.0 * space.gap.tight,
            header_width: control * 7.0,
            video_track_height: tokens.size.thumb.lg + space.pad.panel,
            audio_track_height: tokens.size.thumb.lg + space.gap.stack,
            track_height_min: TRACK_HEIGHT_MIN,
            track_height_max: TRACK_HEIGHT_MAX,
            label_strip: LABEL_STRIP_HEIGHT,
            snap_distance: SNAP_DISTANCE,
            edge_zone: EDGE_ZONE,
            roll_zone: ROLL_ZONE,
            fit_margin: space.gap.stack + space.gap.tight,
            transport_button: control,
            keyframe_glyph: KEYFRAME_GLYPH_SIZE,
            keyframe_snap: KEYFRAME_SNAP,
            keyframe_arrow_width: KEYFRAME_ARROW_WIDTH,
            keyframe_arrow_height: KEYFRAME_ARROW_HEIGHT,
            keyframe_nav_step: KEYFRAME_NAV_STEP,
            marker_width: MARKER_WIDTH,
            work_area_grab: WORK_AREA_GRAB,
            ruler_work_hit_pad: RULER_WORK_HIT_PAD,
            ruler_render_hit_pad: RULER_RENDER_HIT_PAD,
            ruler_label_gap: RULER_LABEL_GAP,
            ruler_tick_gap: RULER_TICK_GAP,
            scrub_step: SCRUB_STEP,
            marker_height: MARKER_HEIGHT,
            marker_hit: MARKER_HIT,
            marker_name_pad: space.gap.inline,
            marker_name_gap: space.gap.inline,
            marker_end_gap: space.gap.tight,
            graph_key: GRAPH_KEY_SIZE,
            graph_key_hit: GRAPH_KEY_HIT,
            graph_handle: tokens.size.handle.sm / 2.0,
            graph_handle_hit: GRAPH_HANDLE_HIT,
            graph_top_inset: space.gap.inline,
            graph_label_inset: space.gap.inline,
            graph_reach: GRAPH_REACH,
            graph_bar_height: control,
            overview_inset_x: space.gap.stack,
            overview_inset_y: space.gap.inline,
            overview_box_outset: space.gap.tight,
            overview_view_min: OVERVIEW_VIEW_MIN,
            ruler_marker_band: RULER_MARKER_BAND,
            ruler_work_height: RULER_WORK_HEIGHT,
            ruler_label_y: RULER_LABEL_Y,
            ruler_tick_inset: RULER_TICK_INSET,
            ruler_major_tick: RULER_MAJOR_TICK,
            ruler_minor_tick: RULER_MINOR_TICK,
            ruler_render_bar: RULER_RENDER_BAR,
            ruler_head_half_width: RULER_HEAD_HALF_WIDTH,
            ruler_head_height: RULER_HEAD_HEIGHT,
            ruler_cap_width: RULER_CAP_WIDTH,
            scroll_knob_radius: tokens.shape.radius.sm,
            scroll_end_zone: SCROLL_END_ZONE,
            header_width_min: HEADER_WIDTH_MIN,
            header_width_max: HEADER_WIDTH_MAX,
            caption_row_height: control,
            header_indent: space.pad.panel,
            ruler_text: TypeRole::Caption,
            marker_text: TypeRole::Caption,
            graph_text: TypeRole::Caption,
            header_text: TypeRole::Caption,
            empty_text: TypeRole::Body,
            clip_text: TypeRole::Caption,
            render_queue_text: TypeRole::Caption,
            preview_text: TypeRole::Caption,
            timecode_text: TypeRole::Mono,
            timecode_large_text: TypeRole::Mono,
            timecode_caption: TypeRole::Caption,
            transport_text: TypeRole::Mono,
        }
    }
}
