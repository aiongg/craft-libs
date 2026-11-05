//! The error checking tag beside a cell (N6.09).

use craft_ui::behavior::ClaimedKey;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::localized::Labels as _;
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::primitives::FocusRing;
use craft_ui::theme::scale::Stroke as Line;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{CornerRadius, IdSalt, Key, Modifiers, Rect, Response, Sense, StrokeKind, Ui, WidgetInfo, pos2, vec2};

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

/// Excel's error checking tag (GridCraft): a `sheet.error_tag` square ([`SheetStyle`], not scaled by the zoom)
/// left of `cell` (right of it when it would cover the row headings), not drawn when outside the
/// cells; the warning-diamond icon in the `status.warn` colour, the problem as tooltip (the app's
/// words, "Number Stored as Text"); a click opens a [`PressMenu`]: the
/// problem as its title, then the fixes. It senses click and drag, so a press on it never reaches
/// the grid under it. A Tab stop (AccessKit `Button` with a popup) named
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
    /// A tag for `cell` in `geometry`, with `problem` as tooltip and menu title and `fixes` as the menu entries.
    pub fn new(id_salt: impl egui::AsIdSalt, cell: CellRef, geometry: &'a SheetGeometry, problem: &'a str, fixes: &'a [&'a str]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), cell, geometry, problem, fixes, labels: None }
    }

    /// Uses these label texts instead of the ones read from the active locale's catalog.
    pub fn labels(mut self, labels: ErrorTagLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Where the tag goes: `style.error_tag` points square and `style.error_tag_gap` from the cell
    /// (neither scaled by the sheet's zoom: GridCraft `menus.rs:424-432` draws the tag at fixed
    /// points), `None` when it is outside the cells.
    // gridcraft: crates/ui-egui/src/grid/menus.rs:424 (placement).
    pub fn rect(&self, style: &SheetStyle) -> Option<Rect> {
        let (side, gap) = (style.error_tag, style.error_tag_gap);
        let cell = self.geometry.cell_rect(self.cell);
        let cells = self.geometry.cells();
        let left = cell.left() - side - gap;
        let left = if left >= cells.left() { left } else { cell.right() + gap };
        let tag = self.geometry.mirror_rect(Rect::from_min_size(pos2(left, cell.top()), vec2(side, side)));
        cells.intersects(tag).then_some(tag)
    }

    // gridcraft: crates/ui-egui/src/grid/menus.rs:416 `error_check_tag`.
    /// Shows the tag and its menu; the response holds the chosen fix.
    pub fn show(self, ui: &mut Ui) -> ErrorTagResponse {
        let tokens = Tokens::of(ui);
        let style = SheetStyle::of(&tokens);
        let Some(tag) = self.rect(&style) else {
            return ErrorTagResponse { response: None, fix: None };
        };
        let labels = self.labels.unwrap_or_else(|| ErrorTagLabels::of(ui));
        let colors = &tokens.color;
        let id = ui.make_persistent_id(self.id_salt);
        let response = ui.interact(tag, id, Sense::click_and_drag()).on_hover_text(self.problem);
        response.widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), &labels.options));

        let radius = CornerRadius::same(style.error_tag_radius.round() as u8);
        let fill = if response.hovered() { colors.state.hover } else { colors.surface.raised };
        let edge = Line::Hairline.egui(ui.ctx(), colors.line.border.to_egui());
        let painter = ui.painter();
        painter.rect_filled(tag, radius, fill.to_egui());
        painter.rect_stroke(tag, radius, edge, StrokeKind::Inside);
        icons::paint_in(
            ui,
            tag,
            Icon::WarningDiamond,
            icons::active(ui.ctx()),
            IconSize::Sm.points(&tokens),
            IconSize::Sm.stroke(),
            colors.status.warn.solid.to_egui(),
        );
        FocusRing::paint_if(ui, tag, radius, response.has_focus());

        let mut entries: Vec<MenuEntry<Option<usize>>> = vec![MenuItem::new(self.problem, None).enabled(false).into(), MenuEntry::Separator];
        entries.extend(self.fixes.iter().enumerate().map(|(i, fix)| MenuEntry::from(MenuItem::new(*fix, Some(i)))));
        let alt_down = response.has_focus() && ui.input_mut(|input| ClaimedKey::exact(Modifiers::ALT, Key::ArrowDown).consume(input));
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
