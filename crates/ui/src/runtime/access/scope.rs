//! The label scope (12.4): a container with a side label records the label's node, and the node-emitting
//! components inside register with it. At the container's report step the runtime patches the nodes in the same
//! frame. The merge rule uses the same path: a `Role::Merged` container's name, description and states go into the
//! one child's node.

use crate::runtime::egui::{access as adapter, memory::Service};
use craft_theme::Flags;
use egui::{Context, Id, Rect};

/// The label a container shows beside its body.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FieldLabel {
    /// The label's node id.
    pub id: Id,
    /// The label's rect.
    pub rect: Rect,
    /// The label is pressed this pass; a press on it focuses the control.
    pub pressed: bool,
    /// The label's text.
    pub text: String,
}

impl FieldLabel {
    /// A label of `text` at `rect`, the node `id`.
    pub fn new(id: Id, rect: Rect, text: impl Into<String>) -> Self {
        FieldLabel { id, rect, pressed: false, text: text.into() }
    }
}

/// What a scope does at its container's report step.
#[derive(Clone, Debug)]
pub(crate) enum Kind {
    /// The controls inside are named by this label.
    #[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the container body step opens label scopes (F5)"))]
    Label(FieldLabel),
    /// The body shows one component whose node takes the container's facts.
    Merge,
}

struct Scope {
    container: Id,
    kind: Kind,
    nodes: Vec<(Id, bool)>,
}

#[derive(Default)]
struct Stack(Vec<Scope>);

/// Opens the scope of `container`; its body shows inside it.
pub(crate) fn begin(ctx: &Context, container: Id, kind: Kind) {
    Service::<Stack>::of(ctx).with(|stack| stack.0.push(Scope { container, kind, nodes: Vec::new() }));
}

/// The innermost label scope's label, read by `cx.field_label()`.
pub(crate) fn field_label(ctx: &Context) -> Option<FieldLabel> {
    Service::<Stack>::of(ctx).with(|stack| {
        stack.0.iter().rev().find_map(|scope| match &scope.kind {
            Kind::Label(label) => Some(label.clone()),
            Kind::Merge => None,
        })
    })
}

/// Registers the node `id` with the innermost scope, as its component fills it.
pub(crate) fn register(ctx: &Context, id: Id, keeps_name: bool) {
    Service::<Stack>::of(ctx).with(|stack| {
        if let Some(scope) = stack.0.last_mut() {
            scope.nodes.push((id, keeps_name));
        }
    });
}

/// What a merged container adds to its child's node.
pub(crate) struct Merged {
    pub name: String,
    pub description: Option<String>,
    pub flags: Flags,
    pub enabled: bool,
}

/// Closes the scope of `container`, if it opened one, and patches the nodes registered in it. `merged` is the
/// container's own facts, read by a merge scope.
pub(crate) fn finish(ctx: &Context, container: Id, merged: Option<Merged>) {
    let scope =
        Service::<Stack>::of(ctx).with(|stack| (stack.0.last().is_some_and(|scope| scope.container == container)).then(|| stack.0.pop()).flatten());
    let Some(scope) = scope else { return };
    match scope.kind {
        Kind::Label(label) => adapter::patch_label(ctx, scope.container, &label, &scope.nodes),
        Kind::Merge => {
            debug_assert_eq!(scope.nodes.len(), 1, "a merged container shows exactly one component");
            if let (Some(merged), Some((child, _))) = (merged, scope.nodes.first()) {
                adapter::patch_merge(ctx, *child, &merged);
            }
        }
    }
}
