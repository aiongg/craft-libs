//! [`NavigatorStyle`]: the Navigator's look, from tokens.

use egui::{Color32, CornerRadius};

use super::preview::Navigator;
use crate::theme::{ToEgui as _, UiTokens};

/// The Navigator's sizes and colours. [`Navigator::style`] reads them from the tokens; a widget
/// never takes a literal size from app code.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct NavigatorStyle {
    /// The preview box's fill (`surface.canvas`, PhotoCraft's `t.canvas`).
    pub fill: Color32,
    /// The preview box's corner radius (`shape.radius.control`; the token set has no `radius.sm`).
    pub radius: CornerRadius,
    /// Space between the box and the fitted document on each side: `navigator.inset` (8).
    pub inset: f32,
    /// The least height of the preview box: `navigator.min_preview` (40).
    pub min_preview: f32,
    /// The proxy rectangle's colour when [`ProxyColor::Default`](super::ProxyColor::Default):
    /// `navigator.proxy`, PhotoCraft's `rgb(255, 84, 84)`.
    pub proxy: Color32,
    /// The proxy's stroke width, centred on its edge: `navigator.proxy_width` (1.5).
    pub proxy_width: f32,
    /// The proxy's corner radius on an upright view: `navigator.proxy_radius` (2).
    pub proxy_radius: f32,
    /// The 1 pt outline under the proxy, so it reads on any image: `overlay.halo`.
    pub halo: Color32,
    /// The side of a checkerboard square behind transparent pixels: `navigator.checker` (6).
    pub checker: f32,
    /// The two checkerboard colours (`Checkerboard`'s default pair).
    pub checker_colors: [Color32; 2],
    /// The gap above the zoom row and between its controls: `navigator.row_gap` (4).
    pub row_gap: f32,
}

impl Navigator<'_> {
    /// The Navigator's look in `tokens`.
    ///
    /// Skeleton values are PhotoCraft's literals (`panels/navigator.rs:16-28`, `:104-111`); TK2
    /// swaps them for the `navigator.*` tokens.
    pub fn style(tokens: &UiTokens) -> NavigatorStyle {
        // pc: crates/ui-egui/src/panels/navigator.rs:18 (`t.radius_sm`, `t.canvas`), :105 (proxy stroke), :109 (gap 4).
        NavigatorStyle {
            fill: tokens.color.surface.canvas.to_egui(),
            radius: tokens.radius.control,
            inset: 8.0,
            min_preview: 40.0,
            proxy: Color32::from_rgb(255, 84, 84),
            proxy_width: 1.5,
            proxy_radius: 2.0,
            halo: tokens.color.overlay.halo.to_egui(),
            checker: 6.0,
            checker_colors: crate::canvas::CheckerColors::default().colors(),
            row_gap: 4.0,
        }
    }
}
