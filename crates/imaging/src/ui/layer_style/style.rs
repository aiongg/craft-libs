//! The looks of the Layer Style dialog, the Blend If bar and the contour field, from tokens (task
//! TK3: token families `layer_style`, `blend_if`).

use egui::{Color32, FontId};

use craft_theme::TypeRole;
use craft_ui::theme::{ToEgui, UiTokens};

use super::{BlendIfBar, ContourField, LayerStyleDialog};

/// The dialog's sizes and colours beside the library controls it uses (`list.*`, `checkbox.*`,
/// `slider_row.*`, the preset browser's own).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct LayerStyleDialogStyle {
    /// The effect list's width: `layer_style.list_width` (190, PhotoCraft `layer_style.rs:840`).
    pub list_width: f32,
    /// The page column's width: `layer_style.page_width` (330, `:942`).
    pub page_width: f32,
    /// The divider between the list and the page: `line.divider`.
    pub divider: Color32,
    /// A page title (the effect's name): `type.title` (PhotoCraft semibold 14, `:987`).
    pub title_font: FontId,
    pub title_color: Color32,
    /// The label of an absent or switched-off effect: `text.faint` (absent), `text.muted` (off,
    /// #2288).
    pub absent_color: Color32,
    pub off_color: Color32,
    /// The Styles page's thumbnails: `layer_style.preset_thumb` (32, `:962`); the page's scroll
    /// height `layer_style.presets_height` (340, `:954`). The grid's gap and radius are the preset
    /// browser's.
    pub preset_thumb: f32,
    pub presets_height: f32,
    /// The invalid-value line: `status.bad.fg`.
    pub invalid_color: Color32,
}

/// The Blend If bar's sizes and colours. The handle faces (grey 20 and 240) are image tones and
/// stay constants in code.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlendIfBarStyle {
    /// The bar's width range: `blend_if.width_min`, `blend_if.width_max` (120 and 320, PhotoCraft
    /// `layer_style.rs:1370`).
    pub width_min: f32,
    pub width_max: f32,
    /// The bar's height: `blend_if.height` (26, `:1371`).
    pub height: f32,
    /// The ramp's height and side inset: `blend_if.ramp` (12), `blend_if.inset` (6) (`:1372`).
    pub ramp: f32,
    pub inset: f32,
    /// A handle triangle's width and height: `blend_if.handle_width`, `blend_if.handle_height`
    /// (10 and 10, `:1432-1440`).
    pub handle_width: f32,
    pub handle_height: f32,
    /// The ramp's border: `blend_if.ramp_border` (`:1383`).
    pub ramp_border: Color32,
    /// The handles' edge: `text.muted` (3:1 against the panel, as `levels.thumb_edge`).
    pub handle_edge: Color32,
    /// The focus ring around the focused handle: `line.focus`.
    pub focus: Color32,
    /// The readout over the bar ("0 / 20    255"): `type.body`, `text.normal`.
    pub readout_font: FontId,
    pub readout_color: Color32,
}

/// The contour field's sizes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ContourFieldStyle {
    /// The thumbnail: `layer_style.contour_thumb` (36, PhotoCraft `contour.rs:111`); the picker
    /// arrow beside it `layer_style.contour_arrow` (14 wide, `:119`).
    pub thumb: f32,
    pub arrow: f32,
    /// A cell of the preset picker: `layer_style.contour_cell` (40, `:142`).
    pub cell: f32,
    /// The Contour Editor's graph: `layer_style.contour_graph` (220, `:358`).
    pub graph: f32,
    /// The curve in a thumbnail: `text.normal`; the thumbnail's border `layer_style.contour_border`;
    /// its fill `surface.sunken`.
    pub curve: Color32,
    pub border: Color32,
    pub fill: Color32,
}

impl LayerStyleDialog<'_> {
    /// The dialog's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> LayerStyleDialogStyle {
        // pc: crates/ui-egui/src/dialogs/layer_style.rs:840 (190), :942 (330), :954 (340), :962 (32),
        // :965-969 (accent 1.5 outline, radius 2), :987 (semibold 14).
        let (l, c) = (&tokens.component.layer_style, &tokens.color);
        LayerStyleDialogStyle {
            list_width: l.list_width,
            page_width: l.page_width,
            divider: c.line.divider.to_egui(),
            title_font: tokens.font(TypeRole::Title),
            title_color: c.text.normal.to_egui(),
            absent_color: c.text.faint.to_egui(),
            off_color: c.text.muted.to_egui(),
            preset_thumb: l.preset_thumb,
            presets_height: l.presets_height,
            invalid_color: c.status.bad.fg.to_egui(),
        }
    }
}

impl BlendIfBar {
    /// The bar's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> BlendIfBarStyle {
        // pc: crates/ui-egui/src/dialogs/layer_style.rs:1370-1372 (120..320, 26, ramp 12 inset 6),
        // :1383 (field border), :1432-1440 (triangles 10 high, 5 half-wide).
        let (b, c) = (&tokens.component.blend_if, &tokens.color);
        BlendIfBarStyle {
            width_min: b.width_min,
            width_max: b.width_max,
            height: b.height,
            ramp: b.ramp,
            inset: b.inset,
            handle_width: b.handle_width,
            handle_height: b.handle_height,
            ramp_border: b.ramp_border.to_egui(),
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
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:111 (36), :119 (14), :142 (40),
        // :358 (220).
        let (l, c) = (&tokens.component.layer_style, &tokens.color);
        ContourFieldStyle {
            thumb: l.contour_thumb,
            arrow: l.contour_arrow,
            cell: l.contour_cell,
            graph: l.contour_graph,
            curve: c.text.normal.to_egui(),
            border: l.contour_border.to_egui(),
            fill: c.surface.sunken.to_egui(),
        }
    }
}
