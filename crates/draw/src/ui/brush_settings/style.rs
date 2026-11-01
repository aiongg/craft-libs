//! The look of the Brush Settings panel and its shared rows, from the `brush` token family.
#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::theme::UiTokens;
use egui::{Color32, Stroke, Vec2};

/// The sizes of the Brush Settings panel, its section list, its stroke strip and the Brush Tip
/// Shape grid of tips. The same at every Size: PhotoCraft's values.
///
/// The grid of tips is a [`PresetBrowser`](craft_ui::presets::PresetBrowser) the app shows in
/// its Brush Tip Shape pane (no flush, cells `tip_grid_cell`, at most `tip_grid_max` high), each
/// tip drawn `tip_grid_thumb` × [`tip_thumb_scale`](crate::ui::brush_presets::tip_thumb_scale)
/// of its size, with the size as the row's caption.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushSettingsStyle {
    /// The panel's preferred width: `brush.panel_width` (540).
    pub panel_width: f32,
    /// The section list's width: `brush.section_list_width` (150).
    pub section_list_width: f32,
    /// A section row's height: `brush.section_row` (23).
    pub section_row: f32,
    /// The enable box's side: `brush.section_box` (13).
    pub section_box: f32,
    /// The lock's hit area and icon: `brush.section_lock` (18, 12).
    pub section_lock: [f32; 2],
    /// Where the names start: `brush.section_name_inset` (20).
    pub section_name_inset: f32,
    /// Before the first switch row: `brush.section_switch_gap` (6).
    pub section_switch_gap: f32,
    /// The lock icon when locked: `text.default`.
    pub lock_on: Color32,
    /// The lock icon when open: `text.faint`.
    pub lock_off: Color32,
    /// The stroke strip's height: `brush.strip_height` (76).
    pub strip_height: f32,
    /// The strip's fill: `field.fill`.
    pub strip_fill: Color32,
    /// The strip's border: 1 pt `field.border`.
    pub strip_border: Stroke,
    /// The line between the list and the pane, and the hairline above the strip: `line.divider`.
    pub divider: Stroke,
    /// The Control select's width in a [`DynamicControlRow`](super::DynamicControlRow):
    /// `brush.control_select` (130).
    pub control_select: f32,
    /// A cell of the grid of tips: `brush.tip_grid_cell` (44 × 50).
    pub tip_grid_cell: Vec2,
    /// A tip's side in that grid before its size scale: `brush.tip_grid_thumb` (34).
    pub tip_grid_thumb: f32,
    /// The grid's greatest height: `brush.tip_grid_max` (104).
    pub tip_grid_max: f32,
}

impl BrushSettingsStyle {
    /// The style from `tokens`.
    pub fn of(tokens: &UiTokens) -> Self {
        // new: token task TK1 (pc: crates/ui-egui/src/panels/brush_panel.rs:56-57, :250-330; brush_sections.rs:266-282).
        todo!()
    }
}

/// The sizes and colours of the [`TipEllipse`](super::TipEllipse).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TipEllipseStyle {
    /// The widget's side: `brush.ellipse_size` (84).
    pub size: f32,
    /// The disc's radius: `brush.ellipse_disc` (40).
    pub disc: f32,
    /// The ellipse's major radius: `brush.ellipse_radius` (34).
    pub radius: f32,
    /// A roundness dot's radius: `brush.ellipse_dot` (4).
    pub dot: f32,
    /// How near a dot a press grabs it: `brush.ellipse_grab` (7).
    pub grab: f32,
    /// The ellipse outline and arrow width: `brush.ellipse_line` (1.5).
    pub line: f32,
    /// How far the arrow reaches past the ellipse: `brush.ellipse_arrow_reach` (5).
    pub arrow_reach: f32,
    /// The arrow head's length and width: `brush.ellipse_head` (7, 8).
    pub head: Vec2,
    /// The disc: `field.fill`, with a 1 pt `field.border`.
    pub disc_fill: Color32,
    /// The disc's border.
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

impl TipEllipseStyle {
    /// The style from `tokens`.
    pub fn of(tokens: &UiTokens) -> Self {
        // new: token task TK1 (pc: crates/ui-egui/src/panels/brush_sections.rs:192-242).
        todo!()
    }
}
