//! [`TimelineStyle`]: the timeline widgets' sizes, from the `timeline` tokens.

use craft_ui::theme::UiTokens;

/// The sizes every timeline widget reads, one `timeline.*` token each (craft-theme's `timeline`
/// family, with FilmCraft's and EffectCraft's values). Runtime tokens, so apps override them
/// through `sizes.json`.
/// No widget of this crate takes a size from anywhere else.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TimelineStyle {
    /// The timecode and toolbar strip over the headers, beside the ruler: `timeline.top_height`.
    pub top_height: f32,
    /// The ruler's height: `timeline.ruler_height`.
    pub ruler_height: f32,
    /// The scroll bar's height: `timeline.scrollbar_height`.
    pub scrollbar_height: f32,
    /// The least thumb length of the scroll bar: `timeline.scroll_thumb_min`.
    pub scroll_thumb_min: f32,
    /// The divider between video and audio tracks: `timeline.divider_height`.
    pub divider_height: f32,
    /// The Mix row at the bottom: `timeline.mix_row_height`.
    pub mix_row_height: f32,
    /// The track headers' default width: `timeline.header_width`.
    pub header_width: f32,
    /// A new video track's height: `timeline.video_track_height`.
    pub video_track_height: f32,
    /// A new audio track's height: `timeline.audio_track_height`.
    pub audio_track_height: f32,
    /// The least and most a track's height can be dragged to: `timeline.track_height_min`,
    /// `timeline.track_height_max`.
    pub track_height_min: f32,
    pub track_height_max: f32,
    /// The colour label strip at a header's leading edge: `timeline.label_strip`.
    pub label_strip: f32,
    /// How near a snap target must be: `timeline.snap_distance`.
    pub snap_distance: f32,
    /// The widest a clip's edge zone gets on each side of an edge: `timeline.edge_zone`.
    pub edge_zone: f32,
    /// Within this distance of a cut the Selection tool rolls (no-modifier trims):
    /// `timeline.roll_zone`.
    pub roll_zone: f32,
    /// The room [`TimeView::fit`](crate::TimeView::fit) leaves: `timeline.fit_margin`.
    pub fit_margin: f32,
    /// A transport button's square: `timeline.transport_button`.
    pub transport_button: f32,
    /// A keyframe glyph's box: `timeline.keyframe_glyph`; a hold key's square inside it:
    /// `timeline.keyframe_hold`; a bezier key's circle radius: `timeline.keyframe_bezier`.
    pub keyframe_glyph: f32,
    pub keyframe_hold: f32,
    pub keyframe_bezier: f32,
    /// How near the playhead or another key a Shift-dragged key snaps:
    /// `timeline.keyframe_snap`.
    pub keyframe_snap: f32,
    /// The keyframe navigator's arrow box (`timeline.keyframe_arrow_width` ×
    /// `timeline.keyframe_arrow_height`) and the room between its parts
    /// (`timeline.keyframe_nav_step`).
    pub keyframe_arrow_width: f32,
    pub keyframe_arrow_height: f32,
    pub keyframe_nav_step: f32,
    /// A marker flag's width: `timeline.marker_width`.
    pub marker_width: f32,
    /// How near a work-area end must be to grab it: `timeline.work_area_grab`.
    pub work_area_grab: f32,
    /// The least room between two ruler labels and between two ticks:
    /// `timeline.ruler_label_gap`, `timeline.ruler_tick_gap`.
    pub ruler_label_gap: f32,
    pub ruler_tick_gap: f32,
    /// A drag of this length moves a timecode field's value one frame:
    /// `timeline.scrub_step`.
    pub scrub_step: f32,
}

impl TimelineStyle {
    /// The style from the theme's tokens: `TimelineStyle::of(&Tokens::of(ui))`.
    pub fn of(tokens: &UiTokens) -> TimelineStyle {
        let t = &tokens.component.timeline;
        TimelineStyle {
            top_height: t.top_height,
            ruler_height: t.ruler_height,
            scrollbar_height: t.scrollbar_height,
            scroll_thumb_min: t.scroll_thumb_min,
            divider_height: t.divider_height,
            mix_row_height: t.mix_row_height,
            header_width: t.header_width,
            video_track_height: t.video_track_height,
            audio_track_height: t.audio_track_height,
            track_height_min: t.track_height_min,
            track_height_max: t.track_height_max,
            label_strip: t.label_strip,
            snap_distance: t.snap_distance,
            edge_zone: t.edge_zone,
            roll_zone: t.roll_zone,
            fit_margin: t.fit_margin,
            transport_button: t.transport_button,
            keyframe_glyph: t.keyframe_glyph,
            keyframe_hold: t.keyframe_hold,
            keyframe_bezier: t.keyframe_bezier,
            keyframe_snap: t.keyframe_snap,
            keyframe_arrow_width: t.keyframe_arrow_width,
            keyframe_arrow_height: t.keyframe_arrow_height,
            keyframe_nav_step: t.keyframe_nav_step,
            marker_width: t.marker_width,
            work_area_grab: t.work_area_grab,
            ruler_label_gap: t.ruler_label_gap,
            ruler_tick_gap: t.ruler_tick_gap,
            scrub_step: t.scrub_step,
        }
    }
}
