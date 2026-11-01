//! Brushes: the cursor's thresholds, the resize gesture, the preset cards, the picker, the options-bar chip, the Brush Settings panel and the tip ellipse. PhotoCraft's values, the same at every Size.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Brushes: the cursor, resize gesture, preset cards, picker, chip, settings panel and tip ellipse.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushTokens {
    /// Screen points of vertical resize travel from level 0 to 1: `brush.resize_travel`.
    pub resize_travel: f32,
    /// Below this on-screen radius the outline is unreadable and the crosshair shows: `brush.tip_min_radius`.
    pub tip_min_radius: f32,
    /// The arm of that crosshair: `brush.tip_min_crosshair`.
    pub tip_min_crosshair: f32,
    /// The picker's width before the user drags its grip: `brush.picker_width`.
    pub picker_width: f32,
    /// Its height: `brush.picker_height`.
    pub picker_height: f32,
    /// Its least width: `brush.picker_min_width`.
    pub picker_min_width: f32,
    /// Its least height: `brush.picker_min_height`.
    pub picker_min_height: f32,
    /// The Size and Hardness block's greatest width: `brush.picker_head_max`.
    pub picker_head_max: f32,
    /// The least height of the card list: `brush.picker_list_min`.
    pub picker_list_min: f32,
    /// Between the head and the search row, and around the hairline: `brush.picker_gap`.
    pub picker_gap: f32,
    /// Room the footer slider leaves for the grip: `brush.picker_grip_room`.
    pub picker_grip_room: f32,
    /// Padding inside a card's tile: `brush.card_pad`.
    pub card_pad: f32,
    /// The tile's inset in its cell, at both ends: `brush.card_inset`.
    pub card_inset: f32,
    /// The tip cell's width: `brush.card_tip_width`.
    pub card_tip_width: f32,
    /// The tip thumbnail's base side: `brush.card_tip_base`.
    pub card_tip_base: f32,
    /// The name row's height: `brush.card_name_height`.
    pub card_name_height: f32,
    /// The name's inset from the row's ends: `brush.card_name_inset`.
    pub card_name_inset: f32,
    /// The stroke's inset from the tip cell: `brush.card_stroke_inset`.
    pub card_stroke_inset: f32,
    /// The stroke's least width: `brush.card_stroke_min`.
    pub card_stroke_min: f32,
    /// The stroke's greatest width: `brush.card_stroke_max`.
    pub card_stroke_max: f32,
    /// The stroke's height: `brush.card_stroke_height`.
    pub card_stroke_height: f32,
    /// A full card, width then height: `brush.card_max`.
    pub card_max: [f32; 2],
    /// A tip-only cell, width then height: `brush.card_tip_only`.
    pub card_tip_only: [f32; 2],
    /// A tip-and-name cell, width then height: `brush.card_tip_name`.
    pub card_tip_name: [f32; 2],
    /// The card tile's corner radius: `brush.card_radius`.
    pub card_radius: f32,
    /// The options-bar chip, width then height: `brush.chip`.
    pub chip: [f32; 2],
    /// The chip's tip image side: `brush.chip_tip`.
    pub chip_tip: f32,
    /// The tip image's centre from the chip's leading top corner: `brush.chip_tip_at`.
    pub chip_tip_at: [f32; 2],
    /// The chip's chevron side: `brush.chip_chevron`.
    pub chip_chevron: f32,
    /// The Brush Settings panel's preferred width: `brush.panel_width`.
    pub panel_width: f32,
    /// The section list's width: `brush.section_list_width`.
    pub section_list_width: f32,
    /// A section row's height: `brush.section_row`.
    pub section_row: f32,
    /// The enable box's side: `brush.section_box`.
    pub section_box: f32,
    /// The lock's hit area then its icon: `brush.section_lock`.
    pub section_lock: [f32; 2],
    /// Where section names start: `brush.section_name_inset`.
    pub section_name_inset: f32,
    /// Before the first switch row: `brush.section_switch_gap`.
    pub section_switch_gap: f32,
    /// The stroke strip's height: `brush.strip_height`.
    pub strip_height: f32,
    /// The Control select's width in a dynamic control row: `brush.control_select`.
    pub control_select: f32,
    /// The tip ellipse widget's side: `brush.ellipse_size`.
    pub ellipse_size: f32,
    /// The disc's radius: `brush.ellipse_disc`.
    pub ellipse_disc: f32,
    /// The ellipse's major radius: `brush.ellipse_radius`.
    pub ellipse_radius: f32,
    /// A roundness dot's radius: `brush.ellipse_dot`.
    pub ellipse_dot: f32,
    /// How near a dot a press grabs it: `brush.ellipse_grab`.
    pub ellipse_grab: f32,
    /// The ellipse outline and arrow width: `brush.ellipse_line`.
    pub ellipse_line: f32,
    /// How far the arrow reaches past the ellipse: `brush.ellipse_arrow_reach`.
    pub ellipse_arrow_reach: f32,
    /// The arrow head's length then width: `brush.ellipse_head`.
    pub ellipse_head: [f32; 2],
    /// A cell of the grid of tips, width then height: `brush.tip_grid_cell`.
    pub tip_grid_cell: [f32; 2],
    /// A tip's side in that grid before its size scale: `brush.tip_grid_thumb`.
    pub tip_grid_thumb: f32,
    /// The grid's greatest height: `brush.tip_grid_max`.
    pub tip_grid_max: f32,
    /// The resize preview's default tint: `brush.preview_tint`.
    pub preview_tint: Rgba,
    /// A preset card's tile: `brush.card_fill`, `surface.raised` darkened to 90 % in gamma space.
    pub card_fill: Rgba,
}

impl BrushTokens {
    fn color_mut(&mut self, f: BrushField) -> &mut Rgba {
        match f {
            BrushField::PreviewTint => &mut self.preview_tint,
            BrushField::CardFill => &mut self.card_fill,
        }
    }
}

/// An overridable brush token. Paths: `preview_tint`, `card_fill`, e.g. `brush.preview_tint`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BrushField {
    /// `brush.preview_tint`
    PreviewTint,
    /// `brush.card_fill`
    CardFill,
}

impl BrushField {
    const fn path(self) -> &'static str {
        match self {
            BrushField::PreviewTint => "preview_tint",
            BrushField::CardFill => "card_fill",
        }
    }
}

impl Family for BrushTokens {
    type Field = BrushField;
    const FIELDS: &'static [BrushField] = &[BrushField::PreviewTint, BrushField::CardFill];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { BrushTokens;
        "resize_travel" 0.0..=1024.0 => resize_travel;
        "tip_min_radius" 0.0..=64.0 => tip_min_radius;
        "tip_min_crosshair" 0.0..=64.0 => tip_min_crosshair;
        "picker_width" 0.0..=1024.0 => picker_width;
        "picker_height" 0.0..=1024.0 => picker_height;
        "picker_min_width" 0.0..=1024.0 => picker_min_width;
        "picker_min_height" 0.0..=1024.0 => picker_min_height;
        "picker_head_max" 0.0..=1024.0 => picker_head_max;
        "picker_list_min" 0.0..=1024.0 => picker_list_min;
        "picker_gap" 0.0..=64.0 => picker_gap;
        "picker_grip_room" 0.0..=128.0 => picker_grip_room;
        "card_pad" 0.0..=64.0 => card_pad;
        "card_inset" 0.0..=64.0 => card_inset;
        "card_tip_width" 0.0..=1024.0 => card_tip_width;
        "card_tip_base" 0.0..=1024.0 => card_tip_base;
        "card_name_height" 0.0..=1024.0 => card_name_height;
        "card_name_inset" 0.0..=64.0 => card_name_inset;
        "card_stroke_inset" 0.0..=64.0 => card_stroke_inset;
        "card_stroke_min" 0.0..=1024.0 => card_stroke_min;
        "card_stroke_max" 0.0..=1024.0 => card_stroke_max;
        "card_stroke_height" 0.0..=1024.0 => card_stroke_height;
        "card_max.0" 0.0..=1024.0 => card_max[0];
        "card_max.1" 0.0..=1024.0 => card_max[1];
        "card_tip_only.0" 0.0..=1024.0 => card_tip_only[0];
        "card_tip_only.1" 0.0..=1024.0 => card_tip_only[1];
        "card_tip_name.0" 0.0..=1024.0 => card_tip_name[0];
        "card_tip_name.1" 0.0..=1024.0 => card_tip_name[1];
        "card_radius" 0.0..=64.0 => card_radius;
        "chip.0" 0.0..=1024.0 => chip[0];
        "chip.1" 0.0..=1024.0 => chip[1];
        "chip_tip" 0.0..=1024.0 => chip_tip;
        "chip_tip_at.0" 0.0..=1024.0 => chip_tip_at[0];
        "chip_tip_at.1" 0.0..=1024.0 => chip_tip_at[1];
        "chip_chevron" 0.0..=128.0 => chip_chevron;
        "panel_width" 0.0..=1024.0 => panel_width;
        "section_list_width" 0.0..=1024.0 => section_list_width;
        "section_row" 0.0..=1024.0 => section_row;
        "section_box" 0.0..=64.0 => section_box;
        "section_lock.0" 0.0..=128.0 => section_lock[0];
        "section_lock.1" 0.0..=128.0 => section_lock[1];
        "section_name_inset" 0.0..=128.0 => section_name_inset;
        "section_switch_gap" 0.0..=64.0 => section_switch_gap;
        "strip_height" 0.0..=1024.0 => strip_height;
        "control_select" 0.0..=1024.0 => control_select;
        "ellipse_size" 0.0..=1024.0 => ellipse_size;
        "ellipse_disc" 0.0..=1024.0 => ellipse_disc;
        "ellipse_radius" 0.0..=1024.0 => ellipse_radius;
        "ellipse_dot" 0.0..=64.0 => ellipse_dot;
        "ellipse_grab" 0.0..=64.0 => ellipse_grab;
        "ellipse_line" 0.0..=16.0 => ellipse_line;
        "ellipse_arrow_reach" 0.0..=64.0 => ellipse_arrow_reach;
        "ellipse_head.0" 0.0..=64.0 => ellipse_head[0];
        "ellipse_head.1" 0.0..=64.0 => ellipse_head[1];
        "tip_grid_cell.0" 0.0..=1024.0 => tip_grid_cell[0];
        "tip_grid_cell.1" 0.0..=1024.0 => tip_grid_cell[1];
        "tip_grid_thumb" 0.0..=1024.0 => tip_grid_thumb;
        "tip_grid_max" 0.0..=1024.0 => tip_grid_max;
    };

    fn defaults(s: &Semantic<'_>) -> BrushTokens {
        let c = s.color;
        BrushTokens {
            resize_travel: values::BRUSH_RESIZE_TRAVEL,
            tip_min_radius: values::BRUSH_TIP_MIN_RADIUS,
            tip_min_crosshair: values::BRUSH_TIP_MIN_CROSSHAIR,
            picker_width: values::BRUSH_PICKER_WIDTH,
            picker_height: values::BRUSH_PICKER_HEIGHT,
            picker_min_width: values::BRUSH_PICKER_MIN_WIDTH,
            picker_min_height: values::BRUSH_PICKER_MIN_HEIGHT,
            picker_head_max: values::BRUSH_PICKER_HEAD_MAX,
            picker_list_min: values::BRUSH_PICKER_LIST_MIN,
            picker_gap: values::BRUSH_PICKER_GAP,
            picker_grip_room: values::BRUSH_PICKER_GRIP_ROOM,
            card_pad: values::BRUSH_CARD_PAD,
            card_inset: values::BRUSH_CARD_INSET,
            card_tip_width: values::BRUSH_CARD_TIP_WIDTH,
            card_tip_base: values::BRUSH_CARD_TIP_BASE,
            card_name_height: values::BRUSH_CARD_NAME_HEIGHT,
            card_name_inset: values::BRUSH_CARD_NAME_INSET,
            card_stroke_inset: values::BRUSH_CARD_STROKE_INSET,
            card_stroke_min: values::BRUSH_CARD_STROKE_MIN,
            card_stroke_max: values::BRUSH_CARD_STROKE_MAX,
            card_stroke_height: values::BRUSH_CARD_STROKE_HEIGHT,
            card_max: values::BRUSH_CARD_MAX,
            card_tip_only: values::BRUSH_CARD_TIP_ONLY,
            card_tip_name: values::BRUSH_CARD_TIP_NAME,
            card_radius: values::BRUSH_CARD_RADIUS,
            chip: values::BRUSH_CHIP,
            chip_tip: values::BRUSH_CHIP_TIP,
            chip_tip_at: values::BRUSH_CHIP_TIP_AT,
            chip_chevron: values::BRUSH_CHIP_CHEVRON,
            panel_width: values::BRUSH_PANEL_WIDTH,
            section_list_width: values::BRUSH_SECTION_LIST_WIDTH,
            section_row: values::BRUSH_SECTION_ROW,
            section_box: values::BRUSH_SECTION_BOX,
            section_lock: values::BRUSH_SECTION_LOCK,
            section_name_inset: values::BRUSH_SECTION_NAME_INSET,
            section_switch_gap: values::BRUSH_SECTION_SWITCH_GAP,
            strip_height: values::BRUSH_STRIP_HEIGHT,
            control_select: values::BRUSH_CONTROL_SELECT,
            ellipse_size: values::BRUSH_ELLIPSE_SIZE,
            ellipse_disc: values::BRUSH_ELLIPSE_DISC,
            ellipse_radius: values::BRUSH_ELLIPSE_RADIUS,
            ellipse_dot: values::BRUSH_ELLIPSE_DOT,
            ellipse_grab: values::BRUSH_ELLIPSE_GRAB,
            ellipse_line: values::BRUSH_ELLIPSE_LINE,
            ellipse_arrow_reach: values::BRUSH_ELLIPSE_ARROW_REACH,
            ellipse_head: values::BRUSH_ELLIPSE_HEAD,
            tip_grid_cell: values::BRUSH_TIP_GRID_CELL,
            tip_grid_thumb: values::BRUSH_TIP_GRID_THUMB,
            tip_grid_max: values::BRUSH_TIP_GRID_MAX,
            preview_tint: Rgba::rgb(0xff, 0x00, 0x00).with_alpha(values::BRUSH_PREVIEW_ALPHA),
            card_fill: values::darkened(c.surface.raised, values::BRUSH_CARD_FILL_SHARE),
        }
    }

    fn path(f: BrushField) -> String {
        f.path().to_owned()
    }

    fn from_path(path: &str) -> Option<BrushField> {
        Self::FIELDS.iter().copied().find(|f| f.path() == path)
    }

    fn kind(_: BrushField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: BrushField) -> OverrideValue {
        let mut copy = *self;
        OverrideValue::Color(*copy.color_mut(f))
    }

    fn set(&mut self, f: BrushField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            *self.color_mut(f) = c;
        }
    }

    // No contrast pairs: these are fills and lines on the document or beside it.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
