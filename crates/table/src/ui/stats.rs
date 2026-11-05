//! Selection statistics in the status bar (N6.08).

use craft_ui::localized::{Labels as _, fmt};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{CursorIcon, IdSalt, Label, PointerButton, Rect, Response, RichText, Sense, Ui, WidgetInfo, vec2};

use super::style::SheetStyle;

craft_ui::words! {
    /// The statistics' names, English by default (GridCraft's, Excel's).
    #[non_exhaustive]
    pub struct StatsLabels {
        average: "stats.average" = "Average: {v}";
        count: "stats.count" = "Count: {v}";
        numerical_count: "stats.numericalCount" = "Numerical Count: {v}";
        minimum: "stats.minimum" = "Minimum: {v}";
        maximum: "stats.maximum" = "Maximum: {v}";
        sum: "stats.sum" = "Sum: {v}";
        average_item: "stats.averageItem" = "Average";
        count_item: "stats.countItem" = "Count";
        numerical_count_item: "stats.numericalCountItem" = "Numerical Count";
        minimum_item: "stats.minimumItem" = "Minimum";
        maximum_item: "stats.maximumItem" = "Maximum";
        sum_item: "stats.sumItem" = "Sum";
        click_to_copy: "stats.clickToCopy" = "Click to copy";
    }
}

/// A selection's statistics, worked out by the app over its cells.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectionStats {
    /// Non-empty cells.
    pub count: u64,
    /// Cells holding numbers.
    pub numeric: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
}

impl SelectionStats {
    /// `None` without numbers.
    pub fn average(&self) -> Option<f64> {
        (self.numeric > 0).then(|| self.sum / self.numeric as f64)
    }
}

/// Which statistics show; one settings section (Excel's status bar menu is app-wide).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StatsShown {
    pub average: bool,
    pub count: bool,
    pub numerical_count: bool,
    pub minimum: bool,
    pub maximum: bool,
    pub sum: bool,
}

impl Default for StatsShown {
    /// Excel's: Average, Count and Sum.
    fn default() -> Self {
        Self { average: true, count: true, numerical_count: false, minimum: false, maximum: false, sum: true }
    }
}

/// One statistic of the status bar: the label with its value, and the value alone, which a click
/// copies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatPart {
    /// "Average: 3.5".
    pub text: String,
    /// "3.5".
    pub value: String,
}

/// The status bar's statistics, left to right: Average, Count, Numerical Count, Minimum, Maximum,
/// Sum, each when shown and the selection has a value for it (Count and Numerical Count with any
/// cell, the others with numbers). `format` formats a number in the active cell's number format.
// gridcraft: crates/ui-egui/src/tabs.rs:441 `selection_statistics`.
pub fn stats_parts(stats: &SelectionStats, shown: &StatsShown, labels: &StatsLabels, format: impl Fn(f64) -> String) -> Vec<StatPart> {
    let numbers = stats.numeric > 0;
    let any = stats.count > 0;
    let mut parts = Vec::new();
    let mut push = |label: &str, value: String| parts.push(StatPart { text: fmt(label, &[("v", &value)]), value });
    if let Some(average) = stats.average().filter(|_| shown.average) {
        push(&labels.average, format(average));
    }
    if shown.count && any {
        push(&labels.count, stats.count.to_string());
    }
    if shown.numerical_count && any {
        push(&labels.numerical_count, stats.numeric.to_string());
    }
    if shown.minimum && numbers {
        push(&labels.minimum, format(stats.min));
    }
    if shown.maximum && numbers {
        push(&labels.maximum, format(stats.max));
    }
    if shown.sum && numbers {
        push(&labels.sum, format(stats.sum));
    }
    parts
}

/// One statistic of the menu.
#[derive(Clone, Copy)]
enum Statistic {
    Average,
    Count,
    NumericalCount,
    Minimum,
    Maximum,
    Sum,
}

/// The statistics in the status bar for a selection of more than one cell, separated by wide
/// spaces. A click on a statistic copies its value without the label (GridCraft `tabs.rs:421`);
/// a secondary click opens the menu of check items (one per statistic), which changes `shown`
/// directly. The readout is as wide as the room it is given, so its menu opens with no statistic
/// shown (GridCraft opens it from the whole status bar). Text (AccessKit labels); the menu
/// items are the Tab stops.
pub struct SelectionStatsReadout<'a> {
    id_salt: IdSalt,
    parts: &'a [StatPart],
    shown: &'a mut StatsShown,
    labels: Option<StatsLabels>,
}

impl<'a> SelectionStatsReadout<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, parts: &'a [StatPart], shown: &'a mut StatsShown) -> Self {
        Self { id_salt: IdSalt::new(id_salt), parts, shown, labels: None }
    }

    pub fn labels(mut self, labels: StatsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // gridcraft: crates/ui-egui/src/tabs.rs:323 `status_bar`, `:479` `status_bar_menu`.
    pub fn show(self, ui: &mut Ui) -> Response {
        let Self { id_salt, parts, shown, labels } = self;
        let labels = labels.unwrap_or_else(|| StatsLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = SheetStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let font = tokens.font(style.stats_text);
        let color = tokens.color.text.muted.to_egui();
        let room = ui.available_width();
        let mut rects = Vec::with_capacity(parts.len());
        // The readout keeps one text line of height with no statistic shown.
        let line = ui.fonts_mut(|f| f.row_height(&font));
        let row = ui.horizontal(|ui| {
            ui.set_min_height(line);
            ui.spacing_mut().item_spacing.x = style.stats_gap;
            for part in parts {
                let label = ui.add(Label::new(RichText::new(&part.text).font(font.clone()).color(color)).selectable(false));
                rects.push(label.rect);
            }
        });
        let area = Rect::from_min_size(row.response.rect.min, vec2(room.max(row.response.rect.width()), row.response.rect.height()));
        let mut response = ui.interact(area, id, Sense::click());
        let over = |at: Option<egui::Pos2>| at.and_then(|at| rects.iter().position(|rect| rect.contains(at)));
        if over(response.hover_pos()).is_some() {
            response = response.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(&labels.click_to_copy);
        }
        if response.clicked_by(PointerButton::Primary)
            && let Some(part) = over(response.interact_pointer_pos())
        {
            ui.ctx().copy_text(parts[part].value.clone());
        }
        response.widget_info(|| WidgetInfo::new(Role::GenericContainer));
        let item = |label: &str, statistic: Statistic, on: bool| MenuEntry::from(MenuItem::new(label, statistic).check(on).keep_open(true));
        let entries = [
            item(&labels.average_item, Statistic::Average, shown.average),
            item(&labels.count_item, Statistic::Count, shown.count),
            item(&labels.numerical_count_item, Statistic::NumericalCount, shown.numerical_count),
            item(&labels.minimum_item, Statistic::Minimum, shown.minimum),
            item(&labels.maximum_item, Statistic::Maximum, shown.maximum),
            item(&labels.sum_item, Statistic::Sum, shown.sum),
        ];
        if let Some(statistic) = ContextMenu::new(&entries).min_width(style.status_menu_min_width).show(ui, &response).chosen {
            let flag = match statistic {
                Statistic::Average => &mut shown.average,
                Statistic::Count => &mut shown.count,
                Statistic::NumericalCount => &mut shown.numerical_count,
                Statistic::Minimum => &mut shown.minimum,
                Statistic::Maximum => &mut shown.maximum,
                Statistic::Sum => &mut shown.sum,
            };
            *flag = !*flag;
        }
        response
    }
}
