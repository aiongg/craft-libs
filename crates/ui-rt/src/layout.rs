//! One layout pass: constraints go down, sizes come up. Each node caches its last constraints and
//! size, so a clean node with the same constraints returns at once. There is no measuring run.
//! After the pass, each window's layer children are laid out and placed against their anchors.

use std::ops::Range;
use std::sync::Arc;

use craft_theme::Tokens;
use emath::Vec2;

use crate::geom::Constraints;
use crate::id::{NodeId, WindowId};
use crate::layer::Layer;
use crate::runtime::Runtime;
use crate::state::{Access, Phase, Reader, Reads, Store, Writes};
use crate::text::{TextEngine, TextLayout, TextSpec};
use crate::tree::{Dirty, Role};
use crate::window::Env;

pub struct LayoutCx<'a> {
    rt: &'a mut Runtime,
    text: &'a mut dyn TextEngine,
    node: NodeId,
}

impl Reads for LayoutCx<'_> {
    fn access(&mut self) -> Access<'_> {
        Access { store: &mut self.rt.store, reader: Some(Reader::Node(self.node, Phase::Layout)) }
    }
}

/// Layout may write cells: a virtual list publishes the row range it needs, and the build that
/// reads it runs again in the same frame.
impl Writes for LayoutCx<'_> {
    fn store_mut(&mut self) -> &mut Store {
        &mut self.rt.store
    }
}

impl LayoutCx<'_> {
    pub fn node(&self) -> NodeId {
        self.node
    }

    pub fn env(&self) -> Env {
        self.rt.env_of(self.node)
    }

    pub fn tokens(&mut self) -> Arc<Tokens> {
        let tokens = self.env().tokens;
        tokens.get(self)
    }

    pub fn scale(&mut self) -> f32 {
        let scale = self.env().scale;
        scale.get(self)
    }

    pub fn child_count(&self) -> usize {
        self.rt.tree.nodes.get(&self.node).map_or(0, |n| n.children.len())
    }

    pub fn child_at(&self, index: usize) -> NodeId {
        self.rt.tree.nodes.get(&self.node).and_then(|n| n.children.get(index).copied()).unwrap_or(NodeId(0))
    }

    /// Lays out one child. A clean child given its previous constraints is not entered.
    pub fn layout_child(&mut self, child: NodeId, c: Constraints) -> Vec2 {
        layout_node(self.rt, self.text, child, c)
    }

    /// Sets a child's offset in this node's space. Moving a child repaints nothing.
    pub fn place_child(&mut self, child: NodeId, offset: Vec2) {
        debug_assert_eq!(self.rt.tree.nodes.get(&child).and_then(|n| n.parent), Some(self.node), "place_child on a node that is not a child");
        set_offset(self.rt, child, offset);
    }

    /// Shapes text at the current scale. Reading the scale records the read, so a scale change
    /// lays this node out again.
    pub fn text_layout(&mut self, spec: &TextSpec) -> TextLayout {
        let scale = self.scale();
        self.text.layout(spec, scale)
    }

    pub fn text_engine(&mut self) -> &mut dyn TextEngine {
        self.text
    }
}

fn set_offset(rt: &mut Runtime, id: NodeId, offset: Vec2) {
    let Some(n) = rt.tree.nodes.get_mut(&id) else {
        return;
    };
    if n.frame.offset != offset {
        n.frame.offset = offset;
        rt.note_moved(id);
    }
}

pub(crate) fn layout_node(rt: &mut Runtime, text: &mut dyn TextEngine, id: NodeId, c: Constraints) -> Vec2 {
    let Some(node) = rt.tree.nodes.get_mut(&id) else {
        return Vec2::ZERO;
    };
    if !node.dirty.has(Dirty::LAYOUT) && node.constraints == Some(c) {
        return node.frame.size;
    }
    let Some(mut el) = node.element.take() else {
        return node.frame.size;
    };
    node.constraints = Some(c);
    let window = node.window;
    let size = c.constrain(el.layout(&mut LayoutCx { rt: &mut *rt, text, node: id }, c));
    let boundary = c.is_tight() || el.relayout_boundary();
    rt.counters.laid_out += 1;
    if let Some(w) = rt.windows.get_mut(&window) {
        w.relayout = true;
    }
    let fixed_parent = rt.tree.nodes.get(&id).and_then(|n| n.parent).and_then(|p| rt.tree.nodes.get(&p)).is_some_and(|p| p.is_fixed());
    let Some(node) = rt.tree.nodes.get_mut(&id) else {
        return size;
    };
    node.element = Some(el);
    node.dirty.remove(Dirty::LAYOUT);
    node.boundary = boundary || fixed_parent;
    if node.frame.size != size {
        node.frame.size = size;
        rt.tree.mark_paint(id);
        rt.access_dirty(id);
    }
    size
}

/// The constraints of a node directly on a layer: the app root fills the window; a layer child
/// gets what its [`crate::LayerSpec`] asks for.
fn layer_child_constraints(rt: &Runtime, id: NodeId, layer: Layer, window: WindowId) -> Option<Constraints> {
    let w = rt.windows.get(&window)?;
    if layer == Layer::Content {
        return Some(Constraints::tight(w.viewport));
    }
    let spec = w.layered.get(&id)?;
    let anchor = spec.anchor_node().and_then(|a| rt.window_rect(a));
    Some(spec.constraints(w.viewport, anchor))
}

/// Lays out every queued relayout boundary, owners first. A boundary whose size changed anyway
/// (an element that broke its `relayout_boundary` promise) queues its parent.
pub(crate) fn layout_pass(rt: &mut Runtime, text: &mut dyn TextEngine) {
    while !rt.tree.layout_q.is_empty() {
        let queue = std::mem::take(&mut rt.tree.layout_q);
        let queue = rt.tree.by_depth(queue);
        for id in queue {
            let Some(node) = rt.tree.nodes.get(&id) else {
                continue;
            };
            if !node.dirty.has(Dirty::LAYOUT) {
                continue;
            }
            let (window, before) = (node.window, node.frame.size);
            let Some(parent) = node.parent else { continue };
            let layer = match rt.tree.nodes.get(&parent).map(|p| p.role) {
                Some(Role::Layer(layer)) => Some(layer),
                _ => None,
            };
            let c = match layer {
                Some(layer) => layer_child_constraints(rt, id, layer, window),
                None => node.constraints,
            };
            let Some(c) = c else {
                rt.tree.mark_layout(parent);
                continue;
            };
            let after = layout_node(rt, text, id, c);
            if layer.is_none() && after != before {
                rt.tree.mark_layout(parent);
            }
        }
    }
}

/// Lays out and places the children of every layer above content, in each window where a layout
/// ran this frame: anchors may have moved, and a child's size decides which side it fits on.
pub(crate) fn place_layers(rt: &mut Runtime, text: &mut dyn TextEngine) {
    let windows: Vec<WindowId> = rt.windows.iter().filter(|(_, w)| w.relayout).map(|(id, _)| *id).collect();
    for window in windows {
        let Some(w) = rt.windows.get(&window) else { continue };
        let (viewport, layers) = (w.viewport, w.layers);
        for layer_node in &layers[1..] {
            let children = rt.tree.nodes.get(layer_node).map(|n| n.children.clone()).unwrap_or_default();
            for id in children {
                let Some(spec) = rt.windows.get(&window).and_then(|w| w.layered.get(&id)).copied() else { continue };
                let anchor = spec.anchor_node().and_then(|a| rt.window_rect(a));
                let size = layout_node(rt, text, id, spec.constraints(viewport, anchor));
                let current = rt.tree.nodes.get(&id).map_or(Vec2::ZERO, |n| n.frame.offset);
                set_offset(rt, id, spec.place(size, viewport, anchor, current));
            }
        }
        if let Some(w) = rt.windows.get_mut(&window) {
            w.relayout = false;
        }
    }
}

/// The virtual list hook: the band of rows a list builds. It keeps `current` while the visible
/// rows stay inside it, so scrolling within the band rebuilds nothing; otherwise it returns the
/// visible rows plus `overscan` on each side. Rows outside the band are not built, so their nodes
/// drop.
///
/// The list's layout calls this and writes the result to a cell its build reads; the run loop then
/// rebuilds the list in the same frame.
pub fn virtual_band(offset: f32, viewport: f32, row_extent: f32, count: usize, overscan: usize, current: Range<usize>) -> Range<usize> {
    if count == 0 || row_extent <= 0.0 {
        return 0..0;
    }
    let first = ((offset.max(0.0) / row_extent).floor() as usize).min(count - 1);
    let end = (((offset.max(0.0) + viewport) / row_extent).ceil() as usize).clamp(first + 1, count);
    let inside = current.start <= first && end <= current.end && current.end <= count;
    if inside { current } else { first.saturating_sub(overscan)..(end + overscan).min(count) }
}
