//! The label-scope patch, the merge into a child's node, and the status node of announcements.

use crate::runtime::access::scope::{FieldLabel, Merged};
use craft_theme::Flags;
use egui::{Context, Id};

/// Names the controls `nodes` registered in the scope of `container` by `label`: one control is labelled by it
/// (described by it when it keeps its own name), several make the container's node labelled by it.
pub(crate) fn patch_label(ctx: &Context, container: Id, label: &FieldLabel, nodes: &[(Id, bool)]) {
    if nodes.is_empty() {
        return;
    }
    ctx.accesskit_node_builder(label.id, |node| {
        if node.role() == accesskit::Role::Unknown {
            node.set_role(accesskit::Role::Label);
        }
        if node.label().is_none() {
            node.set_label(label.text.clone());
        }
    });
    let target = label.id.accesskit_id();
    let describes = label.role == crate::runtime::access::scope::FieldLabelRole::Describes;
    match nodes {
        [(control, keeps_name)] => {
            let keeps_name = describes || *keeps_name;
            ctx.accesskit_node_builder(*control, |node| {
                if keeps_name {
                    let mut described = node.described_by().to_vec();
                    described.push(target);
                    node.set_described_by(described);
                } else {
                    node.clear_label();
                    node.set_labelled_by(vec![target]);
                }
            });
        }
        _ => {
            ctx.accesskit_node_builder(container, |node| node.set_labelled_by(vec![target]));
        }
    }
}

/// Adds a merged container's name, description and states to its child's node.
pub(crate) fn patch_merge(ctx: &Context, child: Id, merged: &Merged) {
    ctx.accesskit_node_builder(child, |node| {
        if !merged.name.is_empty() {
            let own = node.label().map(str::to_owned);
            node.set_label(match own {
                Some(own) if !own.is_empty() => format!("{own}, {}", merged.name),
                _ => merged.name.clone(),
            });
        }
        if let Some(text) = &merged.description {
            let own = node.description().map(str::to_owned);
            node.set_description(match own {
                Some(own) if !own.is_empty() => format!("{own}. {text}"),
                _ => text.clone(),
            });
        }
        if !merged.enabled {
            node.set_disabled();
        }
        if merged.flags.contains(Flags::SELECTED) {
            node.set_selected(true);
        }
        if merged.flags.contains(Flags::MIXED) {
            node.set_toggled(accesskit::Toggled::Mixed);
        } else if merged.flags.contains(Flags::CHECKED) {
            node.set_toggled(accesskit::Toggled::True);
        }
        if merged.flags.contains(Flags::INVALID) {
            node.set_invalid(accesskit::Invalid::True);
        }
        if merged.flags.contains(Flags::BUSY) {
            node.set_busy();
        }
    });
}

/// The status node `id`: one live node holding `text`, none while there is nothing to say.
pub(crate) fn write_status(ctx: &Context, id: Id, text: &str, assertive: bool) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(accesskit::Role::Status);
        node.set_live(if assertive { accesskit::Live::Assertive } else { accesskit::Live::Polite });
        node.set_live_atomic();
        if !text.is_empty() {
            node.set_label(text);
        }
    });
}
