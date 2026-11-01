//! The filter button in a table's header cell (N6.10).

use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::list::SortOrder;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::primitives::FocusRing;
use craft_ui::theme::scale::Stroke as Line;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{CornerRadius, IdSalt, Rect, Response, Sense, StrokeKind, Ui, WidgetInfo, pos2, vec2};

use super::style::SheetStyle;

craft_ui::words! {
    /// The filter button's words, English by default (Excel's).
    #[non_exhaustive]
    pub struct FilterButtonLabels {
        filter: "filterButton.filter" = "{column} (Showing All)";
        filtered: "filterButton.filtered" = "{column}: Filter applied";
    }
}

/// The button's square in a header cell `cell` at `zoom`: side the cell's height less twice the
/// inset, clamped to `style.filter_button_min`…`style.filter_button_max` (`sheet.filter_*`)
/// and no wider than the cell's reserved filter space less the inset, `style.filter_button_inset
/// × zoom` from the bottom-right corner.
// gridcraft: crates/ui-egui/src/grid/tables.rs:4 `filter_button_rect`.
pub fn filter_button_rect(cell: Rect, zoom: f32, space: f32, style: &SheetStyle) -> Rect {
    let inset = style.filter_button_inset * zoom;
    let side = (cell.height() - 2.0 * style.filter_button_inset)
        .clamp(style.filter_button_min, style.filter_button_max)
        .min((space * zoom - inset).max(0.0));
    Rect::from_min_size(pos2(cell.right() - side - inset, cell.bottom() - side - inset), vec2(side, side))
}

/// Excel's filter arrow (GridCraft): a small drop-down button in the header cell, a funnel when
/// the column is filtered (and a sort arrow when sorted, from the app). A click is the app's
/// (it opens its filter menu under the button). A Tab stop named by [`FilterButtonLabels`] with
/// the column's name; Alt+Down opens it from the cell, as Excel.
pub struct FilterButton {
    id_salt: IdSalt,
    rect: Rect,
    filtered: bool,
    sorted: Option<SortOrder>,
    column: String,
    labels: Option<FilterButtonLabels>,
}

impl FilterButton {
    /// `column`: the header's text, for the accessible name.
    pub fn new(id_salt: impl egui::AsIdSalt, rect: Rect, column: impl Into<String>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), rect, filtered: false, sorted: None, column: column.into(), labels: None }
    }

    pub fn labels(mut self, labels: FilterButtonLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn filtered(mut self, on: bool) -> Self {
        self.filtered = on;
        self
    }

    /// The column's sort, if any.
    pub fn sorted(mut self, order: Option<SortOrder>) -> Self {
        self.sorted = order;
        self
    }

    // gridcraft: crates/ui-egui/src/grid/tables.rs (heading paint and the button), `grid.rs`
    // (the filter menu opening).
    pub fn show(self, ui: &mut Ui) -> Response {
        let Self { id_salt, rect, filtered, sorted, column, labels } = self;
        let labels = labels.unwrap_or_else(|| FilterButtonLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = SheetStyle::of(&tokens);
        let colors = &tokens.color;
        let id = ui.make_persistent_id(id_salt);
        let response = ui.interact(rect, id, Sense::click());
        let name = fmt(if filtered { &labels.filtered } else { &labels.filter }, &[("column", &column)]);
        response.widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), &name));
        let radius = CornerRadius::same(style.filter_button_radius.round() as u8);
        let fill = if response.hovered() { colors.state.hover } else { colors.surface.raised };
        let edge = Line::Hairline.egui(ui.ctx(), colors.line.control_edge.to_egui());
        ui.painter().rect_filled(rect, radius, fill.to_egui());
        ui.painter().rect_stroke(rect, radius, edge, StrokeKind::Inside);
        let icon = match (filtered, sorted) {
            (true, _) => Icon::Funnel,
            (false, Some(SortOrder::Ascending)) => Icon::ChevronUp,
            (false, _) => Icon::ChevronDown,
        };
        let glyph = rect.shrink(style.filter_button_glyph_inset);
        icons::paint_in(ui, glyph, icon, icons::active(ui.ctx()), glyph.width(), IconSize::Sm.stroke(), colors.text.normal.to_egui());
        FocusRing::paint_if(ui, rect, radius, response.has_focus());
        response
    }
}
