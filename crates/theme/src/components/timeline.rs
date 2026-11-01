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
