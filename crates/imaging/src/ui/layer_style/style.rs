//! The looks of the Layer Style dialog, the Blend If bar and the contour field, from tokens.

use egui::{Color32, FontId};

use craft_theme::TypeRole;
use craft_ui::theme::{ToEgui, UiTokens};

use super::{BlendIfBar, ContourField, LayerStyleDialog};

/// The effect list's width, in control heights.
const LIST_ROWS: f32 = 8.0;
/// The page column's width, in control heights.
const PAGE_ROWS: f32 = 13.75;
/// The Styles page's scroll height, in control heights.
const PRESETS_ROWS: f32 = 14.0;
/// The Contour Editor's graph side, as a share of the page width.
const GRAPH_SHARE: f32 = 2.0 / 3.0;
/// The narrowest Blend If bar.
const BLEND_IF_WIDTH_MIN: f32 = 120.0;
/// The widest Blend If bar.
const BLEND_IF_WIDTH_MAX: f32 = 320.0;

/// The dialog's sizes and colours beside the library controls it uses (`list.*`, `checkbox.*`,
/// `slider_row.*`, the preset browser's own).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct LayerStyleDialogStyle {
    /// The effect list's width: 8 control heights.
    pub list_width: f32,
    /// The page column's width: 13.75 control heights.
    pub page_width: f32,
    /// The divider between the list and the page: `line.divider`.
    pub divider: Color32,
    /// A page title (the effect's name): `type.title`.
    pub title_font: FontId,
    #[allow(missing_docs)]
    pub title_color: Color32,
    /// The label of an absent or switched-off effect: `text.faint` (absent), `text.muted` (off,
    /// #2288).
    pub absent_color: Color32,
    #[allow(missing_docs)]
    pub off_color: Color32,
    /// The Styles page's thumbnails: `size.thumb.sm`; the page's scroll
    /// height 14 control heights. The grid's gap and radius are the preset
    /// browser's.
    pub preset_thumb: f32,
    #[allow(missing_docs)]
    pub presets_height: f32,
    /// The invalid-value line: `status.bad.fg`.
    pub invalid_color: Color32,
}

/// The Blend If bar's sizes and colours. The handle faces (grey 20 and 240) are image tones and
/// stay constants in code.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlendIfBarStyle {
    /// The bar's width range: 120 to 320.
    pub width_min: f32,
    #[allow(missing_docs)]
    pub width_max: f32,
    /// The bar's height: `size.control.sm`.
    pub height: f32,
    /// The ramp's height and side inset: `size.icon.sm`, `space.gap.inline`.
    pub ramp: f32,
    #[allow(missing_docs)]
    pub inset: f32,
    /// A handle triangle's width and height: `size.handle.md` each.
    pub handle_width: f32,
    #[allow(missing_docs)]
    pub handle_height: f32,
    /// The ramp's border: `line.control_edge`.
    pub ramp_border: Color32,
    /// The handles' edge: `text.muted` (3:1 against the panel, as a Levels thumb's).
    pub handle_edge: Color32,
    /// The focus ring around the focused handle: `line.focus`.
    pub focus: Color32,
    /// The readout over the bar ("0 / 20    255"): `type.body`, `text.normal`.
    pub readout_font: FontId,
    #[allow(missing_docs)]
    pub readout_color: Color32,
}

/// The contour field's sizes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ContourFieldStyle {
    /// The thumbnail: `size.thumb.md`; the picker arrow beside it `size.icon.md` wide.
    pub thumb: f32,
    #[allow(missing_docs)]
    pub arrow: f32,
    /// A cell of the preset picker: `size.thumb.md`.
    pub cell: f32,
    /// The Contour Editor's graph: two thirds of the page width.
    pub graph: f32,
}

impl LayerStyleDialog<'_> {
    /// The dialog's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> LayerStyleDialogStyle {
        let c = &tokens.color;
        LayerStyleDialogStyle {
            list_width: tokens.size.control.md * LIST_ROWS,
            page_width: tokens.size.control.md * PAGE_ROWS,
            divider: c.line.divider.to_egui(),
            title_font: tokens.font(TypeRole::Title),
            title_color: c.text.normal.to_egui(),
            absent_color: c.text.faint.to_egui(),
            off_color: c.text.muted.to_egui(),
            preset_thumb: tokens.size.thumb.sm,
            presets_height: tokens.size.control.md * PRESETS_ROWS,
            invalid_color: c.status.bad.fg.to_egui(),
        }
    }
}

impl BlendIfBar {
    /// The bar's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> BlendIfBarStyle {
        let c = &tokens.color;
        BlendIfBarStyle {
            width_min: BLEND_IF_WIDTH_MIN,
            width_max: BLEND_IF_WIDTH_MAX,
            height: tokens.size.control.sm,
            ramp: tokens.size.icon.sm,
            inset: tokens.space.gap.inline,
            handle_width: tokens.size.handle.md,
            handle_height: tokens.size.handle.md,
            ramp_border: c.line.control_edge.to_egui(),
            handle_edge: c.text.muted.to_egui(),
            focus: c.line.focus.to_egui(),
            readout_font: tokens.font(TypeRole::Body),
            readout_color: c.text.normal.to_egui(),
        }
    }
}

impl ContourField<'_> {
    /// The field's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> ContourFieldStyle {
        ContourFieldStyle {
            thumb: tokens.size.thumb.md,
            arrow: tokens.size.icon.md,
            cell: tokens.size.thumb.md,
            graph: tokens.size.control.md * PAGE_ROWS * GRAPH_SHARE,
        }
    }
}
