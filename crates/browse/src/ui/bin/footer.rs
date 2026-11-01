//! The bin footer: view buttons, zoom, the host's actions and their drops (`FooterAction`).

use craft_ui::behavior::ListDrag;
use craft_ui::canvas::textures::TextureKey;
use craft_ui::composites::{FooterAction, PanelFooter};
use craft_ui::controls::{ControlSize, Segmented};
use craft_ui::icons::Icon;
use craft_ui::list::{SortOrder, TableColumn, TableEvent, TableSort};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use egui::{Id, Ui};

use super::{BinEvent, BinLabels, BinView};
use crate::ui::grid::{ThumbnailSlider, TileZoom};

/// The zoom slider's key step, in points.
pub(super) const ZOOM_STEP: f32 = 8.0;

/// What the footer shows and reads.
pub(super) struct FooterInput<'a> {
    pub id: Id,
    pub view: BinView,
    pub icon_size: f32,
    pub zoom_range: (f32, f32),
    pub default_size: f32,
    pub actions: &'a [FooterAction],
    pub columns: &'a [TableColumn<String>],
    pub icon_sort: Option<&'a (String, bool)>,
    pub labels: &'a BinLabels,
}

/// A pick in the Sort Icons menu.
#[derive(Clone, Debug, PartialEq, Eq)]
enum SortPick {
    UserOrder,
    By(String),
}

/// Draws the footer: a `PanelFooter` (height `panel_footer`) with the view `Segmented` (List,
/// Icon, Freeform glyphs), a `ThumbnailSlider`, Sort Icons (Icon view), each action as an
/// `PanelFooter::button`, or `PanelFooter::drop_button::<ListDrag<K>>` when it accepts drops, and
/// New Bin. Returns the footer's events.
pub(super) fn footer_events<K: TextureKey>(ui: &mut Ui, input: &FooterInput<'_>) -> Vec<BinEvent<K>> {
    // filmcraft: crates/ui-egui/src/panels/project.rs:555 (`footer`).
    let FooterInput { id, view, icon_size, zoom_range, default_size, actions, columns, icon_sort, labels } = *input;
    let mut events = Vec::new();
    PanelFooter::show(ui, |ui| {
        // The bar fills from the reading end: New Bin, the host's actions (last first, so they
        // read in the host's order), Sort Icons, then the slider and the view buttons.
        if PanelFooter::button(ui, Icon::NewFolder, &labels.new_bin).clicked() {
            events.push(BinEvent::NewBin);
        }
        for action in actions.iter().rev() {
            if action.accepts_drops {
                // The enabled state goes in as an argument: a disabled `Ui` would keep the button
                // disabled under an accepted drag.
                let (button, dropped) =
                    PanelFooter::drop_button::<ListDrag<K>>(ui, action.icon, &action.label, action.enabled, |drag| !drag.items.is_empty());
                if let Some(drag) = dropped {
                    events.push(BinEvent::FooterDrop { button: action.id, rows: drag.items.clone() });
                } else if button.clicked() {
                    events.push(BinEvent::Footer { button: action.id });
                }
            } else if ui.add_enabled_ui(action.enabled, |ui| PanelFooter::button(ui, action.icon, &action.label)).inner.clicked() {
                events.push(BinEvent::Footer { button: action.id });
            }
        }
        if view != BinView::List {
            sort_menu(ui, id, columns, icon_sort, labels, &mut events);
        }
        let zoom = TileZoom::Range { min: zoom_range.0, max: zoom_range.1, step: ZOOM_STEP, default: default_size };
        if let Some(event) = ThumbnailSlider::new((id, "zoom"), icon_size, zoom).show(ui).event {
            events.push(BinEvent::Zoom(event));
        }
        let mut picked = view;
        Segmented::new((id, "view"), &labels.views, &mut picked)
            .size(ControlSize::Sm)
            .icon_item(BinView::List, Icon::List, labels.list_view.as_str())
            .icon_item(BinView::Icon, Icon::Grid, labels.icon_view.as_str())
            .icon_item(BinView::Freeform, Icon::LayoutGrid, labels.freeform_view.as_str())
            .show(ui);
        if picked != view {
            events.push(BinEvent::View(picked));
        }
    });
    events
}

/// The Sort Icons button and its menu: User Order and one entry per sortable column; the column
/// of the current sort again flips its direction.
fn sort_menu<K>(
    ui: &mut Ui,
    id: Id,
    columns: &[TableColumn<String>],
    icon_sort: Option<&(String, bool)>,
    labels: &BinLabels,
    events: &mut Vec<BinEvent<K>>,
) {
    let slot = id.with("sort-menu-open");
    let open = ui.ctx().data(|d| d.get_temp::<bool>(slot)).unwrap_or(false);
    let button = PanelFooter::menu_button(ui, Icon::Menu, &labels.sort_icons, open);
    let mut entries = vec![MenuEntry::Item(MenuItem::new(labels.user_order.as_str(), SortPick::UserOrder).radio(icon_sort.is_none()))];
    entries.extend(columns.iter().map(|column| {
        MenuEntry::Item(
            MenuItem::new(column.label(), SortPick::By(column.key().clone())).radio(icon_sort.is_some_and(|(key, _)| key == column.key())),
        )
    }));
    let open_at = button.clicked().then_some(OpenAt::Rect(button.rect));
    let menu = ContextMenu::new(&entries).own_gestures(false).open_at(open_at).show(ui, &button);
    ui.ctx().data_mut(|d| d.insert_temp(slot, menu.open));
    match menu.chosen {
        Some(SortPick::UserOrder) => events.push(BinEvent::Table(TableEvent::Unsorted)),
        Some(SortPick::By(column)) => {
            let current = icon_sort.map(|(key, descending)| TableSort {
                column: key.clone(),
                order: if *descending { SortOrder::Descending } else { SortOrder::Ascending },
            });
            events.push(BinEvent::Table(TableEvent::Sort(TableSort::clicked(current.as_ref(), column, SortOrder::Ascending))));
        }
        None => {}
    }
}
