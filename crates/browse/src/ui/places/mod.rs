//! `PlacesTree`: the folder tree with lazy children, reveal, the keep, hide and reveal menu, and
//! drops.
//!
//! ```ignore
//! let out = PlacesTree::<PhotoId>::new("places", &places, &mut folder_cache)
//!     .current(browsing.as_deref())
//!     .extras(&[PlaceExtra::new("folder.rename", tr("Rename Folder…"))])
//!     .accepts(|path, drag| drag.items.iter().all(|id| catalog.can_move_into(*id, path)))
//!     .show(ui);
//! for event in out.events {
//!     match event {
//!         PlacesEvent::Open(path) => app.browse(path),
//!         PlacesEvent::Dropped { place, items } => app.move_into(place, items.items),
//!         other => settings.places.apply(&other),            // keep, remove, restore, recents
//!     }
//! }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod places_tests;
pub(crate) mod stories;
mod style;

use std::path::{Path, PathBuf};

use craft_ui::behavior::ListDrag;
use egui::{AsIdSalt, Response, Ui};

use crate::places::{FolderCache, Places, PlacesSettings};

pub use style::PlacesStyle;

/// A host item in a folder row's menu, after the library's (LightCraft: Rename Folder…, Move
/// Folder To…, the library actions).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceExtra {
    /// The id reported in `PlacesEvent::Extra`.
    pub id: &'static str,
    /// The item's text, translated.
    pub label: String,
    /// Enabled.
    pub enabled: bool,
}

impl PlaceExtra {
    /// An enabled item.
    pub fn new(id: &'static str, label: impl Into<String>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/folders.rs:56 (the folder menu: Keep in Local, Remove from Local).
        todo!()
    }
}

/// What the tree reported, in order.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PlacesEvent<K> {
    /// A click or Enter on a row: browse `path`.
    Open(PathBuf),
    /// Keep in Places / Add to Favorites on a folder that is not a root.
    Keep(PathBuf),
    /// Remove from Places on a root (the folder stays).
    Remove(PathBuf),
    /// One hidden root restored.
    Restore(PathBuf),
    /// Restore All.
    RestoreAll,
    /// Clear Recent Directories.
    ClearRecent,
    /// A host menu item on `path`.
    Extra { id: &'static str, path: PathBuf },
    /// A `ListDrag<K>` released over a row `.accepts(..)` took.
    Dropped { place: PathBuf, items: ListDrag<K> },
}

impl PlacesSettings {
    /// Applies `Keep`, `Remove`, `Restore`, `RestoreAll` and `ClearRecent`; ignores the rest.
    /// Returns whether the settings changed (the host saves them).
    pub fn apply<K>(&mut self, event: &PlacesEvent<K>) -> bool {
        // new: the host applies the menu's settings events in one call.
        todo!()
    }
}

/// What [`PlacesTree::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the tree's events"]
pub struct PlacesOutput<K> {
    /// The tree.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<PlacesEvent<K>>,
}

craft_ui::words! {
    /// The tree's words, English by default.
    #[non_exhaustive]
    pub struct PlacesLabels {
        favorites: "browse.placesFavorites" = "Favorites";
        places: "browse.places" = "Places";
        volumes: "browse.placesVolumes" = "Volumes";
        network: "browse.placesNetwork" = "Network";
        recent: "browse.placesRecent" = "Recent Folders";
        none: "browse.placesNone" = "(none)";
        home: "browse.placesHome" = "Home";
        desktop: "browse.placesDesktop" = "Desktop";
        documents: "browse.placesDocuments" = "Documents";
        downloads: "browse.placesDownloads" = "Downloads";
        pictures: "browse.placesPictures" = "Pictures";
        movies: "browse.placesMovies" = "Movies";
        music: "browse.placesMusic" = "Music";
        keep: "browse.placesKeep" = "Keep in Places";
        add_favorite: "browse.placesAddFavorite" = "Add to Favorites";
        remove: "browse.placesRemove" = "Remove from Places";
        /// macOS; Windows and Linux use `reveal_other`.
        reveal_mac: "browse.placesRevealMac" = "Show in Finder";
        reveal_other: "browse.placesRevealOther" = "Show in File Manager";
        clear_recent: "browse.placesClearRecent" = "Clear Recent Folders";
        restore_all: "browse.placesRestoreAll" = "Restore All";
        loading: "browse.placesLoading" = "Loading…";
    }
}

/// The hidden-places row: "Show {n} hidden places".
pub const HIDDEN_PLACES: craft_ui::localized::words::Counted =
    craft_ui::localized::words::Counted::new("browse.placesHidden", "Show {n} hidden place", "Show {n} hidden places");

/// Which rows take a drag: the host's `.accepts(..)`.
type Accepts<'a, K> = Box<dyn Fn(&Path, &ListDrag<K>) -> bool + 'a>;

/// The folder tree (N8.06): a `List` tree of `TreeRow`s (library row height and indent).
///
/// - **Rows.** Group headings are `SectionLabel`s in `text.faint` (the `PlacesLabels` word of
///   each `PlaceGroupKind`); an empty group shows `none`. Root rows show the place's name (the
///   `PlacesLabels` word for a standard folder), folder rows their name; glyph `Icon::Folder` in
///   `text.muted`; selected (the `current` folder's row) is the list's neutral fill.
/// - **Children.** A row's children come from `FolderCache::subfolders` when it opens: `Loading`
///   shows one `loading` row in `text.faint`; `Failed` shows nothing and the triangle goes;
///   depth stops at 8 levels. When the window regains focus (`ViewportInfo::focused` turns true)
///   the tree calls `FolderCache::refresh`; opening a row calls `refresh_folder` (risk 8: folders
///   are not watched). At the end of the pass `FolderCache::end_pass`.
/// - **Click.** A click or Enter reports `Open(path)`; the triangle and a double click open and
///   close (the `List::tree` keys: → opens, ← closes then goes to the parent, Alt for every
///   level).
/// - **Reveal.** When `current` changes, the rows on the way down to it in the owner root
///   (`Places::owner`) open once, and the row scrolls into view; collapsing afterwards sticks.
///   A copy of one folder in two roots opens independently (open state is per root and path).
/// - **Menu** (secondary click, Menu key, Shift+F10): on a folder that is not a root `keep`
///   (or `add_favorite` when the host shows a Favorites group); on a root `remove`; the reveal
///   item (desktop only: `craft_platform::open::reveal`, run by the tree); `clear_recent` in the
///   Recent group; then the host's `PlaceExtra` items. Hidden roots come back through a
///   [`HIDDEN_PLACES`] row at the end: with one hidden root a click restores it, with several it
///   opens a menu of them and `restore_all`.
/// - **Drop.** While egui's payload is a `ListDrag<K>` over a row that `.accepts(path, drag)`
///   takes, the row shows the list's drop outline (`paint_drop_mark`, `DropMark::Into`); a release
///   reports `Dropped`.
/// - **Keys and focus.** One Tab stop; the list keys; Enter opens.
/// - **AccessKit.** `Tree` with `TreeItem`s (`level`, `expanded`); headings are `Heading`s.
/// - **RTL.** Indent and triangle mirror.
pub struct PlacesTree<'a, K> {
    id_salt: egui::Id,
    places: &'a Places,
    cache: &'a mut FolderCache,
    current: Option<&'a Path>,
    favorites_group: bool,
    extras: &'a [PlaceExtra],
    accepts: Option<Accepts<'a, K>>,
    labels: Option<PlacesLabels>,
}

impl<'a, K: Clone + Send + Sync + 'static> PlacesTree<'a, K> {
    /// The tree of `places`, listing children through `cache`.
    pub fn new(id_salt: impl AsIdSalt, places: &'a Places, cache: &'a mut FolderCache) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/folders.rs:14 (`folder_tree`).
        todo!()
    }

    /// The folder being browsed: its row is selected and revealed when it changes.
    #[must_use]
    pub fn current(self, current: Option<&'a Path>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/folders.rs:280 (`reveal_chosen`).
        todo!()
    }

    /// The host's menu items for folder rows.
    #[must_use]
    pub fn extras(self, extras: &'a [PlaceExtra]) -> Self {
        // new: builder.
        todo!()
    }

    /// Which rows take a drag of `ListDrag<K>`.
    #[must_use]
    pub fn accepts(self, accepts: impl Fn(&Path, &ListDrag<K>) -> bool + 'a) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/folders.rs:376 (`folder_drop_target`).
        todo!()
    }

    /// The words.
    #[must_use]
    pub fn labels(self, labels: PlacesLabels) -> Self {
        // new: builder.
        todo!()
    }

    /// Draws the tree and reports what happened.
    pub fn show(self, ui: &mut Ui) -> PlacesOutput<K> {
        // lightcraft: crates/ui-egui/src/panels/left/folders.rs:312 (`folder_rows`); filmcraft: crates/ui-egui/src/panels/media_browser.rs:262 (`tree`: headings, "(none)").
        todo!()
    }
}
