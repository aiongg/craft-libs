//! How the gallery's chrome applies craft-ui's value controls. The gallery keeps no undo history,
//! so every reported value is applied at once and no gesture can be cancelled: Esc ends a drag or
//! a typing run with the value shown.

use craft_ui::edit::{EditPolicy, EditResponse};

/// The policy of the chrome's value controls.
pub(crate) fn policy() -> EditPolicy {
    EditPolicy::default().cancellable(false)
}

/// Writes the value `edit` reported this frame into `value`; true when it did.
pub(crate) fn apply<T: Clone>(edit: &EditResponse<T>, value: &mut T) -> bool {
    match edit.event.as_ref().and_then(|event| event.value()) {
        Some(reported) => {
            value.clone_from(reported);
            true
        }
        None => false,
    }
}
