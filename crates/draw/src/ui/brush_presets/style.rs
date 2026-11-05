//! The look of the brush preset cards, the picker and the chip, from the `brush` token family.

use egui::{Color32, CornerRadius, Vec2};

/// The sizes and colours of a brush preset card at scale 1 (the card multiplies the sizes by its
/// scale). The same at every Size: PhotoCraft's values.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushCardStyle {
    /// Padding inside the tile: `brush.card_pad` (4).
    pub pad: f32,
    /// The tile's inset in its cell, at both ends: `brush.card_inset` (4).
    pub inset: f32,
    /// The tip cell's width: `brush.card_tip_width` (46).
    pub tip_width: f32,
    /// The tip thumbnail's base side: `brush.card_tip_base` (44.8, PhotoCraft's 28 × 1.6).
    pub tip_base: f32,
    /// The name row's height: `brush.card_name_height` (20).
    pub name_height: f32,
    /// The name's inset from the row's ends: `brush.card_name_inset` (6).
    pub name_inset: f32,
    /// A compact card's tip thumbnail sits this far inside its cell's shorter side:
    /// `brush.card_tip_compact_inset` (2).
    pub tip_compact_inset: f32,
    /// The size number's distance from the bottom of the tip cell: `brush.card_number_gap` (1).
    pub number_gap: f32,
    /// The tip thumbnail's distance above the size number: `brush.card_tip_gap` (2).
    pub tip_gap: f32,
    /// The stroke's inset from the tip cell: `brush.card_stroke_inset` (8).
    pub stroke_inset: f32,
    /// The stroke's least and greatest width: `brush.card_stroke_min`, `brush.card_stroke_max`
    /// (24, 260).
    pub stroke_width: [f32; 2],
    /// The stroke's height: `brush.card_stroke_height` (36).
    pub stroke_height: f32,
    /// A full card: `brush.card_max` (300 × 70).
    pub max: Vec2,
    /// A tip-only cell: `brush.card_tip_only` (60 × 70).
    pub tip_only: Vec2,
    /// A tip-and-name cell: `brush.card_tip_name` (150 × 70).
    pub tip_name: Vec2,
    /// The tile's corners: `brush.card_radius` (3).
    pub radius: CornerRadius,
    /// The tile: `brush.card_fill` (`surface.raised` at 90 % in gamma space, opaque).
    pub fill: Color32,
    /// The tile under the pointer: `state.hover`.
    pub hover: Color32,
    /// The current preset's tile: `state.selected` (the ring is the browser's `preset.ring.*`).
    pub selected: Color32,
    /// The name: `text.muted`, `label` role.
    pub name: Color32,
    /// The size number: `text.faint`, `caption` role.
    pub number: Color32,
}

/// The sizes of the Brush Preset picker. The frame, padding and grip are the `Popover`'s.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushPickerStyle {
    /// The size before the user drags the grip: `brush.picker_width` × `brush.picker_height`
    /// (300 × 420).
    pub size: Vec2,
    /// The least size: `brush.picker_min_width` × `brush.picker_min_height` (230 × 170).
    pub min_size: Vec2,
    /// The Size and Hardness block's greatest width: `brush.picker_head_max` (350).
    pub head_max: f32,
    /// The least height of the card list: `brush.picker_list_min` (80).
    pub list_min: f32,
    /// Between the head and the search row, and around the hairline: `brush.picker_gap` (6).
    pub gap: f32,
    /// Room the footer slider leaves for the grip: `brush.picker_grip_room` (20).
    pub grip_room: f32,
    /// The search field's and the card scale slider's least width: `brush.picker_field_min` (60).
    pub field_min: f32,
}

/// The sizes and colours of the options-bar chip.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushPresetChipStyle {
    /// The chip: `brush.chip` (44 × 30).
    pub size: Vec2,
    /// The tip image's side: `brush.chip_tip` (14).
    pub tip: f32,
    /// The tip image's centre from the chip's leading top corner: `brush.chip_tip_at` (14, 11).
    pub tip_at: Vec2,
    /// The chevron's side: `brush.chip_chevron` (10).
    pub chevron: f32,
    /// The size number's centre from the chip's top: `brush.chip_number_at` (25).
    pub number_at: f32,
    /// The chevron's centre from the chip's trailing end: `brush.chip_chevron_inset` (9).
    pub chevron_inset: f32,
    /// The hover and open fill: `state.hover`.
    pub hover: Color32,
    /// The size number: `text.muted`, `caption` role.
    pub number: Color32,
    /// The chevron: `text.faint`.
    pub chevron_color: Color32,
    /// The corners: `radius.control`.
    pub radius: CornerRadius,
}
