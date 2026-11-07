//! What the accessibility trials read: the AccessKit tree a frame produced, and the label scope a container opens.

use crate::runtime::access::{FieldLabel, scope};
use crate::runtime::egui::access::node_key;
use accesskit::{Node, NodeId, Role, TreeUpdate};
use egui::{Context, Id};

/// The AccessKit tree update of one frame.
#[non_exhaustive]
pub struct AccessTree {
    update: TreeUpdate,
}

impl AccessTree {
    pub(crate) fn new(update: TreeUpdate) -> Self {
        AccessTree { update }
    }

    /// The update of a frame that made none.
    pub(crate) fn no_frame() -> TreeUpdate {
        TreeUpdate { nodes: Vec::new(), tree: None, tree_id: accesskit::TreeId::ROOT, focus: NodeId(0) }
    }

    fn lookup(&self, id: NodeId) -> Option<&Node> {
        self.update.nodes.iter().find(|(node, _)| *node == id).map(|(_, node)| node)
    }

    /// The node of `id`.
    pub fn node(&self, id: Id) -> Option<&Node> {
        self.lookup(NodeId(node_key(id)))
    }

    /// Every node of the update.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.update.nodes.iter().map(|(_, node)| node)
    }

    /// The nodes of `role`.
    pub fn with_role(&self, role: Role) -> Vec<&Node> {
        self.nodes().filter(|node| node.role() == role).collect()
    }

    /// The nearest ancestor of `id` that screen readers see: AccessKit's consumer drops generic containers.
    pub fn parent_of(&self, id: Id) -> Option<NodeId> {
        let mut at = NodeId(node_key(id));
        loop {
            let (parent, _) = self.update.nodes.iter().find(|(_, node)| node.children().contains(&at))?;
            if self.lookup(*parent)?.role() != Role::GenericContainer {
                return Some(*parent);
            }
            at = *parent;
        }
    }

    /// Whether `node` is the node of `id`.
    pub fn is(&self, node: NodeId, id: Id) -> bool {
        node == NodeId(node_key(id))
    }

    /// The node `id` as an AccessKit id, for comparing relation targets.
    pub fn key(id: Id) -> NodeId {
        NodeId(node_key(id))
    }
}

/// Opens the label scope of `container` with `label`, as a field container's body step does.
pub fn begin_label_scope(ctx: &Context, container: Id, label: FieldLabel) {
    scope::begin(ctx, container, scope::Kind::Label(label));
}

/// Closes the scope of `container` and patches the nodes registered in it, as its report step does.
pub fn finish_scope(ctx: &Context, container: Id) {
    scope::finish(ctx, container, None);
}
