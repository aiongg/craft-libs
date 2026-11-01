//! The corner widgets' geometry over [`fillet`](crate::geometry::fillet::fillet) (task PE-A5):
//! placement, the hide rules, the hit test and the drag maths. Document units unless a name says
//! screen.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task PE-A5)")]

use egui::Pos2;
use kurbo::{Point, Vec2};

use super::{CornerEdit, CornerPoint, CornerRef, CornerTarget};
use craft_ui::canvas::CanvasView;

/// A widget the pass shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Widget {
    pub corner: CornerRef,
    /// Its centre on the screen.
    pub at: Pos2,
    /// How far the arc's centre moves per unit of radius ([`Fillet::step`](crate::geometry::fillet::Fillet::step)).
    pub step: Vec2,
    /// The corner's radius now, capped at [`max_radius`].
    pub radius: f64,
}

/// The largest radius `corner` takes: `fillet(prev, vertex, next, 1e12)`'s radius, 0 for a
/// degenerate corner. For a rectangle's corner, `min(w, h) / 2` (lead call 23).
pub(super) fn max_radius(corner: &CornerPoint) -> f64 {
    // pc: crates/ui-egui/src/tools/corner_radius.rs:91-97 `max_radius`, :113 (`fillet(.., 1e12)`).
    todo!()
}

/// Where `corner`'s widget sits for `radius`: at the arc's centre (`vertex + step * r`), with
/// `r = radius.max(inset).min(max_radius)` where the inset is `inset_screen / zoom * √2 / |step|`
/// (measured like a rectangle's: `inset_screen` points along each axis of a right angle), so a
/// sharp or slightly rounded corner's widget sits `corner.inset` points inside. `None` for a
/// degenerate corner.
pub(super) fn widget_at(corner: &CornerPoint, radius: f64, inset_screen: f32, zoom: f64) -> Option<Point> {
    // pc: crates/ui-egui/src/tools/corner_radius.rs:107-120 `widget`, :22 `MIN_INSET`.
    todo!()
}

/// Whether `corner`'s widget shows: both its sides at least `min_side_screen` points long on the
/// screen (`corner.min_side`), and its inner angle (`2 * half_angle`, degrees) not wider than
/// `hide_above` degrees (180 by default; VectorCraft's preference passes 177).
pub(super) fn shown(corner: &CornerPoint, view: &CanvasView, min_side_screen: f32, hide_above: f64) -> bool {
    // vectorcraft: crates/tools/src/corners.rs:23-25 (`MIN_INSET_PX` 10, `MIN_SIDE_PX` 30);
    // crates/engine/src/prefs.rs:60, :355 (`hide_corner_widget_above`, 177).
    todo!()
}

/// The widgets of `targets` the pass shows, in target order and corner order (the host lists the
/// topmost target first).
pub(super) fn widgets(targets: &[CornerTarget], view: &CanvasView, inset_screen: f32, min_side_screen: f32, hide_above: f64) -> Vec<Widget> {
    // pc: crates/ui-egui/src/tools/corner_radius.rs:322-327 (every corner of every target).
    todo!()
}

/// The widget under the screen point `at`: the nearest within `reach` screen points
/// (`corner.hit`); of equal distances the earlier (topmost) one.
pub(super) fn hit(widgets: &[Widget], at: Pos2, reach: f32) -> Option<usize> {
    // pc: crates/ui-egui/src/tools/corner_radius.rs:209-221 `hit` (7 points).
    todo!()
}

/// The dragged radius: the radius at the press plus the pointer's move from `press` to `pointer`
/// projected on `step` (`dot(move, step) / |step|²`), not below 0; `from` when not finite. A
/// click without movement gives `from`, so it changes nothing.
pub(super) fn dragged_radius(from: f64, step: Vec2, press: Point, pointer: Point) -> f64 {
    // pc: crates/ui-egui/src/tools/corner_radius.rs:250-256.
    todo!()
}

/// The edit a drag of `grabbed` to `radius` makes: every target takes `radius`, capped at each
/// corner's [`max_radius`]; with `one_corner` (⌥) an `individual` target changes only the grabbed
/// corner (the grabbed target's; other individual targets the corner with the same index), and a
/// target without `individual` changes all its corners anyway. Targets whose radii stay the same
/// are left out.
pub(super) fn drag_edit(targets: &[CornerTarget], grabbed: CornerRef, radius: f64, one_corner: bool) -> CornerEdit {
    // pc: crates/ui-egui/src/tools/corner_radius.rs:257-268 (same radius within each shape's
    // maximum; ⌥ for rectangles only), :279-282 (unchanged layers skipped).
    todo!()
}
