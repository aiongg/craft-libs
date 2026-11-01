//! [`PerspectiveSession`]: the planes of a Perspective Warp while it is open, and the plane
//! geometry its screens need (linking, snapping, straightening) (task TR-T7). The warp map itself
//! (PhotoCraft's piecewise homography) stays the app's (LC20, LC21).

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T7)")]

use kurbo::{Point, Rect};

use craft_ui::dialog::LocalHistory;
use craft_ui::edit::EditEvent;

/// Corners closer than this in the source (each axis) are one linked corner (PhotoCraft
/// `LINK_EPS`).
pub const LINK_DISTANCE: f64 = 0.5;

/// The smallest plane Layout adds, in document units each way (LC22).
pub const PLANE_MINIMUM: f64 = 4.0;

/// One plane: its corners in Layout (`source`) and in Warp (`target`), clockwise from the top
/// left, in document units (PhotoCraft `Plane { src, dst }`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub source: [Point; 4],
    pub target: [Point; 4],
}

impl Plane {
    /// A plane whose target is its source.
    pub fn identity(corners: [Point; 4]) -> Self {
        Plane { source: corners, target: corners }
    }

    /// The point at plane coordinates `(u, v)` (each 0 to 1 across), through the source corners
    /// or, with `target`, the target corners; the first corner for a degenerate quad.
    pub fn point(&self, u: f64, v: f64, target: bool) -> Point {
        // pc: crates/algo/src/perspective.rs:255 (`plane_point`).
        todo!()
    }
}

/// A plane's corner: the plane's index and the corner's (0 to 3, clockwise from the top left).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaneCorner {
    pub plane: usize,
    pub corner: usize,
}

/// Perspective Warp's two modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlaneMode {
    /// Draw planes and fit their corners to the picture's lines; the picture does not move.
    #[default]
    Layout,
    /// Drag the target corners; the picture follows.
    Warp,
}

/// Warp mode's straightening buttons (Photoshop's Straighten, Level, Both).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Straighten {
    /// Straighten: near-vertical edges become vertical.
    Vertical,
    /// Level: near-horizontal edges become horizontal.
    Horizontal,
    /// Both.
    Both,
}

/// The groups of linked corners: corners within [`LINK_DISTANCE`] of each other in the source,
/// in plane and corner order.
pub fn linked_corners(planes: &[Plane]) -> Vec<Vec<PlaneCorner>> {
    // pc: crates/algo/src/perspective.rs:47 (`linked_corners`).
    todo!()
}

/// Moves each group of linked corners to one source and one target position, their means.
pub fn unify(planes: &mut [Plane]) {
    // pc: crates/algo/src/perspective.rs:66 (`unify`).
    todo!()
}

/// Makes target edges within 45° of horizontal exactly horizontal and/or those within 45° of
/// vertical exactly vertical (each edge's two corner groups meet at their mean), moving linked
/// corners together. Unifies first.
pub fn straighten(planes: &mut [Plane], how: Straighten) {
    // pc: crates/algo/src/perspective.rs:127 (`straighten`).
    todo!()
}

/// An edit of the perspective session, from [`PerspectiveWarp`](super::PerspectiveWarp) or
/// [`PerspectiveBar`](super::PerspectiveBar).
#[derive(Clone, Debug, PartialEq)]
pub enum PerspectiveEdit {
    /// The planes as a whole: a corner drag (Begin, Update, Commit, Cancel) or a one-shot change
    /// (Set): a new plane, a removed plane, Remove Planes, an arrow nudge.
    Planes(EditEvent<Vec<Plane>>),
    /// A mode tab. Entering Warp [unifies](unify) linked corners; Warp needs at least one plane.
    Mode(PlaneMode),
    /// A straightening button (Warp mode).
    Straighten(Straighten),
}

/// What one undo step holds.
#[derive(Clone, Debug, PartialEq)]
struct PerspectiveDraft {
    planes: Vec<Plane>,
    mode: PlaneMode,
}

/// The planes of an open Perspective Warp, its mode and its undo steps.
///
/// - Changes only through [`apply`](Self::apply) and [`replace`](Self::replace). Replacing the
///   planes from outside (the app's control channel) bumps the [generation](Self::generation), so
///   the overlay ends a corner drag that held the old planes' indices (PhotoCraft bug #952).
/// - One undo step per drag, one-shot change, mode switch or straightening, and per run of arrow
///   nudges within the [nudge pause](Self::nudge_pause).
#[derive(Clone, Debug)]
pub struct PerspectiveSession {
    /// The layer's bounds: Insert adds a plane over their middle half.
    bounds: Rect,
    draft: PerspectiveDraft,
    history: LocalHistory<PerspectiveDraft>,
    before: Option<PerspectiveDraft>,
    generation: u64,
    nudge_pause: Option<f64>,
    last_nudge: Option<f64>,
}

impl PerspectiveSession {
    /// A session in Layout mode with no planes over the layer's `bounds`.
    pub fn new(bounds: Rect) -> Self {
        let draft = PerspectiveDraft { planes: Vec::new(), mode: PlaneMode::Layout };
        Self { bounds, history: LocalHistory::new(&draft), draft, before: None, generation: 0, nudge_pause: None, last_nudge: None }
    }

    /// The app's Nudge Bundle Pause in seconds (as `TransformSession::nudge_pause`).
    pub fn nudge_pause(mut self, seconds: Option<f64>) -> Self {
        self.nudge_pause = seconds;
        self
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn planes(&self) -> &[Plane] {
        &self.draft.planes
    }

    pub fn mode(&self) -> PlaneMode {
        self.draft.mode
    }

    /// Changes whenever [`replace`](Self::replace) sets the planes.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether a corner drag is running.
    pub fn dragging(&self) -> bool {
        self.before.is_some()
    }

    /// A plane covering the middle half of the bounds: what Insert adds in Layout mode.
    pub fn inserted_plane(&self) -> Plane {
        // new: the keyboard path (Insert) of the transform spec's key table.
        todo!()
    }

    /// The planes to commit; `None` with no planes or when every target is its source.
    pub fn result(&self) -> Option<&[Plane]> {
        // pc: crates/ui-egui/src/tools/perspective_ui.rs:157 (`commit`); crates/algo/src/perspective.rs:232
        // (`is_identity`).
        todo!()
    }

    /// Applies one edit at `now` (seconds). `Mode(Warp)` with no planes is refused; entering Warp
    /// unifies linked corners; a Layout edit moves source and target together, a Warp edit the
    /// target only (the layout is kept).
    pub fn apply(&mut self, edit: PerspectiveEdit, now: f64) {
        // pc: crates/ui-egui/src/tools/perspective_ui.rs:165 (`set_mode`), :108 (`set_corner`),
        // :310 (`options_bar`: Warp needs a plane).
        todo!()
    }

    /// Sets the planes from outside the widgets (the app's control channel), ends any drag and
    /// bumps the generation. One undo step.
    pub fn replace(&mut self, planes: Vec<Plane>) {
        // pc: crates/ui-egui/src/tools/perspective_ui.rs:190 (#952).
        todo!()
    }

    /// Records an undo step when the draft has come to rest changed (as
    /// `TransformSession::settle`).
    pub fn settle(&mut self, now: f64, busy: bool) {
        // new: PhotoCraft's Perspective Warp has no undo inside the session; the session's steps
        // follow the transform session's rule.
        todo!()
    }

    /// Edit › Undo while the session is open (the app routes it here; the session reads no undo
    /// key): steps back one undo step; false while a drag runs or with nothing to undo.
    pub fn undo(&mut self) -> bool {
        // new: as `settle`.
        todo!()
    }

    /// Steps forward one undone step.
    pub fn redo(&mut self) -> bool {
        // new: as `settle`.
        todo!()
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
}
