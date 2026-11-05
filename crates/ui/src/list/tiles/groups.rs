//! `TileGroup` headers, pinned while their group scrolls.

use crate::controls::TooltipExt as _;
use crate::localized::Labels as _;
use crate::primitives::{Divider, Interactive};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use craft_theme::TypeRole;
use egui::accesskit::{Action, Role};
use egui::text::LayoutJob;
use egui::{Rect, Response, Sense, Ui, pos2, vec2};

use crate::list::tree::with_row;
use crate::list::{ListRow, TreeText, paint_disclosure};

use super::{TileGroup, TileStyle};

/// What a group header reported.
pub(super) struct HeaderOut {
    pub response: Response,
    /// The state the disclosure chevron or an AccessKit Expand or Collapse asks for.
    pub toggled: Option<bool>,
}

/// Draws a group header in `rect`: title `BodyStrong` `tile.group.text` at the start, detail
/// `Caption` `tile.group.count` at the end, hover wash `state.hover` × 0.5; pinned copies on
/// `tile.group.pinned` with a `line.divider` hairline below. A `Heading` node named title and
/// detail. Senses clicks; not focusable. With `disclosure` (the group's open state), a chevron
/// in a `list.disclosure` column at the start, before the title.
pub(super) fn group_header(ui: &mut Ui, rect: Rect, group: &TileGroup, pinned: bool, style: &TileStyle, disclosure: Option<bool>) -> HeaderOut {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:656 (`group_header`); deckcraft: crates/ui-egui/src/panels/thumbs.rs:146 (section rows).
    let tokens = Tokens::of(ui);
    let id = ui.scope_id().with(("tile-group", group.start, pinned));
    let response = ui.interact(rect, id, Sense::click());
    let name = if group.detail.is_empty() { group.title.clone() } else { format!("{}, {}", group.title, group.detail) };
    let accessible = name.clone();
    response.widget_info(|| egui::WidgetInfo::labeled(Role::Label, true, &accessible));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(Role::Heading);
        node.set_label(name);
        if let Some(open) = disclosure {
            node.set_expanded(open);
            node.add_action(if open { Action::Collapse } else { Action::Expand });
        }
    });
    let mut toggled = disclosure.and_then(|open| {
        if ui.input(|i| i.has_accesskit_action_request(response.id, Action::Expand)) {
            Some(true)
        } else if ui.input(|i| i.has_accesskit_action_request(response.id, Action::Collapse)) {
            Some(false)
        } else {
            None
        }
        .filter(|want| *want != open)
    });
    if !ui.is_rect_visible(rect) {
        return HeaderOut { response, toggled };
    }
    let rtl = crate::direction::is_rtl(ui);
    let pad = style.inset + 2.0;
    let mut title_pad = pad;
    if let Some(open) = disclosure {
        let row = ListRow::<usize>::style(&tokens);
        let width = row.disclosure;
        let x = if rtl { rect.right() - pad - width } else { rect.left() + pad };
        let column = Rect::from_min_size(pos2(x, rect.top()), vec2(width, rect.height()));
        let text = TreeText::of(ui);
        let (pattern, tip) = if open { (&text.collapse, &text.collapse_tip) } else { (&text.expand, &text.expand_tip) };
        let label = group.title.as_str();
        let chevron = Interactive::with_id(id.with("disclosure"), Role::Button, with_row(pattern, label))
            .expanded(open)
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, column)
            .finish(ui)
            .tooltip(with_row(tip, label));
        if chevron.clicked() {
            toggled = Some(!open);
        }
        let color = if chevron.hovered() { row.icon_hover } else { row.icon };
        paint_disclosure(ui.painter(), column, open, rtl, color);
        title_pad += width;
    }
    let painter = ui.painter();
    if pinned {
        painter.rect_filled(rect, 0.0, style.group_pinned);
        Divider::paint(painter, rect.left_bottom(), rect.right_bottom(), &tokens);
    }
    if response.hovered() {
        painter.rect_filled(rect, 0.0, tokens.color.state.hover.to_egui().gamma_multiply(0.5));
    }
    let text = |role: TypeRole, color, s: &str| painter.layout_job(LayoutJob::single_section(s.to_owned(), role_format(&tokens, role, color)));
    let title = text(TypeRole::BodyStrong, style.group_text, &group.title);
    let detail = (!group.detail.is_empty()).then(|| text(TypeRole::Caption, style.group_count, &group.detail));
    let title_x = if rtl { rect.right() - title_pad - title.size().x } else { rect.left() + title_pad };
    let title_y = rect.center().y - title.size().y / 2.0;
    painter.with_clip_rect(rect).galley(pos2(title_x, title_y), title, style.group_text);
    if let Some(detail) = detail {
        let x = if rtl { rect.left() + pad } else { rect.right() - pad - detail.size().x };
        painter.galley(pos2(x, rect.center().y - detail.size().y / 2.0), detail, style.group_count);
    }
    HeaderOut { response, toggled }
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
