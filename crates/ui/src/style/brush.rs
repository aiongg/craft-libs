//! Sizes and colours of the brush picker, preset cards, chip and settings panel.

use craft_theme::{Rgba, Tokens};

/// Picker height in rows of the large thumbnail.
const PICKER_ROWS: f32 = 8.75;
/// Picker width in large thumbnails.
const PICKER_WIDTH_THUMBS: f32 = 6.25;
const PICKER_MIN_WIDTH: f32 = 230.0;
const PICKER_MIN_HEIGHT: f32 = 170.0;
/// Greatest width of the Size and Hardness block.
const PICKER_HEAD_MAX: f32 = 350.0;
/// Least height of the card list.
const PICKER_LIST_MIN: f32 = 80.0;
/// Least width of the search field and the card scale slider.
const PICKER_FIELD_MIN: f32 = 60.0;
/// Share of the large thumbnail that a card's tip image fills.
const CARD_TIP_FILL: f32 = 0.97;
/// Distance of the size number from the bottom of the tip cell.
const CARD_NUMBER_GAP: f32 = 1.0;
const CARD_STROKE_MIN: f32 = 24.0;
const CARD_STROKE_MAX: f32 = 260.0;
const CARD_MAX: [f32; 2] = [300.0, 70.0];
/// Height of a tip-only and a tip-and-name cell.
const CARD_HEIGHT: f32 = 70.0;
/// Width that a card's name takes beside its tip.
const CARD_NAME_ROOM: f32 = 90.0;
/// Share of the chip's height at which the tip image is centred.
const CHIP_TIP_CENTER_Y: f32 = 0.37;
const CHIP_CHEVRON_SIZE: f32 = 10.0;
/// Width of the Brush Settings content beside the section list.
const SETTINGS_CONTENT_WIDTH: f32 = 354.0;
/// Width of the widest section label.
const SECTION_LABEL_WIDTH: f32 = 134.0;
/// Width of the longest option label in a control select.
const CONTROL_SELECT_LABEL_WIDTH: f32 = 104.0;
/// Chevron width in a control select.
const CONTROL_SELECT_CHEVRON: f32 = 10.0;
/// Radius of the tip ellipse's major axis in large thumbnails.
const ELLIPSE_RADIUS_SHARE: f32 = 0.7;
const TIP_GRID_MAX: f32 = 104.0;
/// Share of `surface.raised` that a preset card's tile keeps, in gamma space.
const CARD_FILL_SHARE: f32 = 0.9;

/// The brush panels' geometry and colours that the semantic tokens don't name directly. The pointer
/// and resize gesture are `canvas::brush::BrushStyle`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushPanelStyle {
    /// The picker's width before the user drags its grip.
    pub picker_width: f32,
    /// The picker's height before the user drags its grip.
    pub picker_height: f32,
    /// The picker's least width.
    pub picker_min_width: f32,
    /// The picker's least height.
    pub picker_min_height: f32,
    /// The Size and Hardness block's greatest width.
    pub picker_head_max: f32,
    /// The least height of the card list.
    pub picker_list_min: f32,
    /// The search field's and the card scale slider's least width.
    pub picker_field_min: f32,
    /// The tip thumbnail's base side.
    pub card_tip_base: f32,
    /// The size number's distance from the bottom of the tip cell.
    pub card_number_gap: f32,
    /// The stroke's least width.
    pub card_stroke_min: f32,
    /// The stroke's greatest width.
    pub card_stroke_max: f32,
    /// A full card, width then height.
    pub card_max: [f32; 2],
    /// A tip-only cell, width then height.
    pub card_tip_only: [f32; 2],
    /// A tip-and-name cell, width then height.
    pub card_tip_name: [f32; 2],
    /// The options-bar chip, width then height.
    pub chip: [f32; 2],
    /// The tip image's centre from the chip's leading top corner.
    pub chip_tip_at: [f32; 2],
    /// The chip's chevron side.
    pub chip_chevron: f32,
    /// The chip's size number, its centre's distance from the chip's top.
    pub chip_number_at: f32,
    /// The Brush Settings panel's preferred width.
    pub panel_width: f32,
    /// The section list's width.
    pub section_list_width: f32,
    /// The stroke strip's height.
    pub strip_height: f32,
    /// The Control select's width in a dynamic control row.
    pub control_select: f32,
    /// The tip ellipse widget's side.
    pub ellipse_size: f32,
    /// The disc's radius.
    pub ellipse_disc: f32,
    /// The ellipse's major radius.
    pub ellipse_radius: f32,
    /// A roundness dot's radius.
    pub ellipse_dot: f32,
    /// The arrow head's length then width.
    pub ellipse_head: [f32; 2],
    /// The grid of tips' greatest height.
    pub tip_grid_max: f32,
    /// A preset card's tile.
    pub card_fill: Rgba,
    /// The stroke strip's fill.
    pub strip_fill: Rgba,
    /// The stroke strip's border.
    pub strip_border: Rgba,
    /// The tip ellipse's disc fill.
    pub ellipse_disc_fill: Rgba,
    /// The tip ellipse's disc border.
    pub ellipse_disc_border: Rgba,
}

impl BrushPanelStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        let (size, space, c) = (&tokens.size, &tokens.space, &tokens.color);
        let thumb = size.thumb.lg;
        let tip_only_width = thumb + space.gap.inline * 2.0;
        let chip = [thumb, size.control.md];
        Self {
            picker_width: thumb * PICKER_WIDTH_THUMBS,
            picker_height: thumb * PICKER_ROWS,
            picker_min_width: PICKER_MIN_WIDTH,
            picker_min_height: PICKER_MIN_HEIGHT,
            picker_head_max: PICKER_HEAD_MAX,
            picker_list_min: PICKER_LIST_MIN,
            picker_field_min: PICKER_FIELD_MIN,
            card_tip_base: thumb * CARD_TIP_FILL,
            card_number_gap: CARD_NUMBER_GAP,
            card_stroke_min: CARD_STROKE_MIN,
            card_stroke_max: CARD_STROKE_MAX,
            card_max: CARD_MAX,
            card_tip_only: [tip_only_width, CARD_HEIGHT],
            card_tip_name: [tip_only_width + CARD_NAME_ROOM, CARD_HEIGHT],
            chip,
            chip_tip_at: [space.gap.inline + size.icon.md / 2.0, chip[1] * CHIP_TIP_CENTER_Y],
            chip_chevron: CHIP_CHEVRON_SIZE,
            chip_number_at: size.control.sm - 1.0,
            panel_width: SETTINGS_CONTENT_WIDTH + SECTION_LABEL_WIDTH + space.pad.control * 2.0 + space.pad.panel * 3.0,
            section_list_width: SECTION_LABEL_WIDTH + space.pad.control * 2.0,
            strip_height: thumb + size.panel_header,
            control_select: CONTROL_SELECT_LABEL_WIDTH + CONTROL_SELECT_CHEVRON + space.pad.control * 2.0,
            ellipse_size: thumb + space.pad.dialog * 2.0,
            ellipse_disc: thumb - space.gap.stack,
            ellipse_radius: thumb * ELLIPSE_RADIUS_SHARE,
            ellipse_dot: size.handle.sm / 2.0,
            ellipse_head: [size.handle.sm, size.handle.md],
            tip_grid_max: TIP_GRID_MAX,
            card_fill: darkened(c.surface.raised, CARD_FILL_SHARE),
            strip_fill: c.surface.sunken,
            strip_border: c.line.control_edge,
            ellipse_disc_fill: c.surface.sunken,
            ellipse_disc_border: c.line.control_edge,
        }
    }
}

/// `c` with each colour channel multiplied by `share`, in gamma (sRGB) space.
fn darkened(c: Rgba, share: f32) -> Rgba {
    let [r, g, b, a] = c.0;
    let k = |v: u8| (f32::from(v) * share + 0.5) as u8;
    Rgba([k(r), k(g), k(b), a])
}
