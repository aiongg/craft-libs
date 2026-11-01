//! [`TransformOverlay`]: the open session's box on the canvas, which drives a
//! [`TransformBox`](super::TransformBox) and turns its reports into [`TransformEdit`]s.

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T8)")]

use egui::{AsIdSalt, Id, IdSalt, Modifiers, Response, Ui};
use kurbo::Point;

use craft_ui::canvas::quad::{BoxPart, BoxRules};
use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::edit::EditPolicy;

use super::{TransformEdit, TransformLabels, TransformNames, TransformSession, TransformSettings, TransformWarp};

/// The host's snapping as [`TransformBox::snap`](super::TransformBox::snap) takes it.
type Snap<'a> = Box<dyn Fn(BoxPart, Point, Modifiers) -> Point + 'a>;

/// The open [`TransformSession`]'s box on the canvas.
///
/// - Box phase: shows a [`TransformBox`](super::TransformBox) on the session's box with the
///   session's [rules](TransformSession::box_rules) (or the app's [`rules`](Self::rules)), its
///   reference point hidden unless the session [shows it](TransformSession::reference_point),
///   and reports every box event as [`TransformEdit::Box`]: a corner drag as Begin, Update and
///   Commit, Esc during a drag as Cancel, an arrow key as Set. The box's apply (Enter, a click
///   outside, a double click inside) is [`TransformEdit::Commit`]; its discard (Esc with no drag)
///   is [`TransformEdit::Cancel`].
/// - Warp phase: draws nothing, senses nothing and claims nothing; the app's warp widgets own
///   the canvas.
/// - Keyboard, AccessKit and RTL are the box's: one Tab stop, the group "Free Transform" with its
///   parts (whole box, 4 corners, 4 edges and the reference point when shown); Page Down and Up
///   pick parts; arrows move the box or the picked part, ⇧ ten. Undo and Redo are the app's Edit
///   commands, not keys of the overlay.
pub struct TransformOverlay<'a, W> {
    id_salt: IdSalt,
    session: &'a TransformSession<W>,
    rules: Option<BoxRules>,
    settings: Option<&'a TransformSettings>,
    names: TransformNames,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    readout: Option<TransformLabels>,
    pixel_grid: bool,
    snap: Option<Snap<'a>>,
    modifiers: Option<Modifiers>,
    policy: EditPolicy,
}

/// What the overlay reported this pass.
#[derive(Debug)]
pub struct TransformOverlayResponse<W> {
    /// The box's id ([`TransformBox::focus`](super::TransformBox::focus) gives it the keyboard
    /// when the session opens).
    pub id: Id,
    /// The edits in the order they happened; the app applies each with
    /// [`TransformSession::apply`].
    pub edits: Vec<TransformEdit<W>>,
    /// The box took this pass's press.
    pub claimed: bool,
    /// The part under the pointer (the app sets the cursor from it).
    pub hovered: Option<BoxPart>,
    /// The keyboard's picked part while the box has focus.
    pub picked: Option<BoxPart>,
}

impl<'a, W: TransformWarp> TransformOverlay<'a, W> {
    /// The overlay of `session`.
    pub fn new(id_salt: impl AsIdSalt, session: &'a TransformSession<W>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            session,
            rules: None,
            settings: None,
            names: TransformNames::default(),
            units: None,
            readout: None,
            pixel_grid: false,
            snap: None,
            modifiers: None,
            policy: EditPolicy::default(),
        }
    }

    /// The app's own box rules in place of [`TransformSession::box_rules`]: VectorCraft's 45°
    /// rotate step, a Move tool's rules. The session's mode and what it allows still win.
    pub fn rules(mut self, rules: BoxRules) -> Self {
        self.rules = Some(rules);
        self
    }

    /// The persisted section the box rules read: legacy scaling and the aspect link. Without it,
    /// the defaults.
    pub fn settings(mut self, settings: &'a TransformSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// The box's AccessKit names; English by default.
    pub fn names(mut self, names: TransformNames) -> Self {
        self.names = names;
        self
    }

    /// The ruler units of the document's axes: the parts' values and the readout's numbers.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// Shows the readout beside the pointer during a drag (Show Transformation Values).
    pub fn readout(mut self, labels: TransformLabels) -> Self {
        self.readout = Some(labels);
        self
    }

    /// Rounds dragged boxes to whole document units (Snap Transforms to Pixel Grid; the app's
    /// rule for shape layers and paths).
    pub fn pixel_grid(mut self, on: bool) -> Self {
        self.pixel_grid = on;
        self
    }

    /// The app's snapping of a dragged part (guides, a distorted corner).
    pub fn snap(mut self, snap: impl Fn(BoxPart, Point, Modifiers) -> Point + 'a) -> Self {
        self.snap = Some(Box::new(snap));
        self
    }

    /// The modifiers every test reads in place of the input's (a touch host's sticky modifiers).
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Draws the box over `view` and senses through the host's `canvas` response (Box phase); in
    /// the Warp phase returns an empty response at once.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> TransformOverlayResponse<W> {
        // pc: crates/ui-egui/src/tools/transform_tool/pointer.rs:65 (`pointer`), :60
        // (`accepts_outside_click`). The box's own behaviour is `TransformBox::show`.
        todo!()
    }
}
