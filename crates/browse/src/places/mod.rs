//! The roots of a folder tree and lazy folder listings (`Place`, `Places`, `PlacesSettings`,
//! `Directories`, `SystemDirectories`, `FolderCache`, `same_folder`, `folder_within`). Folder
//! access goes only through [`Directories`]; [`SystemDirectories`] is desktop-only.
//!
//! ```ignore
//! // Each pass: the roots from the cache, resolved against the settings and the browsed folder.
//! let roots = cache.roots();                                   // listed in a job, None at first
//! let places = Places::resolve(&PlacesInput {
//!     standard: roots.map_or(&[][..], |r| &r.standard), volumes: roots.map_or(&[][..], |r| &r.volumes),
//!     settings: &settings.places, browsing: browsing.as_deref(),
//!     unlisted: browsing.as_deref().and_then(|b| cache.unlisted(b)).unwrap_or(&[]),
//!     groups: &[PlaceGroupKind::Places, PlaceGroupKind::Volumes],
//! });
//! settings.places.session_root = places.session_root.clone();  // kept for the session only
//! ```

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

mod cache;
mod directories;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use cache::{FolderCache, Listing, Roots};
#[cfg(not(target_arch = "wasm32"))]
pub use directories::SystemDirectories;
pub use directories::{Directories, Folder, StandardFolder, Volume};

/// What kind of root a place is; the tree's icon, name and menu follow from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    /// A standard folder: the tree names it with `PlacesLabels` (Home, not the user's folder name).
    Standard(StandardFolder),
    /// A folder the user kept (Keep in Places, Add to Favorites).
    Favorite,
    /// A mounted volume or drive.
    Volume,
    /// A network volume.
    Network,
    /// A recently browsed folder.
    Recent,
    /// The browsed folder, listed for this session because no root holds it.
    Session,
}

/// One root of the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// The folder, as the platform spells it.
    pub path: PathBuf,
    /// Its name: the folder's or volume's own; the tree shows the `PlacesLabels` word instead for
    /// [`PlaceKind::Standard`].
    pub name: String,
    /// What it is.
    pub kind: PlaceKind,
}

/// The headings of the tree. The host lists the ones it shows, in order (lead call 24): FilmCraft
/// shows Favorites, Volumes, Network, Recent; LightCraft shows Places only, with its kept folders
/// in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlaceGroupKind {
    /// Kept folders. Without this group they join `Places`, after the standard folders.
    Favorites,
    /// Standard folders (and, without `Favorites`, the kept ones), then the session root.
    Places,
    /// Local volumes and drives.
    Volumes,
    /// Network volumes. Without this group they join `Volumes`.
    Network,
    /// Recent folders, newest first.
    Recent,
}

/// One heading and its roots, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceGroup {
    /// The heading.
    pub kind: PlaceGroupKind,
    /// Its roots; an empty group shows "(none)".
    pub places: Vec<Place>,
}

/// What [`Places::resolve`] reads.
#[derive(Clone, Copy, Debug)]
pub struct PlacesInput<'a> {
    /// The standard folders that exist, from [`FolderCache::roots`].
    pub standard: &'a [(StandardFolder, PathBuf)],
    /// The volumes, from [`FolderCache::roots`].
    pub volumes: &'a [Volume],
    /// The saved favourites, hidden roots and recents, and the session root.
    pub settings: &'a PlacesSettings,
    /// The folder being browsed, if any.
    pub browsing: Option<&'a Path>,
    /// The hidden folders on the way to `browsing` (dot folders, hidden attribute), from
    /// [`FolderCache::unlisted`]; empty while unknown.
    pub unlisted: &'a [PathBuf],
    /// The groups the host shows, in order.
    pub groups: &'a [PlaceGroupKind],
}

/// The tree's roots and where the browsed folder sits among them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Places {
    /// The groups, in the host's order.
    pub groups: Vec<PlaceGroup>,
    /// The root the browsed folder lies in, as (group, place): the innermost one by path length.
    /// Its tree opens on the way down to the folder. `None` when the way passes through a hidden
    /// folder, or nothing is browsed.
    pub owner: Option<(usize, usize)>,
    /// The session root listed for the browsed folder (the previous one while browsing stays
    /// below it, else the browsed folder); the host stores it in
    /// [`PlacesSettings::session_root`] for the next pass.
    pub session_root: Option<PathBuf>,
    /// The hidden roots, for the `Show N hidden places` row.
    pub hidden: Vec<Place>,
}

impl Places {
    /// The roots, a pure port of LightCraft's `local_places_with_unlisted` with its rules and
    /// tests: the standard folders, then the favourites (in the Favorites group when listed, else
    /// in Places), each folder once however it is spelled ([`same_folder`], first spelling wins),
    /// hidden ones left out (they go to `hidden`, keeping their standard kind so the row reads
    /// "Home"); volumes in Volumes (network ones in Network when listed); recents in Recent. Then,
    /// only when no root holds the browsed folder ([`folder_within`], and no `unlisted` folder
    /// lies between the root and it), a session root at the end of Places: the settings' session
    /// root while browsing stays below it, else the browsed folder itself; never a hidden one.
    /// `owner` is the innermost root holding the browsed folder whose way down passes no hidden
    /// root.
    pub fn resolve(input: &PlacesInput<'_>) -> Places {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:156 (`local_places_with_unlisted`).
        todo!()
    }

    /// The place at (group, place).
    pub fn get(&self, at: (usize, usize)) -> Option<&Place> {
        // new: a lookup for the tree and the host.
        todo!()
    }
}

/// The places a user keeps, hides and recently visited: a `craft_host::Section` (`KEY`
/// `"places"`), saved by the host. Reading is lenient (serde defaults; an unreadable entry is
/// dropped).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PlacesSettings {
    /// Kept folders, in the order kept.
    pub favorites: Vec<PathBuf>,
    /// Roots the user removed (standard folders, volumes); they come back through the hidden row.
    pub hidden: Vec<PathBuf>,
    /// Recent folders, newest first, at most [`PlacesSettings::RECENT_MAX`].
    pub recent: Vec<PathBuf>,
    /// The session root of the last [`Places::resolve`]; not saved.
    #[serde(skip)]
    pub session_root: Option<PathBuf>,
}

impl craft_host::Section for PlacesSettings {
    const KEY: &'static str = "places";
}

impl PlacesSettings {
    /// Recent folders kept: 10 (FilmCraft's `RECENT_MAX`).
    pub const RECENT_MAX: usize = 10;

    /// Keep in Places / Add to Favorites: appends `folder` unless an equal one ([`same_folder`])
    /// is kept; also takes it out of `hidden`. Returns whether anything changed.
    pub fn keep(&mut self, folder: &Path) -> bool {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:156 (`saved` roots).
        todo!()
    }

    /// Remove from Places: a favourite is dropped from `favorites`; any other root is added to
    /// `hidden`. The folder itself is never touched. Returns whether anything changed.
    pub fn remove(&mut self, folder: &Path) -> bool {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:97 (`hidden_menu`, "Remove from Local").
        todo!()
    }

    /// Takes `folder` out of `hidden`.
    pub fn restore(&mut self, folder: &Path) -> bool {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:97 (`hidden_menu`: one at a time, or all).
        todo!()
    }

    /// Empties `hidden`.
    pub fn restore_all(&mut self) -> bool {
        // lightcraft: crates/ui-egui/src/panels/left/local.rs:97 (`hidden_menu`, Restore All).
        todo!()
    }

    /// Puts `folder` first in `recent`, dropping an equal entry and anything past `RECENT_MAX`.
    pub fn visited(&mut self, folder: &Path) {
        // filmcraft: crates/engine/src/media_browser.rs:80 (`RECENT_MAX` 10, newest first).
        todo!()
    }

    /// Clear Recent Directories.
    pub fn clear_recent(&mut self) -> bool {
        // filmcraft: crates/ui-egui/src/panels/media_browser.rs:358 (Clear Recent Directories).
        todo!()
    }
}

/// Whether `a` and `b` name the same folder however they are spelled: compared by their folder
/// key (separators `/`, `.` folded, no trailing separator; lower case on Windows and macOS, whose
/// file systems usually ignore case). Lexical, no I/O.
pub fn same_folder(a: &Path, b: &Path) -> bool {
    // lightcraft: crates/ui-egui/src/panels/left/local.rs:117 (`same_folder` over `folder_key`).
    todo!()
}

/// Whether `path` is `root` or lies inside it, by the same folder key as [`same_folder`].
pub fn folder_within(path: &Path, root: &Path) -> bool {
    // lightcraft: crates/catalog/src/query.rs:692 (`folder_within`).
    todo!()
}
