//! The looks of the pin warp, the perspective warp and the one mesh painter, from tokens.

use egui::{Color32, Stroke};

use craft_theme::Rgba;
use craft_ui::primitives::{Handle, HandleShape, HandleStyle};
use craft_ui::style::transform::TransformStyle;
use craft_ui::theme::{ToEgui, UiTokens};

/// A pin's fill.
const PIN_FILL: Rgba = Rgba([255, 214, 10, 255]);
/// A pin's ring and a selected pin's core.
const PIN_RING: Rgba = Rgba([0, 0, 0, 255]);
/// The fixed-rotation ring and its line.
const PIN_TURN: Rgba = Rgba([255, 255, 255, 255]);
/// The mesh's lines.
const MESH_LINE: Rgba = Rgba([40, 40, 40, 150]);

/// The lines of a deformed mesh ([`paint_mesh`](super::paint_mesh)): Puppet Warp's pin mesh and
/// Liquify's mesh, one look (S7-workspaces draws Liquify's with it).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MeshStyle {
    /// `stroke.thin` of a translucent dark grey (rgba 40, 40, 40, 150).
    pub line: Stroke,
}

impl MeshStyle {
    /// The mesh look in `tokens`.
    pub fn of(tokens: &UiTokens) -> MeshStyle {
        MeshStyle { line: Stroke::new(tokens.stroke.thin, MESH_LINE.to_egui()) }
    }
}

/// The look and reach of [`PinWarp`](super::pin::PinWarp).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PinWarpStyle {
    /// A pin's disc diameter: 11.
    pub pin: f32,
    /// Its fill: #FFD60A.
    pub fill: Color32,
    /// Its ring: `stroke.thin` of black.
    pub ring: Stroke,
    /// A selected pin's core diameter: 4, in black.
    pub core: f32,
    #[allow(missing_docs)]
    pub core_color: Color32,
    /// The fixed-rotation ring's radius and the ⌥-turn reach: 18.
    pub turn_radius: f32,
    /// The fixed-rotation ring and its line: `stroke.thin` of white.
    pub turn: Stroke,
    /// Distance within which a press takes a pin: 12.
    pub hit: f32,
    /// The mesh lines when Show Mesh is on.
    pub mesh: MeshStyle,
}

impl PinWarpStyle {
    /// The pin warp look in `tokens`; [`PinWarp::style`](super::PinWarp::style) is the public form.
    pub(crate) fn of(tokens: &UiTokens) -> PinWarpStyle {
        let t = TransformStyle::new(tokens);
        PinWarpStyle {
            pin: t.pin,
            fill: PIN_FILL.to_egui(),
            ring: Stroke::new(tokens.stroke.thin, PIN_RING.to_egui()),
            core: t.pin_core,
            core_color: PIN_RING.to_egui(),
            turn_radius: t.pin_turn,
            turn: Stroke::new(tokens.stroke.thin, PIN_TURN.to_egui()),
            hit: t.hit,
            mesh: MeshStyle::of(tokens),
        }
    }
}

/// The look and reach of [`PerspectiveWarp`](super::perspective::PerspectiveWarp). Planes take the
/// box's and the warp grid's lines.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PerspectiveWarpStyle {
    /// A plane's outline: `stroke.thin` of `overlay.line`, bare, as the box.
    pub line: Stroke,
    /// A plane's thirds: a hairline of `overlay.line` at 0.7 alpha, as a one-patch
    /// warp's thirds.
    pub thirds: Stroke,
    /// The corners: a Round `Handle` of diameter 9, ring
    /// `stroke.thin` of `overlay.line`, fill `overlay.handle_fill`.
    pub corner: HandleStyle,
    /// The rubber band of a new plane in Layout mode: `stroke.thin` of `overlay.line`.
    pub band: Stroke,
    /// Distance within which a press takes a corner, and within which a corner snaps to another
    /// plane's: 12.
    pub hit: f32,
}

impl PerspectiveWarpStyle {
    /// The perspective warp look in `tokens`; [`PerspectiveWarp::style`](super::perspective::PerspectiveWarp::style) is the public form.
    pub(crate) fn of(tokens: &UiTokens) -> PerspectiveWarpStyle {
        let (t, o) = (TransformStyle::new(tokens), &tokens.color.overlay);
        let thin = tokens.stroke.thin;
        let line = o.line.to_egui();
        let mut corner = Handle::style(tokens, HandleShape::Round);
        corner.size = t.plane_corner;
        corner.fill = o.handle_fill.to_egui();
        corner.stroke = Stroke::new(thin, line);
        PerspectiveWarpStyle {
            line: Stroke::new(thin, line),
            thirds: Stroke::new(t.warp_minor, line.gamma_multiply(0.7)),
            corner,
            band: Stroke::new(thin, line),
            hit: t.hit,
        }
    }
}
