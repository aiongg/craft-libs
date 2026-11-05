//! [`TransformOverlay`]: the open session's box on the canvas, which drives a
//! [`TransformBox`](super::TransformBox) and turns its reports into [`TransformEdit`]s.

use egui::{AsIdSalt, Id, IdSalt, Modifiers, Response, Ui};
use kurbo::Point;

use craft_ui::canvas::quad::{BoxPart, BoxRules};
use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::edit::EditPolicy;
use craft_ui::localized::Labels;

use super::{TransformBox, TransformEdit, TransformLabels, TransformNames, TransformPhase, TransformSession, TransformSettings, TransformWarp};

/// The host's snapping as [`TransformBox::snap`](super::TransformBox::snap) takes it.
type Snap<'a> = Box<dyn Fn(BoxPart, Point, Modifiers) -> Point + 'a>;

/// The open [`TransformSession`]'s box on the canvas.
///
/// - Box phase: shows a [`TransformBox`](super::TransformBox) on the session's box with the
///   session's [rules](TransformSession::box_rules) (or the app's [`rules`](Self::rules)), its
///   reference point hidden unless the session [shows it](TransformSession::reference_point),
///   and reports every box event as [`TransformEdit::Box`]: a corner drag as Begin, Update and
///   Commit, Esc during a drag as Cancel, an arrow key as Set. The box's apply (Enter, a click
///   outside, a double click inside) is [`TransformOverlayResponse::apply`]; its discard (Esc
///   with no drag) is [`discard`](TransformOverlayResponse::discard). The app answers them with
///   [`TransformSession::commit`] and [`discard`](TransformSession::discard) after the edits.
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
    names: Option<TransformNames>,
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
    /// Enter, a click outside or a double click inside: the app calls [`TransformSession::commit`]
    /// after the edits.
    pub apply: bool,
    /// Esc with no drag running: the app calls [`TransformSession::discard`] after the edits.
    pub discard: bool,
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
            names: None,
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

    /// The box's AccessKit names; the words installed in the context by default
    /// ([`TransformNames::of`]).
    pub fn names(mut self, names: TransformNames) -> Self {
        self.names = Some(names);
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

    /// How this control reports its gestures to the host.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Draws the box over `view` and senses through the host's `canvas` response (Box phase); in
    /// the Warp phase returns an empty response at once.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> TransformOverlayResponse<W> {
        // pc: crates/ui-egui/src/tools/transform_tool/pointer.rs:65 (`pointer`), :60
        // (`accepts_outside_click`). The box's own behaviour is `TransformBox::show`.
        let id = ui.make_persistent_id(self.id_salt);
        if self.session.phase() == TransformPhase::Warp {
            return TransformOverlayResponse { id, edits: Vec::new(), apply: false, discard: false, claimed: false, hovered: None, picked: None };
        }
        let session = self.session;
        let defaults = TransformSettings::default();
        let mut rules = self.rules.unwrap_or_else(|| session.box_rules(self.settings.unwrap_or(&defaults)));
        // The session's mode and what it allows win over the app's rules.
        rules.mode = session.current_mode();
        rules.allow = session.allows();
        let names = self.names.unwrap_or_else(|| TransformNames::of(ui));
        // The box takes the stored salt as it is, so both phases give one id.
        let mut tbox = TransformBox::salted(self.id_salt, session.state(), names)
            .rules(rules)
            .reference_point(session.reference_point())
            .pixel_grid(self.pixel_grid)
            .start(session.rect(), session.skew_v())
            .policy(self.policy);
        if let Some((x, y)) = self.units {
            tbox = tbox.units(x, y);
        }
        if let Some(labels) = self.readout {
            tbox = tbox.readout(labels);
        }
        if let Some(snap) = self.snap {
            tbox = tbox.snap(snap);
        }
        if let Some(modifiers) = self.modifiers {
            tbox = tbox.modifiers(modifiers);
        }
        let shown = tbox.show(ui, canvas, view);
        let mut edits = Vec::new();
        if let Some(event) = shown.event {
            edits.push(TransformEdit::Box(event));
        }
        TransformOverlayResponse {
            id: shown.id,
            edits,
            apply: shown.apply,
            discard: shown.discard,
            claimed: shown.claimed,
            hovered: shown.hovered,
            picked: shown.picked,
        }
    }
}
