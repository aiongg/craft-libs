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

pub(crate) use node::{NodeAction, NodeSpec, Toggle, write_node};
pub(crate) use patch::{write_label_patch, write_merge_patch, write_status};
pub(crate) use text::{NodeEnd, RunNode, TextNode, is_text_request, requested_selection, requested_value, write_run, write_text};
#[cfg(feature = "testing")]
pub use tree::{AccessTree, Declared, NodeKey};

/// The AccessKit role of a node.
pub(crate) type NodeRole = accesskit::Role;

/// The AccessKit vocabulary the parity trials read a tree in.
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

/// Withdraws the click egui declared for the widget of `id` from its sensed gestures: a click that only pointer
/// gestures take (a canvas tool's press) is not an action assistive technology can request.
pub(crate) fn withdraw_click(ctx: &Context, id: Id) {
    ctx.accesskit_node_builder(id, |node| node.remove_action(accesskit::Action::Click));
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
