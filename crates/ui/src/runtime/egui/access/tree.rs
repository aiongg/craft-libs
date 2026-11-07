//! A finished AccessKit tree update, read by trials without naming AccessKit.

use super::{NodeRole, node_key};
use crate::runtime::access::{Numeric, Politeness, Role};
use accesskit::{Action, Node, NodeId, TreeUpdate};
use egui::Id;

/// A node's number in the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeKey(u64);

impl NodeKey {
    /// The AccessKit node number.
    pub(crate) fn raw(self) -> u64 {
        self.0
    }

    /// The key of the node of `id`.
    pub fn of(id: Id) -> NodeKey {
        NodeKey(node_key(id))
    }
}

/// An action a node declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Declared {
    /// Focus.
    Focus,
    /// Click.
    Click,
    /// Increment.
    Increment,
    /// Decrement.
    Decrement,
    /// Set a value.
    SetValue,
    /// Expand.
    Expand,
    /// Collapse.
    Collapse,
    /// A custom action.
    Custom,
}

impl Declared {
    fn action(self) -> Action {
        match self {
            Declared::Focus => Action::Focus,
            Declared::Click => Action::Click,
            Declared::Increment => Action::Increment,
            Declared::Decrement => Action::Decrement,
            Declared::SetValue => Action::SetValue,
            Declared::Expand => Action::Expand,
            Declared::Collapse => Action::Collapse,
            Declared::Custom => Action::CustomAction,
        }
    }
}

/// The AccessKit tree update of one frame.
#[non_exhaustive]
pub struct AccessTree {
    update: TreeUpdate,
}

impl AccessTree {
    pub(crate) fn new(update: TreeUpdate) -> Self {
        AccessTree { update }
    }

    /// The tree of a frame that made none.
    pub(crate) fn none() -> Self {
        AccessTree::new(TreeUpdate { nodes: Vec::new(), tree: None, tree_id: accesskit::TreeId::ROOT, focus: NodeId(0) })
    }

    fn node(&self, id: Id) -> Option<&Node> {
        self.lookup(NodeId(node_key(id)))
    }

    fn lookup(&self, id: NodeId) -> Option<&Node> {
        self.update.nodes.iter().find(|(node, _)| *node == id).map(|(_, node)| node)
    }

    /// Whether the node of `id` exists.
    pub fn has(&self, id: Id) -> bool {
        self.node(id).is_some()
    }

    /// The role of the node of `id`.
    pub fn role(&self, id: Id) -> Option<Role> {
        self.node(id).map(|node| Role::Node(node.role()))
    }

    /// The name of the node of `id`.
    pub fn name(&self, id: Id) -> Option<&str> {
        self.node(id)?.label()
    }

    /// The description of the node of `id`.
    pub fn description(&self, id: Id) -> Option<&str> {
        self.node(id)?.description()
    }

    /// The text value of the node of `id`.
    pub fn value(&self, id: Id) -> Option<&str> {
        self.node(id)?.value()
    }

    /// The numeric value and range of the node of `id`.
    pub fn numeric(&self, id: Id) -> Option<Numeric> {
        let node = self.node(id)?;
        let range = node.min_numeric_value()?..=node.max_numeric_value()?;
        Some(Numeric { value: node.numeric_value(), range, step: node.numeric_value_step(), text: node.value().map(str::to_owned) })
    }

    /// Whether the node of `id` declares `action`.
    pub fn declares(&self, id: Id, action: Declared) -> bool {
        self.node(id).is_some_and(|node| node.supports_action(action.action()))
    }

    /// The nodes that name the node of `id`.
    pub fn labelled_by(&self, id: Id) -> Vec<NodeKey> {
        self.node(id).map_or_else(Vec::new, |node| node.labelled_by().iter().map(|key| NodeKey(key.0)).collect())
    }

    /// The nodes that describe the node of `id`.
    pub fn described_by(&self, id: Id) -> Vec<NodeKey> {
        self.node(id).map_or_else(Vec::new, |node| node.described_by().iter().map(|key| NodeKey(key.0)).collect())
    }

    /// The place of the node of `id` in its set: its zero-based index and the size of the set.
    pub fn position(&self, id: Id) -> Option<(usize, usize)> {
        let node = self.node(id)?;
        Some((node.position_in_set()?, node.size_of_set()?))
    }

    /// The expanded state of the node of `id`.
    pub fn expanded(&self, id: Id) -> Option<bool> {
        self.node(id)?.is_expanded()
    }

    /// The live politeness of the node of `id`.
    pub fn live(&self, id: Id) -> Option<Politeness> {
        match self.node(id)?.live()? {
            accesskit::Live::Assertive => Some(Politeness::Assertive),
            accesskit::Live::Polite => Some(Politeness::Polite),
            accesskit::Live::Off => None,
        }
    }

    /// The nodes of `role`.
    pub fn with_role(&self, role: Role) -> Vec<NodeKey> {
        let Role::Node(role) = role else { return Vec::new() };
        self.update.nodes.iter().filter(|(_, node)| node.role() == role).map(|(key, _)| NodeKey(key.0)).collect()
    }

    /// The name of a node by key.
    pub fn name_of(&self, key: NodeKey) -> Option<&str> {
        self.lookup(NodeId(key.0))?.label()
    }

    /// The nearest ancestor of `id` that screen readers see: AccessKit's consumer drops generic containers.
    pub fn parent_of(&self, id: Id) -> Option<NodeKey> {
        self.parent_of_key(NodeKey::of(id))
    }

    /// [`AccessTree::parent_of`] for a node by key.
    pub fn parent_of_key(&self, key: NodeKey) -> Option<NodeKey> {
        let mut at = NodeId(key.0);
        loop {
            let (parent, _) = self.update.nodes.iter().find(|(_, node)| node.children().contains(&at))?;
            if self.lookup(*parent)?.role() != NodeRole::GenericContainer {
                return Some(NodeKey(parent.0));
            }
            at = *parent;
        }
    }
}

impl AccessTree {
    /// Every node.
    pub fn keys(&self) -> Vec<NodeKey> {
        self.update.nodes.iter().map(|(key, _)| NodeKey(key.0)).collect()
    }

    /// The role of a node by key.
    pub fn role_of(&self, key: NodeKey) -> Option<Role> {
        self.lookup(NodeId(key.0)).map(|node| Role::Node(node.role()))
    }

    /// The bounds of a node by key, in points.
    pub fn bounds_of(&self, key: NodeKey) -> Option<egui::Rect> {
        let bounds = self.lookup(NodeId(key.0))?.bounds()?;
        Some(egui::Rect::from_min_max(egui::pos2(bounds.x0 as f32, bounds.y0 as f32), egui::pos2(bounds.x1 as f32, bounds.y1 as f32)))
    }

    /// Whether a node by key declares `action`.
    pub fn declares_key(&self, key: NodeKey, action: Declared) -> bool {
        self.lookup(NodeId(key.0)).is_some_and(|node| node.supports_action(action.action()))
    }

    /// The children of a node by key.
    pub fn children_of(&self, key: NodeKey) -> Vec<NodeKey> {
        self.lookup(NodeId(key.0)).map_or_else(Vec::new, |node| node.children().iter().map(|child| NodeKey(child.0)).collect())
    }
}
