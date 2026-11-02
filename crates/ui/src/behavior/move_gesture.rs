//! A move by drag: direction lock, stroke lock and snapping along a ray.
//!
//! - [`MoveGesture`]: the Move tool's rules as a value the tool owns. ⇧ locks the offset from the
//!   press to 4 or 8 directions ([`lock_direction`]), ⌥ held at the press copies once the pointer
//!   has travelled [`ConstrainStyle::copy_reach`], and the drag reports one
//!   [`EditEvent<MoveValue>`] sequence, so the duplicate and the move are one undo step from the
//!   host's mark at `Begin`.
//! - [`StrokeLock`]: ⇧ during a painting stroke keeps it on a 0°, 45° or 90° line from the
//!   stroke's last point.
//! - [`snap_ray`]: the Gradient tool's ⇧, a point moved onto the nearest ray from the press.
//!
//! No library Move tool: picking what moves and changing the document are the app's (a
//! 60-line tool on `MoveGesture`). The canvas transform box locks its body drags with the same
//! [`lock_direction`].
//!
//! ```no_run
//! use craft_ui::behavior::move_gesture::{ConstrainStyle, Directions, MoveGesture};
//! # fn demo(style: &ConstrainStyle, p: &craft_ui::canvas::ToolPointer, zoom: f64) {
//! let (mut drag, begin) = MoveGesture::begin(style, p.doc, p.modifiers, Directions::Four);
//! // each drag sample:
//! if let Some(update) = drag.drag(p.doc, p.modifiers, zoom) {
//!     let _ = update;
//! }
//! // the release:
//! let commit = drag.end(p.doc, p.modifiers, zoom);
//! # let _ = (begin, commit);
//! # }
//! ```
use egui::Modifiers;
use kurbo::{Point, Vec2};

use crate::edit::{EditEvent, GestureId};
use crate::theme::UiTokens;

/// Extra angle in degrees the pointer must pass the half-way line between two directions by
/// before a locked direction switches, so it doesn't flicker near the diagonal (PhotoCraft's
/// `HYSTERESIS_DEG`).
pub const LOCK_HYSTERESIS_DEGREES: f64 = 8.0;

/// The directions ⇧ locks a move to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Directions {
    /// Horizontal and vertical: the Move tool dragging layers (Photoshop).
    #[default]
    Four,
    /// The axes and the 45° diagonals: selected pixels, a selection outline, a transform box's
    /// body.
    Eight,
}

impl Directions {
    /// How many evenly spaced directions: 4 or 8.
    pub const fn count(self) -> u32 {
        match self {
            Directions::Four => 4,
            Directions::Eight => 8,
        }
    }
}

/// `d` locked to the nearest of `directions` evenly spaced directions (0° = right, document y
/// down). `previous` is the offset this returned last in the same drag: its direction is kept
/// until `d` is more than half a step plus [`LOCK_HYSTERESIS_DEGREES`] away from it. The length
/// is the projection of `d` on the chosen direction, never negative; components under 1e-9
/// become exactly 0, so an axis-locked move stays on the axis. A zero or non-finite `d` comes
/// back unchanged.
pub fn lock_direction(d: Vec2, previous: Option<Vec2>, directions: Directions) -> Vec2 {
    // pc: crates/ui-egui/src/tools/move_mods.rs:42 `constrain` (step TAU / n; k = round(atan2 / step);
    // keep kp while the wrapped offset < step / 2 + 8°; t = max(0, d · dir); clean |v| < 1e-9 to 0).
    // The library's copy is `canvas/quad.rs:472` `constrain`: K2 deletes it and calls this.
    let len = d.hypot();
    if !len.is_finite() || len == 0.0 {
        return d;
    }
    let step = std::f64::consts::TAU / f64::from(directions.count());
    let a = d.atan2();
    let mut k = (a / step).round();
    if let Some(p) = previous.filter(|p| p.hypot() > 0.0) {
        let kp = (p.atan2() / step).round();
        let off = (a - kp * step).rem_euclid(std::f64::consts::TAU);
        let off = off.min(std::f64::consts::TAU - off);
        if off < step / 2.0 + LOCK_HYSTERESIS_DEGREES.to_radians() {
            k = kp;
        }
    }
    let (s, c) = (k * step).sin_cos();
    let t = (d.x * c + d.y * s).max(0.0);
    let clean = |v: f64| if v.abs() < 1e-9 { 0.0 } else { v };
    Vec2::new(clean(t * c), clean(t * s))
}

/// The sizes of the move and stroke rules, from the `constrain` token family.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstrainStyle {
    /// Screen points an ⌥-drag travels before it copies: `constrain.copy_reach` (PhotoCraft 2).
    pub copy_reach: f64,
    /// Screen points a ⇧ stroke travels from its origin before its direction locks:
    /// `constrain.stroke_lock` (PhotoCraft 3).
    pub stroke_lock: f64,
}

impl ConstrainStyle {
    /// The style from `tokens` (`constrain.copy_reach`, `constrain.stroke_lock`).
    pub fn of(tokens: &UiTokens) -> Self {
        // new: token task TK1 adds the `constrain` family (values 2 and 3 at both Sizes).
        let c = &tokens.component.constrain;
        Self { copy_reach: f64::from(c.copy_reach), stroke_lock: f64::from(c.stroke_lock) }
    }
}

/// The value of a move drag.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveValue {
    /// The offset from the press in document units, after the ⇧ lock.
    pub delta: Vec2,
    /// True from the first update after the pointer travelled
    /// [`copy_reach`](ConstrainStyle::copy_reach) screen points with ⌥ held at the press, and for
    /// the rest of the drag. The host duplicates on the first update that carries it (layers in
    /// place; selected pixels and outlines copy the pixels). ⌥ pressed after the press does
    /// nothing.
    pub copy: bool,
}

/// A Move-tool drag in progress: the press, whether ⌥ was held at it, and the last locked
/// offset. The tool keeps it between passes and drops it after [`end`](Self::end) or
/// [`cancel`](Self::cancel).
#[derive(Clone, Debug)]
pub struct MoveGesture {
    id: GestureId,
    start: Point,
    directions: Directions,
    copy_reach: f64,
    alt_at_press: bool,
    value: MoveValue,
    /// The last locked offset, for the hysteresis; `None` while ⇧ is up.
    locked: Option<Vec2>,
}

impl MoveGesture {
    /// Starts a drag at `start` (document units) with the keys held at the press. Returns the
    /// gesture and `EditEvent::Begin` (no value): the host takes its undo mark.
    pub fn begin(style: &ConstrainStyle, start: Point, modifiers: Modifiers, directions: Directions) -> (MoveGesture, EditEvent<MoveValue>) {
        // pc: crates/ui-egui/src/tools/move_mods.rs:124 `filter_event` Down (`MoveDrag { start, alt: mods.alt, .. }`).
        let id = GestureId::next();
        let gesture = MoveGesture {
            id,
            start,
            directions,
            copy_reach: style.copy_reach,
            alt_at_press: modifiers.alt,
            value: MoveValue::default(),
            locked: None,
        };
        (gesture, EditEvent::begin(id))
    }

    /// The pointer moved to `at` (document units) with `modifiers` at `zoom` screen points per
    /// document unit. ⇧ locks the offset to the gesture's directions with hysteresis from the last
    /// locked offset; without ⇧ the offset is free and the lock is forgotten. `copy` turns on when
    /// ⌥ was held at the press and `|delta| × zoom ≥ copy_reach`. Returns `Update` when the value
    /// changed since the last event, else `None`.
    pub fn drag(&mut self, at: Point, modifiers: Modifiers, zoom: f64) -> Option<EditEvent<MoveValue>> {
        // pc: crates/ui-egui/src/tools/move_mods.rs:139 `drag_to` (constrain with `last`; the
        // duplicate threshold `hypot × zoom >= 2.0`, once per drag).
        let next = self.value_at(at, modifiers, zoom);
        if next == self.value {
            return None;
        }
        self.value = next;
        Some(EditEvent::update(self.id, next, modifiers))
    }

    /// The release at `at`: the same lock as [`drag`](Self::drag) applies, so a release where
    /// the last update was ends there. Returns `Commit` with `changed = delta != 0 || copy`; a
    /// press and release without travel commits unchanged (no undo step).
    pub fn end(self, at: Point, modifiers: Modifiers, zoom: f64) -> EditEvent<MoveValue> {
        // pc: crates/ui-egui/src/tools/move_mods.rs:132 `filter_event` Up (`drag_to` then `finish`).
        let mut this = self;
        let value = this.value_at(at, modifiers, zoom);
        EditEvent::commit(this.id, value, value.delta != Vec2::ZERO || value.copy, modifiers)
    }

    /// Esc or a lost pointer: `Cancel` with [`CancelReason::Escape`](crate::edit::CancelReason).
    /// The host restores its mark, which also takes back a copy.
    pub fn cancel(self) -> EditEvent<MoveValue> {
        // new: decisions §2.5 (gestures cancel); PhotoCraft's `abandon` rolls the copy back instead.
        EditEvent::cancel(self.id, crate::edit::CancelReason::Escape)
    }

    /// The value at `at`, updating the remembered lock; `copy` stays on once on.
    fn value_at(&mut self, at: Point, modifiers: Modifiers, zoom: f64) -> MoveValue {
        let d = at - self.start;
        let delta = if modifiers.shift {
            let locked = lock_direction(d, self.locked, self.directions);
            self.locked = Some(locked);
            locked
        } else {
            self.locked = None;
            d
        };
        let copy = self.value.copy || (self.alt_at_press && delta.hypot() * zoom >= self.copy_reach);
        MoveValue { delta, copy }
    }

    /// The value of the last event: the offset and whether it copies.
    pub fn value(&self) -> MoveValue {
        self.value
    }
}

/// ⇧ during a painting stroke (every painting tool, the Freeform Pen; which tools is the app's
/// list): the stroke stays on a line at 0°, 45° or 90° from where the lock began.
///
/// The lock begins at the stroke's last point when ⇧ is down (the stroke's start, or where ⇧ went
/// down mid-stroke). Inside [`stroke_lock`](ConstrainStyle::stroke_lock) screen points of it the
/// point stays at the origin; past them the direction is the nearest 45° ray and holds until ⇧ is
/// released.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeLock {
    reach: f64,
    origin: Option<Point>,
    direction: Option<Vec2>,
}

impl StrokeLock {
    /// No lock yet; `style` gives the screen distance before the direction locks.
    pub fn new(style: &ConstrainStyle) -> Self {
        Self { reach: style.stroke_lock, origin: None, direction: None }
    }

    /// The stroke point for the pointer at `p` (document units), `last` the stroke's last point,
    /// `zoom` screen points per document unit. Without `shift` the point passes through and the
    /// lock is dropped. With it: the lock begins at `last` if none runs; within `reach / zoom` of
    /// the origin the origin comes back; the first point past it fixes the direction (nearest
    /// multiple of 45°, as [`snap_ray`]); every later point is projected on that line.
    pub fn constrain(&mut self, last: Point, p: Point, shift: bool, zoom: f64) -> Point {
        // pc: crates/ui-egui/src/tools/stroke_constraint.rs:65 `constrain` (LOCK_DISTANCE 3.0 / zoom.max(1e-3);
        // `snapped_dir` with FRAC_PI_4; `project`).
        if !shift {
            self.reset();
            return p;
        }
        let origin = *self.origin.get_or_insert(last);
        if self.direction.is_none() {
            if (p - origin).hypot() < self.reach / zoom.max(1e-3) {
                return origin;
            }
            self.direction = snap_direction(origin, p, std::f64::consts::FRAC_PI_4);
        }
        match self.direction {
            Some(dir) => origin + dir * (p - origin).dot(dir),
            None => origin,
        }
    }

    /// Whether a direction is locked.
    pub fn locked(&self) -> bool {
        self.direction.is_some()
    }

    /// Forget the lock (the stroke ended).
    pub fn reset(&mut self) {
        self.origin = None;
        self.direction = None;
    }
}

/// `p` moved onto the nearest ray from `origin` whose angle is a multiple of `step` radians
/// (projected, so the distance along the ray follows the pointer): the Gradient tool's ⇧ with
/// `step` π/4, applied from the press point on every move. `p` comes back unchanged when it is
/// within 1e-9 of `origin`, a coordinate is not finite or `step` is not positive.
pub fn snap_ray(origin: Point, p: Point, step: f64) -> Point {
    // pc: crates/ui-egui/src/tools/stroke_constraint.rs:39 `snapped_dir` (None under 1e-9), :49 `project`, :55 `snap45`.
    match snap_direction(origin, p, step) {
        Some(dir) => origin + dir * (p - origin).dot(dir),
        None => p,
    }
}

/// The unit direction from `origin` towards `p` snapped to the nearest multiple of `step` (π/4
/// for 45° rays); `None` when the points coincide, a coordinate is not finite or `step` is not
/// positive.
pub fn snap_direction(origin: Point, p: Point, step: f64) -> Option<Vec2> {
    let d = p - origin;
    if !(d.x.is_finite() && d.y.is_finite()) || d.hypot() < 1e-9 || step.is_nan() || step <= 0.0 {
        return None;
    }
    let a = (d.atan2() / step).round() * step;
    Some(Vec2::new(a.cos(), a.sin()))
}
