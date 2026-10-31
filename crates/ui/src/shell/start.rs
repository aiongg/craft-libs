//! [`StartScreen`]: the home page an app shows with no document open: New and Open, the recent
//! files as a grid or list, pinned folders and templates.

#![expect(dead_code, unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::path::PathBuf;

use egui::{AsIdSalt, Id, Ui};

/// A recent file as the start screen lists it. The host keeps the list (craft-platform's recent
/// files in step 4) and passes it every frame.
#[derive(Clone, Debug, PartialEq)]
pub struct RecentFile {
    pub path: PathBuf,
    /// The name shown: the file name without its folder.
    pub name: String,
    /// Last opened, in seconds since the Unix epoch, for sorting and the "2 days ago" line.
    pub opened: Option<i64>,
    /// Created, for sorting by Created.
    pub created: Option<i64>,
    /// A thumbnail the host has loaded (`egui::TextureId`), drawn in the grid card.
    pub thumbnail: Option<egui::TextureId>,
}

/// A template card: a named document size or starter file.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub id: String,
    pub name: String,
    /// "1920 × 1080 px".
    pub detail: String,
    pub thumbnail: Option<egui::TextureId>,
}

/// How the recent files show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RecentView {
    /// Cards with thumbnails, as many columns as fit.
    #[default]
    Grid,
    /// One row per file: icon, name, folder, date.
    List,
}

/// The recent files' order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RecentSort {
    /// Most recently opened first.
    #[default]
    Opened,
    Name,
    Created,
}

/// What the user did on the start screen this frame. The host runs it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum StartAction {
    /// New… (the host opens its New Document dialog).
    New,
    /// Open… (the host opens its file dialog).
    Open,
    OpenRecent(PathBuf),
    /// "Remove from Recent" on a card's context menu, or its × on hover.
    RemoveRecent(PathBuf),
    /// "Clear Recent Files" on the context menu.
    ClearRecent,
    /// "Show in Folder" on a card's context menu.
    RevealRecent(PathBuf),
    OpenPinned(PathBuf),
    /// "Pin Folder…" (the host asks for a folder).
    PinFolder,
    UnpinFolder(PathBuf),
    UseTemplate(String),
    /// The view or sort changed; the host stores it with its preferences.
    ViewChanged(RecentView, RecentSort),
}

/// The start screen, drawn in the shell's body when no document is open.
///
/// ```no_run
/// use craft_ui::shell::{RecentFile, StartAction, StartScreen};
/// # fn demo(ui: &mut egui::Ui, recent: &[RecentFile]) {
/// for action in StartScreen::new("home").recent(recent).show(ui) {
///     match action {
///         StartAction::New => { /* open the New dialog */ }
///         StartAction::OpenRecent(path) => { /* open it */ }
///         _ => {}
///     }
/// }
/// # }
/// ```
///
/// Layout: a header row with New and Open (New the primary button), then the sections the host
/// fills: Recent (with the Grid/List toggle and the Sort menu), Pinned Folders, Templates. A
/// section with nothing to show is left out; with no recent file the Recent section shows an empty
/// state. The page scrolls; the cards wrap at the body's width.
///
/// Keyboard: New and Open are the first Tab stops; the recent cards are one grid
/// ([`crate::behavior`]'s grid navigation): arrows move, Enter opens, the Menu key or Shift+F10
/// opens the card's context menu, Delete removes it from Recent. Cmd+N and Cmd+O are the app's
/// shortcuts and need nothing here.
///
/// AccessKit: the page is a `Group` named [`StartLabels::page`] ("Home"); each card is a button named by the file
/// name, with the folder and date as its description.
#[must_use = "show the start screen with `show`"]
pub struct StartScreen<'a> {
    id: Id,
    recent: &'a [RecentFile],
    pinned: &'a [PathBuf],
    templates: &'a [Template],
    view: RecentView,
    sort: RecentSort,
    max_recent: usize,
    labels: Option<StartLabels>,
}

impl<'a> StartScreen<'a> {
    /// A start screen with New and Open and nothing else.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        todo!()
    }

    /// The recent files, most recent first.
    // figcraft: crates/ui-egui/src/home/home_recents.rs:42 (`SHOWN` 12), :48 `View`, :79 `SortKey`;
    // pc: crates/ui-egui/src/canvas/start_screen.rs:99 (`HOME_RECENT` 6).
    pub fn recent(mut self, recent: &'a [RecentFile]) -> Self {
        todo!()
    }

    /// At most `n` recent files show (12 by default).
    pub fn max_recent(mut self, n: usize) -> Self {
        todo!()
    }

    /// The pinned folders, in the host's order.
    // pdfcraft: crates/ui-egui/src/home/folders_ui.rs:15 (`MAX_PINNED` 8), :51 `PinnedFolders`.
    pub fn pinned(mut self, folders: &'a [PathBuf]) -> Self {
        todo!()
    }

    // figcraft: crates/ui-egui/src/home.rs:23 `Section` (Recents, Online, Templates).
    pub fn templates(mut self, templates: &'a [Template]) -> Self {
        todo!()
    }

    /// The view and sort the host stored ([`StartAction::ViewChanged`] reports a change).
    pub fn view(mut self, view: RecentView, sort: RecentSort) -> Self {
        todo!()
    }

    /// Translated labels; default the installed words.
    pub fn labels(mut self, labels: StartLabels) -> Self {
        todo!()
    }

    /// Draws the page in `ui`'s available rect and returns this frame's actions.
    // pc: crates/ui-egui/src/canvas/start_screen.rs:13 `start_screen` (New/Open buttons, recent rows,
    // hover ×, context menu :157-170); figcraft: crates/ui-egui/src/home.rs:54 `Home`; pdfcraft:
    // crates/ui-egui/src/home/folders_ui.rs:199 `pin_folder`, :220 `unpin_folder`.
    pub fn show(self, ui: &mut Ui) -> Vec<StartAction> {
        todo!()
    }
}

/// The recent files in `sort` order (Opened: newest first; Name: by the collator, case-folded;
/// Created: newest first, files without a date last).
// figcraft: crates/ui-egui/src/home/home_recents.rs:79 `SortKey`.
pub fn sort_recent(files: &mut [RecentFile], sort: RecentSort) {
    todo!()
}

crate::words! {
    /// The start screen's labels, English by default.
    #[non_exhaustive]
    pub struct StartLabels {
        page: "start.page" = "Home";
        new: "start.new" = "New…";
        open: "start.open" = "Open…";
        recent: "start.recent" = "Recent";
        no_recent: "start.noRecent" = "Files you open show here.";
        grid: "start.grid" = "Grid";
        list: "start.list" = "List";
        sort: "start.sort" = "Sort";
        sort_opened: "start.sortOpened" = "Last Opened";
        sort_name: "start.sortName" = "Name";
        sort_created: "start.sortCreated" = "Created";
        remove_recent: "start.removeRecent" = "Remove from Recent";
        clear_recent: "start.clearRecent" = "Clear Recent Files";
        reveal: "start.reveal" = "Show in Folder";
        pinned: "start.pinned" = "Pinned Folders";
        pin_folder: "start.pinFolder" = "Pin Folder…";
        unpin: "start.unpin" = "Unpin";
        templates: "start.templates" = "Templates";
    }
}
