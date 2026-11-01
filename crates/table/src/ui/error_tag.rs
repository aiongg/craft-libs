//! The error checking tag beside a cell (N6.09).

use craft_ui::localized::Labels as _;
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::primitives::FocusRing;
use craft_ui::theme::scale::Stroke as Line;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{Align2, CornerRadius, FontId, IdSalt, Key, Modifiers, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2};

use super::style::SheetStyle;

craft_ui::words! {
    /// The tag's accessible name, English by default (Excel's).
    #[non_exhaustive]
    pub struct ErrorTagLabels {
        options: "errorTag.options" = "Error checking options";
    }
}

use super::grid::CellRef;
use super::grid::SheetGeometry;

/// Excel's error checking tag (GridCraft): a `sheet.error_tag` square ([`SheetStyle`](super::SheetStyle), not scaled by the zoom)
/// left of `cell` (right of it when it would cover the row headings), not drawn when outside the
/// cells; an amber diamond with "!" (the `status.warn` fill and ink), the problem as tooltip
/// (the app's words, "Number Stored as Text"); a click opens a [`PressMenu`](craft_ui::menu::PressMenu):
/// the problem as its title, then the fixes. A Tab stop (AccessKit `Button` with a popup) named
/// [`ErrorTagLabels::options`]; Enter, Space or Alt+Down opens the menu.
pub struct ErrorTag<'a> {
    id_salt: IdSalt,
    cell: CellRef,
    geometry: &'a SheetGeometry,
    problem: &'a str,
    fixes: &'a [&'a str],
    labels: Option<ErrorTagLabels>,
}

impl<'a> ErrorTag<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, cell: CellRef, geometry: &'a SheetGeometry, problem: &'a str, fixes: &'a [&'a str]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), cell, geometry, problem, fixes, labels: None }
    }

    pub fn labels(mut self, labels: ErrorTagLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Where the tag goes for a tag `side` points square (`sheet.error_tag`, not scaled by the
    /// sheet's zoom) with `gap` points between it and the cell (`sheet.error_tag_gap`), `None`
    /// when it is outside the cells.
    // gridcraft: crates/ui-egui/src/grid/menus.rs:424 (placement).
    pub fn rect(&self, side: f32, gap: f32) -> Option<Rect> {
        let cell = self.geometry.cell_rect(self.cell);
        let cells = self.geometry.cells();
        let left = cell.left() - side - gap;
        let left = if left >= cells.left() { left } else { cell.right() + gap };
        let tag = self.geometry.mirror_rect(Rect::from_min_size(pos2(left, cell.top()), vec2(side, side)));
        cells.intersects(tag).then_some(tag)
    }

    // gridcraft: crates/ui-egui/src/grid/menus.rs:416 `error_check_tag`.
    pub fn show(self, ui: &mut Ui) -> ErrorTagResponse {
        let tokens = Tokens::of(ui);
        let style = SheetStyle::of(&tokens);
        let Some(tag) = self.rect(style.error_tag, style.error_tag_gap) else {
            return ErrorTagResponse { response: None, fix: None };
        };
        let labels = self.labels.unwrap_or_else(|| ErrorTagLabels::of(ui));
        let colors = &tokens.color;
        let id = ui.make_persistent_id(self.id_salt);
        let response = ui.interact(tag, id, Sense::click()).on_hover_text(self.problem);
        response.widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), &labels.options));

        let radius = CornerRadius::same(style.error_tag_radius.round() as u8);
        let fill = if response.hovered() { colors.state.hover } else { colors.surface.raised };
        let edge = Line::Hairline.egui(ui.ctx(), colors.line.border.to_egui());
        let painter = ui.painter();
        painter.rect_filled(tag, radius, fill.to_egui());
        painter.rect_stroke(tag, radius, edge, StrokeKind::Inside);
        let (centre, reach) = (tag.center(), style.error_tag_mark);
        painter.add(Shape::convex_polygon(
            vec![
                pos2(centre.x, centre.y - reach),
                pos2(centre.x + reach, centre.y),
                pos2(centre.x, centre.y + reach),
                pos2(centre.x - reach, centre.y),
            ],
            colors.status.warn.solid.to_egui(),
            Stroke::NONE,
        ));
        let mark = FontId::new(style.error_tag_mark_text, tokens.font(craft_theme::TypeRole::BodyStrong).family);
        painter.text(centre, Align2::CENTER_CENTER, "!", mark, colors.text.on_status.to_egui());
        FocusRing::paint_if(ui, tag, radius, response.has_focus());

        let mut entries: Vec<MenuEntry<Option<usize>>> = vec![MenuItem::new(self.problem, None).enabled(false).into(), MenuEntry::Separator];
        entries.extend(self.fixes.iter().enumerate().map(|(i, fix)| MenuEntry::from(MenuItem::new(*fix, Some(i)))));
        let alt_down = response.has_focus() && ui.input_mut(|input| input.consume_key(Modifiers::ALT, Key::ArrowDown));
        let fix = PressMenu::new(&entries).keyboard_owner(response.id, alt_down).show(ui, &response).chosen.flatten();
        ErrorTagResponse { response: Some(response), fix }
    }
}

/// What the tag did this pass.
pub struct ErrorTagResponse {
    /// The tag's response; `None` when the tag is outside the cells and not drawn.
    pub response: Option<Response>,
    /// The fix picked this pass, by index into the fixes.
    pub fix: Option<usize>,
}
