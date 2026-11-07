//! The label scope (12.4): a container with a side label records the label's node, and the node-emitting
//! components inside register with it. At the container's report step the runtime patches the nodes in the same
//! frame. The merge rule uses the same path: a `Role::Merged` container's name, description and states go into the
//! one child's node.

use crate::runtime::egui::{access as adapter, memory as store, memory::Service};
use crate::runtime::layout_memory;
use crate::runtime::memory::{self, sub};
use craft_behavior::PressState;
use craft_theme::Flags;
use egui::{Context, Id, Rect, Vec2};

/// Whether a field label names its control or describes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldLabelRole {
    /// The label is the control's name.
    #[default]
    Names,
    /// The control keeps its own name and the label describes it.
    Describes,
}

/// The label a container shows beside its body.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FieldLabel {
    /// The label's node id.
    pub id: Id,
    /// The label's rect.
    pub rect: Rect,
    /// The label's press this pass, for a control that scrubs by its label.
    pub press: PressState,
    /// The label's text.
    pub text: String,
    /// Whether it names or describes the controls.
    pub role: FieldLabelRole,
}

impl FieldLabel {
    /// A label of `text` at `rect`, the node `id`.
    pub fn new(id: Id, rect: Rect, text: impl Into<String>) -> Self {
        FieldLabel { id, rect, press: PressState::IDLE, text: text.into(), role: FieldLabelRole::Names }
    }

    /// Whether the label names or describes the controls.
    #[must_use]
    pub fn role(mut self, role: FieldLabelRole) -> Self {
        self.role = role;
        self
    }

    /// The label's press this pass.
    #[must_use]
    pub fn with_press(mut self, press: PressState) -> Self {
        self.press = press;
        self
    }

    /// The label is held down or being dragged.
    pub fn pressed(&self) -> bool {
        self.press.phase.is_press()
    }
}

/// What a scope does at its container's report step.
#[derive(Clone, Debug)]
pub(crate) enum Kind {
    /// The controls inside are named by this label.
    Label(FieldLabel),
    /// The body shows one component whose node takes the container's facts.
    Merge,
}

struct Scope {
    container: Id,
    /// The component whose eviction takes the facts the scope records.
    owner: Id,
    kind: Kind,
    /// The direct controls: node components inside this scope that sit in no other node component of it.
    nodes: Vec<(Id, bool)>,
    /// Node components inside this scope whose step 16 has not come yet, outermost first.
    open: Vec<Id>,
    /// The first baseline of the first control, from the control's top.
    baseline: Option<f32>,
    /// The control that registered a reset value and whether its value differs from it.
    reset: Option<ResetFact>,
    /// The narrowest width of the first control.
    control_min: Option<f32>,
    /// A control inside scrubs by the label.
    scrub: bool,
}

/// What a control tells its label scope about its reset (10.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResetFact {
    /// The control that takes `Intent::Reset`.
    pub control: Id,
    /// The value differs from the reset value.
    pub differs: bool,
}

#[derive(Default)]
struct Stack(Vec<Scope>);

/// Opens the scope of `container`; its body shows inside it.
pub(crate) fn begin(ctx: &Context, container: Id, owner: Id, kind: Kind) {
    Service::<Stack>::of(ctx).with(|stack| {
        stack.0.push(Scope { container, owner, kind, nodes: Vec::new(), open: Vec::new(), baseline: None, reset: None, control_min: None, scrub: false })
    });
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

/// Notes that the node component `id` created its node at step 8: the node components that run inside it are
/// not direct controls of the scope.
pub(crate) fn enter(ctx: &Context, id: Id) {
    Service::<Stack>::of(ctx).with(|stack| {
        if let Some(scope) = stack.0.last_mut() {
            scope.open.push(id);
        }
    });
}

/// Registers the node `id` with the innermost scope at its step 16, when no other node component of the scope
/// holds it. Children reach step 16 first, so they find their parent still open.
pub(crate) fn register(ctx: &Context, id: Id, keeps_name: bool) {
    Service::<Stack>::of(ctx).with(|stack| {
        let Some(scope) = stack.0.last_mut() else { return };
        let direct = match scope.open.iter().rposition(|open| *open == id) {
            Some(at) => {
                scope.open.truncate(at);
                at == 0
            }
            None => scope.open.is_empty(),
        };
        if direct {
            scope.nodes.push((id, keeps_name));
        }
    });
}

/// Records the first baseline of a control inside the innermost label scope, measured from the control's top.
/// The first control to report wins.
pub(crate) fn record_baseline(ctx: &Context, baseline: f32) {
    Service::<Stack>::of(ctx).with(|stack| {
        if let Some(scope) = stack.0.iter_mut().rev().find(|scope| matches!(scope.kind, Kind::Label(_))) {
            scope.baseline.get_or_insert(baseline);
        }
    });
}

/// Whether a label scope is open: a control then reports its minimum width.
pub(crate) fn in_label(ctx: &Context) -> bool {
    Service::<Stack>::of(ctx).with(|stack| stack.0.iter().any(|scope| matches!(scope.kind, Kind::Label(_))))
}

/// Records the narrowest width of a control inside the innermost label scope. The first control to report wins.
pub(crate) fn record_control_min(ctx: &Context, width: f32) {
    Service::<Stack>::of(ctx).with(|stack| {
        if let Some(scope) = stack.0.iter_mut().rev().find(|scope| matches!(scope.kind, Kind::Label(_))) {
            scope.control_min.get_or_insert(width);
        }
    });
}

/// The narrowest width the first control of `container`'s label scope reported last frame.
pub(crate) fn last_control_min(ctx: &Context, container: Id) -> f32 {
    layout_memory::read(ctx, sub(container, "control_min")).map_or(0.0, |size| size.x)
}

/// Registers the reset of the control `control` with the innermost label scope.
pub(crate) fn register_reset(ctx: &Context, control: Id, differs: bool) {
    Service::<Stack>::of(ctx).with(|stack| {
        if let Some(scope) = stack.0.iter_mut().rev().find(|scope| matches!(scope.kind, Kind::Label(_))) {
            scope.reset.get_or_insert(ResetFact { control, differs });
        }
    });
}

/// Registers that a control of the innermost label scope scrubs by the label, so the label senses a drag.
pub(crate) fn register_scrub(ctx: &Context) {
    Service::<Stack>::of(ctx).with(|stack| {
        if let Some(scope) = stack.0.iter_mut().rev().find(|scope| matches!(scope.kind, Kind::Label(_))) {
            scope.scrub = true;
        }
    });
}

/// Whether a control of the label scope of `container` scrubbed by the label last frame.
pub(crate) fn last_scrub(ctx: &Context, container: Id) -> bool {
    store::peek_clone::<bool>(ctx, scrub_slot(container)).unwrap_or(false)
}

/// The slot of a container's scrub fact; it goes with the scope's owner.
fn scrub_slot(container: Id) -> Id {
    sub(container, "scrub")
}

/// The reset fact the label scope of `container` recorded last frame.
pub(crate) fn last_reset(ctx: &Context, container: Id) -> Option<ResetFact> {
    store::peek_clone::<ResetFact>(ctx, reset_slot(container))
}

/// The slot of a container's reset fact; it goes with the scope's owner.
fn reset_slot(container: Id) -> Id {
    sub(container, "reset_fact")
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
        Kind::Label(label) => {
            adapter::patch_label(ctx, scope.container, &label, &scope.nodes);
            if let Some(baseline) = scope.baseline {
                layout_memory::record(ctx, sub(scope.container, "baseline"), Vec2::new(0.0, baseline));
            }
            layout_memory::record(ctx, sub(scope.container, "control_min"), Vec2::new(scope.control_min.unwrap_or(0.0), 0.0));
            let slot = reset_slot(scope.container);
            match scope.reset {
                Some(fact) => {
                    store::put(ctx, slot, fact);
                    memory::adopt::<ResetFact>(ctx, scope.owner, slot);
                }
                None => drop(store::take::<ResetFact>(ctx, slot)),
            }
            let scrub_slot = scrub_slot(scope.container);
            if scope.scrub {
                store::put(ctx, scrub_slot, true);
                memory::adopt::<bool>(ctx, scope.owner, scrub_slot);
            } else {
                drop(store::take::<bool>(ctx, scrub_slot));
            }
        }
        Kind::Merge => {
            debug_assert_eq!(scope.nodes.len(), 1, "a merged container shows exactly one component");
            if let (Some(merged), Some((child, _))) = (merged, scope.nodes.first()) {
                adapter::patch_merge(ctx, *child, &merged);
            }
        }
    }
}
