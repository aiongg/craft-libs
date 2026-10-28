//! A debug recorder for inspector overlays: the layout boxes the primitives drew and the
//! AccessKit tree of the last pass.
//!
//! Off by default. While [`set_enabled`] is on for a context:
//!
//! - [`Surface`](crate::primitives::Surface), [`Stack`](crate::primitives::Stack),
//!   [`Row`](crate::primitives::Row), [`Interactive`](crate::primitives::Interactive) and
//!   [`Handle`](crate::primitives::Handle) each push a [`Record`] per pass, and [`records`] returns
//!   the previous pass's;
//! - AccessKit is enabled on the context, and [`tree`] returns the tree of the previous pass's
//!   output, on desktop and web alike.
//!
//! When no context has it on, each primitive pays one atomic load.
//!
//! ```
//! use craft_ui::inspect;
//! use craft_ui::primitives::{Gap, Stack};
//!
//! let ctx = egui::Context::default();
//! inspect::set_enabled(&ctx, true);
//! for _ in 0..2 {
//!     ctx.run_ui(Default::default(), |ui| {
//!         Stack::new(Gap::Stack).show(ui, |ui| ui.label("Layers"));
//!     })
//!     .drop_without_applying_deltas();
//! }
//! assert!(inspect::records(&ctx).iter().any(|r| r.kind == inspect::RecordKind::Stack));
//! assert!(inspect::tree(&ctx).is_some());
//! ```

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use egui::accesskit::{Action, Node, NodeId, Role, TreeUpdate};
use egui::plugin::Plugin;
use egui::{Context, FullOutput, Id, LayerId, RawInput, Rect, Ui, pos2};

use crate::primitives::SurfaceKind;

/// The number of contexts with inspection on: zero keeps [`record`] to one load.
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// Which primitive drew a [`Record`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RecordKind {
    /// [`Surface::show`](crate::primitives::Surface::show) of this kind.
    Surface(SurfaceKind),
    /// [`Stack::show`](crate::primitives::Stack::show).
    Stack,
    /// [`Row::show`](crate::primitives::Row::show).
    Row,
    /// [`Interactive`](crate::primitives::Interactive) with this AccessKit role.
    Interactive(Role),
    /// [`Handle::show`](crate::primitives::Handle::show).
    Handle,
}

/// One primitive's layout box in one pass, in the points of its layer.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Record {
    /// The primitive's widget or `Ui` id.
    pub id: Id,
    pub kind: RecordKind,
    /// The whole box: a surface's or interactive element's rect, a layout's rect with its
    /// padding, a handle's hit square.
    pub outer: Rect,
    /// The box inside the padding: a padded layout's contents, a handle's drawn body; else
    /// `outer`. The padding is the band between the two.
    pub content: Rect,
    /// The layer the primitive drew on.
    pub layer: LayerId,
}

/// Turns inspection on or off for `ctx`. On, it enables AccessKit, so the next pass builds a
/// tree. Off, it disables AccessKit again unless it was on before (a screen reader or a test
/// harness had enabled it), and drops the records and the tree.
///
/// [`theme::install`](crate::theme::install) registers the recorder, so it knows from earlier
/// passes whether AccessKit was on; in a context without it, only the current pass tells.
pub fn set_enabled(ctx: &Context, on: bool) {
    track(ctx);
    let changed = ctx
        .with_plugin(|inspector: &mut Inspector| {
            if inspector.on == on {
                return false;
            }
            inspector.on = on;
            if on {
                ACTIVE.fetch_add(1, Ordering::Relaxed);
                let already = inspector.accesskit_seen || ctx.has_accesskit_node(egui::accesskit_root_id());
                inspector.enabled_accesskit = !already;
                ctx.enable_accesskit();
            } else {
                ACTIVE.fetch_sub(1, Ordering::Relaxed);
                if std::mem::take(&mut inspector.enabled_accesskit) {
                    ctx.disable_accesskit();
                }
                inspector.current.clear();
                inspector.previous.clear();
                inspector.tree = None;
            }
            true
        })
        .unwrap_or(false);
    if changed {
        ctx.request_repaint();
    }
}

/// Whether inspection is on for `ctx`.
pub fn enabled(ctx: &Context) -> bool {
    ctx.with_plugin(|inspector: &mut Inspector| inspector.on).unwrap_or(false)
}

/// The records of the previous pass, in the order the primitives finished drawing; empty while
/// inspection is off.
pub fn records(ctx: &Context) -> Vec<Record> {
    ctx.with_plugin(|inspector: &mut Inspector| inspector.previous.clone()).unwrap_or_default()
}

/// The AccessKit tree of the previous pass's output; `None` while inspection is off and until a
/// pass with AccessKit on has ended.
pub fn tree(ctx: &Context) -> Option<Arc<AccessTree>> {
    ctx.with_plugin(|inspector: &mut Inspector| inspector.tree.clone()).flatten()
}

/// Registers the recorder in `ctx`, once. Called by [`theme::install`](crate::theme::install).
pub(crate) fn track(ctx: &Context) {
    if ctx.plugin_opt::<Inspector>().is_none() {
        ctx.add_plugin(Inspector::default());
    }
}

/// Records a primitive's box while inspection is on for `ui`'s context.
#[inline]
pub(crate) fn record(ui: &Ui, id: Id, kind: RecordKind, outer: Rect, content: Rect) {
    if ACTIVE.load(Ordering::Relaxed) != 0 {
        push(ui, Record { id, kind, outer, content, layer: ui.layer_id() });
    }
}

#[inline(never)]
fn push(ui: &Ui, record: Record) {
    ui.ctx().with_plugin(|inspector: &mut Inspector| {
        if inspector.on {
            inspector.current.push(record);
        }
    });
}

/// The recorder: records of this pass and the last, the last tree, and AccessKit's state.
#[derive(Default)]
struct Inspector {
    on: bool,
    /// Inspection enabled AccessKit, so turning it off disables it again.
    enabled_accesskit: bool,
    /// Inspection had enabled AccessKit when the current pass began.
    enabled_for_pass: bool,
    /// The last pass that began without inspection's AccessKit had an AccessKit update.
    accesskit_seen: bool,
    current: Vec<Record>,
    previous: Vec<Record>,
    tree: Option<Arc<AccessTree>>,
}

impl Plugin for Inspector {
    fn debug_name(&self) -> &'static str {
        "craft_ui::inspect"
    }

    fn input_hook(&mut self, ctx: &Context, _: &mut RawInput) {
        self.enabled_for_pass = self.enabled_accesskit;
        // The platform may disable AccessKit (a screen reader quits) while inspection is on.
        if self.on {
            ctx.enable_accesskit();
        }
    }

    fn output_hook(&mut self, ctx: &Context, output: &mut FullOutput) {
        let update = output.platform_output.accesskit_update.as_ref();
        if !self.enabled_for_pass {
            self.accesskit_seen = update.is_some();
        }
        if !self.on {
            return;
        }
        self.previous = std::mem::take(&mut self.current);
        let first = self.tree.is_none();
        self.tree = update.and_then(AccessTree::from_update).map(Arc::new);
        if first && self.tree.is_some() {
            // The overlay reads the tree in the next pass.
            ctx.request_repaint();
        }
    }
}

/// The AccessKit tree of one pass, by node id.
///
/// egui gives every `Ui` and every widget with a role a node, children in the order they were
/// made; bounds are in points.
#[derive(Clone, Debug)]
pub struct AccessTree {
    root: NodeId,
    focus: NodeId,
    nodes: HashMap<NodeId, Node>,
}

impl AccessTree {
    /// The tree of a full update (one with tree info, as egui sends every pass).
    pub fn from_update(update: &TreeUpdate) -> Option<AccessTree> {
        let root = update.tree.as_ref()?.root;
        Some(AccessTree { root, focus: update.focus, nodes: update.nodes.iter().cloned().collect() })
    }

    /// The window's node.
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// The focused node, or the root when nothing has focus.
    pub fn focus(&self) -> NodeId {
        self.focus
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// The node of the widget or `Ui` with `id`, if it has one.
    pub fn of(&self, id: Id) -> Option<NodeId> {
        let node = id.accesskit_id();
        self.nodes.contains_key(&node).then_some(node)
    }

    /// `from` and its descendants, depth first, each with its depth below `from`.
    pub fn walk(&self, from: NodeId) -> Vec<(NodeId, usize)> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![(from, 0)];
        while let Some((id, depth)) = stack.pop() {
            let Some(node) = self.nodes.get(&id) else { continue };
            if !seen.insert(id) {
                continue;
            }
            out.push((id, depth));
            stack.extend(node.children().iter().rev().map(|child| (*child, depth + 1)));
        }
        out
    }

    /// The nodes under `from` (and `from` itself) that Tab visits, in Tab order: enabled
    /// nodes with the Focus action in the order egui made them.
    ///
    /// egui moves focus in the order widgets register in a pass, which is the tree's depth-first
    /// order. A modal layer limits Tab to itself and above; this order does not.
    pub fn focus_order(&self, from: NodeId) -> Vec<NodeId> {
        self.walk(from)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| self.nodes.get(id).is_some_and(|n| n.supports_action(Action::Focus) && !n.is_disabled() && !n.is_hidden()))
            .collect()
    }

    /// The interactive nodes under `from` (and `from` itself): ones that take focus or a click.
    pub fn interactive(&self, from: NodeId) -> Vec<NodeId> {
        self.walk(from).into_iter().map(|(id, _)| id).filter(|id| self.nodes.get(id).is_some_and(is_interactive)).collect()
    }

    /// A node's accessible name, as a screen reader reads it: its label (a text node's value),
    /// else the texts of the nodes it is labelled by (`Response::labelled_by`), else its
    /// placeholder. `None` when nothing names it.
    pub fn name(&self, id: NodeId) -> Option<String> {
        let node = self.nodes.get(&id)?;
        if let Some(text) = own_text(node) {
            return Some(text.to_owned());
        }
        let by: Vec<&str> = node.labelled_by().iter().filter_map(|l| self.nodes.get(l).and_then(own_text)).collect();
        if !by.is_empty() {
            return Some(by.join(" "));
        }
        node.placeholder().filter(|t| !t.trim().is_empty()).map(str::to_owned)
    }

    /// A node's bounds in points.
    pub fn bounds(&self, id: NodeId) -> Option<Rect> {
        let b = self.nodes.get(&id)?.bounds()?;
        Some(Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32)))
    }
}

/// Whether `node` takes focus or a click.
pub fn is_interactive(node: &Node) -> bool {
    node.supports_action(Action::Focus) || node.supports_action(Action::Click)
}

/// The text a node shows as a name: a text node's value, else its label.
fn own_text(node: &Node) -> Option<&str> {
    let text = if node.role() == Role::Label { node.value().or(node.label()) } else { node.label() };
    text.filter(|t| !t.trim().is_empty())
}

#[cfg(test)]
#[path = "inspect_tests.rs"]
mod tests;
