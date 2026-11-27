//! Dialogs on craft-ui-rt: each open dialog is a scope on the modal layer, a layer child of the
//! node whose build shows it (the `DialogHost`'s caller). Dialogs shown later lie above earlier
//! ones; the modal layer blocks the layers below it in its window, and a modal-rank dialog blocks
//! every other window of the app ([`craft_ui_rt::LayerSpec::app_modal`]). Escape, when no focused
//! node takes it, dismisses the top dialog.
//!
//! `DialogHost::run` shows each dialog on its stack through [`show_dialog`], in stack order, keyed
//! by its `DialogId`; a dialog's body runs when it opens or when a node in it is marked.

use super::overlay::{LayerShow, close_reason, dismissed, show_layer};
use super::ui::Ui;
use crate::contract::overlay::Overlay;
use crate::runtime::overlay::CloseReason;

/// Shows the dialog keyed `key` with `body` as its content, placed and dismissed as `overlay`
/// says, as a layer child of the node `ui` builds. `Err` with the reason when ui-rt dismissed it
/// since the last pass: it shows nothing then, and its node goes.
pub fn show_dialog<R: Clone + 'static>(
    ui: &mut Ui<'_, '_>,
    key: u64,
    overlay: &Overlay,
    mut body: impl FnMut(&mut Ui<'_, '_>) -> R,
) -> Result<R, CloseReason> {
    let owner = ui.cx.node();
    let show = LayerShow::of(overlay, owner, ui.flow.region, false, &ui.flow.tokens);
    if let Some(reason) = dismissed(ui.cx, show.layer, key) {
        return Err(close_reason(reason));
    }
    let id = ui.flow.id.with(("craft-dialog", key));
    Ok(show_layer(ui.cx, id, key, show, false, &mut body))
}
