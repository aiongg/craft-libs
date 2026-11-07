//! AccessKit node building, `register_accesskit_parent` and `set_accesskit_focus`: the one place that names
//! AccessKit's node API. `emit` fills nodes, `patch` applies the label scope and the status node, `tree` reads a
//! finished tree update, `roles` names the role constants.

mod emit;
mod patch;
mod roles;
mod rules;
mod tree;

use crate::runtime::egui::memory::Service;
use egui::{Context, FullOutput, Id};

pub(crate) use emit::{FaceFacts, Own, SubDecl, fill_own, fill_sub, sub_node_id};
pub(crate) use patch::{patch_label, patch_merge, write_status};
pub use tree::{AccessTree, Declared, NodeKey};

/// The AccessKit role of a node.
pub(crate) type NodeRole = accesskit::Role;

/// The AccessKit node number of `id`, the key an action request's target is matched by.
pub(crate) fn node_key(id: Id) -> u64 {
    id.accesskit_id().0
}

/// `builder` with `parent` as the node its `Ui`'s AccessKit nodes attach to.
pub(crate) fn with_parent(builder: egui::UiBuilder, parent: Id) -> egui::UiBuilder {
    builder.accessibility_parent(parent)
}

/// Creates the node of `id` empty if it has none. False while AccessKit is off or the widget is invisible (a
/// popup in its first-show sizing pass).
pub(crate) fn create_node(ctx: &Context, id: Id) -> bool {
    ctx.accesskit_node_builder(id, |_| ()).is_some()
}

/// Reports `id` as the focused node of this pass.
pub(crate) fn set_focus(ctx: &Context, id: Id) {
    ctx.set_accesskit_focus(id);
}

/// The `(node, parent)` pairs registered in the running pass, applied to the finished tree update.
#[derive(Default)]
struct Reparent(Vec<(Id, Id)>);

/// Makes `parent` the node `id` attaches to. The fork's `Context::register_accesskit_parent` is `pub(crate)` at the
/// pinned revision, so the pair moves the node in the finished tree update (`apply_parents`); the result is the same
/// tree.
// F4-MERGE: fork pin bump (public `register_accesskit_parent`): this body becomes the one call and `apply_parents` goes.
pub(crate) fn register_parent(ctx: &Context, id: Id, parent: Id) {
    Service::<Reparent>::of(ctx).with(|pairs| pairs.0.push((id, parent)));
}

/// Moves each registered node under its parent in the tree update of the finished pass.
pub(crate) fn apply_parents(ctx: &Context, output: &mut FullOutput) {
    let pairs = Service::<Reparent>::of(ctx).with(|pairs| std::mem::take(&mut pairs.0));
    let Some(update) = output.platform_output.accesskit_update.as_mut() else { return };
    for (child, parent) in pairs {
        let (child, parent) = (child.accesskit_id(), parent.accesskit_id());
        let has = |id| update.nodes.iter().any(|(node, _)| *node == id);
        if child == parent || !has(child) || !has(parent) {
            continue;
        }
        for (id, node) in &mut update.nodes {
            if *id == parent {
                if !node.children().contains(&child) {
                    node.push_child(child);
                }
            } else if node.children().contains(&child) {
                let rest: Vec<_> = node.children().iter().copied().filter(|other| *other != child).collect();
                node.set_children(rest);
            }
        }
    }
}

/// The tree update of a finished frame.
pub(crate) fn tree_of(output: &mut FullOutput) -> Option<AccessTree> {
    output.platform_output.accesskit_update.take().map(AccessTree::new)
}
