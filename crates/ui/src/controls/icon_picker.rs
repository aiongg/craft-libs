//! [`IconPicker`]: a searchable grid of icons or tool tiles in named groups.

use std::ops::Range;

use egui::accesskit::{Action, Role};
use egui::{
    AsIdSalt, Color32, CornerRadius, Event, EventFilter, Id, IdSalt, Key, Modifiers, Painter, Rect, Response, RichText, ScrollArea, Sense, Ui,
    UiBuilder, pos2, vec2,
};

use super::option_list::{load_kept, store_kept};
use super::{SearchField, TooltipExt as _};
use crate::behavior::{GridKey, GridNav};
use crate::direction;
use crate::localized::Labels as _;
use crate::primitives::Interactive;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

/// The corners of a hovered cell (DeckCraft's `CornerRadius::same(4)`).
const CELL_RADIUS: u8 = 4;

crate::words! {
    /// The picker's words, English by default.
    #[non_exhaustive]
    pub struct IconPickerLabels {
        /// The search field's placeholder and name (DeckCraft's).
        search: "iconPicker.search" = "Search icons";
        clear_search: "iconPicker.clearSearch" = "Clear Search";
        /// No item matches (PdfCraft's "No tools match your search." in the picker's words).
        no_match: "iconPicker.noMatch" = "No icons match your search.";
    }
}

/// One cell: what it picks, its name (tooltip, AccessKit, search) and more words it is found by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconPickerItem<K> {
    pub key: K,
    pub label: String,
    /// Words the search also matches ("arrow", "direction"); PdfCraft finds a tool by its menus'
    /// commands this way.
    pub keywords: Vec<String>,
}

/// A titled group of cells; with a search typed the groups with no match are hidden.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconPickerGroup<K> {
    pub name: String,
    pub items: Vec<IconPickerItem<K>>,
}

/// What the picker reported this pass: a composite of the search field and the grid.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct IconPickerOutput<K> {
    /// The grid: its focus and its rect.
    pub response: Response,
    /// The cell clicked or activated with Enter this pass. A host showing the picker in a
    /// popover closes it.
    pub picked: Option<K>,
}

/// Paints one cell's item into the rect, in the colour given.
type PaintCell<'a, K> = Box<dyn Fn(&Painter, Rect, &K, Color32) + 'a>;

/// A searchable icon grid, after DeckCraft's Insert › Icons popup, with PdfCraft's All Tools
/// search by keywords.
///
/// ```ignore
/// Popover::new("icons", PopoverAnchor::below(&button)).show(ui, &mut open, |ui, pop| {
///     let out = IconPicker::new("icons", "Icons", &groups, |p, r, key, c| paint_icon(p, r, key, c)).show(ui);
///     if let Some(name) = out.picked { app.run("insert.icon", name); pop.close(); }
/// });
/// ```
///
/// - **Search.** A [`SearchField`](crate::controls::SearchField) at the top, the full width,
///   which takes the focus when the picker first shows. It keeps the items whose label or a
///   keyword holds the typed text, ignoring case ([`icon_matches`]); with nothing typed every
///   group shows. The query lives in egui memory under the picker's id.
/// - **Grid.** Each group's name in the `Caption` role and `text.muted`, then its cells in rows
///   that wrap: 36-point squares 3 points apart (DeckCraft's `icon_cell`; `icon_picker.cell`,
///   `icon_picker.gap`), the item painted 6 points in by the host's `paint` in `text.normal`
///   (`text.disabled` in a disabled picker). Hover fills a cell with `state.hover` at 4-point
///   corners and shows its label as the tooltip. The body scrolls past 420 points (DeckCraft).
/// - **Keys.** ↓ from the search field moves into the grid; the arrows move the active cell
///   over every shown cell ([`GridNav`](crate::behavior::GridNav)); Enter
///   or Space picks it; ↑ from the first row goes back to the search field; typing a letter in
///   the grid goes to the search field with it, in the same pass (the letter is typed after the
///   focus moves). The grid is one Tab stop, on the active cell: the keys move over the cells as
///   rows of [`GridNav::ragged`](crate::behavior::GridNav::ragged), one row per line of each
///   group, so ↑ and ↓ keep the column across group headers; the active cell draws the focus
///   ring.
/// - **AccessKit.** The cells are a `Grid` named by the picker's label; each cell a `GridCell`
///   named by its label.
pub struct IconPicker<'a, K> {
    id_salt: IdSalt,
    label: String,
    groups: &'a [IconPickerGroup<K>],
    paint: PaintCell<'a, K>,
    max_height: f32,
    labels: Option<IconPickerLabels>,
}

impl<'a, K: Clone + PartialEq + Send + Sync + 'static> IconPicker<'a, K> {
    /// A picker named `label` over `groups`; `paint(painter, rect, key, color)` draws an item.
    pub fn new(
        id_salt: impl AsIdSalt,
        label: impl Into<String>,
        groups: &'a [IconPickerGroup<K>],
        paint: impl Fn(&Painter, Rect, &K, Color32) + 'a,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), groups, paint: Box::new(paint), max_height: 420.0, labels: None }
    }

    /// The most the grid takes before it scrolls (default 420).
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = height;
        self
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: IconPickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // deckcraft: crates/ui-egui/src/ribbon/insert.rs:199 icon_library, :217 icon_cell
    // pdfcraft: crates/ui-egui/src/panels/all_tools_ui.rs:45 show (groups, search)
    pub fn show(self, ui: &mut Ui) -> IconPickerOutput<K> {
        let Self { id_salt, label, groups, paint, max_height, labels } = self;
        let labels = labels.unwrap_or_else(|| IconPickerLabels::of(ui));
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let search_salt = (id_salt, "search");
        let search_id = Interactive::id_in(ui, search_salt);
        let enabled = ui.is_enabled();
        let rtl = direction::is_rtl(ui);
        let (cell, gap, inset) = {
            let picker = &tokens.component.icon_picker;
            (picker.cell, picker.gap, picker.inset)
        };
        let columns = GridNav::columns(ui.available_width() - ui.spacing().scroll.allocated_width(), 0.0, gap, cell);

        let first = load_kept::<()>(ui, id.with("kept")).is_none();
        store_kept(ui, id.with("kept"), ());
        if first {
            ctx.memory_mut(|m| m.request_focus(search_id));
        }
        let query_id = id.with("query");
        let mut query = ctx.data(|d| d.get_temp::<String>(query_id)).unwrap_or_default();

        // The keys move the focus between the search field and the cells before either is drawn,
        // so a letter typed in the grid reaches the search field in this pass.
        let shown = visible(groups, &query);
        let ids: Vec<Id> = (0..shown.iter().map(|g| g.items.len()).sum()).map(|at| id.with(("cell", at))).collect();
        let nav = GridNav::ragged(shown.iter().flat_map(|g| rows_of(g.items.len(), columns).map(|row| row.len()))).rtl(rtl);
        let focused = ids.iter().position(|&cell| ctx.memory(|m| m.has_focus(cell)));
        let mut stop = focused.unwrap_or(0);
        let mut moved = false;
        if enabled && let Some(from) = focused {
            let typed =
                ctx.input(|i| i.events.iter().any(|e| matches!(e, Event::Text(t) if t.chars().any(|c| !c.is_control() && !c.is_whitespace()))));
            if typed || (nav.row_of(from) == 0 && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp))) {
                ctx.memory_mut(|m| m.request_focus(search_id));
            } else {
                ctx.memory_mut(|m| {
                    m.set_focus_lock_filter(ids[from], EventFilter { horizontal_arrows: true, vertical_arrows: true, ..EventFilter::default() })
                });
                ctx.input_mut(|i| {
                    for (key, grid_key) in [
                        (Key::ArrowRight, GridKey::Right),
                        (Key::ArrowLeft, GridKey::Left),
                        (Key::ArrowDown, GridKey::Down),
                        (Key::ArrowUp, GridKey::Up),
                        (Key::Home, GridKey::Home),
                        (Key::End, GridKey::End),
                    ] {
                        if i.consume_key(Modifiers::NONE, key) {
                            stop = nav.step(stop, grid_key, None).index;
                            moved = true;
                        }
                    }
                });
                if moved {
                    ctx.memory_mut(|m| m.request_focus(ids[stop]));
                }
            }
        } else if enabled
            && !ids.is_empty()
            && ctx.memory(|m| m.has_focus(search_id))
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown))
        {
            ctx.memory_mut(|m| m.request_focus(ids[0]));
            stop = 0;
            moved = true;
        }
        if let Some(requested) = ids.iter().position(|&cell| ctx.input(|i| i.has_accesskit_action_request(cell, Action::Focus))) {
            stop = requested;
            ctx.memory_mut(|m| m.request_focus(ids[stop]));
        }

        let edit = SearchField::new(search_salt, labels.search.as_str(), &query, labels.clear_search.as_str())
            .placeholder(labels.search.as_str())
            .enabled(enabled)
            .keys(EventFilter { vertical_arrows: true, ..EventFilter::default() })
            .show(ui);
        if let Some(text) = edit.event.as_ref().and_then(|e| e.value()) {
            query.clone_from(text);
            ctx.data_mut(|d| d.insert_temp(query_id, query.clone()));
        }
        let shown = visible(groups, &query);
        let ids: Vec<Id> = (0..shown.iter().map(|g| g.items.len()).sum()).map(|at| id.with(("cell", at))).collect();
        let stop = stop.min(ids.len().saturating_sub(1));

        let mut picked = None;
        let grid = ui.new_child(
            UiBuilder::new()
                .id_salt((id_salt, "grid"))
                .max_rect(ui.available_rect_before_wrap())
                .accessibility_role(Role::Grid)
                .accessibility_label(label.as_str()),
        );
        let mut grid = grid;
        if shown.is_empty() {
            grid.add_space(ui.spacing().item_spacing.y);
            grid.label(labels.no_match.as_str());
        } else {
            grid.add_space(ui.spacing().item_spacing.y);
            ScrollArea::vertical().id_salt((id_salt, "scroll")).max_height(max_height).auto_shrink([false, true]).show(&mut grid, |ui| {
                ui.spacing_mut().item_spacing.y = gap;
                let forward = direction::forward(ui);
                let mut at = 0;
                for group in &shown {
                    let heading =
                        RichText::new(group.name).font(tokens.font(craft_theme::TypeRole::Caption)).color(tokens.color.text.muted.to_egui());
                    ui.label(heading);
                    for row in rows_of(group.items.len(), columns) {
                        let (_, strip) = ui.allocate_space(vec2(columns as f32 * (cell + gap) - gap, cell));
                        for (column, item) in group.items[row].iter().enumerate() {
                            let x = direction::start_x(ui, strip) + forward * column as f32 * (cell + gap);
                            let rect =
                                Rect::from_min_max(pos2(x.min(x + forward * cell), strip.top()), pos2(x.max(x + forward * cell), strip.bottom()));
                            let cell_id = ids[at];
                            let mut cell = Interactive::with_id(cell_id, Role::GridCell, item.label.as_str())
                                .corner_radius(CornerRadius::same(CELL_RADIUS))
                                .state_layer(false)
                                .enabled(enabled);
                            if at != stop {
                                cell = cell.sense(Sense::CLICK);
                            }
                            let response = cell.interact_at(ui, rect).finish(ui);
                            if ui.is_rect_visible(rect) {
                                if response.hovered() && enabled {
                                    ui.painter().rect_filled(rect, CornerRadius::same(CELL_RADIUS), tokens.color.state.hover.to_egui());
                                }
                                let color = if enabled { tokens.color.text.normal } else { tokens.color.text.disabled }.to_egui();
                                paint(&ui.painter().with_clip_rect(rect), rect.shrink(inset), &item.key, color);
                            }
                            if moved && at == stop {
                                response.scroll_to_me(None);
                            }
                            if enabled && response.clicked() {
                                picked = Some(item.key.clone());
                            }
                            response.tooltip(item.label.as_str());
                            at += 1;
                        }
                    }
                }
            });
        }
        let response = ui.allocate_rect(grid.min_rect(), Sense::hover());
        IconPickerOutput { response, picked }
    }
}

/// Whether `item` matches `query`: the trimmed query is empty, or its label or a keyword holds
/// it, ignoring case.
// deckcraft: crates/ui-egui/src/ribbon/insert.rs:205 (q); pdfcraft: crates/ui-egui/src/panels/all_tools_ui.rs:29 matches
pub fn icon_matches<K>(item: &IconPickerItem<K>, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty() || item.label.to_lowercase().contains(&query) || item.keywords.iter().any(|k| k.to_lowercase().contains(&query))
}

/// A group's name and the items of it that match the query.
struct Shown<'a, K> {
    name: &'a str,
    items: Vec<&'a IconPickerItem<K>>,
}

/// The groups with at least one matching item.
fn visible<'a, K>(groups: &'a [IconPickerGroup<K>], query: &str) -> Vec<Shown<'a, K>> {
    groups
        .iter()
        .map(|g| Shown { name: &g.name, items: g.items.iter().filter(|item| icon_matches(item, query)).collect() })
        .filter(|g| !g.items.is_empty())
        .collect()
}

/// The ranges of a group's `len` items on each row of `columns`.
fn rows_of(len: usize, columns: usize) -> impl Iterator<Item = Range<usize>> {
    (0..len.div_ceil(columns)).map(move |row| row * columns..((row + 1) * columns).min(len))
}
