//! [`PinWarp`]: Puppet Warp's pins on the canvas, placed, moved, turned and removed by pointer and
//! keyboard (task TR-T12).

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T12)")]

use egui::{AsIdSalt, Id, IdSalt, Modifiers, Response, Ui};

use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::edit::EditPolicy;
use craft_ui::words;

use super::{PinEdit, PinMeshView, PinWarpSession};
use crate::ui::warp::PinWarpStyle;

words! {
    /// The AccessKit words of [`PinWarp`]. `Default` is English.
    pub struct PinWarpLabels {
        group: "pinWarp.group" = "Puppet Warp";
        pin: "pinWarp.pin" = "Pin {n} of {count}";
        turn: "pinWarp.turn" = "turn {degrees}°";
        position: "pinWarp.position" = "{x}, {y}";
    }
}

/// An open Puppet Warp's pins over the warped picture (drawn by the transform preview under the
/// guides), and the mesh lines when Show Mesh is on.
///
/// Pointer (strict `<` on `transform.hit` as the box):
/// - A click on the mesh away from pins places a pin at the rest position under it
///   ([`PinMeshView::rest_of`]), selects it, and the drag that follows moves it: one step. A click
///   off the mesh does nothing.
/// - A press on a pin selects it and drags it; Shift-click adds it to the selection or takes it
///   out; dragging a selected pin moves every selected pin.
/// - ⌥-click on a pin deletes it (one step).
/// - ⌥-drag within `transform.pin_turn` of a selected pin, not on it, turns it: the drag's angle
///   about the pin becomes its fixed rotation, which it keeps (Photoshop; VectorCraft).
/// - Esc during a drag puts the pins back (Cancel); with no drag it discards the session.
///
/// Keys (one Tab stop, `PartFocus`): Page Down and Up pick the next and previous pin (every pin,
/// then none); Home and End the first and last; arrows move the picked pin (⇧ ten, through
/// `behavior::nudge`); Delete or Backspace removes the selected pins; Insert places a pin at the
/// rest position under the pointer, or at the [mesh's centre](PinMeshView::centre) when the
/// pointer is outside; ⌘A selects every pin; Enter applies.
///
/// AccessKit: a group named [`PinWarpLabels::group`]; each pin a graphics object "Pin 2 of 5"
/// with value "x, y" (and "turn N°" when fixed) and its selected state. The mesh lines are
/// decoration. Nothing mirrors in a right-to-left interface.
pub struct PinWarp<'a> {
    id_salt: IdSalt,
    session: &'a PinWarpSession,
    labels: PinWarpLabels,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    modifiers: Option<Modifiers>,
    policy: EditPolicy,
}

/// What [`PinWarp::show`] reported this pass.
#[derive(Clone, Debug, PartialEq)]
pub struct PinWarpResponse {
    pub id: Id,
    /// The edits in the order they happened; the app applies each with
    /// [`PinWarpSession::apply`].
    pub edits: Vec<PinEdit>,
    /// Enter: the app commits ([`PinWarpSession::result`]).
    pub apply: bool,
    /// Esc with no drag running: the app discards the session.
    pub discard: bool,
    /// The overlay took this pass's press.
    pub claimed: bool,
    /// The pin under the pointer.
    pub hovered: Option<usize>,
}

impl<'a> PinWarp<'a> {
    /// The pins of `session`.
    pub fn new(id_salt: impl AsIdSalt, session: &'a PinWarpSession) -> Self {
        Self { id_salt: IdSalt::new(id_salt), session, labels: PinWarpLabels::default(), units: None, modifiers: None, policy: EditPolicy::default() }
    }

    pub fn labels(mut self, labels: PinWarpLabels) -> Self {
        self.labels = labels;
        self
    }

    /// The ruler units of the pins' AccessKit values.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The modifiers every test reads in place of the input's.
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The look and reach in `tokens`.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> PinWarpStyle {
        PinWarpStyle::of(tokens)
    }

    /// Draws the mesh lines (when shown), the pins and the picked pin's focus ring over `view`, and
    /// senses through the host's `canvas` response. `mesh` is the solver's mesh this pass.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView, mesh: PinMeshView<'_>) -> PinWarpResponse {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:293 (`pointer`), :365 (`draw`), :154
        // (`pin_at`); crates/ui-egui/src/dialogs/distort_ui.rs:409 (`tolerance`, 8, replaced by
        // `transform.hit` 12).
        todo!()
    }
}
