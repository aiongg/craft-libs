//! AccessKit node building, `register_accesskit_parent` and `set_accesskit_focus`.

/// The AccessKit node number of `id`, the key an action request's target is matched by.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the component pipeline (F3) registers and reads it"))]
pub(crate) fn node_key(id: egui::Id) -> u64 {
    id.accesskit_id().0
}

/// The AccessKit role of a node.
pub(crate) type NodeRole = accesskit::Role;

/// The roles the library names as constants.
pub(crate) mod roles {
    use super::NodeRole;

    pub(crate) const BUTTON: NodeRole = accesskit::Role::Button;
    pub(crate) const GROUP: NodeRole = accesskit::Role::Group;
    pub(crate) const RADIO_GROUP: NodeRole = accesskit::Role::RadioGroup;
    pub(crate) const LIST: NodeRole = accesskit::Role::List;
    pub(crate) const LIST_ITEM: NodeRole = accesskit::Role::ListItem;
    pub(crate) const SLIDER: NodeRole = accesskit::Role::Slider;
}

/// `builder` with `parent` as the node its `Ui`'s AccessKit nodes attach to.
pub(crate) fn with_parent(builder: egui::UiBuilder, parent: egui::Id) -> egui::UiBuilder {
    builder.accessibility_parent(parent)
}
