//! The looks of the pin warp, the perspective warp and the one mesh painter, from tokens. The token leaves are `transform.pin`, `pin_ring`, `pin_core`, `pin_turn`,
//! `pin_turn_line`, `mesh_line`, `plane_corner` and `plane_ring`, and the colours `overlay.pin`,
//! `pin_ring`, `pin_turn` and `mesh`.

use egui::{Color32, Stroke};

use craft_ui::primitives::{Handle, HandleShape, HandleStyle};
use craft_ui::theme::{ToEgui, UiTokens};

/// The lines of a deformed mesh ([`paint_mesh`](super::paint_mesh)): Puppet Warp's pin mesh and
/// Liquify's mesh, one look (S7-workspaces draws Liquify's with it).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MeshStyle {
    /// `transform.mesh_line` (0.6) of `overlay.mesh` (rgba 40, 40, 40, 150; PhotoCraft
    /// `puppet_ui.rs:367`).
    pub line: Stroke,
}

impl MeshStyle {
    /// The mesh look in `tokens`.
    pub fn of(tokens: &UiTokens) -> MeshStyle {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:367.
        MeshStyle { line: Stroke::new(tokens.component.transform.mesh_line, tokens.color.overlay.mesh.to_egui()) }
    }
}

/// The look and reach of [`PinWarp`](super::pin::PinWarp).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PinWarpStyle {
    /// A pin's disc diameter: `transform.pin` (11; PhotoCraft radius 5.5).
    pub pin: f32,
    /// Its fill: `overlay.pin` (#FFD60A).
    pub fill: Color32,
    /// Its ring: `transform.pin_ring` (1.25) of `overlay.pin_ring` (black).
    pub ring: Stroke,
    /// A selected pin's core diameter: `transform.pin_core` (4), in `overlay.pin_ring`.
    pub core: f32,
    #[allow(missing_docs)]
    pub core_color: Color32,
    /// The fixed-rotation ring's radius and the ⌥-turn reach: `transform.pin_turn` (18).
    pub turn_radius: f32,
    /// The fixed-rotation ring and its line: `transform.pin_turn_line` (1.0) of `overlay.pin_turn`
    /// (white).
    pub turn: Stroke,
    /// Distance within which a press takes a pin: `transform.hit` (12, LC12).
    pub hit: f32,
    /// The mesh lines when Show Mesh is on.
    pub mesh: MeshStyle,
}

impl PinWarpStyle {
    /// The pin warp look in `tokens`; [`PinWarp::style`](super::PinWarp::style) is the public form.
    pub(crate) fn of(tokens: &UiTokens) -> PinWarpStyle {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:382 (disc, ring), :385 (core), :388 (turn ring
        // and line).
        let (t, o) = (&tokens.component.transform, &tokens.color.overlay);
        PinWarpStyle {
            pin: t.pin,
            fill: o.pin.to_egui(),
            ring: Stroke::new(t.pin_ring, o.pin_ring.to_egui()),
            core: t.pin_core,
            core_color: o.pin_ring.to_egui(),
            turn_radius: t.pin_turn,
            turn: Stroke::new(t.pin_turn_line, o.pin_turn.to_egui()),
            hit: t.hit,
            mesh: MeshStyle::of(tokens),
        }
    }
}

/// The look and reach of [`PerspectiveWarp`](super::perspective::PerspectiveWarp). Planes take the
/// box's and the warp grid's lines, not PhotoCraft's 1.5 accent outline and white thirds (LC13).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PerspectiveWarpStyle {
    /// A plane's outline: `transform.line` of `overlay.line`, bare, as the box.
    pub line: Stroke,
    /// A plane's thirds: `transform.warp_minor` of `overlay.line` at 0.7 alpha, as a one-patch
    /// warp's thirds.
    pub thirds: Stroke,
    /// The corners: a Round `Handle` of diameter `transform.plane_corner` (9), ring
    /// `transform.plane_ring` (1.25) of `overlay.line`, fill `overlay.handle_fill`.
    pub corner: HandleStyle,
    /// The rubber band of a new plane in Layout mode: `transform.line` of `overlay.line`.
    pub band: Stroke,
    /// Distance within which a press takes a corner, and within which a corner snaps to another
    /// plane's: `transform.hit` (12, LC12).
    pub hit: f32,
}

impl PerspectiveWarpStyle {
    /// The perspective warp look in `tokens`; [`PerspectiveWarp::style`](super::perspective::PerspectiveWarp::style) is the public form.
    pub(crate) fn of(tokens: &UiTokens) -> PerspectiveWarpStyle {
        // pc: crates/ui-egui/src/tools/perspective_ui.rs:284 (`draw`: thirds, outline, corners,
        // rubber band).
        let (t, o) = (&tokens.component.transform, &tokens.color.overlay);
        let line = o.line.to_egui();
        let mut corner = Handle::style(tokens, HandleShape::Round);
        corner.size = t.plane_corner;
        corner.fill = o.handle_fill.to_egui();
        corner.stroke = Stroke::new(t.plane_ring, line);
        PerspectiveWarpStyle {
            line: Stroke::new(t.line, line),
            thirds: Stroke::new(t.warp_minor, line.gamma_multiply(0.7)),
            corner,
            band: Stroke::new(t.line, line),
            hit: t.hit,
        }
    }
}
