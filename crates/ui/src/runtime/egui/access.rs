//! AccessKit node building, `register_accesskit_parent` and `set_accesskit_focus`: the one place that names
//! AccessKit's node API. `emit` fills nodes, `patch` applies the label scope and the status node, `tree` reads a
//! finished tree update, `roles` names the role constants.

mod emit;
mod patch;
mod roles;
mod rules;
mod text;
#[cfg(feature = "testing")]
mod tree;

#[cfg(feature = "testing")]
use egui::FullOutput;
use egui::{Context, Id};

pub(crate) use emit::{FaceFacts, Own, SubDecl, fill_own, fill_sub, sub_node_id};
pub(crate) use patch::{patch_label, patch_merge, write_status};
pub(crate) use text::{NodeEnd, RunNode, TextNode, is_text_request, requested_selection, requested_value, write_run, write_text};
#[cfg(feature = "testing")]
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

/// Whether AccessKit is running: nodes are built only then.
pub(crate) fn enabled(ctx: &Context) -> bool {
    ctx.is_accesskit_enabled()
}

/// Reports `id` as the focused node of this pass.
pub(crate) fn set_focus(ctx: &Context, id: Id) {
    ctx.set_accesskit_focus(id);
}

/// Makes `parent` the node `id` attaches to. Call it before the node of `id` is created.
pub(crate) fn register_parent(ctx: &Context, id: Id, parent: Id) {
    ctx.register_accesskit_parent(id, parent);
}

/// The tree update of a finished frame.
#[cfg(feature = "testing")]
pub(crate) fn tree_of(output: &mut FullOutput) -> Option<AccessTree> {
    output.platform_output.accesskit_update.take().map(AccessTree::new)
}
