//! [`NavigatorStyle`]: the Navigator's look, from tokens.

use egui::{Color32, CornerRadius};

use super::preview::Navigator;
use crate::theme::{ToEgui as _, UiTokens};

/// The proxy rectangle's colour when the host chooses none.
pub const DEFAULT_PROXY_COLOR: Color32 = Color32::from_rgb(255, 84, 84);
/// The proxy's default stroke width, centred on its edge.
pub const DEFAULT_PROXY_WIDTH: f32 = 1.5;
const MIN_PREVIEW: f32 = 40.0;

/// The Navigator's sizes and colours. [`Navigator::style`] reads them from the tokens; a widget
/// never takes a literal size from app code.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct NavigatorStyle {
    /// The preview box's fill (`surface.canvas`, PhotoCraft's `t.canvas`).
    pub fill: Color32,
    /// The preview box's corner radius (`shape.radius.control`).
    pub radius: CornerRadius,
    /// Space between the box and the fitted document on each side: `space.pad.control`.
    pub inset: f32,
    /// The least height of the preview box.
    pub min_preview: f32,
    /// The proxy rectangle's colour when [`ProxyColor::Default`](super::ProxyColor::Default):
    pub proxy: Color32,
    /// The proxy's stroke width, centred on its edge.
    pub proxy_width: f32,
    /// The proxy's corner radius on an upright view: `shape.radius.sm`.
    pub proxy_radius: f32,
    /// The 1 pt outline under the proxy, so it reads on any image: `overlay.halo`.
    pub halo: Color32,
    /// The halo's width, also the margin the proxy's edges are clipped to: `stroke.thin`.
    pub halo_width: f32,
    /// The side of a checkerboard square behind transparent pixels: `size.checker`.
    pub checker: f32,
    /// The two checkerboard colours (`Checkerboard`'s default pair).
    pub checker_colors: [Color32; 2],
    /// The gap above the zoom row and between its controls: `space.gap.row`.
    pub row_gap: f32,
}

impl Navigator<'_> {
    /// The Navigator's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> NavigatorStyle {
        NavigatorStyle {
            fill: tokens.color.surface.canvas.to_egui(),
            radius: tokens.radius.control,
            inset: tokens.space.pad.control,
            min_preview: MIN_PREVIEW,
            proxy: DEFAULT_PROXY_COLOR,
            proxy_width: DEFAULT_PROXY_WIDTH,
            proxy_radius: tokens.shape.radius.sm,
            halo: tokens.color.overlay.halo.to_egui(),
            halo_width: tokens.stroke.thin,
            checker: tokens.size.checker,
            checker_colors: crate::canvas::CheckerColors::default().colors(),
            row_gap: tokens.space.gap.row,
        }
    }
}
