//! [`CornerWidgets`]: the round widgets inside the corners of the host's shapes.

use egui::{AsIdSalt, Id, IdSalt, Response, Ui};

use super::{CornerEdit, CornerKind, CornerRef, CornerStyle, CornerTarget, CornerWidgetsLabels};
use craft_ui::behavior::nudge;
use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::edit::{EditEvent, EditPolicy};

/// What a pass of [`CornerWidgets::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "apply the corner edit and answer cycle_kind and activate"]
pub struct CornerWidgetsResponse {
    /// The overlay's id, also its Tab stop's.
    pub id: Id,
    /// The radii: a drag reports Begin past `bezier.click_slop`, Update every move, Commit on
    /// release (Esc cancels, the rest of the press does nothing), each value every changed
    /// target's radii from the press; an arrow key a Set. The host makes one history step for
    /// every target in the value.
    pub event: Option<EditEvent<CornerEdit>>,
    /// ⌥-click without drag on a widget, with [`kinds`](CornerWidgets::kinds) on: the corner and
    /// the kind it turns into ([`CornerKind::next`] of its kind). The host applies it to the
    /// corners shown with it.
    pub cycle_kind: Option<(CornerRef, CornerKind)>,
    /// A double-click on a widget, or Enter on the picked one: the host opens its Corners dialog
    /// for that corner (S7-shapes).
    pub activate: Option<CornerRef>,
    /// The pointer is over a widget or a drag is under way: the host shows its corner-radius
    /// cursor.
    pub hovered: bool,
    /// The widgets own the current press: the host's tool leaves it alone.
    pub claimed: bool,
}

/// One round widget inside each corner of the host's shapes, dragged to round them (PhotoCraft's
/// and Photoshop's live corner widgets, VectorCraft's Live Corners).
///
/// ```no_run
/// use craft_draw::ui::corner::{CornerTarget, CornerWidgets};
/// use craft_ui::canvas::{CanvasView, RulerUnits};
/// # fn demo(ui: &mut egui::Ui, canvas: &egui::Response, view: &CanvasView, targets: &[CornerTarget], px: &RulerUnits) {
/// let r = CornerWidgets::new("corners", targets, px).show(ui, canvas, view);
/// if let Some(edit) = r.event.as_ref().and_then(|e| e.value()) {
///     // preview the outlines with `edit`; on Commit, one history step for every target in it
/// }
/// # }
/// ```
///
/// - Look: per shown corner a circle `corner.widget` across in `overlay.handle_fill`, outlined
///   `corner.line` in `overlay.line`, with a `corner.dot` dot in `overlay.line`, at the centre of
///   the corner's arc and at least `corner.inset` screen points inside the corner. Hidden when a
///   side is shorter than `corner.min_side` screen points or the corner's angle is wider than
///   [`hide_above`](Self::hide_above).
/// - Pointer: a press within `corner.hit` screen points of a widget takes it (nearest first, of
///   equal distances the first target's; list the topmost shape first). A drag sets the radius
///   at the press plus the move projected on the corner's centre step; every target takes that
///   radius, each capped at its own maximum (lead call 23); ⌥ changes only the dragged corner of
///   `individual` targets. A click without movement changes nothing. While dragging the readout
///   "R: 25.0 px" (the canvas `Readout`, in the [ruler units](Self::new)) follows the pointer;
///   the host previews the outlines from the event values. ⌥-click reports
///   [`cycle_kind`](CornerWidgetsResponse::cycle_kind) with [`kinds`](Self::kinds); a
///   double-click (`PrimaryPress::clicks`) reports [`activate`](CornerWidgetsResponse::activate).
///   A press with ⌘ is not taken.
/// - Keyboard: one Tab stop (`PartFocus`). Page Down and Page Up pick the next and previous
///   widget; the arrows change the radius of the picked widget's corner on every target by
///   [`steps`](Self::steps) (1, ⇧ 10; up and right grow it), ⌥ that corner only; Enter reports
///   `activate`; Esc leaves the group.
/// - AccessKit: a group named [`CornerWidgetsLabels::group`] with one object per widget named
///   "Corner i of n", valued "radius 12 px".
/// - Geometry is in document space and does not mirror right to left.
pub struct CornerWidgets<'a> {
    #[expect(dead_code, reason = "skeleton: read by task PE-B4")]
    id_salt: IdSalt,
    #[expect(dead_code, reason = "skeleton: read by task PE-B4")]
    targets: &'a [CornerTarget],
    #[expect(dead_code, reason = "skeleton: read by task PE-B4")]
    units: &'a RulerUnits,
    kinds: bool,
    hide_above: f64,
    steps: (f64, f64),
    labels: Option<CornerWidgetsLabels>,
    look: Option<CornerStyle>,
    policy: EditPolicy,
}

impl<'a> CornerWidgets<'a> {
    /// The widgets of the host's `targets`, topmost first, with radii read out in `units`.
    pub fn new(id_salt: impl AsIdSalt, targets: &'a [CornerTarget], units: &'a RulerUnits) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            targets,
            units,
            kinds: false,
            hide_above: 180.0,
            steps: (nudge::STEP, nudge::BIG_STEP),
            labels: None,
            look: None,
            policy: EditPolicy::default(),
        }
    }

    /// Whether ⌥-click cycles the corner kind (VectorCraft); off by default (lead call 24).
    pub fn kinds(mut self, kinds: bool) -> Self {
        self.kinds = kinds;
        self
    }

    /// The widest inner angle, in degrees, whose corner shows a widget; 180 by default
    /// (VectorCraft's preference passes 177).
    pub fn hide_above(mut self, degrees: f64) -> Self {
        self.hide_above = degrees;
        self
    }

    /// The arrow keys' radius change in document units, plain and with ⇧ (1 and 10, lead call 25).
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.steps = (step, big_step);
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: CornerWidgetsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// A look of the host's in place of [`style`](Self::style)'s.
    pub fn look(mut self, style: CornerStyle) -> Self {
        self.look = Some(style);
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Draws the widgets over `view` and senses through the host's `canvas` response. The host
    /// shows them before its tool input, as an overlay that claims only presses on a widget.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-B4)")]
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> CornerWidgetsResponse {
        // pc: crates/ui-egui/src/tools/corner_radius.rs:229-297 `pointer` (⌘ not taken :233; drag
        // :246-270; one step on release :271-295), :299-330 `draw` (readout "R:" with `{r:.1} px`
        // :313), :225-227 `hovers`. vectorcraft: crates/tools/src/corners.rs:233 (⌥-click cycles).
        // new: `Claim`, `PrimaryPress::clicks`, `Gesture<CornerEdit>`, `PartFocus`, `Readout`, the
        // maths in `geometry.rs`.
        todo!()
    }
}
