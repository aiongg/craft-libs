//! `LinkList`: the Links panel's rows, filter, sort, info section and actions.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::collections::HashMap;

use craft_docpath::links::{Link, LinkStatus};
use craft_ui::behavior::Selection;
use craft_ui::canvas::textures::{TextureKey, ThumbSource, Thumbnails};
use craft_ui::list::ThumbSize;
use egui::{AsIdSalt, Response, Ui};

/// Which links the list shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LinkFilter {
    /// Every link.
    #[default]
    All,
    /// `LinkStatus::is_missing`.
    Missing,
    /// Modified.
    Modified,
    /// Embedded.
    Embedded,
}

/// The list's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LinkSort {
    /// By name (`natural_cmp`).
    #[default]
    Name,
    /// By extension, then name.
    Kind,
    /// Missing first, then Modified, then the rest; then name.
    Status,
}

/// The list's saved choices (VectorCraft's panel options); the host stores it.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LinkListState {
    /// The filter.
    pub filter: LinkFilter,
    /// The order.
    pub sort: LinkSort,
    /// Row thumbnails: None, Small, Medium, Large.
    pub thumbnails: ThumbSize,
    /// The Link Info section is open.
    pub info_open: bool,
}

/// What the list reported, in order. Each is a `Set` the host runs as its own command, one undo
/// step each.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinkListEvent<K> {
    /// The selection changed; already applied.
    Selected,
    /// Relink…: a file pick for the selection (Photoshop, InDesign).
    Relink(Vec<K>),
    /// Relink to Folder…: a folder pick for the selection.
    RelinkToFolder(Vec<K>),
    /// Find missing links: the host opens the `RelinkDialog` on the missing set.
    FindMissing,
    /// Update Link on the modified links of the selection.
    Update(Vec<K>),
    /// Edit Original (the host's `openable_original` check stays its own).
    EditOriginal(K),
    /// Break Link (embed).
    BreakLink(Vec<K>),
    /// Reveal in Finder.
    Reveal(K),
    /// Go To Link (double click).
    GoTo(K),
    /// The state changed (filter, sort, thumbnails, info section); the host saves it.
    State,
}

/// What [`LinkList::show`] returns.
#[derive(Debug)]
#[must_use = "the host runs the list's events"]
pub struct LinkListOutput<K> {
    /// The list.
    pub response: Response,
    /// What happened, in order.
    pub events: Vec<LinkListEvent<K>>,
}

craft_ui::words! {
    /// The Links panel's words, English by default.
    #[non_exhaustive]
    pub struct LinkListLabels {
        list: "browse.links" = "Links";
        show_all: "browse.linksShowAll" = "Show All";
        show_missing: "browse.linksShowMissing" = "Show Missing";
        show_modified: "browse.linksShowModified" = "Show Modified";
        show_embedded: "browse.linksShowEmbedded" = "Show Embedded";
        sort_name: "browse.linksSortName" = "Sort by Name";
        sort_kind: "browse.linksSortKind" = "Sort by Kind";
        sort_status: "browse.linksSortStatus" = "Sort by Status";
        relink: "browse.linksRelink" = "Relink…";
        relink_folder: "browse.linksRelinkFolder" = "Relink to Folder…";
        find_missing: "browse.linksFindMissing" = "Find Missing Links…";
        update: "browse.linksUpdate" = "Update Link";
        edit_original: "browse.linksEditOriginal" = "Edit Original";
        break_link: "browse.linksBreak" = "Break Link";
        reveal: "browse.linksReveal" = "Reveal in Finder";
        go_to: "browse.linksGoTo" = "Go to Link";
        info: "browse.linksInfo" = "Link Info";
        /// `{n}` the page.
        page: "browse.linksPage" = "Page {n}";
        missing: "browse.linksMissing" = "Missing";
        modified: "browse.linksModified" = "Modified";
        unreadable: "browse.linksUnreadable" = "Unreadable";
        /// `{reason}` from `Refused`'s words.
        refused: "browse.linksRefused" = "Not followed: {reason}";
        refused_network: "browse.linksRefusedNetwork" = "the path is on another computer";
        refused_device: "browse.linksRefusedDevice" = "the path names a device, not a file";
        refused_url: "browse.linksRefusedUrl" = "the link is not a file path";
        refused_invalid: "browse.linksRefusedInvalid" = "the path is not valid";
        embedded: "browse.linksEmbedded" = "Embedded";
    }
}

/// A host slot drawn inside the widget.
type Slot<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The thumbnails a widget draws with: the cache and the app's source, keyed by the widget's key.
type ThumbPair<'a, K> = (&'a Thumbnails<K>, &'a dyn ThumbSource<K>);

/// The Links panel (N5.05): rows of the document's links with thumbnail, name, page and status.
///
/// - **Rows.** `ListRow`s, striped: thumbnail per `LinkListState::thumbnails` (`ThumbSize`,
///   through `Thumbnails` keyed by the link key: `state` asked before the row is built, `paint`
///   in its thumbnail closure), name, page ([`LinkListLabels::page`] in `Caption` `text.faint`),
///   the [`StatusBadge`](super::StatusBadge) as a trailing item; filtered by `filter` and sorted
///   by `sort`.
/// - **Selection.** A click replaces; Cmd toggles; Shift selects the range from the last click
///   made without Shift (with Cmd adds); a Shift-click whose anchor row is filtered out acts as a
///   plain click (VectorCraft's `click`); double click: `GoTo`.
/// - **Actions** (the row under the list, `.actions(true)`; also the panel menu): Relink… (needs
///   a selection), Relink to Folder… (a selection), Update Link (any selected is Modified), Edit
///   Original (any selected is linked and not missing), Break Link (any linked), Reveal (one
///   linked, not missing), Go to Link (one). Find Missing Links… (any link missing).
/// - **Filter and sort** in the panel menu, radio items; the thumbnail size too; all in
///   `LinkListState` (`State` reported).
/// - **Info section.** A `SectionHeader` "Link Info" (`info_open`) with the rows the host gives in
///   `.info(..)` (path, size, modified, dimensions, page).
/// - **AccessKit.** A `ListBox`; each row named "name, Page n, status in words", `selected`.
pub struct LinkList<'a, K: TextureKey> {
    id_salt: egui::Id,
    links: &'a [Link<K>],
    statuses: &'a HashMap<K, LinkStatus>,
    selection: Option<&'a mut Selection<K>>,
    state: Option<&'a mut LinkListState>,
    thumbnails: Option<ThumbPair<'a, K>>,
    actions: bool,
    info: Option<Slot<'a>>,
    labels: Option<LinkListLabels>,
}

impl<'a, K: TextureKey + std::fmt::Debug> LinkList<'a, K> {
    /// The list of `links` with their known `statuses` (from `LinkMonitor::statuses`; a link
    /// without one shows no badge).
    pub fn new(id_salt: impl AsIdSalt, links: &'a [Link<K>], statuses: &'a HashMap<K, LinkStatus>) -> Self {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:300 (`show`).
        todo!()
    }

    /// The selection.
    #[must_use]
    pub fn selection(self, selection: &'a mut Selection<K>) -> Self {
        // new: builder.
        todo!()
    }

    /// The saved filter, sort, thumbnails and info state.
    #[must_use]
    pub fn state(self, state: &'a mut LinkListState) -> Self {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:56 (`options`).
        todo!()
    }

    /// Row thumbnails keyed by the link key.
    #[must_use]
    pub fn thumbnails(self, thumbnails: &'a Thumbnails<K>, source: &'a dyn ThumbSource<K>) -> Self {
        // new: builder.
        todo!()
    }

    /// Shows the action row under the list (default on).
    #[must_use]
    pub fn actions(self, on: bool) -> Self {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:300 (the bottom bar).
        todo!()
    }

    /// The Link Info section's body for the lead link.
    #[must_use]
    pub fn info(self, info: impl FnOnce(&mut Ui) + 'a) -> Self {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:300 (Show Link Info).
        todo!()
    }

    /// The words.
    #[must_use]
    pub fn labels(self, labels: LinkListLabels) -> Self {
        // new: builder.
        todo!()
    }

    /// Draws the panel body and reports what happened.
    pub fn show(self, ui: &mut Ui) -> LinkListOutput<K> {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:300 (`show`), :280 (`click`).
        todo!()
    }
}
