//! Scrolls a newly focused control of a dialog's body into view.

use egui::{Id, Ui};

/// Scrolls the control that has just taken the focus into view, when it lies in the body being
/// laid out in `ui` (the scroll area's content). Called after the body, so the control's rect is
/// from this pass; a control outside the body, such as a footer button, is left alone.
pub(super) fn scroll_focus_into_view(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let last = ui.make_persistent_id("last-focus");
    let focused = ctx.memory(|m| m.focused());
    let before = ctx.data(|d| d.get_temp::<Option<Id>>(last)).flatten();
    ctx.data_mut(|d| d.insert_temp(last, focused));
    if let Some(id) = focused.filter(|id| Some(*id) != before)
        && let Some(response) = ctx.read_response(id)
        && ui.min_rect().contains_rect(response.rect)
    {
        ui.scroll_to_rect(response.rect, None);
    }
}
