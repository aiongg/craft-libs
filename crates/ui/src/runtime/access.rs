//! `Role` and the AccessKit emitter.

use crate::runtime::egui::access::{NodeRole, roles};

/// What a component is to assistive technology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Role {
    /// No node.
    None,
    /// The node merges into its container's node.
    Merged,
    /// A node of this AccessKit role.
    Node(NodeRole),
}

impl Role {
    /// A button.
    pub const BUTTON: Role = Role::Node(roles::BUTTON);
    /// A grouping of controls.
    pub const GROUP: Role = Role::Node(roles::GROUP);
    /// A radio group.
    pub const RADIO_GROUP: Role = Role::Node(roles::RADIO_GROUP);
    /// A list.
    pub const LIST: Role = Role::Node(roles::LIST);
    /// A list item.
    pub const LIST_ITEM: Role = Role::Node(roles::LIST_ITEM);
    /// A slider.
    pub const SLIDER: Role = Role::Node(roles::SLIDER);
}

/// The facts a component adds to its AccessKit node.
// F3-LATER: runtime::access (name, description, value, numeric, relations, sub-nodes and actions are F4)
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Access {}
