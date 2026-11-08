//! AccessKit node building, `register_accesskit_parent` and `set_accesskit_focus`: the one place that names
//! AccessKit's node API. `node` writes a decided node, `patch` applies the label scope and the status node, `tree`
//! reads a finished tree update. What a node says is decided in `runtime::access`.

mod node;
#[cfg(feature = "testing")]
pub(crate) mod parity;
mod patch;
mod roles;
mod text;
#[cfg(feature = "testing")]
mod tree;

#[cfg(feature = "testing")]
use egui::FullOutput;
use egui::{Context, Id};
use std::collections::HashSet;

use super::memory::Service;

pub(crate) use node::{NodeAction, NodeSpec, Toggle, write_node};
pub(crate) use patch::{write_label_patch, write_merge_patch, write_status};
pub(crate) use text::{NodeEnd, RunNode, TextNode, is_text_request, requested_selection, requested_value, write_run, write_text};
#[cfg(feature = "testing")]
pub use tree::{AccessTree, Declared, NodeKey};

/// The AccessKit role of a node.
pub(crate) type NodeRole = accesskit::Role;

/// The AccessKit vocabulary the parity trials read a tree in.
#[cfg(feature = "testing")]
pub(crate) use accesskit::{Action, Role, Toggled};

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

/// Whether AccessKit is running and the widget `id` is visible: nodes are built only then. Creates the node of `id`.
pub(crate) fn enabled(ctx: &Context, id: Id) -> bool {
    create_node(ctx, id)
}

/// Names the node of `id`, a widget node egui made for a text part, and notes it as the runtime's.
pub(crate) fn set_label(ctx: &Context, id: Id, label: &str) {
    super::widgets::note_node(ctx, id);
    ctx.accesskit_node_builder(id, |node| node.set_label(label));
}

/// Hides the node of `id` from assistive technology and strips its actions: a hit-test widget the runtime made for
/// a part that has its own sub-node, or to stand in for a root on another layer. The sub-node or the root's own
/// node carries the actions.
pub(crate) fn hide(ctx: &Context, id: Id) {
    crate::runtime::egui::widgets::note_node(ctx, id);
    ctx.accesskit_node_builder(id, |node| {
        node.set_hidden();
        node.clear_actions();
    });
}

/// Reports `id` as the focused node of this pass.
pub(crate) fn set_focus(ctx: &Context, id: Id) {
    ctx.set_accesskit_focus(id);
}

/// The runtime's nodes of a pass in the order it reported them: each reported parent link, and the ids of the nodes
/// written. egui lists a node's children in the order their nodes were first created, which is not always the order
/// the component reported them in.
#[derive(Default)]
struct Reported {
    links: Vec<(Id, Id)>,
    written: HashSet<Id>,
}

/// Makes `parent` the node `id` attaches to. Call it before the node of `id` is created.
pub(crate) fn register_parent(ctx: &Context, id: Id, parent: Id) {
    ctx.register_accesskit_parent(id, parent);
    Service::<Reported>::run(ctx, |reported| reported.links.push((parent, id)));
}

/// Notes that the runtime wrote the node of `id`.
pub(crate) fn note_written(ctx: &Context, id: Id) {
    Service::<Reported>::run(ctx, |reported| {
        reported.written.insert(id);
    });
}

/// The pass-end step: the children of each node the runtime wrote follow the order the runtime reported them in,
/// which is reading order. Children egui added that the runtime did not report follow them in egui's order.
pub(crate) fn order_children(ctx: &Context) {
    let reported = Service::<Reported>::run(ctx, std::mem::take);
    let mut parents: Vec<(Id, Vec<Id>)> = Vec::new();
    for (parent, id) in reported.links {
        match parents.iter_mut().find(|(held, _)| *held == parent) {
            Some((_, ids)) if !ids.contains(&id) => ids.push(id),
            Some(_) => {}
            None => parents.push((parent, vec![id])),
        }
    }
    for (parent, ids) in parents.into_iter().filter(|(parent, ids)| ids.len() > 1 && reported.written.contains(parent)) {
        ctx.accesskit_node_builder(parent, |node| {
            let current = node.children().to_vec();
            let mut ordered: Vec<_> = ids.iter().map(|id| id.accesskit_id()).filter(|id| current.contains(id)).collect();
            let rest: Vec<_> = current.iter().filter(|id| !ordered.contains(id)).copied().collect();
            ordered.extend(rest);
            node.set_children(ordered);
        });
    }
}

/// The tree update of a finished frame.
#[cfg(feature = "testing")]
pub(crate) fn tree_of(output: &mut FullOutput) -> Option<AccessTree> {
    output.platform_output.accesskit_update.take().map(AccessTree::new)
}
