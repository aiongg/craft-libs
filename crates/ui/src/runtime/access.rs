//! `Role` and the AccessKit emitter.

/// What a component is to assistive technology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Role {
    /// No node.
    None,
    /// The node merges into its container's node.
    Merged,
    /// A node of this AccessKit role.
    Node(accesskit::Role),
}

impl Role {
    /// A button.
    pub const BUTTON: Role = Role::Node(accesskit::Role::Button);
    /// A grouping of controls.
    pub const GROUP: Role = Role::Node(accesskit::Role::Group);
    /// A radio group.
    pub const RADIO_GROUP: Role = Role::Node(accesskit::Role::RadioGroup);
    /// A list.
    pub const LIST: Role = Role::Node(accesskit::Role::List);
    /// A list item.
    pub const LIST_ITEM: Role = Role::Node(accesskit::Role::ListItem);
    /// A slider.
    pub const SLIDER: Role = Role::Node(accesskit::Role::Slider);
}

/// The facts a component adds to its AccessKit node.
// F3-LATER: runtime::access (name, description, value, numeric, relations, sub-nodes and actions are F4)
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Access {}
