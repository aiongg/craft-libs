//! The look of the brush preset cards, the picker and the chip, from the `brush` token family.

use egui::{Color32, CornerRadius, Vec2};

/// The sizes and colours of a brush preset card at scale 1 (the card multiplies the sizes by its
/// scale). The same at every Size: PhotoCraft's values.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushCardStyle {
    /// Padding inside the tile.
    pub pad: f32,
    /// The tile's inset in its cell, at both ends.
    pub inset: f32,
    /// The tip cell's width.
    pub tip_width: f32,
    /// The tip thumbnail's base side.
    pub tip_base: f32,
    /// The name row's height.
    pub name_height: f32,
    /// The name's inset from the row's ends.
    pub name_inset: f32,
    /// A compact card's tip thumbnail sits this far inside its cell's shorter side.
    pub tip_compact_inset: f32,
    /// The size number's distance from the bottom of the tip cell.
    pub number_gap: f32,
    /// The tip thumbnail's distance above the size number.
    pub tip_gap: f32,
    /// The stroke's inset from the tip cell.
    pub stroke_inset: f32,
    /// The stroke's least and greatest width.
    pub stroke_width: [f32; 2],
    /// The stroke's height.
    pub stroke_height: f32,
    /// A full card.
    pub max: Vec2,
    /// A tip-only cell.
    pub tip_only: Vec2,
    /// A tip-and-name cell.
    pub tip_name: Vec2,
    /// The tile's corners.
    pub radius: CornerRadius,
    /// The tile.
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
    /// The size before the user drags the grip.
    pub size: Vec2,
    /// The least size.
    pub min_size: Vec2,
    /// The Size and Hardness block's greatest width.
    pub head_max: f32,
    /// The least height of the card list.
    pub list_min: f32,
    /// Between the head and the search row, and around the hairline.
    pub gap: f32,
    /// Room the footer slider leaves for the grip.
    pub grip_room: f32,
    /// The search field's and the card scale slider's least width.
    pub field_min: f32,
}

/// The sizes and colours of the options-bar chip.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushPresetChipStyle {
    /// The chip.
    pub size: Vec2,
    /// The tip image's side.
    pub tip: f32,
    /// The tip image's centre from the chip's leading top corner.
    pub tip_at: Vec2,
    /// The chevron's side.
    pub chevron: f32,
    /// The size number's centre from the chip's top.
    pub number_at: f32,
    /// The chevron's centre from the chip's trailing end.
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
