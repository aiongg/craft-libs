//! The window's `Id -> NodeId` map (docs/contract-rt.md section 15.9, item 7): craft `Id`s name
//! components and parts, ui-rt names nodes. A node enters the map in its first run
//! ([`Kept::note_id`]) and leaves it with its node, so a lookup finds the live node or nothing.
//!
//! Access relations ([`super::access::Access::labelled_by`]) name their targets by `Id`; the
//! access node of a component resolves them here when ui-rt builds it, after every sibling is
//! built. A target that is not in the map yet is waited for: its entry rebuilds the waiters' access
//! nodes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use accesskit::Node;
use craft_behavior::{PartRef, PressCancel};
use craft_ui_rt::{BuildCx, InteractFact, NodeId};
use smallvec::SmallVec;

use super::node::{ComponentNode, Kept};
use crate::contract::Id;
use crate::runtime::focus::FocusTarget;

#[derive(Default)]
struct Table {
    nodes: HashMap<u64, NodeId>,
    /// The `Id` each node in `nodes` answers to.
    ids: HashMap<NodeId, Id>,
    /// Nodes whose access node named an `Id` the map did not have, by that `Id`.
    waiting: HashMap<u64, SmallVec<[NodeId; 2]>>,
}

/// One window's `Id -> NodeId` map, provided by its window root. Cloning shares it.
#[derive(Clone, Default)]
pub(crate) struct NodeIds(Rc<RefCell<Table>>);

impl NodeIds {
    /// The live node of `id`.
    pub(crate) fn node_of(&self, id: Id) -> Option<NodeId> {
        self.0.borrow().nodes.get(&id.value()).copied()
    }

    /// The nodes that waited for `id`, now that it is in the map.
    fn insert(&self, id: Id, node: NodeId) -> SmallVec<[NodeId; 2]> {
        let mut table = self.0.borrow_mut();
        table.nodes.insert(id.value(), node);
        table.ids.insert(node, id);
        table.waiting.remove(&id.value()).unwrap_or_default()
    }

    fn wait(&self, id: u64, waiter: NodeId) {
        let mut table = self.0.borrow_mut();
        let list = table.waiting.entry(id).or_default();
        if !list.contains(&waiter) {
            list.push(waiter);
        }
    }
}

/// A node's entry in the map: it leaves it when the node is dropped, unless the `Id` already names
/// another node.
pub(crate) struct IdEntry {
    ids: NodeIds,
    id: u64,
    node: NodeId,
}

impl Drop for IdEntry {
    fn drop(&mut self) {
        let mut table = self.ids.0.borrow_mut();
        if table.nodes.get(&self.id) == Some(&self.node) {
            table.nodes.remove(&self.id);
        }
        if table.ids.get(&self.node).is_some_and(|id| id.value() == self.id) {
            table.ids.remove(&self.node);
        }
        for list in table.waiting.values_mut() {
            list.retain(|n| *n != self.node);
        }
    }
}

impl Kept {
    /// Records that the node `cx` builds is the one named `id`. A run calls it where it sets
    /// `kept.id`; a node whose entry is current costs a comparison.
    pub(crate) fn note_id(&mut self, cx: &mut BuildCx<'_>, id: Id) {
        let node = cx.node();
        if self.ids.as_ref().is_some_and(|e| e.id == id.value() && e.node == node) {
            return;
        }
        let Some(ids) = cx.provided::<NodeIds>() else { return };
        for waiter in ids.insert(id, node) {
            cx.request_access_of(waiter);
        }
        self.ids = Some(IdEntry { ids, id: id.value(), node });
    }

    /// `access` with its relations resolved to nodes: ids that name no node (yet) are dropped, and
    /// their owner waits for them.
    pub(crate) fn resolve_relations(&self, node: &mut Node) {
        type Get = fn(&Node) -> &[accesskit::NodeId];
        type Set = fn(&mut Node, Vec<accesskit::NodeId>);
        type Clear = fn(&mut Node);
        let relations: [(Get, Set, Clear); 4] = [
            (Node::labelled_by, Node::set_labelled_by, Node::clear_labelled_by),
            (Node::described_by, Node::set_described_by, Node::clear_described_by),
            (Node::controls, Node::set_controls, Node::clear_controls),
            (Node::flow_to, Node::set_flow_to, Node::clear_flow_to),
        ];
        for (get, set, clear) in relations {
            let named: Vec<accesskit::NodeId> = get(node).to_vec();
            if named.is_empty() {
                continue;
            }
            let mut found = Vec::with_capacity(named.len());
            for target in named {
                let at = self.ids.as_ref().and_then(|e| {
                    let hit = e.ids.0.borrow().nodes.get(&target.0).copied();
                    if hit.is_none() {
                        e.ids.wait(target.0, e.node);
                    }
                    hit
                });
                if let Some(at) = at {
                    found.push(at.access_id());
                }
            }
            if found.is_empty() {
                clear(node);
            } else {
                set(node, found);
            }
        }
    }
}

/// The `Id` the live node `node` answers to, in the window of the node `cx` builds.
pub(crate) fn id_of(cx: &BuildCx<'_>, node: NodeId) -> Option<Id> {
    cx.provided::<NodeIds>()?.0.borrow().ids.get(&node).copied()
}

/// The live node of `id` in the window `cx` builds in.
pub(crate) fn node_of(cx: &BuildCx<'_>, id: Id) -> Option<NodeId> {
    cx.provided::<NodeIds>()?.node_of(id)
}

/// `Ui::hovers`: whether the pointer is over `id`'s node. A tracked read of that fact.
pub(crate) fn hovers(cx: &mut BuildCx<'_>, id: Id) -> bool {
    node_of(cx, id).is_some_and(|node| cx.read_interaction(node, InteractFact::Hovered))
}

/// `Ui::is_focused`: whether `id`'s node has key focus. A tracked read.
pub(crate) fn is_focused(cx: &mut BuildCx<'_>, id: Id) -> bool {
    node_of(cx, id).is_some_and(|node| cx.read_interaction(node, InteractFact::Focused))
}

/// `Ui::focus_in_scope`: whether focus is inside the scope `scope` names; `None` when no such node
/// exists. A tracked read.
pub(crate) fn focus_in_scope(cx: &mut BuildCx<'_>, scope: Id) -> Option<bool> {
    let node = node_of(cx, scope)?;
    Some(cx.read_interaction(node, InteractFact::FocusWithin))
}

/// `Ui::request_focus`: moves key focus to the node `target` names, once the build pass has made
/// it. `false` when no live node answers to it.
pub(crate) fn request_focus(cx: &mut BuildCx<'_>, target: FocusTarget) -> bool {
    let id = match target {
        FocusTarget::Id(id) | FocusTarget::Part(id, _) => id,
        _ => return false,
    };
    match node_of(cx, id) {
        Some(node) => {
            cx.focus_node(node);
            true
        }
        None => false,
    }
}

/// The component node `owner` names and its kept facts, when it is one.
pub(crate) fn component<'a>(cx: &'a BuildCx<'_>, owner: Id) -> Option<craft_ui_rt::Peek<'a, Kept>> {
    let node = node_of(cx, owner)?;
    let parent = cx.runtime().parent(node)?;
    cx.peek::<ComponentNode>(parent, owner.value())
}

/// `Ui::cancel_press_of`: ends the press `owner` holds, as the host did (`PressCancel::Host`); its
/// owner gets the cancel. Nothing happens when it holds none.
pub(crate) fn cancel_press_of(cx: &mut BuildCx<'_>, owner: Id) {
    if component(cx, owner).is_some_and(|p| p.local.press.is_some()) {
        cx.cancel_press(PressCancel::Host);
    }
}

/// `Ui::pressed_part`: the part `owner`'s open press went down on. `None` when it holds no press or
/// the press is on the component's own shape.
pub(crate) fn pressed_part(cx: &BuildCx<'_>, owner: Id) -> Option<PartRef> {
    let kept = component(cx, owner)?.local;
    kept.press?;
    kept.parts.iter().find(|p| p.id == kept.press_part).map(|p| p.part)
}
