//! [`box_distances`]: the distances between two boxes, as lines and measures for the existing
//! [`SmartGuides`](craft_ui::canvas::SmartGuides). No egui state.

#![expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TO-R1)")]

use kurbo::Rect;

use craft_ui::canvas::{Measure, Orientation, SmartLine};

/// The distances from box `a` (the selection's bounds) to box `b` (the hovered layer's, the
/// canvas, or an ⌥-drag's copy), per axis: the gap between facing edges when the boxes are apart
/// on that axis, else the distances between like edges (left to left, right to right). Zero and
/// non-finite distances are dropped. A distance runs through the middle of the boxes' overlap on
/// the other axis, else through `a`'s centre; where it ends beside `b`, a dashed extension
/// ([`SmartLine::extension`]) continues `b`'s edge to it. `label` formats a length in document
/// units along an axis ([`Orientation::Horizontal`]: a distance along x) in the app's ruler unit.
///
/// ```
/// use craft_draw::ui::measure::box_distances;
/// use craft_ui::canvas::{SmartGuides, kurbo::Rect};
/// # fn demo(ui: &egui::Ui, view: &craft_ui::canvas::CanvasView) {
/// let (lines, measures) = box_distances(Rect::new(0.0, 0.0, 10.0, 10.0), Rect::new(30.0, 0.0, 40.0, 10.0), |len, _| format!("{len} px"));
/// SmartGuides::new(&lines, &measures).show(ui, view);
/// # }
/// ```
///
/// The app decides when they show (Smart Guides on, ⌘ held with no button down over another layer
/// or the empty canvas, an ⌥-drag copy).
pub fn box_distances(a: Rect, b: Rect, label: impl Fn(f64, Orientation) -> String) -> (Vec<SmartLine>, Vec<Measure>) {
    // pc: crates/ui-egui/src/tools/move_distance.rs:50 `measure` (each `Measure { horizontal, at,
    // from, to }` becomes a `Measure` with its label; each extension `[x0, y0, x1, y1]` a
    // `SmartLine` with `extension: true`).
    todo!()
}
