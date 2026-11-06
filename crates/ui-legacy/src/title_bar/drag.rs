//! The part of the bar that moves the window: the drag area, and the double-click both it and the
//! modal overlay answer.

use egui::{Context, Id, Rect, Ui, UiBuilder, accesskit};

use super::{Title, resize};

/// Makes the whole `bar` the window's drag area, below the controls drawn after it, for the bar
/// whose ids derive from `scope`. Returns the area's widget id.
///
/// AccessKit: a `TitleBar` node named by the title text, described by the unsaved label, with no
/// click action. The area takes no keyboard focus.
pub(super) fn area(ui: &mut Ui, bar: Rect, scope: Id, title: Option<&Title>) -> Id {
    let ctx = ui.ctx().clone();
    let mut area = ui.new_child(UiBuilder::new().id_salt(scope.with("drag")).max_rect(bar));
    let empty = craft_window::title_bar::empty_area(&mut area, bar);
    if empty.move_started {
        resize::hand_off(&ctx);
    }
    let id = empty.response.id;
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(accesskit::Role::TitleBar);
        node.clear_actions();
        if let Some(title) = title {
            node.set_label(title.name());
            if let Some(description) = title.description() {
                node.set_description(description);
            }
        }
    });
    id
}

/// What a double-click on the empty bar asks of the window: maximize or restore on Windows and
/// Linux, the system setting on macOS.
pub(super) fn double_clicked(ctx: &Context) {
    let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
    if let Some(command) = craft_window::title_bar::double_click_command(craft_window::title_bar::action(), maximized) {
        ctx.send_viewport_cmd(command);
    }
}
