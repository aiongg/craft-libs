//! AccessKit: the rt window's tree, kept from its updates, given to egui as nodes under the
//! panel on each pass where egui builds access nodes (AccessKit is on). The tree is flattened
//! under the panel: egui offers no public way to give a node another parent. Action requests on
//! those nodes go back to the rt nodes they stand for.

use std::collections::HashMap;

use accesskit::{Action, ActionRequest, Node, NodeId, Role, TreeUpdate};
use craft_ui_rt::Vec2;

#[derive(Default)]
pub(crate) struct AccessMirror {
    nodes: HashMap<NodeId, Node>,
    root: Option<NodeId>,
    /// egui node id to the rt node it stands for, as given in the last pass.
    proxies: HashMap<u64, NodeId>,
}

/// The actions passed through to the rt nodes.
const ACTIONS: [Action; 4] = [Action::Click, Action::Focus, Action::Expand, Action::Collapse];

impl AccessMirror {
    pub fn apply(&mut self, update: TreeUpdate) {
        if let Some(tree) = update.tree {
            if self.root != Some(tree.root) {
                self.nodes.clear();
            }
            self.root = Some(tree.root);
        }
        for (id, node) in update.nodes {
            let dropped: Vec<NodeId> =
                self.nodes.get(&id).map(|old| old.children().iter().filter(|c| !node.children().contains(c)).copied().collect()).unwrap_or_default();
            for child in dropped {
                self.remove(child);
            }
            self.nodes.insert(id, node);
        }
    }

    fn remove(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.remove(&id) {
            for &child in node.children() {
                self.remove(child);
            }
        }
    }

    /// Gives egui one node per rt node under the panel, when AccessKit is on. Bounds are in
    /// window (egui screen) points.
    pub fn emit(&mut self, ctx: &egui::Context, panel: egui::Id) {
        if ctx.accesskit_node_builder(panel, |_| ()).is_none() {
            return;
        }
        self.proxies.clear();
        let Some(root) = self.root else { return };
        let mut stack = vec![(root, Vec2::ZERO)];
        while let Some((id, origin)) = stack.pop() {
            let Some(node) = self.nodes.get(&id) else { continue };
            let offset = node.transform().map_or(Vec2::ZERO, |t| {
                let [.., x, y] = t.as_coeffs();
                Vec2::new(x as f32, y as f32)
            });
            let origin = origin + offset;
            for &child in node.children().iter().rev() {
                stack.push((child, origin));
            }
            if id == root || node.role() == Role::GenericContainer {
                continue;
            }
            let proxy = panel.with(id.0);
            self.proxies.insert(proxy.value(), id);
            ctx.accesskit_node_builder(proxy, |b| copy(node, origin, b));
        }
    }

    /// `request` aimed at the rt node its target stands for; `None` when it targets none of the
    /// window's nodes.
    pub fn action(&self, request: &ActionRequest) -> Option<ActionRequest> {
        let target = *self.proxies.get(&request.target_node.0)?;
        Some(ActionRequest { target_node: target, ..request.clone() })
    }
}

fn copy(node: &Node, origin: Vec2, b: &mut Node) {
    b.set_role(node.role());
    if let Some(label) = node.label() {
        b.set_label(label);
    }
    if let Some(description) = node.description() {
        b.set_description(description);
    }
    if let Some(value) = node.value() {
        b.set_value(value);
    }
    if let Some(toggled) = node.toggled() {
        b.set_toggled(toggled);
    }
    if let Some(selected) = node.is_selected() {
        b.set_selected(selected);
    }
    if let Some(r) = node.bounds() {
        let (x, y) = (f64::from(origin.x), f64::from(origin.y));
        b.set_bounds(accesskit::Rect::new(r.x0 + x, r.y0 + y, r.x1 + x, r.y1 + y));
    }
    for action in ACTIONS {
        if node.supports_action(action) {
            b.add_action(action);
        }
    }
}
