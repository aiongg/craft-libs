//! [`TimelineStyle`]: the timeline widgets' sizes, from the `timeline` tokens.

use craft_theme::TypeRole;
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
    #[allow(missing_docs)]
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
    /// A keyframe glyph's box: `timeline.keyframe_glyph`.
    pub keyframe_glyph: f32,
    /// How near the playhead or another key a Shift-dragged key snaps:
    /// `timeline.keyframe_snap`.
    pub keyframe_snap: f32,
    /// The keyframe navigator's arrow box (`timeline.keyframe_arrow_width` ×
    /// `timeline.keyframe_arrow_height`) and the room between its parts
    /// (`timeline.keyframe_nav_step`).
    pub keyframe_arrow_width: f32,
    #[allow(missing_docs)]
    pub keyframe_arrow_height: f32,
    #[allow(missing_docs)]
    pub keyframe_nav_step: f32,
    /// A marker flag's width: `timeline.marker_width`.
    pub marker_width: f32,
    /// How near a work-area end must be to grab it: `timeline.work_area_grab`.
    pub work_area_grab: f32,
    /// How far the work-area bar's hit area reaches above and below the bar:
    /// `timeline.ruler_work_hit_pad`.
    pub ruler_work_hit_pad: f32,
    /// How far a render-bar span's hover area reaches above and below the bar:
    /// `timeline.ruler_render_hit_pad`.
    pub ruler_render_hit_pad: f32,
    /// The least room between two ruler labels and between two ticks:
    /// `timeline.ruler_label_gap`, `timeline.ruler_tick_gap`.
    pub ruler_label_gap: f32,
    #[allow(missing_docs)]
    pub ruler_tick_gap: f32,
    /// A drag of this length moves a timecode field's value one frame:
    /// `timeline.scrub_step`.
    pub scrub_step: f32,
    /// A marker flag's height at most (EffectCraft `markers_ui.rs:287`): `timeline.marker_height`.
    pub marker_height: f32,
    /// A marker's hit width (`markers_ui.rs:310`): `timeline.marker_hit`.
    pub marker_hit: f32,
    /// The room either side of a marker's name inside its box (`markers_ui.rs:304`): `timeline.marker_name_pad`.
    pub marker_name_pad: f32,
    /// The room between a marker's flag and its name box (`markers_ui.rs:381`, `:389`): `timeline.marker_name_gap`.
    pub marker_name_gap: f32,
    /// The room between a range marker's end and a name box after it (`markers_ui.rs:383`): `timeline.marker_end_gap`.
    pub marker_end_gap: f32,
    /// A graph key's square (EffectCraft `graph.rs:543`): `timeline.graph_key`.
    pub graph_key: f32,
    /// A graph key's hit square (`graph.rs:545`): `timeline.graph_key_hit`.
    pub graph_key_hit: f32,
    /// A graph handle's dot radius (`graph.rs:488`): `timeline.graph_handle`.
    pub graph_handle: f32,
    /// A graph handle's hit square (`graph.rs:490`): `timeline.graph_handle_hit`.
    pub graph_handle_hit: f32,
    /// The room over the graph's plot (`graph.rs:412`): `timeline.graph_top_inset`.
    pub graph_top_inset: f32,
    /// The room between the plot's edge and its value labels (`graph.rs:441`): `timeline.graph_label_inset`.
    pub graph_label_inset: f32,
    /// How far past the plot a key still counts as shown (`graph.rs:479`): `timeline.graph_reach`.
    pub graph_reach: f32,
    /// The graph's bottom bar (`graph.rs:411`): `timeline.graph_bar_height`.
    pub graph_bar_height: f32,
    /// The room either side of the overview's runs (SoundCraft `universe.rs:61`): `timeline.overview_inset_x`.
    pub overview_inset_x: f32,
    /// The room over and under the overview's runs (`universe.rs:61`): `timeline.overview_inset_y`.
    pub overview_inset_y: f32,
    /// How far the overview's view box reaches past the runs (`universe.rs:81`): `timeline.overview_box_outset`.
    pub overview_box_outset: f32,
    /// The least width the overview takes the time area to have (`universe.rs:80`): `timeline.overview_view_min`.
    pub overview_view_min: f32,
    /// The ruler's marker band at its top: `timeline.ruler_marker_band`.
    pub ruler_marker_band: f32,
    /// The work-area bar's height under the marker band: `timeline.ruler_work_height`.
    pub ruler_work_height: f32,
    /// How far below the ruler's top the time labels' centre line is: `timeline.ruler_label_y`.
    pub ruler_label_y: f32,
    /// How far above the ruler's bottom the ticks start: `timeline.ruler_tick_inset`.
    pub ruler_tick_inset: f32,
    /// A labelled tick's length: `timeline.ruler_major_tick`.
    pub ruler_major_tick: f32,
    /// An unlabelled tick's length: `timeline.ruler_minor_tick`.
    pub ruler_minor_tick: f32,
    /// The render and cache bar's height at the ruler's bottom: `timeline.ruler_render_bar`.
    pub ruler_render_bar: f32,
    /// Half the playhead head's width: `timeline.ruler_head_half_width`.
    pub ruler_head_half_width: f32,
    /// The playhead head's height: `timeline.ruler_head_height`.
    pub ruler_head_height: f32,
    /// A work-area end cap's width: `timeline.ruler_cap_width`.
    pub ruler_cap_width: f32,
    /// The radius of the knobs at the scroll thumb's ends: `timeline.scroll_knob_radius`.
    pub scroll_knob_radius: f32,
    /// How near a scroll thumb's end a press must be to resize the view: `timeline.scroll_end_zone`.
    pub scroll_end_zone: f32,
    /// The least the track headers' width can be dragged to: `timeline.header_width_min`.
    pub header_width_min: f32,
    /// The most the track headers' width can be dragged to: `timeline.header_width_max`.
    pub header_width_max: f32,
    /// A caption row's height when the app gives none: `timeline.caption_row_height`.
    pub caption_row_height: f32,
    /// How far each level of a track's depth indents its header: `timeline.header_indent`.
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
            keyframe_snap: t.keyframe_snap,
            keyframe_arrow_width: t.keyframe_arrow_width,
            keyframe_arrow_height: t.keyframe_arrow_height,
            keyframe_nav_step: t.keyframe_nav_step,
            marker_width: t.marker_width,
            work_area_grab: t.work_area_grab,
            ruler_work_hit_pad: t.ruler_work_hit_pad,
            ruler_render_hit_pad: t.ruler_render_hit_pad,
            ruler_label_gap: t.ruler_label_gap,
            ruler_tick_gap: t.ruler_tick_gap,
            scrub_step: t.scrub_step,
            marker_height: t.marker_height,
            marker_hit: t.marker_hit,
            marker_name_pad: t.marker_name_pad,
            marker_name_gap: t.marker_name_gap,
            marker_end_gap: t.marker_end_gap,
            graph_key: t.graph_key,
            graph_key_hit: t.graph_key_hit,
            graph_handle: t.graph_handle,
            graph_handle_hit: t.graph_handle_hit,
            graph_top_inset: t.graph_top_inset,
            graph_label_inset: t.graph_label_inset,
            graph_reach: t.graph_reach,
            graph_bar_height: t.graph_bar_height,
            overview_inset_x: t.overview_inset_x,
            overview_inset_y: t.overview_inset_y,
            overview_box_outset: t.overview_box_outset,
            overview_view_min: t.overview_view_min,
            ruler_marker_band: t.ruler_marker_band,
            ruler_work_height: t.ruler_work_height,
            ruler_label_y: t.ruler_label_y,
            ruler_tick_inset: t.ruler_tick_inset,
            ruler_major_tick: t.ruler_major_tick,
            ruler_minor_tick: t.ruler_minor_tick,
            ruler_render_bar: t.ruler_render_bar,
            ruler_head_half_width: t.ruler_head_half_width,
            ruler_head_height: t.ruler_head_height,
            ruler_cap_width: t.ruler_cap_width,
            scroll_knob_radius: t.scroll_knob_radius,
            scroll_end_zone: t.scroll_end_zone,
            header_width_min: t.header_width_min,
            header_width_max: t.header_width_max,
            caption_row_height: t.caption_row_height,
            header_indent: t.header_indent,
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
