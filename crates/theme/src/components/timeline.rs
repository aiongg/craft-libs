//! The timeline: the sizes of its strips, tracks, clips, keyframes and markers. Colours come from
//! the semantic tokens. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Timeline tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TimelineTokens {
    /// The timecode and toolbar strip over the track headers: [`values::TIMELINE_TOP_HEIGHT`].
    pub top_height: f32,
    /// The ruler's height: [`values::TIMELINE_RULER_HEIGHT`].
    pub ruler_height: f32,
    /// The scroll bar's height: [`values::TIMELINE_SCROLLBAR_HEIGHT`].
    pub scrollbar_height: f32,
    /// The least thumb length of the scroll bar: [`values::TIMELINE_SCROLL_THUMB_MIN`].
    pub scroll_thumb_min: f32,
    /// The divider between video and audio tracks: [`values::TIMELINE_DIVIDER_HEIGHT`].
    pub divider_height: f32,
    /// The Mix row at the bottom: [`values::TIMELINE_MIX_ROW_HEIGHT`].
    pub mix_row_height: f32,
    /// The track headers' default width: [`values::TIMELINE_HEADER_WIDTH`].
    pub header_width: f32,
    /// A new video track's height: [`values::TIMELINE_VIDEO_TRACK_HEIGHT`].
    pub video_track_height: f32,
    /// A new audio track's height: [`values::TIMELINE_AUDIO_TRACK_HEIGHT`].
    pub audio_track_height: f32,
    /// The least a track's height can be dragged to: [`values::TIMELINE_TRACK_HEIGHT_MIN`].
    pub track_height_min: f32,
    /// The most a track's height can be dragged to: [`values::TIMELINE_TRACK_HEIGHT_MAX`].
    pub track_height_max: f32,
    /// The colour label strip at a header's leading edge: [`values::TIMELINE_LABEL_STRIP`].
    pub label_strip: f32,
    /// How near a snap target must be: [`values::TIMELINE_SNAP_DISTANCE`].
    pub snap_distance: f32,
    /// The widest a clip's edge zone gets on each side of an edge: [`values::TIMELINE_EDGE_ZONE`].
    pub edge_zone: f32,
    /// The distance from a cut within which the Selection tool rolls: [`values::TIMELINE_ROLL_ZONE`].
    pub roll_zone: f32,
    /// The room `TimeView::fit` leaves: [`values::TIMELINE_FIT_MARGIN`].
    pub fit_margin: f32,
    /// A transport button's square: [`values::TIMELINE_TRANSPORT_BUTTON`].
    pub transport_button: f32,
    /// A keyframe glyph's box: [`values::TIMELINE_KEYFRAME_GLYPH`].
    pub keyframe_glyph: f32,
    /// A hold key's square inside the glyph box: [`values::TIMELINE_KEYFRAME_HOLD`].
    pub keyframe_hold: f32,
    /// A bezier key's circle radius: [`values::TIMELINE_KEYFRAME_BEZIER`].
    pub keyframe_bezier: f32,
    /// How near the playhead or another key a Shift-dragged key snaps: [`values::TIMELINE_KEYFRAME_SNAP`].
    pub keyframe_snap: f32,
    /// The keyframe navigator's arrow box width: [`values::TIMELINE_KEYFRAME_ARROW_WIDTH`].
    pub keyframe_arrow_width: f32,
    /// The keyframe navigator's arrow box height: [`values::TIMELINE_KEYFRAME_ARROW_HEIGHT`].
    pub keyframe_arrow_height: f32,
    /// The room between the keyframe navigator's parts: [`values::TIMELINE_KEYFRAME_NAV_STEP`].
    pub keyframe_nav_step: f32,
    /// A marker flag's width: [`values::TIMELINE_MARKER_WIDTH`].
    pub marker_width: f32,
    /// How near a work-area end must be to grab it: [`values::TIMELINE_WORK_AREA_GRAB`].
    pub work_area_grab: f32,
    /// The least room between two ruler labels: [`values::TIMELINE_RULER_LABEL_GAP`].
    pub ruler_label_gap: f32,
    /// The least room between two ruler ticks: [`values::TIMELINE_RULER_TICK_GAP`].
    pub ruler_tick_gap: f32,
    /// The drag length that moves a timecode field's value one frame: [`values::TIMELINE_SCRUB_STEP`].
    pub scrub_step: f32,
    /// A marker flag's height at most (EffectCraft `markers_ui.rs:287`): [`values::TIMELINE_MARKER_HEIGHT`].
    pub marker_height: f32,
    /// A marker's hit width (`markers_ui.rs:310`): [`values::TIMELINE_MARKER_HIT`].
    pub marker_hit: f32,
    /// The room either side of a marker's name inside its box (`markers_ui.rs:304`): [`values::TIMELINE_MARKER_NAME_PAD`].
    pub marker_name_pad: f32,
    /// The room between a marker's flag and its name box (`markers_ui.rs:381`, `:389`): [`values::TIMELINE_MARKER_NAME_GAP`].
    pub marker_name_gap: f32,
    /// The room between a range marker's end and a name box after it (`markers_ui.rs:383`): [`values::TIMELINE_MARKER_END_GAP`].
    pub marker_end_gap: f32,
    /// A graph key's square (EffectCraft `graph.rs:543`): [`values::TIMELINE_GRAPH_KEY`].
    pub graph_key: f32,
    /// A graph key's hit square (`graph.rs:545`): [`values::TIMELINE_GRAPH_KEY_HIT`].
    pub graph_key_hit: f32,
    /// A graph handle's dot radius (`graph.rs:488`): [`values::TIMELINE_GRAPH_HANDLE`].
    pub graph_handle: f32,
    /// A graph handle's hit square (`graph.rs:490`): [`values::TIMELINE_GRAPH_HANDLE_HIT`].
    pub graph_handle_hit: f32,
    /// The room over the graph's plot (`graph.rs:412`): [`values::TIMELINE_GRAPH_TOP_INSET`].
    pub graph_top_inset: f32,
    /// The room between the plot's edge and its value labels (`graph.rs:441`): [`values::TIMELINE_GRAPH_LABEL_INSET`].
    pub graph_label_inset: f32,
    /// How far past the plot a key still counts as shown (`graph.rs:479`): [`values::TIMELINE_GRAPH_REACH`].
    pub graph_reach: f32,
    /// The graph's bottom bar (`graph.rs:411`): [`values::TIMELINE_GRAPH_BAR_HEIGHT`].
    pub graph_bar_height: f32,
    /// The room either side of the overview's runs (SoundCraft `universe.rs:61`): [`values::TIMELINE_OVERVIEW_INSET_X`].
    pub overview_inset_x: f32,
    /// The room over and under the overview's runs (`universe.rs:61`): [`values::TIMELINE_OVERVIEW_INSET_Y`].
    pub overview_inset_y: f32,
    /// How far the overview's view box reaches past the runs (`universe.rs:81`): [`values::TIMELINE_OVERVIEW_BOX_OUTSET`].
    pub overview_box_outset: f32,
    /// The least width the overview takes the time area to have (`universe.rs:80`): [`values::TIMELINE_OVERVIEW_VIEW_MIN`].
    pub overview_view_min: f32,
    /// The ruler's marker band at its top: [`values::TIMELINE_RULER_MARKER_BAND`].
    pub ruler_marker_band: f32,
    /// The work-area bar's height under the marker band: [`values::TIMELINE_RULER_WORK_HEIGHT`].
    pub ruler_work_height: f32,
    /// How far below the ruler's top the time labels' centre line is: [`values::TIMELINE_RULER_LABEL_Y`].
    pub ruler_label_y: f32,
    /// How far above the ruler's bottom the ticks start: [`values::TIMELINE_RULER_TICK_INSET`].
    pub ruler_tick_inset: f32,
    /// A labelled tick's length: [`values::TIMELINE_RULER_MAJOR_TICK`].
    pub ruler_major_tick: f32,
    /// An unlabelled tick's length: [`values::TIMELINE_RULER_MINOR_TICK`].
    pub ruler_minor_tick: f32,
    /// The render and cache bar's height at the ruler's bottom: [`values::TIMELINE_RULER_RENDER_BAR`].
    pub ruler_render_bar: f32,
    /// Half the playhead head's width: [`values::TIMELINE_RULER_HEAD_HALF_WIDTH`].
    pub ruler_head_half_width: f32,
    /// The playhead head's height: [`values::TIMELINE_RULER_HEAD_HEIGHT`].
    pub ruler_head_height: f32,
    /// A work-area end cap's width: [`values::TIMELINE_RULER_CAP_WIDTH`].
    pub ruler_cap_width: f32,
    /// The radius of the knobs at the scroll thumb's ends: [`values::TIMELINE_SCROLL_KNOB_RADIUS`].
    pub scroll_knob_radius: f32,
    /// How near a scroll thumb's end a press must be to resize the view: [`values::TIMELINE_SCROLL_END_ZONE`].
    pub scroll_end_zone: f32,
    /// The least the track headers' width can be dragged to: [`values::TIMELINE_HEADER_WIDTH_MIN`].
    pub header_width_min: f32,
    /// The most the track headers' width can be dragged to: [`values::TIMELINE_HEADER_WIDTH_MAX`].
    pub header_width_max: f32,
    /// A caption row's height when the app gives none: [`values::TIMELINE_CAPTION_ROW_HEIGHT`].
    pub caption_row_height: f32,
    /// How far each level of a track's depth indents its header: [`values::TIMELINE_HEADER_INDENT`].
    pub header_indent: f32,
}

/// Timeline tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimelineField {}

impl Family for TimelineTokens {
    type Field = TimelineField;
    const FIELDS: &'static [TimelineField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { TimelineTokens;
        "top_height" 8.0..=256.0 => top_height;
        "ruler_height" 8.0..=256.0 => ruler_height;
        "scrollbar_height" 4.0..=64.0 => scrollbar_height;
        "scroll_thumb_min" 8.0..=256.0 => scroll_thumb_min;
        "divider_height" 1.0..=32.0 => divider_height;
        "mix_row_height" 8.0..=256.0 => mix_row_height;
        "header_width" 64.0..=640.0 => header_width;
        "video_track_height" 8.0..=600.0 => video_track_height;
        "audio_track_height" 8.0..=600.0 => audio_track_height;
        "track_height_min" 4.0..=64.0 => track_height_min;
        "track_height_max" 64.0..=2048.0 => track_height_max;
        "label_strip" 1.0..=32.0 => label_strip;
        "snap_distance" 1.0..=64.0 => snap_distance;
        "edge_zone" 1.0..=32.0 => edge_zone;
        "roll_zone" 0.5..=16.0 => roll_zone;
        "fit_margin" 0.0..=64.0 => fit_margin;
        "transport_button" 12.0..=64.0 => transport_button;
        "keyframe_glyph" 6.0..=48.0 => keyframe_glyph;
        "keyframe_hold" 3.0..=32.0 => keyframe_hold;
        "keyframe_bezier" 1.0..=24.0 => keyframe_bezier;
        "keyframe_snap" 1.0..=64.0 => keyframe_snap;
        "keyframe_arrow_width" 4.0..=64.0 => keyframe_arrow_width;
        "keyframe_arrow_height" 4.0..=64.0 => keyframe_arrow_height;
        "keyframe_nav_step" 4.0..=64.0 => keyframe_nav_step;
        "marker_width" 2.0..=32.0 => marker_width;
        "work_area_grab" 1.0..=64.0 => work_area_grab;
        "ruler_label_gap" 16.0..=512.0 => ruler_label_gap;
        "ruler_tick_gap" 2.0..=64.0 => ruler_tick_gap;
        "scrub_step" 0.25..=16.0 => scrub_step;
        "marker_height" 4.0..=64.0 => marker_height;
        "marker_hit" 4.0..=64.0 => marker_hit;
        "marker_name_pad" 0.0..=32.0 => marker_name_pad;
        "marker_name_gap" 0.0..=64.0 => marker_name_gap;
        "marker_end_gap" 0.0..=32.0 => marker_end_gap;
        "graph_key" 3.0..=32.0 => graph_key;
        "graph_key_hit" 3.0..=48.0 => graph_key_hit;
        "graph_handle" 1.0..=24.0 => graph_handle;
        "graph_handle_hit" 3.0..=48.0 => graph_handle_hit;
        "graph_top_inset" 0.0..=64.0 => graph_top_inset;
        "graph_label_inset" 0.0..=32.0 => graph_label_inset;
        "graph_reach" 0.0..=64.0 => graph_reach;
        "graph_bar_height" 12.0..=128.0 => graph_bar_height;
        "overview_inset_x" 0.0..=64.0 => overview_inset_x;
        "overview_inset_y" 0.0..=64.0 => overview_inset_y;
        "overview_box_outset" 0.0..=32.0 => overview_box_outset;
        "overview_view_min" 1.0..=2048.0 => overview_view_min;
        "ruler_marker_band" 1.0..=128.0 => ruler_marker_band;
        "ruler_work_height" 1.0..=64.0 => ruler_work_height;
        "ruler_label_y" 4.0..=256.0 => ruler_label_y;
        "ruler_tick_inset" 0.0..=32.0 => ruler_tick_inset;
        "ruler_major_tick" 1.0..=64.0 => ruler_major_tick;
        "ruler_minor_tick" 1.0..=64.0 => ruler_minor_tick;
        "ruler_render_bar" 0.5..=16.0 => ruler_render_bar;
        "ruler_head_half_width" 1.0..=32.0 => ruler_head_half_width;
        "ruler_head_height" 2.0..=64.0 => ruler_head_height;
        "ruler_cap_width" 1.0..=16.0 => ruler_cap_width;
        "scroll_knob_radius" 1.0..=32.0 => scroll_knob_radius;
        "scroll_end_zone" 1.0..=64.0 => scroll_end_zone;
        "header_width_min" 32.0..=640.0 => header_width_min;
        "header_width_max" 64.0..=1024.0 => header_width_max;
        "caption_row_height" 8.0..=128.0 => caption_row_height;
        "header_indent" 0.0..=64.0 => header_indent;
    };

    fn defaults(_: &Semantic<'_>) -> TimelineTokens {
        TimelineTokens {
            top_height: values::TIMELINE_TOP_HEIGHT,
            ruler_height: values::TIMELINE_RULER_HEIGHT,
            scrollbar_height: values::TIMELINE_SCROLLBAR_HEIGHT,
            scroll_thumb_min: values::TIMELINE_SCROLL_THUMB_MIN,
            divider_height: values::TIMELINE_DIVIDER_HEIGHT,
            mix_row_height: values::TIMELINE_MIX_ROW_HEIGHT,
            header_width: values::TIMELINE_HEADER_WIDTH,
            video_track_height: values::TIMELINE_VIDEO_TRACK_HEIGHT,
            audio_track_height: values::TIMELINE_AUDIO_TRACK_HEIGHT,
            track_height_min: values::TIMELINE_TRACK_HEIGHT_MIN,
            track_height_max: values::TIMELINE_TRACK_HEIGHT_MAX,
            label_strip: values::TIMELINE_LABEL_STRIP,
            snap_distance: values::TIMELINE_SNAP_DISTANCE,
            edge_zone: values::TIMELINE_EDGE_ZONE,
            roll_zone: values::TIMELINE_ROLL_ZONE,
            fit_margin: values::TIMELINE_FIT_MARGIN,
            transport_button: values::TIMELINE_TRANSPORT_BUTTON,
            keyframe_glyph: values::TIMELINE_KEYFRAME_GLYPH,
            keyframe_hold: values::TIMELINE_KEYFRAME_HOLD,
            keyframe_bezier: values::TIMELINE_KEYFRAME_BEZIER,
            keyframe_snap: values::TIMELINE_KEYFRAME_SNAP,
            keyframe_arrow_width: values::TIMELINE_KEYFRAME_ARROW_WIDTH,
            keyframe_arrow_height: values::TIMELINE_KEYFRAME_ARROW_HEIGHT,
            keyframe_nav_step: values::TIMELINE_KEYFRAME_NAV_STEP,
            marker_width: values::TIMELINE_MARKER_WIDTH,
            work_area_grab: values::TIMELINE_WORK_AREA_GRAB,
            ruler_label_gap: values::TIMELINE_RULER_LABEL_GAP,
            ruler_tick_gap: values::TIMELINE_RULER_TICK_GAP,
            scrub_step: values::TIMELINE_SCRUB_STEP,
            marker_height: values::TIMELINE_MARKER_HEIGHT,
            marker_hit: values::TIMELINE_MARKER_HIT,
            marker_name_pad: values::TIMELINE_MARKER_NAME_PAD,
            marker_name_gap: values::TIMELINE_MARKER_NAME_GAP,
            marker_end_gap: values::TIMELINE_MARKER_END_GAP,
            graph_key: values::TIMELINE_GRAPH_KEY,
            graph_key_hit: values::TIMELINE_GRAPH_KEY_HIT,
            graph_handle: values::TIMELINE_GRAPH_HANDLE,
            graph_handle_hit: values::TIMELINE_GRAPH_HANDLE_HIT,
            graph_top_inset: values::TIMELINE_GRAPH_TOP_INSET,
            graph_label_inset: values::TIMELINE_GRAPH_LABEL_INSET,
            graph_reach: values::TIMELINE_GRAPH_REACH,
            graph_bar_height: values::TIMELINE_GRAPH_BAR_HEIGHT,
            overview_inset_x: values::TIMELINE_OVERVIEW_INSET_X,
            overview_inset_y: values::TIMELINE_OVERVIEW_INSET_Y,
            overview_box_outset: values::TIMELINE_OVERVIEW_BOX_OUTSET,
            overview_view_min: values::TIMELINE_OVERVIEW_VIEW_MIN,
            ruler_marker_band: values::TIMELINE_RULER_MARKER_BAND,
            ruler_work_height: values::TIMELINE_RULER_WORK_HEIGHT,
            ruler_label_y: values::TIMELINE_RULER_LABEL_Y,
            ruler_tick_inset: values::TIMELINE_RULER_TICK_INSET,
            ruler_major_tick: values::TIMELINE_RULER_MAJOR_TICK,
            ruler_minor_tick: values::TIMELINE_RULER_MINOR_TICK,
            ruler_render_bar: values::TIMELINE_RULER_RENDER_BAR,
            ruler_head_half_width: values::TIMELINE_RULER_HEAD_HALF_WIDTH,
            ruler_head_height: values::TIMELINE_RULER_HEAD_HEIGHT,
            ruler_cap_width: values::TIMELINE_RULER_CAP_WIDTH,
            scroll_knob_radius: values::TIMELINE_SCROLL_KNOB_RADIUS,
            scroll_end_zone: values::TIMELINE_SCROLL_END_ZONE,
            header_width_min: values::TIMELINE_HEADER_WIDTH_MIN,
            header_width_max: values::TIMELINE_HEADER_WIDTH_MAX,
            caption_row_height: values::TIMELINE_CAPTION_ROW_HEIGHT,
            header_indent: values::TIMELINE_HEADER_INDENT,
        }
    }

    fn path(f: TimelineField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TimelineField> {
        None
    }

    fn kind(f: TimelineField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TimelineField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TimelineField, _: OverrideValue) {
        match f {}
    }

    // Glyphs and bars drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
