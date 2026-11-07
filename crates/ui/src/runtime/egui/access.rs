//! AccessKit node building, `register_accesskit_parent` and `set_accesskit_focus`.

use egui::{Context, FullOutput, Id};

/// The AccessKit node number of `id`, the key an action request's target is matched by.
pub(crate) fn node_key(id: Id) -> u64 {
    id.accesskit_id().0
}

/// `builder` with `parent` as the node its `Ui`'s AccessKit nodes attach to.
pub(crate) fn with_parent(builder: egui::UiBuilder, parent: Id) -> egui::UiBuilder {
    builder.accessibility_parent(parent)
}

/// Creates the node of `id` if it has none and runs `write` on it. `None` while AccessKit is off or the widget is
/// invisible (a popup in its first-show sizing pass).
pub(crate) fn with_node<R>(ctx: &Context, id: Id, write: impl FnOnce(&mut accesskit::Node) -> R) -> Option<R> {
    ctx.accesskit_node_builder(id, write)
}

/// Reports `id` as the focused node of this pass.
pub(crate) fn set_focus(ctx: &Context, id: Id) {
    ctx.set_accesskit_focus(id);
}

/// The `(node, parent)` pairs registered in the running pass, applied to the finished tree update.
#[derive(Clone, Default)]
struct Reparent(Vec<(Id, Id)>);

fn reparent_id() -> Id {
    Id::unique("craft_ui.access.reparent")
}

/// Makes `parent` the node `id` attaches to. The fork's `Context::register_accesskit_parent` is `pub(crate)` at the
/// pinned revision, so the pair moves the node in the finished tree update (`apply_parents`); the result is the same
/// tree.
// F4-MERGE: fork pin bump (public `register_accesskit_parent`): this body becomes the one call and `apply_parents` goes.
pub(crate) fn register_parent(ctx: &Context, id: Id, parent: Id) {
    ctx.data_mut(|data| data.get_temp_mut_or_default::<Reparent>(reparent_id()).0.push((id, parent)));
}

/// Moves each registered node under its parent in the tree update of the finished pass.
pub(crate) fn apply_parents(ctx: &Context, output: &mut FullOutput) {
    let pairs = ctx.data_mut(|data| data.remove_temp::<Reparent>(reparent_id())).unwrap_or_default().0;
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
