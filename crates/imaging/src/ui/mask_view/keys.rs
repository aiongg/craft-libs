//! [`MaskViewKeys`]: the view keys of Select and Mask (task SE-F6).

#![expect(unused_variables, clippy::todo, reason = "skeleton: task SE-F6")]

use egui::Context;

use super::{MaskViewChange, MaskViewState};

/// The view keys, read once per pass from the context's input while the workspace is open.
///
/// With no ⌘, Ctrl or ⌥ held, on a press (not a repeat): a view's letter (O M V A T K Y) picks
/// it, F picks the next view, J toggles Show Edge, P toggles Show Original; X held hides the view
/// until it is released. Nothing is read while a text field has the keyboard or a popup is open
/// (the View Mode drop-down, a menu): `hide_view` is then cleared. R, `[` and `]` (the Refine Edge
/// Brush) are the workspace's.
pub struct MaskViewKeys;

impl MaskViewKeys {
    /// Applies this pass's keys to `state` and returns the changes, in order.
    pub fn read(ctx: &Context, state: &mut MaskViewState) -> Vec<MaskViewChange> {
        // pc: crates/ui-egui/src/canvas/select_mask_view.rs:131-176 `apply_key`, `keys`
        // (`text_edit_focused() || Popup::is_any_open`; `plain = !command && !ctrl && !alt`;
        // `key_down(X)`).
        todo!()
    }
}
