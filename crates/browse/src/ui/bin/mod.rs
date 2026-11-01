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
//!     .footer_actions(&[BinFooterAction::new("newItem", Icon::Plus, tr("New Item")).accepts_drops(true)])
//!     .show(ui);
//! for event in out.events {
//!     match event {
//!         BinEvent::Open(id) => open_source(id),
//!         BinEvent::Rename { key, name } => cmd("project.rename", key, name),
//!         BinEvent::FooterDrop { action: "newItem", items } => cmd("project.newSequence", items),
//!         _ => {}
//!     }
//! }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod bin_tests;
mod footer;
mod freeform;
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
use craft_ui::behavior::{ListDrag, Move, Selection, SelectionChange};
use craft_ui::canvas::textures::{TextureKey, ThumbSource, Thumbnails};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{ContextMenuRequest, TableColumn, TableEvent, TableLayout, ThumbSize};
use egui::{AsIdSalt, Ui, Vec2};

use super::grid::{TileBadge, TileStatus};

pub use footer::BinFooterAction;
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
        todo!()
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
}

impl<K: Eq + Hash> Default for BinState<K> {
    fn default() -> Self {
        // new: the root, no search, nothing open, default settings.
        todo!()
    }
}

impl<K: Clone + Eq + Hash> BinState<K> {
    /// Applies the view-state events (`Navigate`, `Search`, `View`, `Zoom`, `Table`, and the
    /// List view's opens and closes) to the state; document events are left to the host.
    pub fn apply(&mut self, event: &BinEvent<K>) {
        // new: the host applies view state in one call, as `SectionLayout::apply`.
        todo!()
    }
}

/// What the bin reported, in order; each pass's events are handled in that pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum BinEvent<K> {
    /// The selection changed; already applied.
    Selection(SelectionChange<K>),
    /// Enter or a double click on an item.
    Open(K),
    /// Enter or a double click on a bin, with how.
    OpenBin { key: K, how: OpenHow },
    /// The breadcrumb, Up or a search result moved the shown bin.
    Navigate(Option<K>),
    /// A List-view bin opened or closed (→, ←, the triangle, a screen reader's Expand and
    /// Collapse; with Alt, one per bin under the shown bin).
    Expanded { key: K, open: bool },
    /// A rename committed (Enter, a click elsewhere); a `Set`, one undo step. Esc cancels in the
    /// editor and reports nothing.
    Rename { key: K, name: String },
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
    /// A click on a host footer action (its id).
    Footer(&'static str),
    /// A drop on a host footer action that accepts drops (`PanelFooter::drop_target`).
    FooterDrop { action: &'static str, items: Vec<K> },
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
        search: "browse.binSearch" = "Search";
        up: "browse.binUp" = "Up One Level";
        root: "browse.binRoot" = "Project";
        /// `{selected}` of `{total}`.
        selected_of: "browse.binSelectedOf" = "{selected} of {total} items selected";
        list_view: "browse.binListView" = "List View";
        icon_view: "browse.binIconView" = "Icon View";
        freeform_view: "browse.binFreeformView" = "Freeform View";
        new_bin: "browse.binNew" = "New Bin";
        sort_icons: "browse.binSortIcons" = "Sort Icons";
        user_order: "browse.binUserOrder" = "User Order";
        rename: "browse.binRename" = "Rename";
        no_matches: "browse.binNoMatches" = "No items match the search";
        empty: "browse.binEmpty" = "This bin is empty";
    }
}

/// The bin's item count words: "{n} items".
pub const ITEMS: craft_ui::localized::words::Counted = craft_ui::localized::words::Counted::new("browse.binItems", "{n} item", "{n} items");

/// A host slot drawn inside the widget.
type Slot<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The thumbnails a widget draws with: the cache and the app's source, keyed by the widget's key.
type ThumbPair<'a, K> = (&'a Thumbnails<K>, &'a dyn ThumbSource<K>);

/// The project or media panel (N8.02).
///
/// - **Header.** A `BreadcrumbBar` of the shown bin's ancestors (`BinModel::parent`, the root
///   named [`BinLabels::root`]; ancestors `text.normal`, the last `BodyStrong`) after an Up
///   `IconButton`; a `SearchField` (45 % of the width, 80–240); the item count in `Caption`
///   `text.faint` at the end ([`ITEMS`], or [`BinLabels::selected_of`] with a selection). Then
///   the host's `.header(..)` slot (FilmCraft's preview area).
/// - **Search.** Typing filters live by [`name_matches`]; a non-empty search lists matches from
///   every sub-bin, flat; Esc clears, a second Esc leaves the field (the library's rule).
/// - **List view** (`list.rs`): a tree `Table` (`Table::tree`, named by the row's name) over the
///   shown bin's rows, bins before items, open bins' children under them; the table draws the
///   first column's indent (`list.indent` per depth) and disclosure triangle, and the bin's cell
///   closure draws the label chip 12 pt (radius 2.5), thumbnail (`ThumbSize`), icon and name;
///   other columns `BinModel::cell` in `text.muted`; striped. Click, Shift range, Cmd toggle as
///   the grid; Enter or double click: `Open` or `OpenBin { how }`; → and ← open and close bins,
///   Alt for every level (the table's `TableEvent::Open`, reported as `Expanded`; one with `all`
///   becomes one `Expanded` per bin under the shown bin, read from `BinModel::children`, so
///   `BinState::apply` needs no model); F2 renames the lead; a click on a selected item's name
///   starts a rename after the double-click time (FilmCraft's slow click); Cmd/Ctrl+] and [ move
///   in User Order.
/// - **Icon view** (`icon.rs`): a `TileGrid` in `Cards { side: icon_size, aspect: 16 / 9,
///   caption: true }`, bins as cards before the items, hover scrub, In and Out bars.
/// - **Freeform view** (`freeform.rs`): cards at the host's [`FreeformCard`] positions.
/// - **Drag and drop.** Rows and cards drag the selection (`Drag`); a bin row or card is a drop
///   target (outline `list.drop`, `Drop { into: Some(bin) }`), the view background takes drops
///   into the shown bin (`into: None`); a footer action with `accepts_drops` reports
///   `FooterDrop` through `PanelFooter::drop_target::<ListDrag<K>>`.
/// - **Footer** (`footer.rs`): a `PanelFooter` with the view `Segmented` (List, Icon,
///   Freeform), a `ThumbnailSlider` over `.zoom_range(..)` (Esc during the drag restores), Sort
///   Icons in Icon view, the host's actions, New Bin.
/// - **Context menus.** `ContextMenu` for an item, a bin and the background.
/// - **AccessKit.** The List view is the tree table's `TreeGrid` of `TreeItem`s (`level`,
///   `expanded`); the Icon view as `TileGrid`; the search, slider and footer buttons named.
/// - **RTL.** Breadcrumb, columns and the footer mirror; the disclosure sits at the reading start.
///
/// Risk 7: `Table`'s cell closure gets a `Ui` clipped to the cell inside the row's `Ui`, whose
/// click sense is registered before the cells, so the chip, thumbnail, icon and name draw in the
/// first cell after the table's indent and triangle. The tree's keys (→ and ← open and close, Alt
/// for every level) and the `TreeItem` role with `level` and `expanded` are `Table::tree`'s
/// (`crates/ui/src/list/table.rs`, a shared-additions task B11 depends on).
pub struct MediaBin<'a, K: TextureKey, M: ?Sized> {
    id_salt: egui::Id,
    state: &'a mut BinState<K>,
    model: &'a M,
    selection: Option<&'a mut Selection<K>>,
    thumbnails: Option<ThumbPair<'a, K>>,
    columns: &'a [TableColumn<String>],
    hover_scrub: bool,
    header: Option<Slot<'a>>,
    actions: &'a [BinFooterAction],
    freeform: &'a [FreeformCard<K>],
    snap: Option<f32>,
    zoom_range: (f32, f32),
    labels: Option<BinLabels>,
}

impl<'a, K, M> MediaBin<'a, K, M>
where
    K: TextureKey + std::fmt::Debug,
    M: BinModel<K> + ?Sized,
{
    /// A bin panel over `model`, its view state in `state`.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut BinState<K>, model: &'a M) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:276 (`show`).
        todo!()
    }

    /// The selection the bin applies changes to.
    #[must_use]
    pub fn selection(self, selection: &'a mut Selection<K>) -> Self {
        // new: builder.
        todo!()
    }

    /// Thumbnails keyed by the item key.
    #[must_use]
    pub fn thumbnails(self, thumbnails: &'a Thumbnails<K>, source: &'a dyn ThumbSource<K>) -> Self {
        // new: builder.
        todo!()
    }

    /// The List view's columns; the first is the name column (key `"name"`).
    #[must_use]
    pub fn columns(self, columns: &'a [TableColumn<String>]) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project_views.rs:497 (`list_view` columns from Metadata Display).
        todo!()
    }

    /// Hover scrub in the Icon view.
    #[must_use]
    pub fn hover_scrub(self, on: bool) -> Self {
        // new: builder.
        todo!()
    }

    /// The host's area above the content (a preview).
    #[must_use]
    pub fn header(self, header: impl FnOnce(&mut Ui) + 'a) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:510 (`preview_area`).
        todo!()
    }

    /// The host's footer buttons, after the view buttons and slider.
    #[must_use]
    pub fn footer_actions(self, actions: &'a [BinFooterAction]) -> Self {
        // new: builder.
        todo!()
    }

    /// The Freeform view's card positions.
    #[must_use]
    pub fn freeform(self, cards: &'a [FreeformCard<K>]) -> Self {
        // new: builder.
        todo!()
    }

    /// Freeform snapping to a grid of `step` points.
    #[must_use]
    pub fn snap(self, step: Option<f32>) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project_views.rs:1014 (grid snap lines).
        todo!()
    }

    /// The zoom slider's range in points (default 48–400, FilmCraft's).
    #[must_use]
    pub fn zoom_range(self, min: f32, max: f32) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:555 (zoom 48–400).
        todo!()
    }

    /// The words.
    #[must_use]
    pub fn labels(self, labels: BinLabels) -> Self {
        // new: builder.
        todo!()
    }

    /// Draws the panel and reports what happened.
    pub fn show(self, ui: &mut Ui) -> BinOutput<K> {
        // filmcraft: crates/ui-egui/src/panels/project.rs:389 (`body`: breadcrumb, search, count, preview, view, footer).
        todo!()
    }
}
