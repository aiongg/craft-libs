//! `TileGroup` headers, pinned while their group scrolls.

use crate::primitives::Divider;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Rect, Response, Sense, Ui, pos2};

use super::{TileGroup, TileStyle};

/// Draws a group header in `rect`: title `BodyStrong` `tile.group.text` at the start, detail
/// `Caption` `tile.group.count` at the end, hover wash `state.hover` × 0.5; pinned copies on
/// `tile.group.pinned` with a `line.divider` hairline below. A `Heading` node named title and
/// detail. Senses clicks; not focusable.
pub(super) fn group_header(ui: &mut Ui, rect: Rect, group: &TileGroup, pinned: bool, style: &TileStyle) -> Response {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:656 (`group_header`).
    let tokens = Tokens::of(ui);
    let id = ui.scope_id().with(("tile-group", group.start, pinned));
    let response = ui.interact(rect, id, Sense::click());
    let name = if group.detail.is_empty() { group.title.clone() } else { format!("{}, {}", group.title, group.detail) };
    let accessible = name.clone();
    response.widget_info(|| egui::WidgetInfo::labeled(Role::Label, true, &accessible));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(Role::Heading);
        node.set_label(name);
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    if pinned {
        painter.rect_filled(rect, 0.0, style.group_pinned);
        Divider::paint(painter, rect.left_bottom(), rect.right_bottom(), &tokens);
    }
    if response.hovered() {
        painter.rect_filled(rect, 0.0, tokens.color.state.hover.to_egui().gamma_multiply(0.5));
    }
    let rtl = crate::direction::is_rtl(ui);
    let pad = style.inset + 2.0;
    let text = |role: TypeRole, color, s: &str| painter.layout_job(LayoutJob::single_section(s.to_owned(), role_format(&tokens, role, color)));
    let title = text(TypeRole::BodyStrong, style.group_text, &group.title);
    let detail = (!group.detail.is_empty()).then(|| text(TypeRole::Caption, style.group_count, &group.detail));
    let title_x = if rtl { rect.right() - pad - title.size().x } else { rect.left() + pad };
    let title_y = rect.center().y - title.size().y / 2.0;
    painter.with_clip_rect(rect).galley(pos2(title_x, title_y), title, style.group_text);
    if let Some(detail) = detail {
        let x = if rtl { rect.left() + pad } else { rect.right() - pad - detail.size().x };
        painter.galley(pos2(x, rect.center().y - detail.size().y / 2.0), detail, style.group_count);
    }
    response
}

/// The rect of the pinned header: the header of the group whose rows cross the viewport's top,
/// pinned there, pushed up by the next group's header as it arrives. `None` when the group's own
/// header is in view.
pub(super) fn pinned_rect(headers: &[Rect], viewport: Rect, group_at_top: Option<usize>) -> Option<(usize, Rect)> {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:436 (the current header sticks; the next pushes it up).
    let group = group_at_top?;
    let header = *headers.get(group)?;
    if header.top() >= viewport.top() {
        return None;
    }
    let mut top = viewport.top();
    if let Some(next) = headers.get(group + 1) {
        top = top.min(next.top() - header.height());
    }
    Some((group, Rect::from_min_size(pos2(header.left(), top), header.size())))
}
