//! The props and hooks the body and item steps read.
//!
//! F3-LATER: body (F5) and item (F6) steps. These carry what the simplest form of each step reads: whether the
//! container is an arena root and its focus scope, and the collection's key-independent settings.

use super::SideLabel;
use crate::runtime::access::Politeness;
use crate::runtime::commands::CommandScope;
use crate::runtime::focus::{FocusPolicy, Roving};
use std::marker::PhantomData;

/// What a container or collection adds to `Common`.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ContainerProps {
    /// The container is an arena root: its members register shapes with it and no egui widget.
    pub arena: bool,
    /// A focus scope for the body, when the container has one.
    pub focus: Option<FocusPolicy>,
    /// The container is an instance of this command scope: the scoped commands of the scope reach its body.
    pub command_scope: Option<CommandScope>,
    /// A field label the label step shows beside the body; the text is `Common.label`.
    pub label: Option<SideLabel>,
    /// The body is a live region.
    pub live: Option<Politeness>,
    /// The body's members are one roving stop.
    pub roving: Option<Roving>,
}

impl ContainerProps {
    /// Props with no arena and no focus scope.
    pub const fn new() -> Self {
        ContainerProps { arena: false, focus: None, command_scope: None, label: None, live: None, roving: None }
    }

    /// Makes the container an arena root.
    pub const fn arena(mut self, on: bool) -> Self {
        self.arena = on;
        self
    }

    /// Makes the container an instance of `scope`.
    pub const fn command_scope(mut self, scope: CommandScope) -> Self {
        self.command_scope = Some(scope);
        self
    }

    /// Shows a side label beside the body.
    pub const fn label(mut self, label: SideLabel) -> Self {
        self.label = Some(label);
        self
    }

    /// Makes the body a live region.
    pub const fn live(mut self, politeness: Politeness) -> Self {
        self.live = Some(politeness);
        self
    }

    /// Makes the body's members one roving stop.
    pub const fn roving(mut self, roving: Roving) -> Self {
        self.roving = Some(roving);
        self
    }

    /// Gives the body a focus scope.
    pub const fn focus(mut self, policy: FocusPolicy) -> Self {
        self.focus = Some(policy);
        self
    }
}

/// What a collection reads from the app: plain data.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CollectionProps {}

impl CollectionProps {
    /// Collection props with every setting at its default.
    pub const fn new() -> Self {
        CollectionProps {}
    }
}

/// What `Component::overlay` builds anchored popups with.
// F3-LATER: overlay host (F5)
#[derive(Debug)]
pub struct OverlayUi<'a> {
    _anchor: PhantomData<&'a ()>,
}

impl OverlayUi<'_> {
    pub(crate) fn new() -> Self {
        OverlayUi { _anchor: PhantomData }
    }
}
