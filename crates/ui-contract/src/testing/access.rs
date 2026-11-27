//! What the accessibility tests read: the AccessKit tree the runtime kept for a window, as the
//! last frame sent it.

use std::collections::HashMap;

use accesskit::{Action, Node, NodeId, Role};
use craft_ui_rt::{Rect, Renderer};

use crate::contract::rt::harness::Harness;

/// The access tree of a window's last frame: its nodes (a component's node and its sub-nodes) in
/// tree order, with window bounds.
#[derive(Clone, Debug)]
pub struct AccessView {
    entries: Vec<(NodeId, Node, Rect)>,
    parents: HashMap<NodeId, NodeId>,
    focus: Option<NodeId>,
}

/// A node of the tree, as the queries key it.
pub type NodeKey = NodeId;

impl AccessView {
    fn entry(&self, key: NodeKey) -> Option<&(NodeId, Node, Rect)> {
        self.entries.iter().find(|entry| entry.0 == key)
    }

    /// The tree's nodes, in tree order.
    pub fn keys(&self) -> Vec<NodeKey> {
        self.entries.iter().map(|entry| entry.0).collect()
    }

    /// How many nodes the tree has.
    pub fn node_count(&self) -> usize {
        self.entries.len()
    }

    /// The first node: the window's.
    pub fn root_key(&self) -> Option<NodeKey> {
        self.entries.first().map(|entry| entry.0)
    }

    /// The node AccessKit reports as focused; the root when none is.
    pub fn focused_key(&self) -> NodeKey {
        self.focus.or_else(|| self.root_key()).unwrap_or(NodeId(0))
    }

    /// The node `key` names, as the tree has it.
    pub fn data_of(&self, key: NodeKey) -> Option<&Node> {
        self.entry(key).map(|entry| &entry.1)
    }

    /// Whether the tree has the node `key`.
    pub fn has_key(&self, key: NodeKey) -> bool {
        self.entry(key).is_some()
    }

    /// Whether the node `key` declares `action`.
    pub fn declares_key(&self, key: NodeKey, action: Action) -> bool {
        self.data_of(key).is_some_and(|node| node.supports_action(action))
    }

    /// Whether a column header node declares a sort direction: it sorts when activated, sorted now or not.
    pub fn declares_sort(&self, key: NodeKey) -> bool {
        self.data_of(key).is_some_and(|node| node.sort_direction().is_some())
    }

    /// The node's role.
    pub fn role_of(&self, key: NodeKey) -> Option<Role> {
        self.data_of(key).map(Node::role)
    }

    /// The node's own name.
    pub fn name_of(&self, key: NodeKey) -> Option<&str> {
        self.data_of(key)?.label()
    }

    /// The node's value text.
    pub fn value(&self, key: NodeKey) -> Option<&str> {
        self.data_of(key)?.value()
    }

    /// The node's numeric value.
    pub fn numeric_of(&self, key: NodeKey) -> Option<f64> {
        self.data_of(key)?.numeric_value()
    }

    /// The node's description.
    pub fn description_of(&self, key: NodeKey) -> Option<&str> {
        self.data_of(key)?.description()
    }

    /// The node's bounds in window points.
    pub fn bounds_of(&self, key: NodeKey) -> Option<Rect> {
        self.entry(key).map(|entry| entry.2)
    }

    /// The nodes that label `key`.
    pub fn labelled_by_key(&self, key: NodeKey) -> Vec<NodeKey> {
        self.data_of(key).map(|node| node.labelled_by().to_vec()).unwrap_or_default()
    }

    /// The node's children, in their declared order.
    pub fn children_of(&self, key: NodeKey) -> Vec<NodeKey> {
        self.data_of(key).map(|node| node.children().to_vec()).unwrap_or_default()
    }

    /// The node whose children include `key`.
    pub fn parent_of_key(&self, key: NodeKey) -> Option<NodeKey> {
        self.parents.get(&key).copied()
    }

    /// Whether the node is disabled.
    pub fn is_disabled_key(&self, key: NodeKey) -> bool {
        self.data_of(key).is_some_and(Node::is_disabled)
    }

    /// Whether assistive technology skips the node.
    pub fn is_hidden(&self, key: NodeKey) -> bool {
        self.data_of(key).is_some_and(Node::is_hidden)
    }

    /// The text of the node's text runs (its children of role `TextRun`), in order.
    pub fn run_texts(&self, key: NodeKey) -> Vec<String> {
        self.children_of(key)
            .into_iter()
            .filter(|child| self.role_of(*child) == Some(Role::TextRun))
            .filter_map(|child| self.value(child).map(str::to_owned))
            .collect()
    }

    /// Whether the node lies along a horizontal axis the layout direction does not mirror (a slider or a scroll bar,
    /// which reads left to right in both directions).
    pub fn in_ltr_axis(&self, key: NodeKey) -> bool {
        self.data_of(key).is_some_and(|node| {
            node.orientation() == Some(accesskit::Orientation::Horizontal) && matches!(node.role(), Role::Slider | Role::ScrollBar)
        })
    }

    /// The nodes of `role`, in tree order.
    pub fn with_role(&self, role: Role) -> Vec<NodeKey> {
        self.entries.iter().filter(|entry| entry.1.role() == role).map(|entry| entry.0).collect()
    }
}

/// The node's name: its own, or the first name of the nodes that label it.
pub fn name(tree: &AccessView, key: NodeKey) -> Option<&str> {
    tree.name_of(key).or_else(|| tree.labelled_by_key(key).into_iter().find_map(|label| tree.name_of(label)))
}

/// Whether the node can be acted on: it declares a click, focus, increment or a value change.
pub fn is_interactive(tree: &AccessView, key: NodeKey) -> bool {
    [Action::Click, Action::Focus, Action::Increment, Action::Decrement, Action::SetValue].into_iter().any(|action| tree.declares_key(key, action))
}

/// The rule violations of a frame: interactive nodes with no name.
pub fn violations(tree: &AccessView) -> Vec<String> {
    tree.keys()
        .into_iter()
        .filter(|key| !tree.is_hidden(*key) && is_interactive(tree, *key) && name(tree, *key).is_none())
        .map(|key| format!("an interactive node with no name: {:?}", tree.role_of(key)))
        .collect()
}

impl<S, R: Renderer> Harness<S, R> {
    /// The caret rect of the input method's latest request, in window points: where it places its candidate window.
    pub fn ime_caret(&self) -> Option<Rect> {
        self.driver
            .platform
            .ime_for(self.window())
            .filter_map(|request| match request {
                craft_ui_rt::ImeRequest::Start { cursor, .. } | craft_ui_rt::ImeRequest::Cursor(cursor) => Some(*cursor),
                craft_ui_rt::ImeRequest::Stop => None,
            })
            .last()
    }

    /// The access tree of the last frame.
    pub fn tree(&self) -> AccessView {
        let entries: Vec<(NodeId, Node, Rect)> =
            self.driver.runtime.access_snapshot(self.window()).into_iter().map(|entry| (entry.id, entry.node.clone(), entry.bounds)).collect();
        let parents = entries.iter().flat_map(|(id, node, _)| node.children().iter().map(|child| (*child, *id))).collect();
        let focus = self.driver.platform.updates_for(self.window()).last().map(|update| update.focus);
        AccessView { entries, parents, focus }
    }
}
