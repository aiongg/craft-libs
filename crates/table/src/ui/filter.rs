//! The filter button in a table's header cell (N6.10).

use craft_ui::behavior::ClaimedKey;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::list::SortOrder;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::primitives::FocusRing;
use craft_ui::theme::scale::Stroke as Line;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{CornerRadius, IdSalt, Key, Modifiers, Rect, Response, Sense, StrokeKind, Ui, WidgetInfo, pos2, vec2};

use super::style::SheetStyle;

craft_ui::words! {
    /// The filter button's words, English by default (Excel's).
    #[non_exhaustive]
    pub struct FilterButtonLabels {
        filter: "filterButton.filter" = "{column} (Showing All)";
        filtered: "filterButton.filtered" = "{column}: Filter applied";
    }
}

/// What the filter button reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FilterButtonEvent {
    /// The app opens its filter menu under the button: a click, or Alt+Down on the focused button.
    /// Both inputs in one pass are one event.
    Open,
}

/// What the filter button did this pass.
#[must_use]
pub struct FilterButtonOutput {
    pub response: Response,
    pub event: Option<FilterButtonEvent>,
}

/// The button's square in a header cell `cell` (already at `zoom`): the side is the cell's height
/// less twice `style.filter_button_inset`, clamped to `style.filter_button_min`…
/// `style.filter_button_max` (`sheet.filter_*`; neither the inset nor the clamp follows the zoom,
/// as GridCraft `tables.rs:6-7`), and no wider than the cell's reserved filter `space` × zoom less
/// the inset × zoom, which is also its distance from the bottom-right corner.
// gridcraft: crates/ui-egui/src/grid/tables.rs:4 `filter_button_rect`.
pub fn filter_button_rect(cell: Rect, zoom: f32, space: f32, style: &SheetStyle) -> Rect {
    let inset = style.filter_button_inset * zoom;
    let side = (cell.height() - 2.0 * style.filter_button_inset)
        .clamp(style.filter_button_min, style.filter_button_max)
        .min((space * zoom - inset).max(0.0));
    Rect::from_min_size(pos2(cell.right() - side - inset, cell.bottom() - side - inset), vec2(side, side))
}

/// Excel's filter arrow (GridCraft): a small drop-down button in the header cell, a funnel when
/// the column is filtered, and beside the funnel or chevron the sort arrow of a sorted column
/// (from the app, as Excel). A click or Alt+Down on the focused button reports
/// [`FilterButtonEvent::Open`]: the app opens its filter menu under the button. It senses click
/// and drag, so a press on it never reaches the grid under it. A Tab stop named by
/// [`FilterButtonLabels`] with the column's name.
pub struct FilterButton {
    id_salt: IdSalt,
    filtered: bool,
    sorted: Option<SortOrder>,
    column: String,
    labels: Option<FilterButtonLabels>,
}

impl FilterButton {
    /// `column`: the header's text, for the accessible name.
    pub fn new(id_salt: impl egui::AsIdSalt, column: impl Into<String>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), filtered: false, sorted: None, column: column.into(), labels: None }
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
    /// `rect`: the button's square, from [`filter_button_rect`].
    pub fn show(self, ui: &mut Ui, rect: Rect) -> FilterButtonOutput {
        let Self { id_salt, filtered, sorted, column, labels } = self;
        let labels = labels.unwrap_or_else(|| FilterButtonLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = SheetStyle::of(&tokens);
        let colors = &tokens.color;
        let id = ui.make_persistent_id(id_salt);
        // The small icon size, no larger than the button's own side less its glyph inset.
        let icon_side = IconSize::Sm.points(&tokens).min(rect.height() - 2.0 * style.filter_button_glyph_inset);
        let rect = match sorted {
            Some(_) => Rect::from_min_max(pos2(rect.left() - icon_side - style.filter_button_glyph_inset, rect.top()), rect.max),
            None => rect,
        };
        let response = ui.interact(rect, id, Sense::click_and_drag());
        let name = fmt(if filtered { &labels.filtered } else { &labels.filter }, &[("column", &column)]);
        response.widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), &name));
        let radius = CornerRadius::same(style.filter_button_radius.round() as u8);
        let fill = if response.hovered() { colors.state.hover } else { colors.surface.raised };
        let edge = Line::Hairline.egui(ui.ctx(), colors.line.control_edge.to_egui());
        ui.painter().rect_filled(rect, radius, fill.to_egui());
        ui.painter().rect_stroke(rect, radius, edge, StrokeKind::Inside);
        let base = if filtered { Icon::Funnel } else { Icon::ChevronDown };
        let ink = colors.text.normal.to_egui();
        let (stroke, set) = (IconSize::Sm.stroke(), icons::active(ui.ctx()));
        let (arrow_slot, base_slot) = match sorted {
            None => (None, rect),
            Some(_) => {
                let (left, right) = rect.split_left_right_at_x(rect.left() + icon_side + style.filter_button_glyph_inset);
                (Some(left), right)
            }
        };
        icons::paint_in(ui, base_slot, base, set, icon_side, stroke, ink);
        if let (Some(slot), Some(order)) = (arrow_slot, sorted) {
            let arrow = if order == SortOrder::Ascending { Icon::SortAscending } else { Icon::SortDescending };
            icons::paint_in(ui, slot, arrow, set, icon_side, stroke, ink);
        }
        FocusRing::paint_if(ui, rect, radius, response.has_focus());
        let alt_down = response.has_focus() && ui.input_mut(|input| ClaimedKey::exact(Modifiers::ALT, Key::ArrowDown).consume(input));
        let event = (response.clicked() || alt_down).then_some(FilterButtonEvent::Open);
        FilterButtonOutput { response, event }
    }
}
