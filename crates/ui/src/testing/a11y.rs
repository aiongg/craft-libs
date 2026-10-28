//! Accessibility checks on a kittest harness, for component tests: every interactive node has a
//! role and a label, the Tab order, and a focus ring around the focused node.
//!
//! ```no_run
//! use craft_ui::testing::a11y;
//! use egui_kittest::Harness;
//! use egui_kittest::kittest::Queryable as _;
//!
//! let mut harness = Harness::new_ui(|ui| {
//!     let _ = ui.button("Save");
//!     let _ = ui.button("Cancel");
//! });
//! harness.run();
//! a11y::assert_labelled(&harness);
//! let order: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|stop| stop.label).collect();
//! assert_eq!(order, ["Save", "Cancel"]);
//! harness.get_by_label("Save").focus();
//! harness.run();
//! a11y::assert_focus_ring(&harness, harness.ctx.global_style().visuals.widgets.active.bg_stroke.color);
//! ```

use egui::accesskit::{Action, NodeId, Role};
use egui::epaint::{ColorMode, RectShape};
use egui::{Color32, Key, Rect, Shape, Stroke, StrokeKind};
use egui_kittest::kittest::{AccessKitNode, NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

/// How far inside the focused node's rect a ring's outer edge may lie, in points.
pub const RING_MAX_INSET: f32 = 3.0;
/// How far outside the focused node's rect a ring's outer edge may lie, in points.
pub const RING_MAX_OUTSET: f32 = 6.0;
/// [`tab_order`] stops after this many stops, in case focus never comes back to the first.
pub const MAX_TAB_STOPS: usize = 256;

/// The interactive nodes (ones that take focus or a click) without a role or a label, each as
/// `Role at rect` and the nearest labelled ancestor. A text field's placeholder and a text
/// node's text count as labels. Hidden nodes are skipped.
pub fn unlabelled<State>(harness: &Harness<'_, State>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .map(|node| node.accesskit_node())
        .filter(|node| is_interactive(node) && !node.is_hidden())
        .filter(|node| !has_role(node) || !has_label(node))
        .map(|node| describe(&node))
        .collect()
}

/// Fails the test when [`unlabelled`] finds a node.
#[track_caller]
pub fn assert_labelled<State>(harness: &Harness<'_, State>) {
    let missing = unlabelled(harness);
    assert!(missing.is_empty(), "{} interactive node(s) lack a role or a label:\n{}", missing.len(), missing.join("\n"));
}

/// One stop of the Tab order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabStop {
    pub role: Role,
    /// The accessible label, empty when the node has none.
    pub label: String,
}

/// Presses Tab until focus returns to the first node it reached (or nothing has focus), and
/// returns the nodes focused on the way, in order. Focus moves, so the harness ends with the
/// first stop focused again.
pub fn tab_order<State>(harness: &mut Harness<'_, State>) -> Vec<TabStop> {
    let mut stops = Vec::new();
    let mut first: Option<NodeId> = None;
    for _ in 0..MAX_TAB_STOPS {
        harness.key_press(Key::Tab);
        harness.run();
        let Some(node) = focused(harness) else { break };
        let node = node.accesskit_node();
        let id = node.locate().0;
        if first == Some(id) {
            break;
        }
        first.get_or_insert(id);
        stops.push(TabStop { role: node.role(), label: node.label().unwrap_or_default() });
    }
    stops
}

/// A stroked outline painted in the last frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    /// The outer edge of the stroke.
    pub rect: Rect,
    pub stroke: Stroke,
}

/// The focused node's rect and every stroked rect or closed path of the last frame whose outer
/// edge lies around it: no more than [`RING_MAX_INSET`] inside and [`RING_MAX_OUTSET`] outside.
/// `None` when nothing has focus.
pub fn focus_rings<State>(harness: &Harness<'_, State>) -> Option<(Rect, Vec<Ring>)> {
    let target = focused(harness)?.rect();
    let mut rings = Vec::new();
    for clipped in &harness.output().shapes {
        collect_rings(&clipped.shape, &mut rings);
    }
    let (inner, outer) = (target.shrink(RING_MAX_INSET), target.expand(RING_MAX_OUTSET));
    rings.retain(|ring| ring.rect.contains_rect(inner) && outer.contains_rect(ring.rect));
    Some((target, rings))
}

/// Fails the test unless a ring of `color` is painted around the focused node ([`focus_rings`]).
/// Pass the colour the component draws focus in, e.g. `line.focus`.
#[track_caller]
#[allow(clippy::panic)] // a test assertion
pub fn assert_focus_ring<State>(harness: &Harness<'_, State>, color: Color32) {
    let Some((target, rings)) = focus_rings(harness) else { panic!("no node has keyboard focus") };
    assert!(rings.iter().any(|ring| ring.stroke.color == color), "no {color:?} ring around the focused node at {target:?}; rings there: {rings:?}");
}

fn focused<'h, State>(harness: &'h Harness<'_, State>) -> Option<Node<'h>> {
    harness.query_by(|node| node.is_focused())
}

fn is_interactive(node: &AccessKitNode<'_>) -> bool {
    let data = node.data();
    data.supports_action(Action::Focus) || data.supports_action(Action::Click)
}

fn has_role(node: &AccessKitNode<'_>) -> bool {
    !matches!(node.role(), Role::Unknown | Role::GenericContainer)
}

fn has_label(node: &AccessKitNode<'_>) -> bool {
    let named = |text: Option<String>| text.is_some_and(|t| !t.trim().is_empty());
    // A text node (selectable labels take clicks) is named by its text.
    named(node.label()) || (node.label_comes_from_value() && named(node.value())) || node.data().placeholder().is_some_and(|p| !p.trim().is_empty())
}

fn describe(node: &AccessKitNode<'_>) -> String {
    let ancestor = std::iter::successors(node.parent(), AccessKitNode::parent)
        .find_map(|ancestor| ancestor.label())
        .map_or_else(String::new, |label| format!(" inside {label:?}"));
    format!("{:?} at {:?}{ancestor}", node.role(), node.raw_bounds())
}

fn collect_rings(shape: &Shape, out: &mut Vec<Ring>) {
    let visible = |width: f32, color: Color32| width > 0.0 && color.a() > 0;
    let outer = |rect: Rect, width: f32, kind: StrokeKind| match kind {
        StrokeKind::Inside => rect,
        StrokeKind::Middle => rect.expand(width / 2.0),
        StrokeKind::Outside => rect.expand(width),
    };
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|s| collect_rings(s, out)),
        Shape::Rect(RectShape { rect, stroke, stroke_kind, .. }) if visible(stroke.width, stroke.color) => {
            out.push(Ring { rect: outer(*rect, stroke.width, *stroke_kind), stroke: *stroke });
        }
        Shape::Path(path) if path.closed => {
            if let ColorMode::Solid(color) = path.stroke.color
                && visible(path.stroke.width, color)
            {
                let rect = Rect::from_points(&path.points);
                out.push(Ring { rect: outer(rect, path.stroke.width, path.stroke.kind), stroke: Stroke::new(path.stroke.width, color) });
            }
        }
        _ => {}
    }
}
