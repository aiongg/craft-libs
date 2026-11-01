//! `PlacesTree`: the folder tree with lazy children, reveal, the keep, hide and reveal menu, and
//! drops.
//!
//! ```ignore
//! let out = PlacesTree::<PhotoId>::new("places", &places, &mut folder_cache)
//!     .current(browsing.as_deref())
//!     .row_menu(|_path| vec![MenuItem::new(tr("Rename Folder…"), String::from("folder.rename")).into()])
//!     .accepts(|path, drag| drag.items.iter().all(|id| catalog.can_move_into(*id, path)))
//!     .show(ui);
//! for event in out.events {
//!     match event {
//!         PlacesEvent::Open(path) => app.browse(path),
//!         PlacesEvent::Dropped { place, items } => app.move_into(place, items.items),
//!         PlacesEvent::Menu { target, command } => app.run_folder_command(&command, target),
//!         other => settings.places.apply(&other),            // keep, remove, restore, recents
//!     }
//! }
//! ```

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod places_tests;
pub(crate) mod stories;
mod style;

use std::path::{Path, PathBuf};

use craft_ui::behavior::{ListDrag, RowInfo, Selection, SelectionMode};
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::list::{DropMark, List, ListEvent, ListRow, OpenChange, TreeState, paint_drop_mark};
use craft_ui::localized::{Labels as _, Words};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{AsIdSalt, Context, IdSalt, PointerButton, Rect, Response, Sense, Ui};

use crate::places::{Folder, FolderCache, Listing, PlaceGroupKind, PlaceKind, Places, PlacesSettings, StandardFolder, folder_within, same_folder};

pub use style::PlacesStyle;

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
    /// A pick in the host's folder menu ([`PlacesTree::row_menu`]): its command on the folder `target`.
    Menu { target: PathBuf, command: String },
    /// A `ListDrag<K>` released over a row `.accepts(..)` took.
    Dropped { place: PathBuf, items: ListDrag<K> },
}

impl PlacesSettings {
    /// Applies `Keep`, `Remove`, `Restore`, `RestoreAll` and `ClearRecent`; ignores the rest.
    /// Returns whether the settings changed (the host saves them).
    pub fn apply<K>(&mut self, event: &PlacesEvent<K>) -> bool {
        // new: the host applies the menu's settings events in one call.
        match event {
            PlacesEvent::Keep(path) => self.keep(path),
            PlacesEvent::Remove(path) => self.remove(path),
            PlacesEvent::Restore(path) => self.restore(path),
            PlacesEvent::RestoreAll => self.restore_all(),
            PlacesEvent::ClearRecent => self.clear_recent(),
            PlacesEvent::Open(_) | PlacesEvent::Menu { .. } | PlacesEvent::Dropped { .. } => false,
        }
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
        favorites: "browse.places.favorites" = "Favorites";
        places: "browse.places.places" = "Places";
        volumes: "browse.places.volumes" = "Volumes";
        network: "browse.places.network" = "Network";
        recent: "browse.places.recent" = "Recent Folders";
        none: "browse.places.none" = "(none)";
        home: "browse.places.home" = "Home";
        desktop: "browse.places.desktop" = "Desktop";
        documents: "browse.places.documents" = "Documents";
        downloads: "browse.places.downloads" = "Downloads";
        pictures: "browse.places.pictures" = "Pictures";
        movies: "browse.places.movies" = "Movies";
        music: "browse.places.music" = "Music";
        keep: "browse.places.keep" = "Keep in Places";
        add_favorite: "browse.places.addFavorite" = "Add to Favorites";
        remove: "browse.places.remove" = "Remove from Places";
        /// macOS; Windows and Linux use `reveal_other`.
        reveal_mac: "browse.places.revealMac" = "Show in Finder";
        reveal_other: "browse.places.revealOther" = "Show in File Manager";
        clear_recent: "browse.places.clearRecent" = "Clear Recent Folders";
        restore_all: "browse.places.restoreAll" = "Restore All";
        loading: "browse.places.loading" = "Loading…";
    }
}

/// The hidden-places row: "Show {n} hidden places".
pub const HIDDEN_PLACES: craft_ui::localized::words::Counted =
    craft_ui::localized::words::Counted::new("browse.places.hidden", "Show {n} hidden place", "Show {n} hidden places");

/// The host's entries for a folder row's menu.
type RowMenu<'a> = Box<dyn Fn(&Path) -> Vec<MenuEntry<String>> + 'a>;

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
///   (`Places::owner`, folders compared by [`same_folder`] and [`folder_within`]) open once, as
///   their listings arrive, and the row scrolls into view; collapsing afterwards sticks.
///   A copy of one folder in two roots opens independently (open state is per root and path).
/// - **Menu** (secondary click, Menu key, Shift+F10): on a folder that is not a root `keep`
///   (or `add_favorite` when the host shows a Favorites group); on a root `remove`; the reveal
///   item (desktop only: `craft_platform::open::reveal`, run by the tree); `clear_recent` in the
///   Recent group; then the host's [`row_menu`](PlacesTree::row_menu) entries. Hidden roots come back through a
///   [`HIDDEN_PLACES`] row at the end: with one hidden root a click restores it, with several it
///   opens a menu of them and `restore_all`.
/// - **Drop.** While egui's payload is a `ListDrag<K>` over a row that `.accepts(path, drag)`
///   takes, the row shows the list's drop outline (`paint_drop_mark`, `DropMark::Into`); a release
///   reports `Dropped`.
/// - **Keys and focus.** One Tab stop; the list keys: Up and Down move the lead (the tree keeps
///   its own selection, which follows `current` only when `current` changes), Enter opens the
///   lead's folder, → and ← open and close it.
/// - **AccessKit.** `Tree` with `TreeItem`s (`level`, `expanded`); headings are `Heading`s.
/// - **RTL.** Indent and triangle mirror.
pub struct PlacesTree<'a, K> {
    id_salt: IdSalt,
    places: &'a Places,
    cache: &'a mut FolderCache,
    current: Option<&'a Path>,
    favorites_group: bool,
    row_menu: Option<RowMenu<'a>>,
    accepts: Option<Accepts<'a, K>>,
    labels: Option<PlacesLabels>,
}

impl<'a, K: Clone + Send + Sync + 'static> PlacesTree<'a, K> {
    /// The tree of `places`, listing children through `cache`.
    pub fn new(id_salt: impl AsIdSalt, places: &'a Places, cache: &'a mut FolderCache) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/folders.rs:14 (`folder_tree`).
        let favorites_group = places.groups.iter().any(|group| group.kind == PlaceGroupKind::Favorites);
        Self { id_salt: IdSalt::new(id_salt), places, cache, current: None, favorites_group, row_menu: None, accepts: None, labels: None }
    }

    /// The folder being browsed: its row is selected and revealed when it changes.
    #[must_use]
    pub fn current(mut self, current: Option<&'a Path>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/folders.rs:280 (`reveal_chosen`).
        self.current = current;
        self
    }

    /// The host's entries after the library's in a folder row's menu, built each pass while the
    /// menu is open; a pick reports [`PlacesEvent::Menu`] with the entry's command.
    #[must_use]
    pub fn row_menu(mut self, menu: impl Fn(&Path) -> Vec<MenuEntry<String>> + 'a) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/folders.rs:56 (the folder menu: Rename Folder…, Move Folder To…).
        self.row_menu = Some(Box::new(menu));
        self
    }

    /// Which rows take a drag of `ListDrag<K>`.
    #[must_use]
    pub fn accepts(mut self, accepts: impl Fn(&Path, &ListDrag<K>) -> bool + 'a) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/folders.rs:376 (`folder_drop_target`).
        self.accepts = Some(Box::new(accepts));
        self
    }

    /// The words.
    #[must_use]
    pub fn labels(mut self, labels: PlacesLabels) -> Self {
        // new: builder.
        self.labels = Some(labels);
        self
    }

    /// Draws the tree and reports what happened.
    pub fn show(self, ui: &mut Ui) -> PlacesOutput<K> {
        // lightcraft: crates/ui-egui/src/panels/left/folders.rs:312 (`folder_rows`); filmcraft: crates/ui-egui/src/panels/media_browser.rs:262 (`tree`: headings, "(none)").
        show_tree(self, ui)
    }
}

/// Rows below this depth list no children (LightCraft's cap).
const MAX_DEPTH: usize = 8;

/// A folder row's place in the tree: the group and root it hangs from, and its path. A folder
/// shown under two roots has two rows, each with its own open state.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct At {
    group: usize,
    root: PathBuf,
    path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum RowKey {
    Heading(usize),
    Empty(usize),
    Folder(At),
    /// The placeholder under the folder `At` while its listing runs.
    Loading(At),
    Hidden,
}

enum Content {
    Heading(String),
    Empty(String),
    Loading(String),
    Folder { name: String },
    Hidden(String),
}

/// The rows as shown, in display order.
struct Shown {
    info: Vec<RowInfo<RowKey>>,
    content: Vec<Content>,
}

/// What a context menu was asked for.
#[derive(Clone, Debug)]
enum Target {
    Folder { path: PathBuf, root: Option<PlaceKind> },
    Hidden,
}

#[derive(Clone, Debug)]
enum Command {
    Keep(PathBuf),
    Remove(PathBuf),
    /// Built only where `craft_platform::open::reveal` exists.
    #[cfg(not(target_arch = "wasm32"))]
    Reveal(PathBuf),
    ClearRecent,
    Host {
        path: PathBuf,
        command: String,
    },
    Restore(PathBuf),
    RestoreAll,
}

/// What the tree keeps between passes, in egui memory.
#[derive(Clone, Default)]
struct Memory {
    /// The `current` of the last pass, to see it change.
    current: Option<PathBuf>,
    /// The folder whose way down is still being opened: its listings have not all arrived.
    walk: Option<PathBuf>,
    /// The row to scroll to once its parents have listed.
    pending: Option<RowKey>,
    /// The tree's selection: the lead is where the keys act. It is set from `current` when
    /// `current` changes, and the list moves it after that.
    selection: Selection<RowKey>,
    /// `current` changed and its row is not shown yet, so `selection` is not set from it yet.
    sync: bool,
    /// The window focus of the last pass that knew it.
    focused: Option<bool>,
    /// The target of the open context menu.
    menu: Option<Target>,
}

fn show_tree<K: Clone + Send + Sync + 'static>(tree: PlacesTree<'_, K>, ui: &mut Ui) -> PlacesOutput<K> {
    let PlacesTree { id_salt, places, cache, current, favorites_group, row_menu, accepts, labels } = tree;
    let ctx = ui.ctx().clone();
    let list_id = List::id_in(ui, id_salt);
    let id = list_id.with("tree");
    let labels = labels.unwrap_or_else(|| PlacesLabels::of(ui));
    let words = Words::of(ui);
    let mut memory: Memory = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    let mut open = TreeState::load(&ctx, id);
    let mut events = Vec::new();

    let focused = ui.input(|i| i.viewport().focused);
    if memory.focused == Some(false) && focused == Some(true) {
        cache.refresh();
    }
    if focused.is_some() {
        memory.focused = focused;
    }

    if memory.current.as_deref() != current {
        memory.current = current.map(Path::to_path_buf);
        memory.walk = memory.current.clone();
        memory.sync = true;
    }
    if let Some(target) = memory.walk.take() {
        match reveal(places, cache, &target, &mut open) {
            Reveal::Reached(key) => memory.pending = Some(key),
            Reveal::Waiting => memory.walk = Some(target),
            Reveal::Unreachable => memory.pending = None,
        }
    }

    let shown = rows(places, cache, &open, &labels, (&ctx, &words));
    if memory.pending.as_ref().is_some_and(|key| shown.info.iter().all(|row| &row.key != key)) && !shown.has_loading() {
        memory.pending = None;
    }
    if memory.sync {
        let selected = current.and_then(|current| selected_key(places, &shown, current));
        if let Some(key) = selected {
            memory.selection = Selection::only(key);
            memory.sync = false;
        } else if memory.walk.is_none() && !shown.has_loading() {
            memory.selection = Selection::default();
            memory.sync = false;
        }
    }
    let mut selection = std::mem::take(&mut memory.selection);
    let reveal_key = memory.pending.take_if(|key| shown.info.iter().any(|row| &row.key == key));

    let tokens = Tokens::of(ui);
    let style = PlacesTree::<K>::style(&tokens);
    let mut list = List::new(id_salt, labels.places.clone(), &shown.info[..]).tree(true).mode(SelectionMode::Single).selection(&mut selection);
    if let Some(key) = reveal_key {
        list = list.reveal(key);
    }
    let out = list.show(ui, |at, key| match &shown.content[at] {
        Content::Heading(text) => ListRow::new(key.clone(), text.clone()),
        Content::Empty(text) | Content::Loading(text) => ListRow::new(key.clone(), text.clone()).dimmed(true),
        Content::Folder { name } => ListRow::new(key.clone(), name.clone()).glyph(move |painter, rect| {
            icons::paint_on(painter, rect, Icon::Folder, icons::active(painter.ctx()), rect.width(), IconSize::Md.stroke(), style.icon);
        }),
        Content::Hidden(text) => ListRow::new(key.clone(), text.clone()).glyph(|_, _| {}),
    });
    memory.selection = selection;
    let list_rect = out.response.rect;

    let mut menu_request = None;
    let double_click = ui.input(|i| i.pointer.button_double_clicked(PointerButton::Primary));
    let mut changed = false;
    for event in out.events {
        match event {
            ListEvent::Clicked(RowKey::Folder(at)) => events.push(PlacesEvent::Open(at.path)),
            ListEvent::Activate(RowKey::Folder(at)) if !double_click => events.push(PlacesEvent::Open(at.path)),
            ListEvent::Activate(RowKey::Folder(at)) => {
                let wanted = !open.is_open(&RowKey::Folder(at.clone()));
                changed |= set_open(&mut open, cache, &RowKey::Folder(at), wanted);
            }
            ListEvent::Clicked(RowKey::Hidden) | ListEvent::Activate(RowKey::Hidden) => match places.hidden.as_slice() {
                [only] => events.push(PlacesEvent::Restore(only.path.clone())),
                [] => {}
                _ => {
                    let row = out.rows.iter().find(|(key, _)| *key == RowKey::Hidden).map_or(list_rect, |(_, rect)| *rect);
                    menu_request = Some((Target::Hidden, OpenAt::Rect(row)));
                }
            },
            ListEvent::Open(change) => changed |= apply_open(&mut open, cache, &shown, &change),
            ListEvent::ContextMenu(request) => {
                let place = request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer);
                match request.row {
                    Some(RowKey::Folder(at)) => {
                        let root = places.groups.get(at.group).and_then(|g| g.places.iter().find(|p| p.path == at.root));
                        let root = root.filter(|_| at.path == at.root).map(|p| p.kind);
                        menu_request = Some((Target::Folder { path: at.path, root }, place));
                    }
                    Some(RowKey::Hidden) => menu_request = Some((Target::Hidden, place)),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    if changed {
        ctx.request_repaint();
    }

    drops(ui, &out.rows, list_rect, accepts.as_ref(), &mut events);

    let at = menu_request.map(|(target, at)| {
        memory.menu = Some(target);
        at
    });
    if let Some(target) = &memory.menu {
        let host = match (target, &row_menu) {
            (Target::Folder { path, .. }, Some(menu)) => menu(path),
            _ => Vec::new(),
        };
        let entries = menu_entries(target, places, host, &labels, favorites_group);
        if let Some(command) = ContextMenu::new(&entries).open_at(at).own_gestures(false).show(ui, &out.response).chosen {
            events.extend(run(command));
        }
    }

    open.store(&ctx);
    ctx.data_mut(|d| d.insert_temp(id, memory));
    cache.end_pass();
    PlacesOutput { response: out.response, events }
}

impl Shown {
    fn has_loading(&self) -> bool {
        self.info.iter().any(|row| matches!(row.key, RowKey::Loading(_)))
    }
}

/// How far [`reveal`] got.
enum Reveal {
    /// Every folder on the way is open; the row of the folder to scroll to.
    Reached(RowKey),
    /// A listing on the way has not arrived.
    Waiting,
    /// The folder is not below the owner root, or a listing on the way fails or lacks it.
    Unreachable,
}

/// Opens the owner root and the folders on the way down to `current`, as far as their listings
/// have arrived. Folders are matched by [`same_folder`] and [`folder_within`], so a spelling of
/// `current` that differs from the listing's still finds its row.
fn reveal(places: &Places, cache: &mut FolderCache, current: &Path, open: &mut TreeState) -> Reveal {
    let Some((group, index)) = places.owner else { return Reveal::Unreachable };
    let Some(root) = places.groups.get(group).and_then(|g| g.places.get(index)).map(|p| &p.path) else { return Reveal::Unreachable };
    if !folder_within(current, root) {
        return Reveal::Unreachable;
    }
    let key = |path: &Path| RowKey::Folder(At { group, root: root.clone(), path: path.to_path_buf() });
    let mut path = root.clone();
    for _ in 0..=MAX_DEPTH {
        if same_folder(&path, current) {
            return Reveal::Reached(key(&path));
        }
        open.set(&key(&path), true);
        match cache.subfolders(&path) {
            Listing::Loading => return Reveal::Waiting,
            Listing::Failed(_) => return Reveal::Unreachable,
            Listing::Ready(folders) => match folders.iter().find(|folder| folder_within(current, &folder.path)) {
                Some(folder) => path = folder.path.clone(),
                None => return Reveal::Unreachable,
            },
        }
    }
    Reveal::Unreachable
}

/// The row of `current`: the owner root's, else the first that names it.
fn selected_key(places: &Places, shown: &Shown, current: &Path) -> Option<RowKey> {
    let named = |row: &&RowInfo<RowKey>| matches!(&row.key, RowKey::Folder(at) if same_folder(&at.path, current));
    let owner = places.owner.and_then(|(group, index)| places.groups.get(group)?.places.get(index));
    let in_owner = |row: &&RowInfo<RowKey>| matches!((&row.key, owner), (RowKey::Folder(at), Some(root)) if at.root == root.path);
    shown.info.iter().filter(named).find(in_owner).or_else(|| shown.info.iter().find(named)).map(|row| row.key.clone())
}

fn group_word(labels: &PlacesLabels, kind: PlaceGroupKind) -> &str {
    match kind {
        PlaceGroupKind::Favorites => &labels.favorites,
        PlaceGroupKind::Places => &labels.places,
        PlaceGroupKind::Volumes => &labels.volumes,
        PlaceGroupKind::Network => &labels.network,
        PlaceGroupKind::Recent => &labels.recent,
    }
}

fn standard_word(labels: &PlacesLabels, folder: StandardFolder) -> &str {
    match folder {
        StandardFolder::Home => &labels.home,
        StandardFolder::Desktop => &labels.desktop,
        StandardFolder::Documents => &labels.documents,
        StandardFolder::Downloads => &labels.downloads,
        StandardFolder::Pictures => &labels.pictures,
        StandardFolder::Movies => &labels.movies,
        StandardFolder::Music => &labels.music,
    }
}

/// The display order: each group's heading, its roots with their open folders, then the hidden
/// row.
fn rows(places: &Places, cache: &mut FolderCache, open: &TreeState, labels: &PlacesLabels, (ctx, words): (&Context, &Words)) -> Shown {
    let mut shown = Shown { info: Vec::new(), content: Vec::new() };
    for (group, items) in places.groups.iter().enumerate() {
        shown.info.push(RowInfo::section(RowKey::Heading(group)));
        shown.content.push(Content::Heading(group_word(labels, items.kind).to_owned()));
        if items.places.is_empty() {
            shown.info.push(RowInfo::leaf(RowKey::Empty(group)));
            shown.content.push(Content::Empty(labels.none.clone()));
        }
        for place in &items.places {
            let name = match place.kind {
                PlaceKind::Standard(folder) => standard_word(labels, folder).to_owned(),
                _ => place.name.clone(),
            };
            let at = At { group, root: place.path.clone(), path: place.path.clone() };
            folder_rows(&mut shown, cache, open, labels, at, 0, name);
        }
    }
    if !places.hidden.is_empty() {
        shown.info.push(RowInfo::leaf(RowKey::Hidden));
        let text = words.counted(ctx, HIDDEN_PLACES, &words.get(HIDDEN_PLACES.word), places.hidden.len() as u64);
        shown.content.push(Content::Hidden(text));
    }
    shown
}

fn folder_rows(shown: &mut Shown, cache: &mut FolderCache, open: &TreeState, labels: &PlacesLabels, at: At, depth: usize, name: String) {
    let key = RowKey::Folder(at.clone());
    let is_open = open.is_open(&key);
    let mut children = None;
    let has_children = if depth >= MAX_DEPTH {
        false
    } else if !is_open {
        true
    } else {
        match cache.subfolders(&at.path) {
            Listing::Loading => {
                children = Some(None);
                true
            }
            Listing::Ready([]) | Listing::Failed(_) => false,
            Listing::Ready(folders) => {
                children = Some(Some(folders.to_vec()));
                true
            }
        }
    };
    shown.info.push(RowInfo { key, depth, has_children, open: is_open && has_children, section: false });
    shown.content.push(Content::Folder { name });
    match children {
        Some(None) => {
            shown.info.push(RowInfo { key: RowKey::Loading(at), depth: depth + 1, has_children: false, open: false, section: false });
            shown.content.push(Content::Loading(labels.loading.clone()));
        }
        Some(Some(folders)) => {
            for Folder { path, name } in folders {
                let child = At { group: at.group, root: at.root.clone(), path };
                folder_rows(shown, cache, open, labels, child, depth + 1, name);
            }
        }
        None => {}
    }
}

/// Opens or closes `key`; an opened folder is listed again. Returns whether the state changed.
fn set_open(open: &mut TreeState, cache: &mut FolderCache, key: &RowKey, wanted: bool) -> bool {
    if open.is_open(key) == wanted {
        return false;
    }
    open.set(key, wanted);
    if let (true, RowKey::Folder(at)) = (wanted, key) {
        cache.refresh_folder(&at.path);
    }
    true
}

fn apply_open(open: &mut TreeState, cache: &mut FolderCache, shown: &Shown, change: &OpenChange<RowKey>) -> bool {
    let mut changed = set_open(open, cache, &change.row, change.open);
    if change.all {
        for row in shown.info.iter().filter(|row| row.has_children) {
            changed |= set_open(open, cache, &row.key, change.open);
        }
    }
    changed
}

/// Outlines the folder row under a `ListDrag<K>` the host accepts and reports its release.
fn drops<K: Clone + Send + Sync + 'static>(
    ui: &Ui,
    rows: &[(RowKey, Rect)],
    list_rect: Rect,
    accepts: Option<&Accepts<'_, K>>,
    events: &mut Vec<PlacesEvent<K>>,
) {
    let Some(accepts) = accepts else { return };
    let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) else { return };
    for (key, row) in rows {
        let RowKey::Folder(at) = key else { continue };
        let rect = row.intersect(list_rect);
        if !rect.contains(pointer) {
            continue;
        }
        let response = ui.interact(rect, ui.scope_id().with(("drop", key)), Sense::hover());
        let Some(drag) = response.dnd_hover_payload::<ListDrag<K>>() else { return };
        if !accepts(&at.path, &drag) {
            return;
        }
        if let Some(dropped) = response.dnd_release_payload::<ListDrag<K>>() {
            events.push(PlacesEvent::Dropped { place: at.path.clone(), items: (*dropped).clone() });
        } else {
            paint_drop_mark(ui, rect, DropMark::Into);
        }
        return;
    }
}

fn menu_entries(
    target: &Target,
    places: &Places,
    host: Vec<MenuEntry<String>>,
    labels: &PlacesLabels,
    favorites_group: bool,
) -> Vec<MenuEntry<Command>> {
    let mut entries: Vec<MenuEntry<Command>> = Vec::new();
    match target {
        Target::Hidden => {
            for place in &places.hidden {
                entries.push(MenuItem::new(place.name.clone(), Command::Restore(place.path.clone())).into());
            }
            entries.push(MenuEntry::Separator);
            entries.push(MenuItem::new(labels.restore_all.clone(), Command::RestoreAll).into());
        }
        Target::Folder { path, root } => {
            let keep = if favorites_group { &labels.add_favorite } else { &labels.keep };
            match root {
                None | Some(PlaceKind::Session) => entries.push(MenuItem::new(keep.clone(), Command::Keep(path.clone())).into()),
                Some(PlaceKind::Recent) => entries.push(MenuItem::new(labels.clear_recent.clone(), Command::ClearRecent).into()),
                Some(_) => entries.push(MenuItem::new(labels.remove.clone(), Command::Remove(path.clone())).into()),
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let reveal = if std::env::consts::OS == "macos" { &labels.reveal_mac } else { &labels.reveal_other };
                entries.push(MenuItem::new(reveal.clone(), Command::Reveal(path.clone())).into());
            }
            if !host.is_empty() {
                entries.push(MenuEntry::Separator);
            }
            entries.extend(host.iter().map(|entry| entry.map(&|command: &String| Command::Host { path: path.clone(), command: command.clone() })));
        }
    }
    entries
}

/// The event a menu command reports; revealing runs here and reports nothing.
fn run<K>(command: Command) -> Option<PlacesEvent<K>> {
    match command {
        Command::Keep(path) => Some(PlacesEvent::Keep(path)),
        Command::Remove(path) => Some(PlacesEvent::Remove(path)),
        Command::Restore(path) => Some(PlacesEvent::Restore(path)),
        Command::RestoreAll => Some(PlacesEvent::RestoreAll),
        Command::ClearRecent => Some(PlacesEvent::ClearRecent),
        Command::Host { path, command } => Some(PlacesEvent::Menu { target: path, command }),
        #[cfg(not(target_arch = "wasm32"))]
        Command::Reveal(path) => {
            let _ = craft_platform::open::reveal(&path);
            None
        }
    }
}
