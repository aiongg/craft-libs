//! The pointer's look per window. Nodes name cursors in paint: for a hit shape
//! ([`crate::PaintCx::hit_cursor`]) or for themselves and their descendants
//! ([`crate::PaintCx::cursor`]). A window picks its cursor again only when what decides it
//! changed (the shape under the pointer, a press starting or ending, a drag's target, a repaint of
//! a node that names cursors), and the platform hears only a cursor that differs from the last one
//! sent.
//!
//! Order: a drag over the window shows its operation; a press shows the cursor of the shape it went
//! down on, else of the nearest node above that shape that names one, wherever the pointer goes;
//! otherwise the hovered shape's cursor, else the nearest hovered node's; else the default.
//!
//! A painted pointer ([`PaintedPointer`]) is a drawing the node recorded in its paint. The window
//! shows it at the pointer on its pointer layer, over the platform cursor the pointer names as its
//! base; a confined one (a brush tip, a mark on its canvas) shows on the naming node's layer clipped
//! to the node, under every layer above it. A pointer move only moves the drawing: no node builds or
//! repaints, except a confined drawing, which repaints its own node for its clip.

use std::sync::Arc;

use emath::{Pos2, Rect, Vec2};

use crate::build::BuildCx;
use crate::display::DisplayList;
use crate::element::{Element, ElementBox};
use crate::geom::Constraints;
use crate::id::{ChildKey, NodeId, WindowId};
use crate::layer::{Layer, LayerSpec, Placement};
use crate::layout::LayoutCx;
use crate::paint::PaintCx;
use crate::platform::Cursor;
use crate::runtime::Runtime;
use crate::tree::{Node, Role};

/// A pointer the window draws itself: `art`, recorded with its top left at the origin, shows with
/// its top left at the pointer plus `offset`, over the platform cursor `base` ([`Cursor::Hidden`]
/// for a pointer drawn whole, a system arrow for a badge beside it).
#[derive(Clone, Debug)]
pub struct PaintedPointer {
    pub base: Cursor,
    pub art: Arc<DisplayList>,
    /// The drawing while a press runs, where it differs (an open hand that closes to a fist).
    pub pressed: Option<Arc<DisplayList>>,
    pub offset: Vec2,
    pub size: Vec2,
    /// A mark on the naming node (a brush tip on its canvas): it shows on the node's layer, clipped
    /// to the node's rect, under every layer above. Otherwise it shows over every layer.
    pub confined: bool,
}

impl PartialEq for PaintedPointer {
    /// Drawings compare by identity: a node that repaints records new ones.
    fn eq(&self, other: &Self) -> bool {
        let same = |a: &Option<Arc<DisplayList>>, b: &Option<Arc<DisplayList>>| match (a, b) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        self.base == other.base
            && Arc::ptr_eq(&self.art, &other.art)
            && same(&self.pressed, &other.pressed)
            && self.offset == other.offset
            && self.size == other.size
            && self.confined == other.confined
    }
}

/// The look a shape or node names: its platform cursor and, for a painted pointer, the index of
/// the drawing among its node's ([`crate::PaintCx::hit_painted`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShapeCursor {
    pub(crate) cursor: Cursor,
    pub(crate) painted: Option<u16>,
}

/// The painted pointer a window shows: the node that named it, the drawing, and the node that
/// draws it with where it last placed it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PointerShown {
    owner: NodeId,
    pointer: PaintedPointer,
    node: NodeId,
    placed: Option<(Pos2, bool)>,
}

/// The pointer node's key among its layer's children.
const POINTER_KEY: u32 = 0x5017;

/// Draws a painted pointer; `clip` (local) confines it to the naming node.
struct PointerElement {
    art: Arc<DisplayList>,
    size: Vec2,
    clip: Option<Rect>,
}

impl Element for PointerElement {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        c.constrain(self.size)
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        if let Some(clip) = self.clip {
            cx.clip(clip);
        }
        cx.extend(&self.art);
    }
}

impl Runtime {
    /// Picks `window`'s cursor when something that decides it changed, and queues it for the
    /// platform when it differs from the last one sent.
    pub(crate) fn refresh_cursor(&mut self, window: WindowId) {
        let Some(w) = self.windows.get_mut(&window) else { return };
        if !std::mem::take(&mut w.input.cursor_check) {
            self.follow_pointer(window);
            return;
        }
        let (cursor, painted) = self.pick_cursor(window);
        let Some(w) = self.windows.get_mut(&window) else { return };
        if w.input.cursor != cursor {
            w.input.cursor = cursor;
            self.cursor_requests.push((window, cursor));
        }
        let painted = painted.and_then(|(owner, i)| Some((owner, self.tree.nodes.get(&owner)?.painted.get(usize::from(i))?.clone())));
        self.show_painted(window, painted);
    }

    fn pick_cursor(&self, window: WindowId) -> (Cursor, Option<(NodeId, u16)>) {
        if let Some(c) = self.drag_cursor(window) {
            return (c, None);
        }
        let Some(input) = self.windows.get(&window).map(|w| &w.input) else { return (Cursor::Default, None) };
        let source = input.press_cursor_source().or_else(|| input.hover_cursor_source());
        let Some((node, shape)) = source else { return (Cursor::Default, None) };
        match shape.map(|s| (node, s)).or_else(|| self.node_cursor(node)) {
            Some((owner, s)) => (s.cursor, s.painted.map(|i| (owner, i))),
            None => (Cursor::Default, None),
        }
    }

    /// The cursor of the nearest node from `node` up that names one, with that node.
    fn node_cursor(&self, node: NodeId) -> Option<(NodeId, ShapeCursor)> {
        let mut cur = Some(node);
        while let Some(id) = cur {
            let n = self.tree.nodes.get(&id)?;
            if let Some(cursor) = n.cursor {
                return Some((id, ShapeCursor { cursor, painted: n.cursor_painted }));
            }
            cur = n.parent;
        }
        None
    }

    /// Shows `painted` (its naming node and drawing) as `window`'s painted pointer, or none.
    fn show_painted(&mut self, window: WindowId, painted: Option<(NodeId, PaintedPointer)>) {
        let Some(w) = self.windows.get(&window) else { return };
        let layer = painted.as_ref().and_then(|(owner, pointer)| match pointer.confined {
            true => self.layer_root_of(*owner),
            false => Some(w.layer(Layer::Pointer)),
        });
        let node = layer.map(|l| l.child(ChildKey::Fixed(POINTER_KEY)));
        let shown = self.windows.get(&window).and_then(|w| w.input.painted.as_ref());
        let same = match (&painted, shown) {
            (Some((owner, pointer)), Some(s)) => s.owner == *owner && s.pointer == *pointer && Some(s.node) == node,
            (None, None) => true,
            _ => false,
        };
        if !same {
            let old = self.windows.get_mut(&window).and_then(|w| w.input.painted.take());
            if let Some(old) = old {
                if let Some(w) = self.windows.get_mut(&window) {
                    w.layered.remove(&old.node);
                }
                self.remove_subtree(old.node);
            }
            if let (Some((owner, pointer)), Some(node), Some(layer)) = (painted, node, layer) {
                let element = PointerElement { art: pointer.art.clone(), size: pointer.size, clip: None };
                let mut n = Node::new(Some(layer), None, window, 1, Some(Box::new(ElementBox::new(element))), 0);
                n.boundary = true;
                self.tree.nodes.insert(node, n);
                if let Some(l) = self.tree.nodes.get_mut(&layer) {
                    l.children.push(node);
                }
                if pointer.confined {
                    self.stack_confined(layer, node, owner);
                }
                self.tree.mark_build(node);
                if let Some(w) = self.windows.get_mut(&window) {
                    w.input.painted = Some(PointerShown { owner, pointer, node, placed: None });
                }
            }
        }
        self.follow_pointer(window);
    }

    /// Moves `window`'s painted pointer to the pointer: a layer placement, so nothing repaints, except a
    /// confined drawing, whose clip moves with it, and a change of the drawing a press shows.
    pub(crate) fn follow_pointer(&mut self, window: WindowId) {
        let Some(w) = self.windows.get(&window) else { return };
        let Some(shown) = w.input.painted.as_ref() else { return };
        let (Some(at), pressed) = (w.input.pointer_pos(), w.input.pressing()) else {
            // The pointer left the window: the drawing goes with it until it comes back.
            if let Some(w) = self.windows.get_mut(&window)
                && let Some(shown) = w.input.painted.as_mut()
                && shown.placed.take().is_some()
            {
                let node = shown.node;
                w.layered.remove(&node);
                w.relayout = true;
            }
            return;
        };
        if shown.placed == Some((at, pressed)) {
            return;
        }
        let (node, owner, pointer, was) = (shown.node, shown.owner, shown.pointer.clone(), shown.placed);
        if pointer.confined
            && let Some(layer) = self.tree.nodes.get(&node).and_then(|n| n.parent)
        {
            self.stack_confined(layer, node, owner);
        }
        let min = at + pointer.offset;
        let clip = pointer.confined.then(|| self.window_rect(owner).unwrap_or(Rect::NOTHING).translate(-min.to_vec2()));
        let art = match pressed {
            true => pointer.pressed.clone().unwrap_or_else(|| pointer.art.clone()),
            false => pointer.art.clone(),
        };
        let repaint = was.is_none() || clip.is_some() || was.is_some_and(|(_, p)| p != pressed);
        if repaint && let Some(n) = self.tree.nodes.get_mut(&node) {
            n.element = Some(Box::new(ElementBox::new(PointerElement { art, size: pointer.size, clip })));
            self.tree.mark_build(node);
            self.tree.mark_paint(node);
        }
        let Some(w) = self.windows.get_mut(&window) else { return };
        if let Some(shown) = w.input.painted.as_mut() {
            shown.placed = Some((at, pressed));
        }
        let spec = LayerSpec::new(Placement::At(min));
        if w.layered.insert(node, spec) != Some(spec) {
            w.relayout = true;
        }
    }

    /// Puts the confined pointer `node` right above the layer child of `layer` that holds `owner`, so
    /// every layer child opened above the owner's (a panel, a popup over the canvas) covers it. Only a
    /// move repaints the layer.
    fn stack_confined(&mut self, layer: NodeId, node: NodeId, owner: NodeId) {
        let path = crate::hit::chain(&self.tree, owner);
        let Some(l) = self.tree.nodes.get_mut(&layer) else { return };
        let Some(at) = l.children.iter().position(|c| path.contains(c)) else { return };
        let Some(from) = l.children.iter().position(|c| *c == node) else { return };
        let target = if from > at { at + 1 } else { at };
        if from == target {
            return;
        }
        l.children.remove(from);
        l.children.insert(target, node);
        self.tree.mark_paint(layer);
    }

    /// The layer root `node` paints on: the layer node above its layer child or app root.
    fn layer_root_of(&self, node: NodeId) -> Option<NodeId> {
        let mut cur = Some(node);
        while let Some(id) = cur {
            let n = self.tree.nodes.get(&id)?;
            if matches!(n.role, Role::Layer(_)) {
                return Some(id);
            }
            cur = n.parent;
        }
        None
    }

    /// Cursor changes since the last call; the driver hands them to the platform.
    pub(crate) fn take_cursor_requests(&mut self) -> Vec<(WindowId, Cursor)> {
        std::mem::take(&mut self.cursor_requests)
    }
}
