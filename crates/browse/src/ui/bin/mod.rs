//! `MediaBin` over the app's `BinModel`: breadcrumb, search, count, the List, Icon and Freeform
//! views, the footer.
//!
//! ```ignore
//! let out = MediaBin::new("project", &mut state.bin, &project)       // project: impl BinModel<ItemId>
//!     .selection(&mut state.selection)
//!     .thumbnails(&thumbs, &project_thumbs)
//!     .columns(&columns)
//!     .hover_scrub(prefs.hover_scrub)
//!     .header(|ui| preview_area(ui))
//!     .footer_actions(&[FooterAction::new("newItem", Icon::Plus, tr("New Item")).accepts_drops(true)])
//!     .show(ui);
//! for event in out.events {
//!     match event {
//!         BinEvent::Activate(id) => open_source(id),
//!         BinEvent::Rename { key, text } => cmd("project.rename", key, text),
//!         BinEvent::FooterDrop { button: "newItem", rows } => cmd("project.newSequence", rows),
//!         _ => {}
//!     }
//! }
//! ```

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod bin_tests;
mod footer;
mod freeform;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod freeform_tests;
mod icon;
mod list;
mod search;
#[cfg(test)]
mod search_tests;
pub(crate) mod stories;
mod style;

use std::collections::HashSet;
use std::hash::Hash;

use craft_theme::Rgba;
use craft_theme::TypeRole;
use craft_ui::behavior::{ListDrag, Move, Selection, SelectionChange};
use craft_ui::canvas::textures::{TextureKey, ThumbSource, Thumbnails};
use craft_ui::composites::{FooterAction, PanelFooter};
use craft_ui::controls::{BreadcrumbBar, Crumb, IconButton, SearchField};
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{ContextMenuRequest, OpenChange, SortOrder, TableColumn, TableEvent, TableLayout, ThumbSize};
use craft_ui::localized::{Labels as _, Words};
use craft_ui::theme::{Tokens, TokensOf as _, role_format};
use egui::text::LayoutJob;
use egui::{Align, AsIdSalt, IdSalt, Label, Layout, Ui, Vec2, vec2};

use super::grid::{TileBadge, TileStatus};

/// The Icon and Freeform views' card width before the user zooms, in points (FilmCraft's).
const DEFAULT_ICON_SIZE: f32 = 96.0;

/// The zoom slider's default range, in points (FilmCraft's 48–400).
const MIN_ICON_SIZE: f32 = 48.0;
const MAX_ICON_SIZE: f32 = 400.0;

/// Bins nested deeper than this are not followed, so a model that contains itself ends.
const MAX_DEPTH: usize = 64;

pub use freeform::FreeformCard;
pub use search::name_matches;
pub use style::BinStyle;

/// One child of a bin: a sub-bin or an item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BinEntry<K> {
    /// A bin.
    Bin(K),
    /// An item.
    Item(K),
}

/// What an item or bin shows.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct BinItem {
    /// Its name.
    pub name: String,
    /// The item's kind glyph (video, audio, sequence); bins draw `Icon::Folder` in `text.muted`.
    pub icon: Option<Icon>,
    /// The label chip's colour (the app's).
    pub label_color: Option<Rgba>,
    /// Corner badges in the Icon view; in the List view they follow the name.
    pub badges: Vec<TileBadge>,
    /// Offline or unreadable: the name in `status.bad.fg` with the glyph and text (never colour
    /// alone).
    pub status: TileStatus,
    /// The duration text at the end of an Icon card's caption.
    pub duration: Option<String>,
    /// In and Out as fractions of the thumbnail's width.
    pub range: Option<(f32, f32)>,
    /// Width over height of the picture (16:9 when `None`).
    pub aspect: Option<f32>,
}

/// The app's project, read only. Every call is cheap: the bin reads it for the rows it builds.
pub trait BinModel<K> {
    /// The bins and items in `parent` (`None`: the root), unsorted: the bin sorts by the layout's
    /// sort, bins before items; "User Order" is this order.
    fn children(&self, parent: Option<&K>, out: &mut Vec<BinEntry<K>>);
    /// Fills `item` for `key`.
    fn item(&self, key: &K, item: &mut BinItem);
    /// The text of `key`'s List-view `column` (any column but the name).
    fn cell(&self, key: &K, column: &str) -> String;
    /// Whether `key` is a bin.
    fn is_bin(&self, key: &K) -> bool;
    /// The bin holding `key`; `None` at the root.
    fn parent(&self, key: &K) -> Option<K>;
    /// Items in `bin`, sub-bins included (the count line and folded bins).
    fn count(&self, bin: &K) -> usize;
}

/// The bin's view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BinView {
    /// A table (FilmCraft's List view).
    #[default]
    List,
    /// Cards (`TileGrid` `Cards`, 16:9).
    Icon,
    /// Cards placed by the user.
    Freeform,
}

/// How the user asked to open a bin, from the double click's modifiers (FilmCraft's
/// `open_bin`): plain in place, Shift or Alt in a new tab, Cmd/Ctrl in a new window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpenHow {
    /// In this panel.
    InPlace,
    /// In a new tab (the host's).
    NewTab,
    /// In a floating window (the host's).
    NewWindow,
}

/// The bin's saved look: a `craft_host::Section` the host stores per bin panel.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BinSettings {
    /// The view.
    pub view: BinView,
    /// The card width in the Icon and Freeform views, in points (FilmCraft 48–400).
    pub icon_size: f32,
    /// List-view thumbnails.
    pub thumbnails: ThumbSize,
    /// List-view columns: order, hidden, widths, sort (column keys are the host's ids, `"name"`
    /// for the first column).
    pub columns: TableLayout<String>,
    /// The Icon view's sort (Sort Icons): a column key and whether reversed; `None` is User Order.
    pub icon_sort: Option<(String, bool)>,
}

impl Default for BinSettings {
    fn default() -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:132 (`view_of`: List view, icon size 96).
        Self { view: BinView::List, icon_size: DEFAULT_ICON_SIZE, thumbnails: ThumbSize::None, columns: TableLayout::new(&[]), icon_sort: None }
    }
}

impl craft_host::Section for BinSettings {
    const KEY: &'static str = "media_bin";
}

/// The bin's state between passes, kept by the host (one per panel or tab): what is shown, the
/// search, the open bins and the settings.
#[derive(Clone, Debug)]
pub struct BinState<K> {
    /// The bin shown; `None` is the root.
    pub shown: Option<K>,
    /// The search text.
    pub search: String,
    /// The List view's open bins.
    pub expanded: HashSet<K>,
    /// The saved look.
    pub settings: BinSettings,
    /// The icon size when the zoom gesture under way began, put back by its `Cancel`.
    zoom_before: Option<f32>,
}

impl<K: Eq + Hash> Default for BinState<K> {
    fn default() -> Self {
        // new: the root, no search, nothing open, default settings.
        Self { shown: None, search: String::new(), expanded: HashSet::new(), settings: BinSettings::default(), zoom_before: None }
    }
}

impl<K: Clone + Eq + Hash> BinState<K> {
    /// Applies the view-state events (`Navigate`, `Search`, `View`, `Zoom`, `Table`, and the
    /// List view's opens and closes) to the state; document events are left to the host.
    pub fn apply(&mut self, event: &BinEvent<K>) {
        // new: the host applies view state in one call, as `SectionLayout::apply`.
        match event {
            BinEvent::Navigate(to) => {
                self.shown.clone_from(to);
                self.search.clear();
            }
            BinEvent::OpenBin { key, how: OpenHow::InPlace } => {
                self.shown = Some(key.clone());
                self.search.clear();
            }
            BinEvent::Search(text) => self.search.clone_from(text),
            BinEvent::View(view) => self.settings.view = *view,
            BinEvent::Zoom(zoom) => self.zoom(zoom),
            BinEvent::Open(OpenChange { row, open: true, .. }) => {
                self.expanded.insert(row.clone());
            }
            BinEvent::Open(OpenChange { row, open: false, .. }) => {
                self.expanded.remove(row);
            }
            // The List view's table keeps its own layout and sort; the Icon and Freeform views'
            // Sort Icons menu reports through the same events.
            BinEvent::Table(TableEvent::Sort(sort)) if self.settings.view != BinView::List => {
                self.settings.icon_sort = Some((sort.column.clone(), sort.order == SortOrder::Descending));
            }
            BinEvent::Table(TableEvent::Unsorted) if self.settings.view != BinView::List => self.settings.icon_sort = None,
            _ => {}
        }
    }
}

impl<K> BinState<K> {
    /// The zoom slider's gesture: `Begin` takes the size to put back, updates and commits set it,
    /// `Cancel` restores it.
    fn zoom(&mut self, event: &EditEvent<f64>) {
        match event {
            EditEvent::Begin { .. } => self.zoom_before = Some(self.settings.icon_size),
            EditEvent::Update { value, .. } | EditEvent::Set { value, .. } => self.settings.icon_size = *value as f32,
            EditEvent::Commit { value, .. } => {
                self.settings.icon_size = *value as f32;
                self.zoom_before = None;
            }
            EditEvent::Cancel { .. } => {
                if let Some(before) = self.zoom_before.take() {
                    self.settings.icon_size = before;
                }
            }
            _ => {}
        }
    }
}

/// What the bin reported, in order; each pass's events are handled in that pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum BinEvent<K> {
    /// The selection changed; already applied.
    Selection(SelectionChange<K>),
    /// Enter or a double click on an item.
    Activate(K),
    /// Enter or a double click on a bin, with how.
    OpenBin { key: K, how: OpenHow },
    /// The breadcrumb, Up or a search result moved the shown bin.
    Navigate(Option<K>),
    /// A List-view bin opened or closed (→, ←, the triangle, a screen reader's Expand and
    /// Collapse). With Alt the bin reports one change per bin under the shown bin, each with
    /// `all` false, so [`BinState::apply`] needs no model.
    Open(OpenChange<K>),
    /// A rename committed (Enter, Tab, a click elsewhere) with non-empty text other than the
    /// name; a `Set`, one undo step. Esc cancels in the editor and reports nothing.
    Rename { key: K, text: String },
    /// Items dropped into a bin (`None`: the shown bin). A `Set`.
    Drop { items: Vec<K>, into: Option<K> },
    /// A reorder in a bin shown in User Order.
    Reorder(EditEvent<Move<K>>),
    /// Delete or Backspace.
    Remove(Vec<K>),
    /// A drag of the selection began.
    Drag(ListDrag<K>),
    /// Hover scrub over an item's picture.
    Scrub { key: K, fraction: f32 },
    /// The search text changed.
    Search(String),
    /// A view button.
    View(BinView),
    /// The zoom slider or keys: the new icon size.
    Zoom(EditEvent<f64>),
    /// The List view's table: sort or column changes.
    Table(TableEvent<K, String>),
    /// A click on a host footer action.
    Footer {
        /// The action's id.
        button: &'static str,
    },
    /// A drop on a host footer action that accepts drops (`PanelFooter::drop_button`).
    FooterDrop {
        /// The action's id.
        button: &'static str,
        /// The dragged rows.
        rows: Vec<K>,
    },
    /// The footer's New Bin.
    NewBin,
    /// A Freeform drag committed: the cards' new positions.
    FreeformMoved { keys: Vec<K>, to: Vec<Vec2> },
    /// A context menu on an item, a bin or the background (`row: None`).
    ContextMenu(ContextMenuRequest<K>),
}

/// What [`MediaBin::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the bin's events"]
pub struct BinOutput<K> {
    /// The panel's body.
    pub response: egui::Response,
    /// What happened this pass, in order.
    pub events: Vec<BinEvent<K>>,
}

craft_ui::words! {
    /// The bin's words, English by default.
    #[non_exhaustive]
    pub struct BinLabels {
        search: "browse.bin.search" = "Search";
        clear: "browse.bin.clear" = "Clear";
        views: "browse.bin.views" = "View";
        up: "browse.bin.up" = "Up One Level";
        root: "browse.bin.root" = "Project";
        /// `{selected}` of `{total}`.
        selected_of: "browse.bin.selectedOf" = "{selected} of {total} items selected";
        list_view: "browse.bin.listView" = "List View";
        icon_view: "browse.bin.iconView" = "Icon View";
        freeform_view: "browse.bin.freeformView" = "Freeform View";
        new_bin: "browse.bin.new" = "New Bin";
        sort_icons: "browse.bin.sortIcons" = "Sort Icons";
        user_order: "browse.bin.userOrder" = "User Order";
        no_matches: "browse.bin.noMatches" = "No items match the search";
        empty: "browse.bin.empty" = "This bin is empty";
    }
}

/// The bin's item count words: "{n} items".
pub const ITEMS: craft_ui::localized::words::Counted = craft_ui::localized::words::Counted::new("browse.bin.items", "{n} item", "{n} items");

/// A host slot drawn inside the widget.
type Slot<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The thumbnails a widget draws with: the cache and the app's source, keyed by the widget's key.
type ThumbPair<'a, K> = (&'a Thumbnails<K>, &'a dyn ThumbSource<K>);

/// The project or media panel (N8.02).
///
/// - **Header.** A `BreadcrumbBar` of the shown bin's ancestors (`BinModel::parent`, the root
///   named [`BinLabels::root`]; ancestors `text.normal`, the last `BodyStrong`) after an Up
///   `IconButton`; a `SearchField` that fills the row before the count; the item count in `Caption`
///   `text.faint` at the end ([`ITEMS`], or [`BinLabels::selected_of`] with a selection). Then
///   the host's `.header(..)` slot (FilmCraft's preview area).
/// - **Search.** Typing filters live by [`name_matches`]; a non-empty search lists matches from
///   every sub-bin, flat; Esc clears, a second Esc leaves the field (the library's rule).
/// - **List view** (`list.rs`): a tree `Table` (`Table::tree`, named by the row's name) over the
///   shown bin's rows, bins before items (every bin is a group, so an empty bin has its
///   disclosure triangle), open bins' children under them; the table draws the first column's
///   indent (`list.indent` per depth) and disclosure triangle, and the bin's cell closure draws
///   the label chip, thumbnail (`Table::thumbnails`), icon and the name through
///   `CellContext::name`; other columns `BinModel::cell` in `text.muted`; striped. Selection,
///   the double click, the context menu, the row drag and the rename are the table's. Enter or
///   double click: `Activate` or `OpenBin { how }`; → and ← open and close bins, Alt for every
///   level (the table's `TableEvent::Open`, reported as `Open`; one with `all` becomes one
///   `Open` per bin under the shown bin, read from `BinModel::children`, so `BinState::apply`
///   needs no model); F2 or a slow click on the selected row's name renames it (`Rename`);
///   Delete and Backspace report `Remove`; Cmd/Ctrl + and − and 0 report `Zoom(Set)` steps of
///   the footer slider's range; Cmd/Ctrl+] and [ move in User Order.
/// - **Icon view** (`icon.rs`): a `TileGrid` in `Cards { side: icon_size, aspect: 16 / 9,
///   caption: true }`, bins as cards before the items, hover scrub, In and Out bars.
/// - **Freeform view** (`freeform.rs`): cards at the host's [`FreeformCard`] positions.
/// - **Drag and drop.** Rows and cards drag the selection (`Drag`); a bin row or card is a drop
///   target (outline `list.drop`, `Drop { into: Some(bin) }`), the view background takes drops
///   into the shown bin (`into: None`). A drop reports nothing for items already in the target
///   bin, and none into a bin that is, or lies inside, one of the dragged items. In User Order
///   the List view's own rows also reorder (`Reorder`), by the table's targets; a drag from
///   another list over an item row is a drop on the background, never a reorder. A footer action
///   with `accepts_drops` reports `FooterDrop` through `PanelFooter::drop_button`.
/// - **Footer** (`footer.rs`): a `PanelFooter` with the view `Segmented` (List, Icon,
///   Freeform), a `ThumbnailSlider` over `.zoom_range(..)` (Esc during the drag restores), Sort
///   Icons in Icon view, the host's actions, New Bin.
/// - **Context menus.** `ContextMenu` for an item, a bin and the background.
/// - **AccessKit.** The List view is the tree table's `TreeGrid` of `TreeItem`s (`level`,
///   `expanded`); the Icon view as `TileGrid`; the search, slider and footer buttons named.
/// - **RTL.** Breadcrumb, columns and the footer mirror; the disclosure sits at the reading start.
pub struct MediaBin<'a, K: TextureKey, M: ?Sized> {
    id_salt: IdSalt,
    state: &'a mut BinState<K>,
    model: &'a M,
    selection: Option<&'a mut Selection<K>>,
    thumbnails: Option<ThumbPair<'a, K>>,
    columns: &'a [TableColumn<String>],
    hover_scrub: bool,
    header: Option<Slot<'a>>,
    actions: &'a [FooterAction],
    freeform: &'a [FreeformCard<K>],
    snap: Option<f32>,
    zoom_range: (f32, f32),
    labels: Option<BinLabels>,
    rename: Option<K>,
}

impl<'a, K, M> MediaBin<'a, K, M>
where
    K: TextureKey + std::fmt::Debug,
    M: BinModel<K> + ?Sized,
{
    /// A bin panel over `model`, its view state in `state`.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut BinState<K>, model: &'a M) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:276 (`show`).
        Self {
            id_salt: IdSalt::new(id_salt),
            state,
            model,
            selection: None,
            thumbnails: None,
            columns: &[],
            hover_scrub: false,
            header: None,
            actions: &[],
            freeform: &[],
            snap: None,
            zoom_range: (MIN_ICON_SIZE, MAX_ICON_SIZE),
            labels: None,
            rename: None,
        }
    }

    /// The selection the bin applies changes to.
    #[must_use]
    pub fn selection(self, selection: &'a mut Selection<K>) -> Self {
        // new: builder.
        Self { selection: Some(selection), ..self }
    }

    /// Thumbnails keyed by the item key.
    #[must_use]
    pub fn thumbnails(self, thumbnails: &'a Thumbnails<K>, source: &'a dyn ThumbSource<K>) -> Self {
        // new: builder.
        Self { thumbnails: Some((thumbnails, source)), ..self }
    }

    /// The List view's columns; the first is the name column (key `"name"`).
    #[must_use]
    pub fn columns(self, columns: &'a [TableColumn<String>]) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project_views.rs:497 (`list_view` columns from Metadata Display).
        Self { columns, ..self }
    }

    /// Hover scrub in the Icon view.
    #[must_use]
    pub fn hover_scrub(self, on: bool) -> Self {
        // new: builder.
        Self { hover_scrub: on, ..self }
    }

    /// The host's area above the content (a preview).
    #[must_use]
    pub fn header(self, header: impl FnOnce(&mut Ui) + 'a) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:510 (`preview_area`).
        Self { header: Some(Box::new(header)), ..self }
    }

    /// The host's footer buttons, after the view buttons and slider.
    #[must_use]
    pub fn footer_actions(self, actions: &'a [FooterAction]) -> Self {
        // new: builder.
        Self { actions, ..self }
    }

    /// The Freeform view's card positions.
    #[must_use]
    pub fn freeform(self, cards: &'a [FreeformCard<K>]) -> Self {
        // new: builder.
        Self { freeform: cards, ..self }
    }

    /// Freeform snapping to a grid of `step` points.
    #[must_use]
    pub fn snap(self, step: Option<f32>) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project_views.rs:1014 (grid snap lines).
        Self { snap: step, ..self }
    }

    /// The zoom slider's range in points (default 48–400, FilmCraft's).
    #[must_use]
    pub fn zoom_range(self, min: f32, max: f32) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:555 (zoom 48–400).
        Self { zoom_range: (min, max), ..self }
    }

    /// The words.
    /// Opens the in-place rename of `key` in this pass, as F2 would: in the List view the name
    /// cell's field (`Table::start_rename`), in the Icon view the card's caption
    /// (`TileGrid::start_rename`). In the Freeform view it is the caption's field as in the Icon view. Call it
    /// in the pass that handles the host's Rename command; ignored when `key` is not shown.
    #[must_use]
    pub fn start_rename(self, key: K) -> Self {
        // new: builder, as every rename-capable component has.
        Self { rename: Some(key), ..self }
    }

    #[must_use]
    pub fn labels(self, labels: BinLabels) -> Self {
        // new: builder.
        Self { labels: Some(labels), ..self }
    }

    /// Draws the panel and reports what happened.
    pub fn show(self, ui: &mut Ui) -> BinOutput<K> {
        // filmcraft: crates/ui-egui/src/panels/project.rs:389 (`body`: breadcrumb, search, count, preview, view, footer).
        let Self { id_salt, state, model, selection, thumbnails, columns, hover_scrub, header, actions, freeform, snap, zoom_range, labels, rename } =
            self;
        let id = ui.make_persistent_id(id_salt);
        let labels = labels.unwrap_or_else(|| BinLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let mut events: Vec<BinEvent<K>> = Vec::new();

        // The selection is the host's; without one the bin keeps its own.
        let mut own: Selection<K> = ui.data(|d| d.get_temp(id.with("own-selection"))).unwrap_or_default();
        let hosted = selection.is_some();
        let selection: &mut Selection<K> = selection.unwrap_or(&mut own);

        // Breadcrumb row: Up, then the shown bin's ancestors.
        let mut chain: Vec<Option<K>> = vec![state.shown.clone()];
        while let Some(parent) = chain.last().cloned().flatten().map(|key| model.parent(&key)) {
            if chain.len() > MAX_DEPTH {
                break;
            }
            chain.push(parent);
        }
        let crumbs: Vec<Crumb<Option<K>>> = chain
            .into_iter()
            .rev()
            .map(|key| {
                let label = match &key {
                    Some(key) => {
                        let mut item = BinItem::default();
                        model.item(key, &mut item);
                        item.name
                    }
                    None => labels.root.clone(),
                };
                Crumb { label, key }
            })
            .collect();
        let shown = state.shown.clone();
        direction::horizontal(ui, |ui| {
            let up = ui.add(IconButton::new(Icon::ChevronUp, labels.up.as_str()).enabled(shown.is_some()).id_salt((id, "up")));
            if up.clicked() {
                events.push(BinEvent::Navigate(shown.as_ref().and_then(|key| model.parent(key))));
            }
            if let Some(target) = BreadcrumbBar::new((id, "crumbs"), &crumbs).show(ui).opened {
                events.push(BinEvent::Navigate(target));
            }
        });

        // Search row: the field, then the item count at the end.
        let searching = !state.search.trim().is_empty();
        let mut found = Vec::new();
        if searching {
            search::collect_matches(model, state.shown.as_ref(), &state.search, 0, &mut found);
        }
        let total = match (&state.shown, searching) {
            (_, true) => found.len(),
            (Some(bin), false) => model.count(bin),
            (None, false) => {
                let mut children = Vec::new();
                model.children(None, &mut children);
                children
                    .iter()
                    .map(|entry| match entry {
                        BinEntry::Bin(key) => model.count(key),
                        BinEntry::Item(_) => 1,
                    })
                    .sum()
            }
        };
        let words = Words::of(ui);
        // Only the selected keys the bin shows count: the rest sit in another bin or are not matches.
        let selected =
            selection.items.iter().filter(|key| if searching { found.contains(key) } else { list::within(model, key, state.shown.as_ref()) }).count();
        let count_text = if selected == 0 {
            words.counted(ui.ctx(), ITEMS, &words.get(ITEMS.word), total as u64)
        } else {
            craft_i18n::fmt(&labels.selected_of, &[("selected", &selected.to_string()), ("total", &total.to_string())])
        };
        ui.allocate_ui_with_layout(vec2(ui.available_width(), ui.spacing().interact_size.y), direction::trailing(ui), |ui| {
            let format = role_format(&tokens, TypeRole::Caption, style.count);
            ui.add(Label::new(LayoutJob::single_section(count_text, format)).selectable(false));
            let field = SearchField::new((id, "search"), labels.search.as_str(), &state.search, labels.clear.as_str()).show(ui);
            if let Some(
                EditEvent::Begin { value: Some(value), .. }
                | EditEvent::Update { value, .. }
                | EditEvent::Set { value, .. }
                | EditEvent::Commit { value, .. },
            ) = field.event
            {
                events.push(BinEvent::Search(value));
            }
        });
        if let Some(header) = header {
            header(ui);
        }

        // The body, then the footer at the bottom.
        let footer_style = PanelFooter::style(&tokens, ui.pixels_per_point());
        let body_height = (ui.available_height() - PanelFooter::reserved_height(&footer_style) - ui.spacing().item_spacing.y).max(0.0);
        let view = state.settings.view;
        let body = ui.allocate_ui_with_layout(vec2(ui.available_width(), body_height), Layout::top_down(Align::Min), |ui| match view {
            BinView::List => {
                let input = list::ListInput {
                    id,
                    model,
                    state: &mut *state,
                    selection: &mut *selection,
                    thumbnails,
                    columns,
                    labels: &labels,
                    zoom_range,
                    default_size: DEFAULT_ICON_SIZE,
                    rename: rename.clone(),
                };
                let (response, mut list_events) = list::list_view(ui, input);
                events.append(&mut list_events);
                Some(response)
            }
            BinView::Icon => {
                let input = icon::IconInput {
                    id,
                    model,
                    state: &*state,
                    selection: &mut *selection,
                    thumbnails,
                    hover_scrub,
                    zoom_range,
                    default_size: DEFAULT_ICON_SIZE,
                    labels: &labels,
                    rename,
                };
                let (response, mut icon_events) = icon::icon_view(ui, input);
                events.append(&mut icon_events);
                Some(response)
            }
            BinView::Freeform => {
                let input = freeform::FreeformInput {
                    id,
                    model,
                    state: &*state,
                    selection: &mut *selection,
                    thumbnails,
                    cards: freeform,
                    snap,
                    zoom_range,
                    default_size: DEFAULT_ICON_SIZE,
                    labels: &labels,
                    rename,
                };
                let (response, mut freeform_events) = freeform::freeform_view(ui, input);
                events.append(&mut freeform_events);
                Some(response)
            }
        });
        let response = body.inner.unwrap_or(body.response);
        let input = footer::FooterInput {
            id,
            view,
            icon_size: state.settings.icon_size,
            zoom_range,
            default_size: DEFAULT_ICON_SIZE,
            actions,
            columns,
            icon_sort: state.settings.icon_sort.as_ref(),
            labels: &labels,
        };
        events.extend(footer::footer_events(ui, &input));
        if !hosted {
            ui.data_mut(|d| d.insert_temp(id.with("own-selection"), own));
        }
        BinOutput { response, events }
    }
}
