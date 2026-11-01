//! [`NavigatorStyle`]: the Navigator's look, from tokens.

use egui::{Color32, CornerRadius};

use super::preview::Navigator;
use crate::theme::{ToEgui as _, UiTokens};

/// The width of the `overlay.halo` line.
const HALO_WIDTH: f32 = 1.0;

/// The Navigator's sizes and colours. [`Navigator::style`] reads them from the tokens; a widget
/// never takes a literal size from app code.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct NavigatorStyle {
    /// The preview box's fill (`surface.canvas`, PhotoCraft's `t.canvas`).
    pub fill: Color32,
    /// The preview box's corner radius (`shape.radius.control`).
    pub radius: CornerRadius,
    /// Space between the box and the fitted document on each side: `navigator.inset`.
    pub inset: f32,
    /// The least height of the preview box: `navigator.min_preview`.
    pub min_preview: f32,
    /// The proxy rectangle's colour when [`ProxyColor::Default`](super::ProxyColor::Default):
    /// `navigator.proxy`.
    pub proxy: Color32,
    /// The proxy's stroke width, centred on its edge: `navigator.proxy_width`.
    pub proxy_width: f32,
    /// The proxy's corner radius on an upright view: `navigator.proxy_radius`.
    pub proxy_radius: f32,
    /// The 1 pt outline under the proxy, so it reads on any image: `overlay.halo`.
    pub halo: Color32,
    /// The halo's width, also the margin the proxy's edges are clipped to: the 1 pt line
    /// `overlay.halo` is defined as (decisions §3.6; drawing geometry, no token holds it).
    pub halo_width: f32,
    /// The side of a checkerboard square behind transparent pixels: `navigator.checker`.
    pub checker: f32,
    /// The two checkerboard colours (`Checkerboard`'s default pair).
    pub checker_colors: [Color32; 2],
    /// The gap above the zoom row and between its controls: `navigator.row_gap`.
    pub row_gap: f32,
}

impl Navigator<'_> {
    /// The Navigator's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> NavigatorStyle {
        let n = &tokens.component.navigator;
        NavigatorStyle {
            fill: tokens.color.surface.canvas.to_egui(),
            radius: tokens.radius.control,
            inset: n.inset,
            min_preview: n.min_preview,
            proxy: n.proxy.to_egui(),
            proxy_width: n.proxy_width,
            proxy_radius: n.proxy_radius,
            halo: tokens.color.overlay.halo.to_egui(),
            halo_width: HALO_WIDTH,
            checker: n.checker,
            checker_colors: crate::canvas::CheckerColors::default().colors(),
            row_gap: n.row_gap,
        }
    }
}
