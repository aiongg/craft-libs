//! The look of the Brush Settings panel and its shared rows, from the semantic tokens.

use craft_ui::style::brush::BrushPanelStyle;
use craft_ui::theme::{ToEgui, UiTokens};
use egui::{Color32, CornerRadius, Stroke, Vec2};

/// The sizes of the Brush Settings panel, its section list, its stroke strip and the Brush Tip
/// Shape grid of tips.
///
/// The grid of tips is a [`PresetBrowser`](craft_ui::presets::PresetBrowser) the app shows in
/// its Brush Tip Shape pane (no flush, cells `tip_grid_cell`, at most `tip_grid_max` high), each
/// tip drawn `tip_grid_thumb` × [`tip_thumb_scale`](crate::ui::brush_presets::tip_thumb_scale)
/// of its size, with the size as the row's caption.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushSettingsStyle {
    /// The panel's preferred width.
    pub panel_width: f32,
    /// The section list's width.
    pub section_list_width: f32,
    /// A section row's height.
    pub section_row: f32,
    /// Before the first switch row.
    pub section_switch_gap: f32,
    /// The stroke strip's height.
    pub strip_height: f32,
    /// The strip's corner radius: `radius.control`.
    pub strip_radius: CornerRadius,
    /// The strip's fill: `surface.sunken`.
    pub strip_fill: Color32,
    /// The strip's border: `stroke.thin` of `line.control_edge`.
    pub strip_border: Stroke,
    /// The Control select's width in a [`DynamicControlRow`](super::DynamicControlRow).
    pub control_select: f32,
    /// A cell of the grid of tips.
    pub tip_grid_cell: Vec2,
    /// A tip's side in that grid before its size scale.
    pub tip_grid_thumb: f32,
    /// The grid's greatest height.
    pub tip_grid_max: f32,
}

impl BrushSettingsStyle {
    /// The style from `tokens`.
    pub fn of(tokens: &UiTokens) -> Self {
        let b = BrushPanelStyle::new(tokens);
        BrushSettingsStyle {
            panel_width: b.panel_width,
            section_list_width: b.section_list_width,
            section_row: tokens.size.control.md,
            section_switch_gap: tokens.space.gap.inline,
            strip_height: b.strip_height,
            strip_radius: tokens.radius.control,
            strip_fill: b.strip_fill.to_egui(),
            strip_border: Stroke::new(tokens.stroke.thin, b.strip_border.to_egui()),
            control_select: b.control_select,
            tip_grid_cell: Vec2::new(tokens.size.thumb.lg, tokens.size.thumb.lg + tokens.space.gap.tight),
            tip_grid_thumb: tokens.size.thumb.md,
            tip_grid_max: b.tip_grid_max,
        }
    }
}

/// The sizes and colours of the [`TipEllipse`](super::TipEllipse).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TipEllipseStyle {
    /// The widget's side.
    pub size: f32,
    /// The disc's radius.
    pub disc: f32,
    /// The ellipse's major radius.
    pub radius: f32,
    /// A roundness dot's radius.
    pub dot: f32,
    /// How near a dot a press grabs it.
    pub grab: f32,
    /// The ellipse outline and arrow width.
    pub line: f32,
    /// How far the arrow reaches past the ellipse.
    pub arrow_reach: f32,
    /// The arrow head's length and width.
    pub head: Vec2,
    /// The disc's fill: `surface.sunken`.
    pub disc_fill: Color32,
    /// The disc's border: `stroke.thin` of `line.control_edge`.
    pub disc_border: Stroke,
    /// The cross lines: 1 pt `line.divider`.
    pub cross: Stroke,
    /// The ellipse's outline: `text.default`; its fill is this at 18 %.
    pub ellipse: Color32,
    /// The arrow and the dots' rings: `accent`.
    pub accent: Color32,
    /// The dots: `overlay.handle_fill`.
    pub dot_fill: Color32,
}
