//! [`GripSet`]: per-object grips with cold, warm and hot states (CadCraft's grips).

use egui::{AsIdSalt, Id, IdSalt, Modifiers, Response, Ui};
use kurbo::{Point, Vec2};

use crate::behavior::nudge;
use crate::canvas::{CanvasView, RulerUnits};
use crate::edit::{EditEvent, EditPolicy};

crate::words! {
    /// The AccessKit names of a [`GripSet`], English by default. `grip` takes `{n}` (1-based) and
    /// `{count}`.
    #[non_exhaustive]
    pub struct GripSetLabels {
        group: "gripSet.group" = "Grips";
        grip: "gripSet.grip" = "Grip {n} of {count}";
        more: "gripSet.more" = "More grips";
    }
}

/// How a grip is drawn and hit. Sizes come from `grip.size` (CadCraft's `gripsize` 5 is a half
/// side, so 10 pt across).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum GripShape {
    /// A square `grip.size` across: a vertex, an end point, a dynamic point grip.
    Square,
    /// A rectangle `grip.size` wide and `grip.midpoint_height` high: a polyline segment's
    /// midpoint.
    Midpoint,
    /// A triangle pointing along `direction` (document units; drawn as the view maps it): a
    /// dynamic linear grip, pointing the way its distance grows.
    Arrow { direction: Vec2 },
    /// A downward triangle: a dynamic list grip, which opens the host's list of values.
    Triangle,
}

/// The state a grip is drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GripState {
    /// At rest: `overlay.grip`.
    #[default]
    Cold,
    /// The pointer is within `grip.warm_pad` of it: `overlay.grip_warm`.
    Warm,
    /// Picked for editing ([`Grip::hot`]): `overlay.grip_hot`.
    Hot,
}

/// One grip of a [`GripSet`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Grip {
    /// The host's key for the grip, stable across passes (an entity and a grip index).
    pub key: u64,
    /// Where it sits, in document units.
    pub at: Point,
    pub shape: GripShape,
    /// Whether the host has made it hot (CadCraft's ⇧-click list, the grip being edited).
    pub hot: bool,
}

impl Grip {
    /// A cold grip keyed `key` at `at`.
    pub fn new(key: u64, at: Point, shape: GripShape) -> Self {
        Self { key, at, shape, hot: false }
    }

    /// Makes the grip hot.
    pub fn hot(mut self, hot: bool) -> Self {
        self.hot = hot;
        self
    }
}

/// A click on a grip: the pointer's press and release without a drag, or Enter on the picked
/// grip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GripClick {
    pub key: u64,
    /// ⇧ was held (CadCraft: toggles the grip hot; a plain click starts the grip edit).
    pub shift: bool,
}

/// A grip moved by a drag or an arrow key: the value of the set's [`EditEvent`]s.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GripDrag {
    pub key: u64,
    /// Where the grip was at the press, in document units.
    pub from: Point,
    /// Where the pointer (or the arrow keys) put it.
    pub to: Point,
}

/// What [`GripSet::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GripSetResponse {
    /// The set's id.
    pub id: Id,
    /// A click on a grip.
    pub clicked: Option<GripClick>,
    /// A drag of a grip (`Begin` on the first move past the click slop, `Update`, `Commit`; Esc
    /// cancels) or one arrow press's `Set`.
    pub drag: Option<EditEvent<GripDrag>>,
    /// The grip under the pointer.
    pub hovered: Option<u64>,
    /// The grip the keys act on, picked by the pointer or the Page keys.
    pub picked: Option<u64>,
    /// Whether the set owns the current press: the host skips its tool input.
    pub claimed: bool,
}

/// The grips of the selected objects (CadCraft's): squares, midpoint rectangles, arrows and
/// triangles that are cold, warm under the pointer and hot when the host says so. The set draws
/// and hit-tests them and reports clicks and drags; what a grip does (stretch, move, a menu) is
/// the host's.
///
/// - Look: filled `overlay.grip`, `overlay.grip_warm` or `overlay.grip_hot` by [`GripState`],
///   outlined 1 pt (`grip.stroke`) in `overlay.grip_stroke`. Every grip is drawn.
/// - Pointer, through the host's canvas response and [`Claim`](crate::canvas::Claim) first: the
///   hit area is a grip's bounds expanded by `grip.hit_pad`; a press there takes the grip; a
///   release without a drag is a [`GripClick`] with ⇧ as held; a drag after the press reports
///   [`GripDrag`] events. Warm follows the pointer within the bounds expanded by `grip.warm_pad`.
/// - Keys (one Tab stop through [`PartFocus`](crate::canvas::PartFocus)): Page Down and Page Up
///   pick the next and previous grip, Home and End the first and last; the arrows move the picked
///   grip by [`steps`](Self::steps), one `Set` per press; Enter is a click, ⇧+Enter a ⇧-click.
/// - AccessKit: a group named [`GripSetLabels::group`]; the first [`ACCESS_LIMIT`](Self::ACCESS_LIMIT)
///   grips are parts named "Grip 3 of 12" with their position as the value (in the
///   [`units`](Self::units)) and `selected` when hot; past the limit the group's value adds
///   [`GripSetLabels::more`] and the rest are drawn but not reachable from the keyboard.
/// - Nothing mirrors in a right-to-left interface.
pub struct GripSet<'a> {
    id_salt: IdSalt,
    grips: &'a [Grip],
    labels: Option<GripSetLabels>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    steps: (f64, f64),
    modifiers: Option<Modifiers>,
    policy: EditPolicy,
}

impl<'a> GripSet<'a> {
    /// The grips the AccessKit tree and the keyboard reach: CadCraft draws grips for at most 200
    /// objects.
    pub const ACCESS_LIMIT: usize = 200;

    /// The set of `grips`, in keyboard order.
    pub fn new(id_salt: impl AsIdSalt, grips: &'a [Grip]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            grips,
            labels: None,
            units: None,
            steps: (nudge::STEP, nudge::BIG_STEP),
            modifiers: None,
            policy: EditPolicy::default(),
        }
    }

    /// The AccessKit names; the installed words by default.
    pub fn labels(mut self, labels: GripSetLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The ruler units of the document's x and y axes, for the grips' AccessKit values.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The arrow keys' step and ⇧ step in document units; `nudge::STEP` and `nudge::BIG_STEP` by
    /// default.
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.steps = (step, big_step);
        self
    }

    /// The modifiers every test reads in place of the input state (a touch host's sticky
    /// modifiers).
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// How the drag reports; realtime and cancellable by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Draws the grips over `view` and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> GripSetResponse {
        // cadcraft: crates/ui-egui/src/canvas.rs:1053-1076 (cold, warm within 2, hot fill; 1 pt
        // #103080 outline); crates/ui-egui/src/canvas/grips.rs:71-75 `grip_rect` (2 × gripsize
        // square, midpoint half as high), :78-101 `grip_at` (hit = rect expanded by 3, first 200
        // objects); crates/ui-egui/src/dyngrips.rs:47-66 `outline` (arrow 1.6 × 1.1 gripsize along
        // its screen direction, list triangle 1.3 gripsize), :69-72 `hit`.
        // cadcraft: crates/ui-egui/src/canvas/grips.rs:5-27 `click_grip` (⇧ toggles hot).
        // new: keys and parts through `PartFocus`, as `SelectionFrame`; the look from
        // `GripSet::style`.
        todo!()
    }

    /// The screen outline of a grip of `shape` centred at `centre`, `size` across, for drawing and
    /// for the hit test (its bounds).
    pub(crate) fn outline(shape: GripShape, centre: egui::Pos2, size: f32, midpoint_height: f32, view: &CanvasView) -> Vec<egui::Pos2> {
        // cadcraft: crates/ui-egui/src/dyngrips.rs:47-66 `outline`; crates/ui-egui/src/canvas/grips.rs:71-75.
        todo!()
    }
}
