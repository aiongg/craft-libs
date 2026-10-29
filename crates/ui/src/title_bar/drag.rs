//! The part of the bar that moves the window: the drag area, and the double-click both it and the
//! modal overlay answer.

use egui::{Context, Id, Rect, Ui, UiBuilder, ViewportCommand, accesskit};

use super::resize;

/// Makes the whole `bar` the window's drag area, below the controls drawn after it, for the bar
/// whose ids derive from `scope`. Returns the area's widget id.
///
/// AccessKit: a `TitleBar` node named by the title text, with no click action. The area takes no
/// keyboard focus.
pub(super) fn area(ui: &mut Ui, bar: Rect, scope: Id, title: Option<&str>) -> Id {
    let ctx = ui.ctx().clone();
    let starts = || ctx.viewport(|v| v.commands.iter().filter(|c| matches!(c, ViewportCommand::StartDrag)).count());
    let before = starts();
    let mut area = ui.new_child(UiBuilder::new().id_salt(scope.with("drag")).max_rect(bar));
    let resp = craft_window::title_bar::empty_area(&mut area, bar);
    if starts() > before {
        resize::hand_off(&ctx);
    }
    ctx.accesskit_node_builder(resp.id, |node| {
        node.set_role(accesskit::Role::TitleBar);
        node.clear_actions();
        if let Some(title) = title {
            node.set_label(title);
        }
    });
    resp.id
}

/// What a double-click on the empty bar asks of the window: maximize or restore on Windows and
/// Linux, the system setting on macOS.
pub(super) fn double_clicked(ctx: &Context) {
    let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
    if let Some(command) = craft_window::title_bar::double_click_command(craft_window::title_bar::action(), maximized) {
        ctx.send_viewport_cmd(command);
    }
}
