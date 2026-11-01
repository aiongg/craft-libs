//! [`PerspectiveWarp`]: Perspective Warp's planes on the canvas, drawn, linked, snapped and dragged
//! by pointer and keyboard (task TR-T13).

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T13)")]

use egui::{AsIdSalt, Id, IdSalt, Modifiers, Response, Ui};

use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::edit::EditPolicy;
use craft_ui::words;

use super::{PerspectiveEdit, PerspectiveSession, PlaneCorner};
use crate::ui::warp::PerspectiveWarpStyle;

words! {
    /// The AccessKit words of [`PerspectiveWarp`]. `Default` is English.
    pub struct PerspectiveWarpLabels {
        group: "perspectiveWarp.group" = "Perspective Warp";
        corner: "perspectiveWarp.corner" = "Plane {plane}, corner {corner}";
        linked: "perspectiveWarp.linked" = "linked";
        position: "perspectiveWarp.position" = "{x}, {y}";
    }
}

/// An open Perspective Warp's planes over the picture (the warped picture is drawn by the
/// transform preview through the app's map).
///
/// Pointer (strict `<` on `transform.hit`):
/// - Layout: a drag on the picture away from corners draws a plane (a rubber band), added on
///   release when at least [`PLANE_MINIMUM`](super::PLANE_MINIMUM) each way, its corners snapped
///   to other planes' corners within `transform.hit`; a drag on a corner moves it and every corner
///   linked to it, snapping to another plane's corner, which links them.
/// - Warp: a drag on a corner moves its target and the linked corners'; the picture follows; the
///   layout is kept.
/// - A drag holds its corners by value: when the session's
///   [generation](PerspectiveSession::generation) changes during it, the drag ends (Cancel).
/// - Esc during a drag cancels it; with no drag it discards the session.
///
/// Keys (one Tab stop, `PartFocus`): Page Down and Up pick the next and previous corner (plane by
/// plane, clockwise); arrows move the picked corner and its linked corners (⇧ ten); in Layout,
/// Insert adds [`PerspectiveSession::inserted_plane`] and Delete removes the picked corner's
/// plane; Enter applies.
///
/// AccessKit: a group; each corner a graphics object "Plane 2, corner 3" with value "x, y",
/// "linked" when linked. Nothing mirrors in a right-to-left interface.
pub struct PerspectiveWarp<'a> {
    id_salt: IdSalt,
    session: &'a PerspectiveSession,
    labels: PerspectiveWarpLabels,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    modifiers: Option<Modifiers>,
    policy: EditPolicy,
}

/// What [`PerspectiveWarp::show`] reported this pass.
#[derive(Clone, Debug, PartialEq)]
pub struct PerspectiveWarpResponse {
    pub id: Id,
    /// The edits in order; the app applies each with [`PerspectiveSession::apply`].
    pub edits: Vec<PerspectiveEdit>,
    /// Enter: the app commits ([`PerspectiveSession::result`]).
    pub apply: bool,
    /// Esc with no drag running.
    pub discard: bool,
    pub claimed: bool,
    /// The corner under the pointer.
    pub hovered: Option<PlaneCorner>,
}

impl<'a> PerspectiveWarp<'a> {
    /// The planes of `session`.
    pub fn new(id_salt: impl AsIdSalt, session: &'a PerspectiveSession) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            session,
            labels: PerspectiveWarpLabels::default(),
            units: None,
            modifiers: None,
            policy: EditPolicy::default(),
        }
    }

    pub fn labels(mut self, labels: PerspectiveWarpLabels) -> Self {
        self.labels = labels;
        self
    }

    /// The ruler units of the corners' AccessKit values.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The look and reach in `tokens`.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> PerspectiveWarpStyle {
        PerspectiveWarpStyle::of(tokens)
    }

    /// Draws the planes (thirds, outline, corners), the rubber band and the picked corner's focus
    /// ring over `view`, and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> PerspectiveWarpResponse {
        // pc: crates/ui-egui/src/tools/perspective_ui.rs:202 (`pointer`), :284 (`draw`), :82
        // (`corner_at`), :97 (`snap`), :238 (the 4-unit minimum).
        todo!()
    }
}
