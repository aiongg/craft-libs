//! AccessKit node building, `register_accesskit_parent` and `set_accesskit_focus`.

/// The AccessKit node number of `id`, the key an action request's target is matched by.
pub(crate) fn node_key(id: egui::Id) -> u64 {
    id.accesskit_id().0
}
