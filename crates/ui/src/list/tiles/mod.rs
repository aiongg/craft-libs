//! `TileGrid` over the app's `TileSource`: painting, selection, keys, drag, reorder, marquee,
//! scrub and groups.
//!
//! ```ignore
//! let grid = TileGrid::new("library", &photos)                       // photos: impl TileSource<PhotoId>
//!     .layout(TileLayout::Justified { row_height: state.thumb_size })
//!     .groups(&date_runs)
//!     .selection(&mut state.selection)
//!     .thumbnails(&thumbs, &files.keyed(|id| catalog.file(*id)))
//!     .rating_keys(RatingKeys::Digits)
//!     .show(ui);
//! for event in grid.events {
//!     match event {
//!         TileEvent::Activate(id) => open_detail(id),
//!         TileEvent::Rate { keys, rating } => cmd("photo.rate", keys, rating),
//!         _ => {}
//!     }
//! }
//! ```

mod fake;
mod groups;
mod keys;
mod layout;
#[cfg(test)]
mod layout_tests;
mod paint;
mod rename;
mod strip;
mod style;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tiles_tests;
mod zoom;

use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use crate::behavior::{
    DisplayOrder, DragEnd, DropPosition, ListDrag, ListOptions, ListSelection, Listbox, Move, Reorder, ReorderGesture, RowSpans, Selection,
    SelectionChange, SelectionPick, StackKey, edge_scroll_speed,
};
use crate::canvas::textures::{Priority, TextureKey, TextureState, ThumbRequest, ThumbSource, Thumbnails};
use crate::controls::BadgeTone;
use crate::direction;
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::list::{ContextMenuRequest, DropMark, RenameEnd, drop_line, paint_drop_mark};
use crate::localized::Labels as _;
use crate::patterns::ScrollBarLabels;
use crate::primitives::{FocusRing, Interactive};
use crate::theme::{Tokens, TokensOf as _};
use craft_theme::{Rgba, TypeRole};
use egui::accesskit::{Action, AriaCurrent, Role};
use egui::containers::scroll_area::ScrollSource;
use egui::os::OperatingSystem;
use egui::{
    AsIdSalt, CornerRadius, DragAndDrop, Event, EventFilter, Id, Key, Modifiers, PointerButton, Pos2, Rect, Response, ScrollArea, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, pos2, vec2,
};

use keys::{GridKeyAction, KeyGrid};
use paint::accessible_name;

pub use fake::{DescribeTile, FakeTiles};
pub use layout::{ArrangeSpacing, TileArrangement, TileGroup, TileLayout, TileRow, arrange, card_size, fit_name};
pub use paint::{TileFace, TileLook, paint_tile, stack_text};
pub use rename::{TileRename, TileRenameOutcome, show_tile_rename};
pub use strip::{CaptionPlace, TileMark};
pub use style::TileStyle;
pub use zoom::{ThumbnailSlider, TileZoom, ZoomStep};

/// The tiles of a grid, in display order (`DisplayOrder`: `len`, `key`; no depth, children or
/// sections), and what each shows. `content` is asked only for tiles in view and one viewport
/// height above and below, every pass they are built; `aspect` for every tile when the
/// arrangement is rebuilt.
pub trait TileSource<K>: DisplayOrder<K> {
    /// The picture's width over its height for tile `at`; ignored by `TileLayout::Cards`.
    fn aspect(&self, at: usize) -> f32;

    /// A number that changes whenever any aspect changes, so the grid knows when to rebuild its
    /// arrangement without reading every aspect each pass. The default is the count.
    fn aspects_revision(&self) -> u64 {
        self.len() as u64
    }

    /// Fills `content` for tile `at` (it arrives reset to `TileContent::default()`).
    fn content(&self, at: usize, content: &mut TileContent);
}

/// What one tile shows besides its picture.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TileContent {
    /// The name: the caption, the tooltip's first line, the accessible name's start.
    pub name: String,
    /// The caption's second line (a duration, "3 pages").
    pub detail: Option<String>,
    /// Replaces the name and details the grid builds from `TileLabels::tile_name`; the tips of
    /// `marks` still join it.
    pub accessible_name: Option<String>,
    /// Replaces the tooltip (default: the full name).
    pub tooltip: Option<String>,
    /// Stars 0–5; `None` shows no stars and makes the tile unrateable.
    pub rating: Option<u8>,
    /// A pick or reject flag.
    pub flag: Option<TileFlag>,
    /// The colour label's dot.
    pub label_color: Option<Rgba>,
    /// Corner badges.
    pub badges: Vec<TileBadge>,
    /// Offline or failed.
    pub status: TileStatus,
    /// In and Out as fractions of the picture's width (0..1), drawn as the bar along the bottom.
    pub range: Option<(f32, f32)>,
    /// `false`: not selectable or checkable and drawn at 40 % (Import's duplicates).
    pub enabled: bool,
    /// Drawn under the veil (`tile.veil`), still selectable.
    pub dimmed: bool,
    /// Rejected: the veil, and the flag shows the reject glyph.
    pub rejected: bool,
    /// The stack's size on a stack's first tile ("×N" badge).
    pub stack: Option<usize>,
    /// Hidden from the output (PowerPoint's hidden slide): the picture under the veil as
    /// `dimmed`, and with [`CaptionPlace::Start`] a slash across the name. Still selectable.
    pub hidden: bool,
    /// Glyphs under the name with [`CaptionPlace::Start`] (PowerPoint's animation star), each a
    /// part of the accessible name; ignored with [`CaptionPlace::Below`].
    pub marks: Vec<TileMark>,
}

impl Default for TileContent {
    fn default() -> Self {
        // new: enabled, everything else empty.
        TileContent {
            name: String::new(),
            detail: None,
            accessible_name: None,
            tooltip: None,
            rating: None,
            flag: None,
            label_color: None,
            badges: Vec::new(),
            status: TileStatus::Normal,
            range: None,
            enabled: true,
            dimmed: false,
            rejected: false,
            stack: None,
            hidden: false,
            marks: Vec::new(),
        }
    }
}

/// A pick or reject flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileFlag {
    /// Picked.
    Picked,
    /// Rejected.
    Rejected,
}

/// What a corner badge shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BadgeKind {
    /// A glyph, drawn 12.
    Glyph(Icon),
    /// A short text in the `Caption` role ("Duplicate", "1/3").
    Text(String),
}

/// A corner of the picture, logical: Start and End swap in a right-to-left interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BadgeSlot {
    /// The reading start's top corner.
    StartTop,
    /// The reading end's top corner.
    EndTop,
    /// The reading start's bottom corner (above the badge bar when it shows).
    StartBottom,
    /// The reading end's bottom corner.
    EndBottom,
}

/// A corner badge: a pill 16 high (`tile.badge_height`), stacked inwards when a slot holds
/// several, never over the check box (it moves below it). Its tone is the library's badge tone:
/// `Neutral` on `surface.scrim` with `text.on_strong`, `Accent` on `accent.solid`, `Warn` and
/// `Bad` on `status.*.solid` with `text.on_status`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileBadge {
    /// Where.
    pub slot: BadgeSlot,
    /// What.
    pub kind: BadgeKind,
    /// Its colour.
    pub tone: BadgeTone,
    /// The tooltip, and a part of the tile's accessible name.
    pub tip: String,
}

/// A tile's file status.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TileStatus {
    /// Nothing to say.
    #[default]
    Normal,
    /// The media is offline: a band along the bottom (`tile.status_band`, `status.bad.solid`,
    /// `text.on_status`, the offline glyph 13 and this text) and the name in `status.bad.fg`.
    Offline(String),
    /// The file can't be read: a `Bad` badge with `TileLabels::unreadable`.
    Unreadable,
    /// The picture failed: a `Bad` badge; the thumbnail painter shows the failure icon.
    Failed,
}

/// When the badge bar (rating, flag, colour dot, edited glyph) shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BadgeVisibility {
    /// On hover, on selection, or when a rating, flag or label is set (LightCraft's Auto).
    #[default]
    Auto,
    /// Always.
    Always,
    /// Never. Corner badges still show; the host leaves them out to hide them.
    Never,
}

/// Which keys rate the selection while the grid has focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RatingKeys {
    /// None.
    None,
    /// 1–5 rate, 0 clears (LightCraft).
    Digits,
    /// Cmd/Ctrl + 1–5 rate, Cmd/Ctrl + 0 clears (Bridge).
    #[default]
    CommandDigits,
}

/// What plain Space does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpaceAction {
    /// `Preview(lead)` (Bridge). Ctrl+Space still toggles the lead.
    #[default]
    Preview,
    /// Toggles the lead's selection (or check in check mode), for apps without a preview. Apps
    /// whose users keep Ctrl+Space for the input source on macOS choose this.
    Toggle,
}

/// What the grid reported, in the order it happened; each pass's events are handled in that pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TileEvent<K> {
    /// The selection changed; already applied to the selection given.
    Selection(SelectionChange<K>),
    /// A click on a tile that changed nothing (the only selected tile).
    Clicked(K),
    /// The lead moved without changing the selection (Cmd/Ctrl+arrows).
    LeadMoved(K),
    /// Enter or a double click.
    Activate(K),
    /// Space (Bridge's preview).
    Preview(K),
    /// A star click or a rating key: `keys` get `rating` (0 clears). A `Set`, one undo step.
    Rate {
        /// The rated tiles.
        keys: Vec<K>,
        /// The new rating; 0 clears it.
        rating: u8,
    },
    /// Delete or Backspace on the selection; with nothing picked, on the current tile
    /// ([`TileGrid::current`]).
    Remove(Vec<K>),
    /// A drag of the selection began (once); the payload is also egui's drag payload.
    Drag(ListDrag<K>),
    /// A reorder gesture (with `.reorder(..)`): Begin, Commit, Cancel, or Set for a key move.
    Reorder(EditEvent<Move<K>>),
    /// The pointer is over a picture (with `.hover_scrub(true)`): `fraction` across it from the
    /// reading start, 0..1, every pass while it hovers.
    Scrub {
        /// The hovered tile.
        key: K,
        /// Pointer position across the picture from the reading start, 0.0 to 1.0.
        fraction: f32,
    },
    /// A click on a group header (its tiles were selected; Shift or Cmd added them).
    GroupPicked {
        /// Index of the group in the host's group list.
        group: usize,
        /// The modifier keys held during the click.
        modifiers: Modifiers,
    },
    /// Cmd/Ctrl + / − / 0, or a pinch.
    Zoom(ZoomStep),
    /// A secondary click (or AccessKit's ShowContextMenu) on an unpicked tile replaced the
    /// selection with it, for the menu that follows as `ContextMenu`. A pick, not a move: a host
    /// with a current tile keeps it. Already applied to the selection given.
    ContextPicked(SelectionChange<K>),
    /// A context menu was asked for, on a tile or the background (`row: None`).
    ContextMenu(ContextMenuRequest<K>),
    /// A double click on the background (FilmCraft: import).
    EmptyActivated,
    /// A rename was committed with `text`, different from the tile's name (with `.can_rename(..)`);
    /// followed by `RenameEnded`.
    Renamed {
        /// The renamed tile.
        key: K,
        /// The new name.
        text: String,
    },
    /// A rename closed, and how.
    RenameEnded {
        /// The tile whose rename closed.
        key: K,
        /// How the rename closed.
        end: RenameEnd,
    },
    /// A group header's disclosure chevron, or AccessKit's Expand or Collapse on the header,
    /// asks to open or close group `group` (with [`TileGrid::group_disclosure`]); the host keeps
    /// the open states.
    GroupOpen {
        /// Index of the group in the host's group list.
        group: usize,
        /// True to open the group, false to close it.
        open: bool,
    },
    /// The host's menu for group `group` is asked for: a secondary click on its header (with
    /// [`TileGrid::group_menus`]). `anchor` is the header's rect, `pointer` where the click was.
    GroupContextMenu {
        /// Index of the group in the host's group list.
        group: usize,
        /// The header's rect.
        anchor: Rect,
        /// Where the click was.
        pointer: Option<Pos2>,
    },
    /// A click, Enter or Space on the new item row ([`TileGrid::new_item_row`]).
    NewItem,
}

/// What [`TileGrid::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the grid's events"]
pub struct TileGridOutput<K> {
    /// The grid: its focus and its rect.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<TileEvent<K>>,
    /// While tiles of this grid are dragged with `.reorder(..)`, where they would drop this pass
    /// (the move the release would commit).
    pub drop_target: Option<Move<K>>,
    /// Files dragged in from outside the app are over the gap before this position (with
    /// [`TileGrid::file_drops`]; the count for the gap after the last tile); the host takes the
    /// drop from egui's `dropped_files`.
    pub file_gap: Option<usize>,
}

crate::words! {
    /// The grid's words, English by default.
    #[non_exhaustive]
    pub struct TileLabels {
        /// A tile's accessible name: `{name}`, then `{details}` (the parts below that apply,
        /// joined with ", ").
        tile_name: "tiles.name" = "{name}, {details}";
        picked: "tiles.picked" = "picked";
        rejected: "tiles.rejected" = "rejected";
        /// `{label}` the app's label name.
        label: "tiles.label" = "{label} label";
        offline: "tiles.offline" = "Offline";
        /// An offline tile's accessible detail: `{offline}` (the word above), then the host's
        /// `{message}`.
        offline_with: "tiles.offlineWith" = "{offline}: {message}";
        unreadable: "tiles.unreadable" = "Unreadable";
        failed: "tiles.failed" = "Preview failed";
        /// The stack badge: `{n}` tiles.
        stack: "tiles.stack" = "×{n}";
        /// The tile size slider's accessible name.
        thumbnail_size: "tiles.thumbnailSize" = "Thumbnail Size";
        smaller: "tiles.smaller" = "Smaller Thumbnails";
        larger: "tiles.larger" = "Larger Thumbnails";
        /// The rename field's accessible name.
        rename: "tiles.rename" = "Name";
    }
}

/// The thumbnails a widget draws with: the cache and the app's source, keyed by the widget's key.
type ThumbPair<'a, K> = (&'a Thumbnails<K>, &'a dyn ThumbSource<K>);

type CanRename<'a, K> = Box<dyn Fn(&K) -> bool + 'a>;

/// A host's own drawing on a card (PdfCraft's hover bar, a page stack): called after the tile
/// is painted, with the picture's rect.
pub type CardOverlay<'a, K> = Box<dyn FnMut(&mut Ui, &K, Rect) + 'a>;

/// A virtualised grid of thumbnails with groups, badges, rating, selection, check mode, reorder,
/// marquee, hover scrub, keys and drag (N8.04). The host implements [`TileSource`], gives the
/// thumbnails as `.thumbnails(&Thumbnails<K>, &dyn ThumbSource<K>)` and handles the events.
///
/// - **Layout.** [`arrange`] into a [`TileArrangement`] cached in egui memory under the grid's id
///   (rebuilt when the count, `aspects_revision`, the width, the layout or the groups change);
///   only rows in view and one viewport height either side are built each pass, in a
///   `ScrollArea::show_viewport`. Spacing from [`TileStyle`].
/// - **Thumbnails.** Before painting a tile the grid asks `Thumbnails::state(ctx, source, key,
///   &ThumbRequest::for_cell(side, pixels_per_point), priority)`: `Priority::VISIBLE` in view,
///   `NEAR` within one viewport; then `Thumbnails::paint` draws the picture fitted, the
///   placeholder or the failure icon (risk 9: the state is asked before painting, never inside a
///   paint closure). No thumbnails: the placeholder only.
/// - **Selection.** Click replaces; Shift selects the range from the anchor in display order;
///   Cmd/Ctrl toggles; a click on the background clears (`ListSelection`'s rules, the selection
///   given); a click on the only selected tile changes nothing and reports `Clicked`. Cmd/Ctrl+A
///   selects every enabled tile.
/// - **Keys.** One Tab stop. Arrows, Home, End, Page Up and Down move the lead by `GridNav`
///   (`ragged` rows from the arrangement for `Justified`, `wrapped` for `Cards`; `rtl` mirrors
///   Left and Right); the selection follows; Shift extends; Cmd/Ctrl+arrows move the lead only
///   (`LeadMoved`). Enter: `Activate(lead)`. Space: per [`SpaceAction`]; Ctrl+Space always
///   toggles the lead. Delete and Backspace: `Remove(selection)`, or of the current tile with
///   nothing picked. Esc empties the selection (not during a marquee or a drag, which take it).
///   A secondary click on an unpicked tile: `ContextPicked`, then `ContextMenu`. Rating keys per [`RatingKeys`].
///   Cmd/Ctrl + / − / 0: `Zoom`. Cmd/Ctrl+] and [ move the lead's tiles with `.reorder(..)`
///   (`StackKey`). Menu key and Shift+F10: `ContextMenu`. Keys a focused grid takes are claimed
///   (`KeyClaim`). Type-ahead on names with `.type_ahead(true)`, as `List`.
/// - **Wheel.** One notch scrolls a third of the grid's height (`TileStyle::wheel_share`,
///   LightCraft's `WHEEL_STRIP_SHARE`); pinch reports `Zoom`.
/// - **Rating.** A click on a star in the badge bar or caption rates the selection when the tile
///   is in it, else that tile (`Rate`); a click on the star of the current value clears.
/// - **Drag.** Past egui's drag threshold the grabbed tile joins the selection (Cmd/Ctrl held)
///   or replaces it (not selected), `Drag(ListDrag)` is reported once and set as egui's payload;
///   the host shows the `DragBadge`. Esc cancels.
/// - **Reorder** (`.reorder(Reorder)`): `grid_drop` over the arrangement picks the gap; the
///   insert line is `list::drop_line` in the gap, and `list::paint_drop_mark(Into)` outlines the
///   tile for an `Into` target; `Reorder(Begin)`, then `Commit` on release or
///   `Cancel` (Esc, focus lost, released outside); Alt (Option) copies
///   (`Reorder::copy_with_alt`); edge auto-scroll (`Reorder::auto_scroll`).
/// - **Rename** (`.renamable(true)` or `.can_rename(..)`, tiles with a caption): F2 on the lead,
///   a click on the caption of the only selected tile that no second click follows within
///   egui's double-click interval, or `.start_rename(key)` opens a field in the caption
///   (`TileLabels::rename` names it). Enter and focus loss commit (`Renamed` if the text
///   changed, then `RenameEnded`); Esc cancels (`RenameEnded` alone). Starting another rename
///   commits the open one.
/// - **Marquee.** A primary drag from the background selects every enabled tile the rectangle
///   touches, live; Shift or Cmd/Ctrl adds to the selection the drag began with; Esc restores
///   it. Fill `tile.marquee.fill`, line `tile.marquee.line`.
/// - **Hover scrub** (`.hover_scrub(true)`): `Scrub { key, fraction }` every pass the pointer is
///   over a picture, the fraction from the reading start; the playhead line (`tile.scrub`, 1 pt)
///   is drawn at it.
/// - **Groups.** A header per [`TileGroup`]; click selects the group's tiles (Shift or Cmd adds)
///   and reports `GroupPicked`; the header of the group at the top sticks there and the next
///   header pushes it up; headers are not Tab stops.
/// - **Check mode** (`.check_mode(true)`): a click toggles the tile's check (its selection);
///   Shift-click sets every enabled tile from the last clicked one to this one to this tile's new
///   state; disabled tiles are skipped and can't be checked; the check box is the library's
///   (`paint_box` with `Checkbox::style`) in the start-top corner (`tile.checkbox`, at
///   `tile.inset` + `tile.checkbox_backdrop`, on that margin of `surface.scrim` at 60 %).
/// - **Look.** [`TileStyle`]: selected ring outside the picture (`tile.ring.selected` 2 pt with
///   the 1 pt `tile.ring.halo` outside it), hover ring 1 pt `tile.ring.hover`, the lead's focus
///   ring while focused, no cell fill; caption, badge bar, corner badges, offline band, veil, In
///   and Out bar per the spec's Look table.
/// - **AccessKit.** A `ListBox` named by `.label(..)`, multiselectable, the lead its
///   `active_descendant`; each built tile a `ListBoxOption` named by `accessible_name` or
///   [`TileLabels::tile_name`], `selected`, `position_in_set`, `size_of_set` (the full count),
///   disabled when not enabled, with Click, Focus and ShowContextMenu actions; group headers are
///   `Heading`s named title and detail.
/// - **RTL.** `Justified` rows fill from the right, Start badges sit at the right, arrows mirror,
///   the scrub fraction runs from the right.
pub struct TileGrid<'a, K: TextureKey, S: ?Sized> {
    id_salt: egui::IdSalt,
    source: &'a S,
    layout: TileLayout,
    groups: &'a [TileGroup],
    selection: Option<&'a mut Selection<K>>,
    thumbnails: Option<ThumbPair<'a, K>>,
    badges: BadgeVisibility,
    rating_keys: RatingKeys,
    space: SpaceAction,
    check_mode: bool,
    reorder: Option<Reorder<'a, K>>,
    hover_scrub: bool,
    type_ahead: bool,
    label: String,
    overlay: Option<CardOverlay<'a, K>>,
    labels: Option<TileLabels>,
    can_rename: Option<CanRename<'a, K>>,
    start_rename: Option<K>,
    strip: strip::Strip<'a, K>,
}

impl<'a, K, S> TileGrid<'a, K, S>
where
    K: TextureKey + std::fmt::Debug,
    S: TileSource<K> + ?Sized,
{
    /// A grid over `source`, `Cards { side: 128, aspect: 1, caption: true }` until `.layout`.
    pub fn new(id_salt: impl AsIdSalt, source: &'a S) -> Self {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:286 (`show_inner`).
        Self {
            id_salt: egui::IdSalt::new(id_salt),
            source,
            layout: TileLayout::Cards { side: 128.0, aspect: 1.0, caption: true },
            groups: &[],
            selection: None,
            thumbnails: None,
            badges: BadgeVisibility::default(),
            rating_keys: RatingKeys::default(),
            space: SpaceAction::default(),
            check_mode: false,
            reorder: None,
            hover_scrub: false,
            type_ahead: false,
            label: String::new(),
            overlay: None,
            labels: None,
            can_rename: None,
            start_rename: None,
            strip: strip::Strip::default(),
        }
    }

    /// The layout.
    #[must_use]
    pub fn layout(mut self, layout: TileLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Group headers, in display order.
    #[must_use]
    pub fn groups(mut self, groups: &'a [TileGroup]) -> Self {
        self.groups = groups;
        self
    }

    /// The selection the grid applies changes to (and reports them).
    #[must_use]
    pub fn selection(mut self, selection: &'a mut Selection<K>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// The thumbnails, keyed by the tile key.
    #[must_use]
    pub fn thumbnails(mut self, thumbnails: &'a Thumbnails<K>, source: &'a dyn ThumbSource<K>) -> Self {
        self.thumbnails = Some((thumbnails, source));
        self
    }

    /// When the badge bar shows (default `Auto`).
    #[must_use]
    pub fn badges(mut self, badges: BadgeVisibility) -> Self {
        self.badges = badges;
        self
    }

    /// The rating keys (default `CommandDigits`).
    #[must_use]
    pub fn rating_keys(mut self, keys: RatingKeys) -> Self {
        self.rating_keys = keys;
        self
    }

    /// What Space does (default `Preview`).
    #[must_use]
    pub fn space(mut self, space: SpaceAction) -> Self {
        self.space = space;
        self
    }

    /// Check mode (Import).
    #[must_use]
    pub fn check_mode(mut self, on: bool) -> Self {
        self.check_mode = on;
        self
    }

    /// Lets the user reorder tiles; the rule is the library's `Reorder` (axis ignored).
    #[must_use]
    pub fn reorder(mut self, reorder: Reorder<'a, K>) -> Self {
        self.reorder = Some(reorder);
        self
    }

    /// Reports `Scrub` while the pointer is over a picture.
    #[must_use]
    pub fn hover_scrub(mut self, on: bool) -> Self {
        self.hover_scrub = on;
        self
    }

    /// Type-ahead on tile names.
    #[must_use]
    pub fn type_ahead(mut self, on: bool) -> Self {
        self.type_ahead = on;
        self
    }

    /// The grid's accessible name.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// The host's drawing over each built card.
    #[must_use]
    pub fn card_overlay(mut self, overlay: impl FnMut(&mut Ui, &K, Rect) + 'a) -> Self {
        self.overlay = Some(Box::new(overlay));
        self
    }

    /// Whether tiles can be renamed in place (default false; needs a caption).
    #[must_use]
    pub fn renamable(mut self, renamable: bool) -> Self {
        self.can_rename = renamable.then(|| Box::new(|_: &K| true) as CanRename<'a, K>);
        self
    }

    /// Tiles can be renamed in place where `can_rename` says so: F2 on the lead, a slow click on
    /// the caption of the only selected tile, or [`start_rename`](Self::start_rename). Enter and
    /// blur commit (`Renamed` when the text changed, then `RenameEnded`); Esc cancels.
    #[must_use]
    pub fn can_rename(mut self, can_rename: impl Fn(&K) -> bool + 'a) -> Self {
        self.can_rename = Some(Box::new(can_rename));
        self
    }

    /// Opens the rename field of tile `key` (a context menu's Rename), committing any other open
    /// rename, where the tile can be renamed.
    #[must_use]
    pub fn start_rename(mut self, key: K) -> Self {
        self.start_rename = Some(key);
        self
    }

    /// The words (default: the installed `Words`).
    #[must_use]
    pub fn labels(mut self, labels: TileLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Marks tile `key` as the current one (Acrobat's current page, PowerPoint's current slide):
    /// a `tile.ring_current_width` ring in `tile.ring.current` (`accent.solid`) outside its
    /// picture's edge (PdfCraft's current page), drawn over the selection ring when it is selected; AccessKit `aria_current` (`AriaCurrent::Page`) on its option.
    /// It is not the lead: keys still start from the lead. A new current tile scrolls into view as
    /// little as it takes, except during a drag (Acrobat follows the page it shows).
    #[must_use]
    pub fn current(mut self, key: K) -> Self {
        self.strip.current = Some(key);
        self
    }

    /// Where captions go (default [`CaptionPlace::Below`]); `Start` also works with `Justified`
    /// rows, where each picture gets its column.
    #[must_use]
    pub fn caption_place(mut self, place: CaptionPlace) -> Self {
        self.strip.caption_place = place;
        self
    }

    /// Group headers with a disclosure chevron (`list::paint_disclosure`, as a tree row's) at the
    /// header's reading start; `open[g]` says whether group `g` shows its tiles. A closed group's
    /// tiles leave the arrangement, the keys and AccessKit (its header stays); a click on the
    /// chevron, or AccessKit Expand or Collapse on the header, reports
    /// [`TileEvent::GroupOpen`]; a click elsewhere on the header still picks the group. The
    /// header's AccessKit node gets `expanded`. Groups past the slice's end are open.
    #[must_use]
    pub fn group_disclosure(mut self, open: &'a [bool]) -> Self {
        self.strip.group_open = Some(open);
        self
    }

    /// A secondary click on a group header reports [`TileEvent::GroupContextMenu`] (PowerPoint's
    /// section menu) instead of the background menu.
    #[must_use]
    pub fn group_menus(mut self, on: bool) -> Self {
        self.strip.group_menus = on;
        self
    }

    /// A row after the last tile with `label` (DeckCraft's New Slide hint under the slide strip):
    /// the caption's text in `tile.detail` centred in a cell as wide as a tile, with the hover
    /// fill on hover. A click reports [`TileEvent::NewItem`]; it is a Tab stop of its own after
    /// the grid, an AccessKit `Button` named `label`, and Enter or Space on it report `NewItem`.
    #[must_use]
    pub fn new_item_row(mut self, label: impl Into<String>) -> Self {
        self.strip.new_item = Some(label.into());
        self
    }

    /// Files dragged from outside the app (egui's `hovered_files`) show the insert line in the
    /// gap under the pointer, as a reorder does, and [`TileGridOutput::file_gap`] reports the gap;
    /// the grid scrolls near its edges (`Reorder::auto_scroll`'s rule). The drop itself is the
    /// host's (egui's `dropped_files`).
    #[must_use]
    pub fn file_drops(mut self, on: bool) -> Self {
        self.strip.file_drops = on;
        self
    }

    /// Draws the grid in the space given and reports what happened.
    pub fn show(self, ui: &mut Ui) -> TileGridOutput<K> {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:286 (`show_inner`: layout, rows in view ± one viewport, cells, groups).
        show_grid(self, ui)
    }
}

/// How far the pointer must pinch (a log of the zoom factor) before the grid asks for one `Zoom` step.
const PINCH_STEP: f32 = 0.15;

/// What the cached arrangement was built for.
#[derive(Clone, PartialEq)]
struct Signature {
    len: usize,
    revision: u64,
    width: f32,
    layout: TileLayout,
    spacing: ArrangeSpacing,
    groups: Vec<TileGroup>,
    /// The width of the `Start` caption column; 0 for any other caption place.
    start_width: f32,
    /// The groups that are closed, by index; empty when none is.
    closed: Vec<bool>,
}

/// A marquee under way: where it began in content coordinates, and the selection it began with.
#[derive(Clone)]
struct Marquee<K> {
    start: egui::Vec2,
    base: Selection<K>,
    additive: bool,
    cancelled: bool,
}

/// What a grid keeps between passes, in egui memory under its id.
#[derive(Clone)]
struct State<K> {
    arrangement: Option<(Signature, Arc<TileArrangement>)>,
    /// The scroll offset of the last pass.
    offset: f32,
    /// The scroll area's view on the screen in the last pass.
    view: Option<Rect>,
    /// The column vertical moves keep.
    goal: Option<usize>,
    /// A tile to scroll into view.
    reveal: Option<usize>,
    /// Points to scroll by in the next pass (edge scrolling during a drag).
    drift: f32,
    /// The last tile checked in check mode, where a Shift range starts.
    last_checked: Option<K>,
    marquee: Option<Marquee<K>>,
    /// The tiles a drag carries while it is under way.
    drag: Option<Vec<K>>,
    /// Whether the window had the focus in the previous pass.
    window_focused: bool,
    /// Whether the window lost the focus in this pass.
    window_lost: bool,
    pinch: f32,
    /// The open rename.
    rename: Option<TileRename<K>>,
    /// A click on the caption of the only selected tile and when it happened: it opens the
    /// rename once no second click came within the double-click interval.
    slow_click: Option<(K, f64)>,
    /// The current key of the last pass, to tell a new current tile from an old one.
    current: Option<K>,
}

impl<K> Default for State<K> {
    fn default() -> Self {
        Self {
            arrangement: None,
            offset: 0.0,
            view: None,
            goal: None,
            reveal: None,
            drift: 0.0,
            last_checked: None,
            marquee: None,
            drag: None,
            window_focused: true,
            window_lost: false,
            pinch: 0.0,
            rename: None,
            slow_click: None,
            current: None,
        }
    }
}

impl<K: Clone + Send + Sync + 'static> State<K> {
    fn load(ctx: &egui::Context, id: Id) -> Self {
        ctx.data(|d| d.get_temp::<Self>(id)).unwrap_or_default()
    }

    fn store(self, ctx: &egui::Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}

/// `source`'s tile `at`, built.
fn content_at<K, S: TileSource<K> + ?Sized>(source: &S, at: usize) -> TileContent {
    let mut content = TileContent::default();
    source.content(at, &mut content);
    content
}

/// The tiles as the selection rules see them: a tile that is not enabled counts as a section, so
/// ranges, Select All and the arrows skip it. `content` is asked only for the tiles a rule reads.
struct Order<'a, S: ?Sized>(&'a S);

impl<K, S: TileSource<K> + ?Sized> DisplayOrder<K> for Order<'_, S> {
    fn len(&self) -> usize {
        DisplayOrder::<K>::len(self.0)
    }

    fn key(&self, at: usize) -> &K {
        self.0.key(at)
    }

    fn is_section(&self, at: usize) -> bool {
        !content_at::<K, S>(self.0, at).enabled
    }
}

/// The names of the tiles, for type-ahead.
struct Names {
    text: Vec<String>,
    enabled: Vec<bool>,
}

impl ListOptions for Names {
    fn len(&self) -> usize {
        self.text.len()
    }

    fn option_at(&self, at: usize) -> usize {
        at
    }

    fn enabled(&self, option: usize) -> bool {
        self.enabled.get(option).copied().unwrap_or(false)
    }

    fn text(&self, option: usize) -> Cow<'_, str> {
        Cow::Borrowed(self.text.get(option).map_or("", String::as_str))
    }
}

/// Arrangement coordinates to the screen: the content's corner, mirrored in a right-to-left
/// interface (`arrange` returns left-to-right rects).
#[derive(Clone, Copy)]
struct Mapper {
    origin: Pos2,
    width: f32,
    rtl: bool,
}

impl Mapper {
    fn rect(&self, r: Rect) -> Rect {
        let x = if self.rtl { self.origin.x + self.width - r.max.x } else { self.origin.x + r.min.x };
        Rect::from_min_size(pos2(x, self.origin.y + r.min.y), r.size())
    }
}

/// What a pointer or an assistive technology did to one tile.
enum HitKind {
    /// A click; `on_caption` when it landed on the caption under the picture.
    Click {
        modifiers: Modifiers,
        on_caption: bool,
    },
    /// A click on the stars: the rating to set (0 clears).
    Rate(u8),
    Double,
    Menu(Option<Pos2>),
    DragStart,
    Focus,
    Scrub(f32),
}

struct Hit<K> {
    at: usize,
    key: K,
    rect: Rect,
    kind: HitKind,
}

fn apply<K: Clone>(selection: &mut Selection<K>, events: &mut Vec<TileEvent<K>>, change: SelectionChange<K>) {
    *selection = change.selection.clone();
    events.push(TileEvent::Selection(change));
}

fn change_of<K: Clone + PartialEq>(old: &Selection<K>, pick: SelectionPick, row: Option<K>, new: Selection<K>) -> Option<SelectionChange<K>> {
    (new != *old).then_some(SelectionChange { pick, row, selection: new })
}

/// A key's tile ids derive from the grid's.
fn tile_id<K: std::hash::Hash + std::fmt::Debug>(grid: Id, key: &K) -> Id {
    grid.with(("tile", key))
}

/// The scroll offset that shows row `row` of `arr` in a view `height` high, below the pinned
/// header when there are groups.
fn reveal_offset(arr: &TileArrangement, style: &TileStyle, grouped: bool, row: usize, offset: f32, height: f32) -> Option<f32> {
    let row = arr.rows.get(row)?;
    let top_margin = if grouped { style.group_header + style.gap } else { style.inset };
    if row.top - top_margin < offset {
        Some((row.top - top_margin).max(0.0))
    } else if row.top + row.height + style.inset > offset + height {
        Some(row.top + row.height + style.inset - height)
    } else {
        None
    }
}

/// Spans of the tiles in one row, in reading order, for `Reorder::resolve`; every other tile is
/// out of reach on its side of the row.
struct RowReach {
    len: usize,
    row: Range<usize>,
    spans: Vec<Range<f32>>,
}

impl RowSpans for RowReach {
    fn len(&self) -> usize {
        self.len
    }

    fn span(&self, at: usize) -> Range<f32> {
        if at < self.row.start {
            f32::MIN..f32::MIN
        } else if at >= self.row.end {
            f32::MAX..f32::MAX
        } else {
            self.spans[at - self.row.start].clone()
        }
    }
}

/// Where a drag at `pointer` would drop: the tile and side, resolved by the host's `Reorder`
/// along the row under the pointer.
fn drop_target<K: Clone + PartialEq>(
    reorder: &Reorder<'_, K>,
    order: &(impl DisplayOrder<K> + ?Sized),
    arr: &TileArrangement,
    mapper: &Mapper,
    pointer: Pos2,
    dragged: &[K],
    modifiers: Modifiers,
) -> Option<(usize, Move<K>)> {
    let y = pointer.y - mapper.origin.y;
    let row = arr.rows.partition_point(|r| r.top + r.height < y).min(arr.rows.len().checked_sub(1)?);
    let tiles = arr.rows[row].tiles.clone();
    let sign = if mapper.rtl { -1.0 } else { 1.0 };
    let spans = tiles
        .clone()
        .map(|i| {
            let r = mapper.rect(arr.tiles[i]);
            if mapper.rtl { -r.max.x..-r.min.x } else { r.min.x..r.max.x }
        })
        .collect();
    let reach = RowReach { len: order.len(), row: tiles, spans };
    let resolved = reorder.resolve(order, &reach, pointer.x * sign, dragged, modifiers)?;
    Some((resolved.at, resolved.mv))
}

/// The grid and what it works on for one pass.
struct Pass<'a, 's, K: TextureKey, S: ?Sized> {
    ctx: egui::Context,
    id: Id,
    source: &'a S,
    order: Order<'a, S>,
    rules: ListSelection,
    style: TileStyle,
    labels: TileLabels,
    groups: &'a [TileGroup],
    captioned: bool,
    rtl: bool,
    mac: bool,
    enabled: bool,
    focused: bool,
    len: usize,
    check_mode: bool,
    space: SpaceAction,
    rating_keys: RatingKeys,
    badges: BadgeVisibility,
    hover_scrub: bool,
    can_rename: Option<CanRename<'a, K>>,
    thumbnails: Option<ThumbPair<'a, K>>,
    reorder: Option<Reorder<'a, K>>,
    arr: Arc<TileArrangement>,
    width: f32,
    state: State<K>,
    selection: &'s mut Selection<K>,
    events: Vec<TileEvent<K>>,
    gesture: Option<ReorderGesture<K>>,
    /// Where a drag of this grid's tiles would drop this pass.
    drop_target: Option<Move<K>>,
    strip: strip::Strip<'a, K>,
    /// The width of the `Start` caption column; 0 for any other caption place.
    start_width: f32,
    /// The positions of the tiles in open groups, when a group is closed.
    visible: Option<Vec<usize>>,
    /// The height the new item row adds below the arrangement.
    extra: f32,
    /// Where files hovering over the grid would drop this pass.
    file_gap: Option<usize>,
}

/// What the scroll area's contents found out.
struct Drawn<K> {
    mapper: Mapper,
    background: Response,
    hits: Vec<Hit<K>>,
    group_clicks: Vec<(usize, Modifiers)>,
    /// The id of the lead's tile when it was built this pass.
    lead_tile: Option<Id>,
    /// What the rename field showed, when the renamed tile was built.
    rename: Option<TileRenameOutcome>,
    /// The disclosure and menu events of the group headers.
    group_events: Vec<TileEvent<K>>,
    /// Whether the new item row was activated.
    new_item: bool,
}

/// What one tile draws into while the rows are built.
struct Rows<K> {
    mapper: Mapper,
    modifiers: Modifiers,
    dragging: bool,
    hits: Vec<Hit<K>>,
    lead_tile: Option<Id>,
    rename: Option<TileRenameOutcome>,
}

fn show_grid<K, S>(grid: TileGrid<'_, K, S>, ui: &mut Ui) -> TileGridOutput<K>
where
    K: TextureKey + std::fmt::Debug,
    S: TileSource<K> + ?Sized,
{
    let TileGrid {
        id_salt,
        source,
        layout,
        groups,
        selection,
        thumbnails,
        badges,
        rating_keys,
        space,
        check_mode,
        reorder,
        hover_scrub,
        type_ahead,
        label,
        mut overlay,
        labels,
        can_rename,
        start_rename,
        strip,
    } = grid;
    // A grid whose host gives no selection keeps its own.
    let own_id = ui.make_persistent_id(id_salt).with("own selection");
    let mut own: Selection<K> = ui.ctx().data(|d| d.get_temp(own_id)).unwrap_or_default();
    let selection: &mut Selection<K> = match selection {
        Some(selection) => selection,
        None => &mut own,
    };
    let mut pass = Pass::new(ui, id_salt, source, layout, groups, selection, labels, strip);
    pass.thumbnails = thumbnails;
    pass.badges = badges;
    pass.rating_keys = rating_keys;
    pass.space = space;
    pass.check_mode = check_mode;
    pass.hover_scrub = hover_scrub;
    pass.can_rename = can_rename;
    pass.reorder = reorder;
    pass.gesture = pass.reorder.as_ref().map(|_| ReorderGesture::load(ui, pass.id));

    let menu_key = pass.keys(ui, type_ahead);
    if let Some(key) = start_rename {
        pass.start_rename(key);
    }
    pass.slow_click_due();
    let dragging = DragAndDrop::payload::<ListDrag<K>>(&pass.ctx).is_some_and(|p| p.list == pass.id);
    pass.follow_current(dragging);
    let scroll_to = pass.scroll_request(ui);
    let builder = UiBuilder::new()
        .scope_id(pass.id)
        .sense(Sense::CLICK | Sense::FOCUSABLE)
        .accessibility_role(Role::ListBox)
        .accessibility_label(label)
        .layout(egui::Layout::top_down(egui::Align::Min));
    let scope = ui.scope_builder(builder, |ui| {
        let mut area = ScrollArea::vertical()
            .id_salt("scroll")
            .auto_shrink([false, false])
            .content_margin(0.0)
            .scroll_source(ScrollSource::SCROLL_BAR)
            .vertical_scroll_bar_label(ScrollBarLabels::name(ui));
        if let Some(offset) = scroll_to {
            area = area.vertical_scroll_offset(offset);
        }
        area.show_viewport(ui, |ui, viewport| pass.draw(ui, viewport, dragging, &mut overlay))
    });
    let (response, area) = (scope.response, scope.inner);
    pass.state.offset = area.state.clamped_offset().y;
    pass.state.view = Some(area.inner_rect);
    let view = area.inner_rect;
    let drawn = area.inner;
    let lead_tile = drawn.lead_tile;
    pass.rename_shown(drawn.rename);

    let modifiers = ui.input(|i| i.modifiers);
    for hit in drawn.hits {
        pass.hit(hit, modifiers);
    }
    pass.events.extend(drawn.group_events);
    if drawn.new_item {
        pass.events.push(TileEvent::NewItem);
    }
    pass.group_picks(drawn.group_clicks);
    pass.background(&drawn.background, view, &drawn.mapper, menu_key);
    pass.marquee(ui, &drawn.background, view, &drawn.mapper, dragging, modifiers);
    pass.drag(ui, view, &drawn.mapper);
    pass.file_drop(ui, view, &drawn.mapper);
    let output = pass.finish(ui, &response, lead_tile);
    ui.ctx().data_mut(|d| d.insert_temp(own_id, own));
    output
}

impl<'a, 's, K, S> Pass<'a, 's, K, S>
where
    K: TextureKey + std::fmt::Debug,
    S: TileSource<K> + ?Sized,
{
    #[expect(clippy::too_many_arguments, reason = "what the builders set, passed on as the grid hands it over")]
    fn new(
        ui: &Ui,
        id_salt: egui::IdSalt,
        source: &'a S,
        layout: TileLayout,
        groups: &'a [TileGroup],
        selection: &'s mut Selection<K>,
        labels: Option<TileLabels>,
        strip: strip::Strip<'a, K>,
    ) -> Self {
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let style = TileStyle::style(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let mut state = State::<K>::load(&ctx, id);
        let len = DisplayOrder::<K>::len(source);

        // The arrangement, rebuilt only when what it depends on changed.
        // A solid scroll bar takes its width from the area's inner rect, which never shrinks below
        // the content: the arrangement leaves the bar's room out, or the bar lands past the edge.
        let width = (ui.available_width() - ui.spacing().scroll.allocated_width()).max(1.0);
        let caption_line = tokens.typography.get(TypeRole::Caption).line_height;
        let spacing = ArrangeSpacing {
            gap: style.gap,
            inset: style.inset,
            group_header: style.group_header,
            caption: 2.0 * caption_line + 2.0 * style.caption_gap,
        };
        let start_width = strip.start_width(&style, layout);
        let closed = strip.closed(groups);
        let signature = Signature { len, revision: source.aspects_revision(), width, layout, spacing, groups: groups.to_vec(), start_width, closed };
        if state.arrangement.as_ref().is_none_or(|(old, _)| *old != signature) {
            let aspects: Vec<f32> = (0..len).map(|at| source.aspect(at)).collect();
            let arrangement = strip::arrange_strip(&aspects, groups, &signature.closed, width, layout, spacing, start_width);
            state.arrangement = Some((signature, Arc::new(arrangement)));
        }
        let arr = state.arrangement.as_ref().map(|(_, a)| a.clone()).unwrap_or_default();
        let visible =
            state.arrangement.as_ref().filter(|(s, _)| s.closed.contains(&true)).map(|_| arr.rows.iter().flat_map(|r| r.tiles.clone()).collect());
        let extra = if strip.new_item.is_some() { style.gap + style.group_header } else { 0.0 };
        let window_focused = ctx.input(|i| i.focused);
        let window_lost = state.window_focused && !window_focused;
        state.window_focused = window_focused;
        state.window_lost = window_lost;
        Pass {
            focused: ui.memory(|m| m.has_focus(id)),
            enabled: ui.is_enabled(),
            rtl: direction::is_rtl(ui),
            mac: matches!(ctx.os(), OperatingSystem::Mac | OperatingSystem::IOS),
            labels: labels.unwrap_or_else(|| TileLabels::of(ui)),
            captioned: start_width == 0.0 && matches!(layout, TileLayout::Cards { caption: true, .. }),
            ctx,
            id,
            source,
            order: Order(source),
            rules: ListSelection::default(),
            style,
            groups,
            len,
            check_mode: false,
            space: SpaceAction::default(),
            rating_keys: RatingKeys::default(),
            badges: BadgeVisibility::default(),
            hover_scrub: false,
            can_rename: None,
            thumbnails: None,
            reorder: None,
            arr,
            width,
            state,
            selection,
            events: Vec::new(),
            gesture: None,
            drop_target: None,
            strip,
            start_width,
            visible,
            extra,
            file_gap: None,
        }
    }

    fn lead_at(&self) -> Option<usize> {
        self.selection.lead.as_ref().and_then(|k| self.order.position(k))
    }

    fn apply(&mut self, change: SelectionChange<K>) {
        apply(self.selection, &mut self.events, change);
    }

    fn set_lead(&mut self, key: K) {
        if self.selection.lead.as_ref() != Some(&key) {
            self.selection.lead = Some(key.clone());
            self.events.push(TileEvent::LeadMoved(key));
        }
    }

    fn focus(&self) {
        self.ctx.memory_mut(|m| m.request_focus(self.id));
    }

    /// The rows a page key moves over.
    fn page(&self) -> usize {
        let height = self.state.view.map_or(0.0, |v| v.height());
        let row = self.arr.rows.first().map_or(1.0, |r| r.height + self.style.gap).max(1.0);
        ((height / row).floor() as usize).max(1)
    }

    /// Whether Esc empties the selection: tiles are picked and no marquee or drag is under way
    /// (those take Esc themselves).
    fn takes_escape(&self) -> bool {
        !self.selection.items.is_empty() && self.state.marquee.is_none() && !DragAndDrop::has_any_payload(&self.ctx)
    }

    /// Takes the keys the grid owns from the input and applies them. Returns whether the context
    /// menu was asked for.
    fn keys(&mut self, ui: &Ui, type_ahead: bool) -> bool {
        if !(self.focused && self.enabled) {
            return false;
        }
        let nav = crate::behavior::GridNav::ragged(self.arr.row_lengths()).rtl(self.rtl).page(self.page());
        let listbox = type_ahead.then(|| Listbox::load(ui, self.id.with("type-ahead")));
        let typing = listbox.as_ref().is_some_and(|l| l.typing(ui));
        let grid_keys = KeyGrid {
            mac: self.mac,
            nav: &nav,
            lead: self.lead_at().and_then(|at| self.nav_index(at)),
            goal: self.state.goal,
            space: self.space,
            check_mode: self.check_mode,
            rating: self.rating_keys,
            clearable: self.takes_escape(),
            renamable: self.selection.lead.as_ref().is_some_and(|lead| self.renamable(lead)),
        };
        let stacks = self.reorder.is_some();
        let mac = self.mac;
        let mut commands: Vec<Command> = Vec::new();
        ui.input_mut(|i| {
            i.events.retain(|event| {
                let Event::Key { key, pressed: true, modifiers, .. } = event else { return true };
                let stack = stacks.then(|| StackKey::from_key(*key, *modifiers, mac)).flatten();
                let action = if stack.is_some() { None } else { keys::action(*key, *modifiers, &grid_keys) };
                // Space belongs to a type-ahead prefix under way.
                let typed_space = typing && *key == Key::Space && modifiers.is_none();
                if (stack.is_none() && action.is_none()) || typed_space {
                    return true;
                }
                commands.push(match (action, stack) {
                    (_, Some(stack)) => Command::Stack(stack, *modifiers),
                    (Some(action), None) => Command::Grid(action),
                    (None, None) => return true,
                });
                false
            });
        });
        let mut menu = false;
        for command in commands {
            match command {
                Command::Stack(key, modifiers) => self.stack_key(key, modifiers),
                Command::Grid(GridKeyAction::Menu) => menu = true,
                Command::Grid(action) => self.command(action),
            }
        }
        if let Some(mut listbox) = listbox {
            self.type_ahead(ui, &mut listbox);
            listbox.store(ui);
        }
        menu
    }

    fn stack_key(&mut self, key: StackKey, modifiers: Modifiers) {
        let Some(mv) = self.reorder.as_ref().and_then(|reorder| reorder.key_move(&self.order, &self.selection.items, key, modifiers)) else { return };
        if let Some(gesture) = &mut self.gesture {
            gesture.set(mv);
            self.state.reveal = self.lead_at();
        }
    }

    fn type_ahead(&mut self, ui: &Ui, listbox: &mut Listbox) {
        if !ui.input(|i| i.events.iter().any(|e| matches!(e, Event::Text(_)))) {
            return;
        }
        let names = Names {
            text: (0..self.len).map(|p| content_at::<K, S>(self.source, p).name).collect(),
            enabled: (0..self.len).map(|p| !self.order.is_section(p) && self.nav_index(p).is_some()).collect(),
        };
        listbox.set_active(self.lead_at());
        if listbox.type_ahead(ui, &names)
            && let Some(at) = listbox.active()
        {
            self.state.reveal = Some(at);
            if let Some(change) = self.rules.click(self.selection, &self.order, at, Modifiers::NONE) {
                self.apply(change);
            }
        }
    }

    /// One key's action (the menu is the caller's).
    fn command(&mut self, action: GridKeyAction) {
        match action {
            GridKeyAction::Move { to, goal, extend, lead_only, .. } => self.move_lead(self.nav_position(to), goal, extend, lead_only),
            GridKeyAction::SelectAll => self.select_all(),
            GridKeyAction::Activate => self.events.extend(self.selection.lead.clone().map(TileEvent::Activate)),
            GridKeyAction::Preview => self.events.extend(self.selection.lead.clone().map(TileEvent::Preview)),
            GridKeyAction::ToggleLead => self.toggle_lead(),
            GridKeyAction::Remove => {
                // With nothing picked the key removes the current tile.
                let keys = if self.selection.items.is_empty() { self.strip.current.iter().cloned().collect() } else { self.selection.items.clone() };
                if !keys.is_empty() {
                    self.events.push(TileEvent::Remove(keys));
                }
            }
            GridKeyAction::Clear => {
                if let Some(change) = change_of(
                    self.selection,
                    SelectionPick::Clear,
                    None,
                    Selection { items: Vec::new(), lead: self.selection.lead.clone(), anchor: None },
                ) {
                    self.apply(change);
                }
            }
            GridKeyAction::Rate(rating) => {
                let keys: Vec<K> =
                    if self.selection.items.is_empty() { self.selection.lead.iter().cloned().collect() } else { self.selection.items.clone() };
                if !keys.is_empty() {
                    self.events.push(TileEvent::Rate { keys, rating });
                }
            }
            GridKeyAction::Zoom(step) => self.events.push(TileEvent::Zoom(step)),
            GridKeyAction::Rename => {
                if let Some(lead) = self.selection.lead.clone() {
                    self.start_rename(lead);
                }
            }
            GridKeyAction::Menu => {}
        }
    }

    /// Whether tile `key` can be renamed here: the host allows it and the tile has a caption.
    fn renamable(&self, key: &K) -> bool {
        self.captioned
            && self.can_rename.as_ref().is_some_and(|can| can(key))
            && self.order.position(key).is_some_and(|at| !self.order.is_section(at))
    }

    /// Opens the rename of `key` where it can be renamed, committing another open one.
    fn start_rename(&mut self, key: K) {
        if !self.renamable(&key) || self.state.rename.as_ref().is_some_and(|open| open.key == key) {
            return;
        }
        if let Some(open) = self.state.rename.take() {
            self.end_rename(open, RenameEnd::Blur);
        }
        let at = self.order.position(&key);
        let name = at.map(|at| content_at::<K, S>(self.source, at).name).unwrap_or_default();
        self.state.rename = Some(TileRename { key, text: name, fresh: true });
        self.state.reveal = at;
    }

    /// Closes a rename with `end`: a changed text is reported unless Esc ended it.
    fn end_rename(&mut self, open: TileRename<K>, end: RenameEnd) {
        let name = self.order.position(&open.key).map(|at| content_at::<K, S>(self.source, at).name);
        if end != RenameEnd::Escape && end != RenameEnd::Removed && name.is_some_and(|name| name != open.text) {
            self.events.push(TileEvent::Renamed { key: open.key.clone(), text: open.text });
        }
        self.events.push(TileEvent::RenameEnded { key: open.key, end });
    }

    /// Takes what the rename field showed this pass: the text typed, and its end.
    fn rename_shown(&mut self, outcome: Option<TileRenameOutcome>) {
        let Some(mut open) = self.state.rename.take() else { return };
        let Some(shown) = outcome else {
            // The tile is gone, or out of reach of the scroll area: the rename ends.
            let end = if self.order.position(&open.key).is_some() { RenameEnd::Blur } else { RenameEnd::Removed };
            self.end_rename(open, end);
            return;
        };
        open.text = shown.text;
        open.fresh = false;
        match shown.end {
            Some(end) => {
                self.end_rename(open, end);
                self.focus();
            }
            None => self.state.rename = Some(open),
        }
    }

    /// Notes a click on the caption of the only selected tile (or clears the note).
    fn slow_click(&mut self, key: Option<K>) {
        let now = self.ctx.input(|i| i.time);
        self.state.slow_click = key.map(|key| (key, now));
    }

    /// Opens the rename a slow click asked for, once the double-click interval passed without a
    /// second click and the tile is still the only selected one.
    fn slow_click_due(&mut self) {
        let Some((key, at)) = self.state.slow_click.clone() else { return };
        if self.selection.items != [key.clone()] {
            self.state.slow_click = None;
            return;
        }
        let delay = self.ctx.options(|o| o.input_options.max_double_click_delay);
        let waited = self.ctx.input(|i| i.time) - at;
        if waited >= delay {
            self.state.slow_click = None;
            self.start_rename(key);
        } else {
            self.ctx.request_repaint_after(std::time::Duration::from_secs_f64(delay - waited));
        }
    }

    /// Where tile `at` is among the tiles the arrow keys reach (every tile unless a group is closed);
    /// `None` for a tile in a closed group.
    fn nav_index(&self, at: usize) -> Option<usize> {
        match &self.visible {
            Some(visible) => visible.binary_search(&at).ok(),
            None => Some(at),
        }
    }

    /// The tile at `index` of the tiles the arrow keys reach.
    fn nav_position(&self, index: usize) -> usize {
        self.visible.as_ref().and_then(|v| v.get(index)).copied().unwrap_or(index)
    }

    fn move_lead(&mut self, to: usize, goal: usize, extend: bool, lead_only: bool) {
        let from = self.lead_at().unwrap_or(0);
        // Skip the tiles that are not enabled or hidden by a closed group, in the direction of the move.
        let reachable = |p: &usize| !self.order.is_section(*p) && self.nav_index(*p).is_some();
        let found = if to >= from { (to..self.len).find(reachable) } else { (0..=to).rev().find(reachable) };
        let Some(to) = found else { return };
        self.state.goal = Some(goal);
        self.state.reveal = Some(to);
        let key = self.source.key(to).clone();
        if lead_only || self.check_mode {
            self.set_lead(key);
        } else if let Some(change) = self.rules.click(self.selection, &self.order, to, Modifiers { shift: extend, ..Modifiers::NONE }) {
            self.apply(change);
        } else {
            self.set_lead(key);
        }
    }

    fn select_all(&mut self) {
        let items: Vec<K> =
            (0..self.len).filter(|&p| !self.order.is_section(p) && self.nav_index(p).is_some()).map(|p| self.source.key(p).clone()).collect();
        let lead = self.selection.lead.clone().or_else(|| items.first().cloned());
        let new = Selection { items, lead: lead.clone(), anchor: self.selection.anchor.clone().or_else(|| lead.clone()) };
        if let Some(change) = change_of(self.selection, SelectionPick::Range, lead, new) {
            self.apply(change);
        }
    }

    fn toggle_lead(&mut self) {
        let Some(at) = self.lead_at().filter(|&at| !self.order.is_section(at)) else { return };
        let change = if self.check_mode {
            check_toggle(self.selection, &self.order, at, false, None)
        } else {
            self.rules.toggle_lead(self.selection, &self.order)
        };
        if let Some(change) = change {
            self.state.last_checked = Some(self.source.key(at).clone());
            self.apply(change);
        }
    }

    /// The scroll offset this pass asks for: a tile to reveal, a drag at an edge, the wheel. The
    /// pinch asks for a `Zoom`.
    fn scroll_request(&mut self, ui: &Ui) -> Option<f32> {
        let view_height = self.state.view.map_or(0.0, |v| v.height());
        let max_offset = (self.arr.height + self.extra - view_height).max(0.0);
        let mut scroll_to: Option<f32> = None;
        if let Some(at) = self.state.reveal.take()
            && let Some(row) = self.arr.row_of(at)
        {
            scroll_to = reveal_offset(&self.arr, &self.style, !self.groups.is_empty(), row, self.state.offset, view_height);
        }
        if self.state.drift != 0.0 {
            scroll_to = Some((scroll_to.unwrap_or(self.state.offset) + self.state.drift).clamp(0.0, max_offset));
            self.state.drift = 0.0;
            self.ctx.request_repaint();
        }
        let over = self.enabled && self.state.view.is_some_and(|v| ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| v.contains(p)));
        if !over {
            return scroll_to;
        }
        let share = self.style.wheel_share;
        let mut delta = 0.0;
        ui.input(|i| {
            for event in &i.events {
                if let Event::MouseWheel { unit, delta: d, modifiers, .. } = event
                    && !modifiers.command
                {
                    delta += match unit {
                        egui::MouseWheelUnit::Point => d.y,
                        egui::MouseWheelUnit::Line => d.y * share * view_height,
                        egui::MouseWheelUnit::Page => d.y * view_height,
                    };
                }
            }
        });
        if delta != 0.0 {
            scroll_to = Some((scroll_to.unwrap_or(self.state.offset) - delta).clamp(0.0, max_offset));
        }
        let zoom = ui.input(|i| i.zoom_delta());
        if zoom != 1.0 && zoom.is_finite() && zoom > 0.0 {
            self.state.pinch += zoom.ln();
            if self.state.pinch.abs() >= PINCH_STEP {
                self.events.push(TileEvent::Zoom(if self.state.pinch > 0.0 { ZoomStep::In } else { ZoomStep::Out }));
                self.state.pinch = 0.0;
            }
        }
        scroll_to
    }

    /// The scroll area's contents: the background, the group headers and the tiles in view, then
    /// the pinned header.
    fn draw(&self, ui: &mut Ui, viewport: Rect, dragging: bool, overlay: &mut Option<CardOverlay<'a, K>>) -> Drawn<K> {
        let width = self.width;
        let (content, _) = ui.allocate_exact_size(vec2(width, (self.arr.height + self.extra).max(viewport.height())), Sense::hover());
        let mapper = Mapper { origin: content.min, width, rtl: self.rtl };
        let background = ui.interact(content, self.id.with("background"), Sense::CLICK | Sense::DRAG);
        let mut group_clicks = Vec::new();
        let mut group_events = Vec::new();

        let first = self.arr.headers.partition_point(|h| h.bottom() < viewport.top());
        for (index, header) in self.arr.headers.iter().enumerate().skip(first).take_while(|(_, h)| h.top() <= viewport.bottom()) {
            self.group_header(ui, &mapper, index, *header, false, &mut group_clicks, &mut group_events);
        }

        // Tiles in view are drawn; those within a viewport of it are built, so their pictures are
        // asked for at `NEAR`.
        let mut rows = Rows { mapper, modifiers: ui.input(|i| i.modifiers), dragging, hits: Vec::new(), lead_tile: None, rename: None };
        let reach = viewport.height();
        for row_index in self.arr.rows_between(viewport.top() - reach, viewport.bottom() + reach) {
            let row = &self.arr.rows[row_index];
            for at in row.tiles.clone() {
                self.tile(ui, &mut rows, at, row, overlay);
            }
        }

        let at_top = self.arr.headers.partition_point(|h| h.top() <= viewport.top()).checked_sub(1);
        if let Some((index, rect)) = groups::pinned_rect(&self.arr.headers, viewport, at_top) {
            self.group_header(ui, &mapper, index, rect, true, &mut group_clicks, &mut group_events);
        }
        let new_item = self.new_item_row(ui, &mapper);
        Drawn { mapper, background, hits: rows.hits, group_clicks, lead_tile: rows.lead_tile, rename: rows.rename, group_events, new_item }
    }

    #[expect(clippy::too_many_arguments, reason = "the header's place and the two lists it reports to")]
    fn group_header(
        &self,
        ui: &mut Ui,
        mapper: &Mapper,
        index: usize,
        header: Rect,
        pinned: bool,
        clicks: &mut Vec<(usize, Modifiers)>,
        events: &mut Vec<TileEvent<K>>,
    ) {
        let Some(group) = self.groups.get(index) else { return };
        let disclosure = self.strip.group_open.map(|open| open.get(index).copied().unwrap_or(true));
        let out = groups::group_header(ui, mapper.rect(header), group, pinned, &self.style, disclosure);
        if let Some(open) = out.toggled {
            events.push(TileEvent::GroupOpen { group: index, open });
        } else if out.response.clicked() {
            clicks.push((index, ui.input(|i| i.modifiers)));
        }
        if self.strip.group_menus && out.response.secondary_clicked() {
            self.focus();
            events.push(TileEvent::GroupContextMenu { group: index, anchor: out.response.rect, pointer: out.response.interact_pointer_pos() });
        }
    }

    /// Builds tile `at` of `row`: asks for its picture, draws it when in view and reports what
    /// the pointer did to it.
    fn tile(&self, ui: &mut Ui, rows: &mut Rows<K>, at: usize, row: &TileRow, overlay: &mut Option<CardOverlay<'a, K>>) {
        let key = self.source.key(at).clone();
        let content = content_at::<K, S>(self.source, at);
        let picture = rows.mapper.rect(self.arr.tiles[at]);
        let start = self.start_width > 0.0;
        let caption = if start {
            Some(rows.mapper.rect(strip::start_column(self.arr.tiles[at], self.start_width)))
        } else {
            self.captioned.then(|| {
                Rect::from_min_max(pos2(picture.left(), picture.bottom()), pos2(picture.right(), rows.mapper.origin.y + row.top + row.height))
            })
        };
        let cell = caption.map_or(picture, |c| picture.union(c));
        let in_view = cell.bottom() >= ui.clip_rect().top() && cell.top() <= ui.clip_rect().bottom();
        let texture = self.thumbnails.map(|(thumbs, source)| {
            let request = ThumbRequest::for_cell(picture.width().max(picture.height()), self.ctx.pixels_per_point());
            thumbs.state(&self.ctx, source, &key, &request, if in_view { Priority::VISIBLE } else { Priority::NEAR })
        });
        if !in_view {
            return;
        }
        let tile = tile_id(self.id, &key);
        let selected = self.selection.contains(&key);
        let response = Interactive::with_id(tile, Role::ListBoxOption, accessible_name(&self.ctx, &content, &self.labels, None))
            .sense(Sense::CLICK | Sense::DRAG)
            .selected(selected)
            .position_in_set(at)
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, cell)
            .finish(ui);
        self.ctx.accesskit_node_builder(tile, |node| {
            node.set_size_of_set(self.len);
            if self.strip.current.as_ref() == Some(&key) {
                node.set_aria_current(AriaCurrent::Page);
            }
            node.add_action(Action::Focus);
            node.add_action(Action::ShowContextMenu);
            if !content.enabled {
                node.set_disabled();
            }
        });
        let is_lead = self.selection.lead.as_ref() == Some(&key);
        if is_lead {
            rows.lead_tile = Some(tile);
        }
        let scrub = self.scrub_fraction(ui, rows, &response, picture, &content);
        let renaming = self.state.rename.as_ref().filter(|r| r.key == key && !start);
        let look = TileLook {
            content: &content,
            picture,
            caption: caption.filter(|_| renaming.is_none()),
            caption_place: if start { CaptionPlace::Start } else { CaptionPlace::Below },
            selected,
            hovered: response.hovered() && !rows.dragging,
            lead_focused: self.focused && is_lead,
            checked: self.check_mode.then_some(content.enabled && selected),
            badges: self.badges,
            scrub,
            rtl: self.rtl,
            response: &response,
            face: TileFace::Picture,
        };
        let stars = paint_tile(ui, &self.style, &self.labels, &look, self.thumbnails.map(|(t, _)| t), texture.as_ref());
        if self.strip.current.as_ref() == Some(&key) {
            strip::ring_current(ui.painter(), &self.style, picture);
        }
        if let (Some(rename), Some(band)) = (renaming, caption) {
            rows.rename = Some(show_tile_rename(ui, tile, band, &self.labels.rename, rename));
        }
        if let Some(overlay) = overlay.as_mut() {
            overlay(ui, &key, picture);
        }
        let mark_tip = start.then(|| strip::mark_tip(ui, &Tokens::of(ui), &self.style, &content, caption)).flatten();
        if let Some(tip) = mark_tip.or_else(|| tooltip(&content, texture.as_ref())) {
            response.clone().on_hover_text(tip);
        }
        let mut found = Vec::new();
        if let Some(fraction) = scrub {
            found.push(HitKind::Scrub(fraction));
        }
        found.extend(self.pointer_hits(ui, &response, stars, content.rating, caption.filter(|_| !start), rows.modifiers));
        for kind in found {
            rows.hits.push(Hit { at, key: key.clone(), rect: cell, kind });
        }
    }

    /// The fraction across the picture from the reading start under the pointer, with scrub on.
    fn scrub_fraction(&self, ui: &Ui, rows: &Rows<K>, response: &Response, picture: Rect, content: &TileContent) -> Option<f32> {
        if !(self.hover_scrub && !rows.dragging && content.enabled && response.hovered()) {
            return None;
        }
        let p = ui.input(|i| i.pointer.hover_pos()).filter(|p| picture.contains(*p))?;
        let fraction = ((p.x - picture.left()) / picture.width().max(1.0)).clamp(0.0, 1.0);
        Some(if self.rtl { 1.0 - fraction } else { fraction })
    }

    /// What the pointer or an assistive technology did to a tile this pass.
    fn pointer_hits(
        &self,
        ui: &Ui,
        response: &Response,
        stars: Option<Rect>,
        rating: Option<u8>,
        caption: Option<Rect>,
        modifiers: Modifiers,
    ) -> Vec<HitKind> {
        let mut found = Vec::new();
        if response.double_clicked() {
            found.push(HitKind::Double);
        } else if response.clicked() {
            let pointer = response.interact_pointer_pos();
            found.push(match (stars, rating, pointer) {
                (Some(area), Some(current), Some(p)) if area.contains(p) => {
                    let n = (((p.x - area.left()) / (area.width() / 5.0).max(1.0)).floor() as i32 + 1).clamp(1, 5) as u8;
                    HitKind::Rate(if n == current { 0 } else { n })
                }
                _ => HitKind::Click { modifiers, on_caption: caption.zip(pointer).is_some_and(|(c, p)| c.contains(p)) },
            });
        }
        if response.secondary_clicked() {
            found.push(HitKind::Menu(response.interact_pointer_pos()));
        }
        if ui.input(|i| i.has_accesskit_action_request(response.id, Action::ShowContextMenu)) {
            found.push(HitKind::Menu(None));
        }
        if response.drag_started_by(PointerButton::Primary) {
            found.push(HitKind::DragStart);
        }
        if ui.input(|i| i.has_accesskit_action_request(response.id, Action::Focus)) {
            found.push(HitKind::Focus);
        }
        found
    }

    /// Applies what was done to one tile.
    fn hit(&mut self, hit: Hit<K>, modifiers: Modifiers) {
        let Hit { at, key, rect, kind } = hit;
        let tile_enabled = !self.order.is_section(at);
        if !matches!(kind, HitKind::Scrub(_) | HitKind::Focus) {
            self.state.slow_click = None;
        }
        match kind {
            HitKind::Scrub(fraction) => self.events.push(TileEvent::Scrub { key, fraction }),
            HitKind::Focus => {
                self.focus();
                self.set_lead(key);
            }
            HitKind::Double => {
                self.focus();
                if tile_enabled {
                    self.events.push(TileEvent::Activate(key));
                }
            }
            HitKind::Rate(rating) => {
                self.focus();
                let keys = if self.selection.contains(&key) { self.selection.items.clone() } else { vec![key] };
                self.events.push(TileEvent::Rate { keys, rating });
            }
            HitKind::Click { modifiers, on_caption } => {
                self.focus();
                self.state.goal = None;
                if tile_enabled {
                    let slow = on_caption && modifiers.is_none() && !self.check_mode && self.selection.items == [key.clone()] && self.renamable(&key);
                    self.slow_click(slow.then(|| key.clone()));
                    self.click(at, key, modifiers);
                }
            }
            HitKind::Menu(pointer) => self.menu(at, key, rect, pointer, tile_enabled),
            HitKind::DragStart => self.start_drag(at, key, modifiers, tile_enabled),
        }
    }

    fn click(&mut self, at: usize, key: K, modifiers: Modifiers) {
        if self.check_mode {
            let from = modifiers.shift.then(|| self.state.last_checked.clone()).flatten();
            if let Some(change) = check_toggle(self.selection, &self.order, at, modifiers.shift, from.as_ref()) {
                self.apply(change);
            }
            self.state.last_checked = Some(key);
        } else if let Some(change) = self.rules.click(self.selection, &self.order, at, modifiers) {
            self.apply(change);
        } else {
            self.events.push(TileEvent::Clicked(key));
        }
    }

    fn menu(&mut self, at: usize, key: K, anchor: Rect, pointer: Option<Pos2>, tile_enabled: bool) {
        self.focus();
        let was_selected = self.selection.contains(&key);
        if !was_selected
            && tile_enabled
            && let Some(change) = self.rules.click(self.selection, &self.order, at, Modifiers::NONE)
        {
            *self.selection = change.selection.clone();
            self.events.push(TileEvent::ContextPicked(change));
        }
        self.events.push(TileEvent::ContextMenu(ContextMenuRequest { row: Some(key), selected: was_selected, anchor, pointer, toggle: None }));
    }

    /// A drag of a tile began: it joins the selection (Cmd/Ctrl) or replaces it, and the selection
    /// becomes egui's drag payload.
    fn start_drag(&mut self, at: usize, key: K, modifiers: Modifiers, tile_enabled: bool) {
        self.focus();
        if !tile_enabled {
            return;
        }
        if !self.selection.contains(&key) {
            let pick = if modifiers.command { modifiers } else { Modifiers::NONE };
            if let Some(change) = self.rules.click(self.selection, &self.order, at, pick) {
                self.apply(change);
            }
        }
        let name = content_at::<K, S>(self.source, at).name;
        let payload = ListDrag::new(self.id, &self.order, &self.selection.items, key, name);
        if let Some(gesture) = &mut self.gesture {
            gesture.begin();
            if !gesture.is_active() {
                return;
            }
        }
        self.events.push(TileEvent::Drag(payload.clone()));
        self.state.drag = Some(payload.items.clone());
        DragAndDrop::set_payload(&self.ctx, payload);
    }

    /// Clicks on group headers select the group's tiles.
    fn group_picks(&mut self, clicks: Vec<(usize, Modifiers)>) {
        for (group, modifiers) in clicks {
            self.focus();
            let g = &self.groups[group];
            let end = g.start.saturating_add(g.count).min(self.len);
            let mut items: Vec<K> = if modifiers.shift || modifiers.command { self.selection.items.clone() } else { Vec::new() };
            let tiles: Vec<K> = (g.start.min(end)..end).filter(|&p| !self.order.is_section(p)).map(|p| self.source.key(p).clone()).collect();
            for key in &tiles {
                if !items.contains(key) {
                    items.push(key.clone());
                }
            }
            let new = Selection {
                items,
                lead: tiles.last().cloned().or_else(|| self.selection.lead.clone()),
                anchor: tiles.first().cloned().or_else(|| self.selection.anchor.clone()),
            };
            if let Some(change) = change_of(self.selection, SelectionPick::Range, tiles.first().cloned(), new) {
                self.apply(change);
            }
            self.events.push(TileEvent::GroupPicked { group, modifiers });
        }
    }

    /// The background: a click clears, a double click activates, a secondary click and the menu
    /// key ask for the context menu.
    fn background(&mut self, background: &Response, view: Rect, mapper: &Mapper, menu_key: bool) {
        if background.clicked_by(PointerButton::Primary) && !background.double_clicked() {
            self.focus();
            if !self.check_mode
                && let Some(change) = self.rules.click_empty(self.selection)
            {
                self.apply(change);
            }
        }
        if background.double_clicked() {
            self.events.push(TileEvent::EmptyActivated);
        }
        if background.secondary_clicked() {
            self.focus();
            self.events.push(TileEvent::ContextMenu(ContextMenuRequest {
                row: None,
                selected: false,
                anchor: view,
                pointer: background.interact_pointer_pos(),
                toggle: None,
            }));
        }
        if menu_key {
            let anchor = self.lead_at().and_then(|at| self.arr.tiles.get(at)).filter(|r| **r != Rect::NOTHING).map_or(view, |r| mapper.rect(*r));
            self.events.push(TileEvent::ContextMenu(ContextMenuRequest {
                row: self.selection.lead.clone(),
                selected: self.selection.lead.is_some(),
                anchor,
                pointer: None,
                toggle: None,
            }));
        }
    }

    /// The marquee: a drag from the background selects the tiles it touches, live.
    fn marquee(&mut self, ui: &Ui, background: &Response, view: Rect, mapper: &Mapper, dragging: bool, modifiers: Modifiers) {
        let primary_down = ui.input(|i| i.pointer.primary_down());
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let escape = self.state.marquee.is_some() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        if background.drag_started_by(PointerButton::Primary)
            && self.state.marquee.is_none()
            && !dragging
            && let Some(p) = pointer
        {
            self.focus();
            self.state.marquee = Some(Marquee {
                start: p - mapper.origin,
                base: self.selection.clone(),
                additive: modifiers.shift || modifiers.command,
                cancelled: false,
            });
        }
        let Some(mut marquee) = self.state.marquee.take() else { return };
        if escape && !marquee.cancelled {
            marquee.cancelled = true;
            let base = marquee.base.clone();
            if let Some(change) = change_of(self.selection, SelectionPick::Replace, base.lead.clone(), base) {
                self.apply(change);
            }
        }
        if let Some(p) = pointer.filter(|_| !marquee.cancelled) {
            self.marquee_to(ui, &marquee, p, view, mapper);
        }
        if primary_down {
            self.state.marquee = Some(marquee);
        }
    }

    /// Selects what the marquee from its start to `pointer` touches and draws it.
    fn marquee_to(&mut self, ui: &Ui, marquee: &Marquee<K>, pointer: Pos2, view: Rect, mapper: &Mapper) {
        let rect = Rect::from_two_pos(mapper.origin + marquee.start, pointer);
        let touched: Vec<K> =
            tiles_touching(&self.arr, mapper, rect).into_iter().filter(|&i| !self.order.is_section(i)).map(|i| self.source.key(i).clone()).collect();
        let mut items = if marquee.additive { marquee.base.items.clone() } else { Vec::new() };
        for key in &touched {
            if !items.contains(key) {
                items.push(key.clone());
            }
        }
        let lead = touched.last().cloned().or_else(|| marquee.base.lead.clone().filter(|_| marquee.additive));
        let anchor = touched.first().cloned().or_else(|| marquee.base.anchor.clone().filter(|_| marquee.additive));
        if let Some(change) = change_of(self.selection, SelectionPick::Range, lead.clone(), Selection { items, lead, anchor }) {
            self.apply(change);
        }
        let painter = ui.painter().with_clip_rect(view);
        painter.rect_filled(rect, 0.0, self.style.marquee_fill);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, self.style.marquee_line), StrokeKind::Inside);
        self.state.drift = edge_scroll_speed(pointer.y, view.top(), view.bottom()) * ui.input(|i| i.stable_dt).min(0.05);
    }

    /// A drag of tiles: the reorder target and its insert line, and how the drag ends.
    fn drag(&mut self, ui: &Ui, view: Rect, mapper: &Mapper) {
        let Some(items) = self.state.drag.take() else { return };
        let (released, down, modifiers, latest) = ui.input(|i| (i.pointer.any_released(), i.pointer.any_down(), i.modifiers, i.pointer.latest_pos()));
        let target = self
            .reorder
            .as_ref()
            .zip(latest)
            .and_then(|(reorder, p)| drop_target(reorder, &self.order, &self.arr, mapper, p, &items, modifiers).map(|t| (t, p)));
        let has_payload = DragAndDrop::payload::<ListDrag<K>>(&self.ctx).is_some_and(|p| p.list == self.id);
        let end = if self.state.window_lost {
            Some(DragEnd::FocusLost)
        } else if released || !down {
            Some(DragEnd::Released(target.as_ref().map(|((_, mv), _)| mv.clone())))
        } else if !has_payload {
            // egui's drag and drop takes Esc and drops the payload before the grid sees the key.
            Some(if DragAndDrop::aborted_by_escape(&self.ctx) { DragEnd::Escape } else { DragEnd::PayloadCleared })
        } else {
            None
        };
        if let Some(end) = end {
            if !matches!(end, DragEnd::Released(None)) && has_payload {
                DragAndDrop::clear_payload(&self.ctx);
            }
            if let Some(gesture) = self.gesture.as_mut().filter(|g| g.is_active()) {
                gesture.end(end, modifiers);
            }
            return;
        }
        if let Some(((at, mv), pointer)) = target {
            self.drop_target = Some(mv.clone());
            let tile = mapper.rect(self.arr.tiles[at]);
            if mv.position == DropPosition::Into {
                paint_drop_mark(ui, tile, DropMark::Into);
            } else {
                let gap = self.style.gap / 2.0;
                let x = if (mv.position == DropPosition::Before) != self.rtl { tile.left() - gap } else { tile.right() + gap };
                drop_line(ui, pos2(x, tile.top()), pos2(x, tile.bottom()));
            }
            if let Some(reorder) = &self.reorder {
                self.state.drift = reorder.auto_scroll(pointer, view, ui.input(|i| i.stable_dt));
            }
        }
        self.state.drag = Some(items);
    }

    /// AccessKit for the list box, the focus ring of an empty grid, and the state stored.
    fn finish(mut self, ui: &Ui, response: &Response, lead_tile: Option<Id>) -> TileGridOutput<K> {
        if let Some(event) = self.gesture.take().and_then(ReorderGesture::finish) {
            self.events.push(TileEvent::Reorder(event));
        }
        let bounds = response.rect;
        let (len, focused, id) = (self.len, self.focused, self.id);
        self.ctx.accesskit_node_builder(id, |node| {
            node.set_bounds(egui::accesskit::Rect {
                x0: bounds.min.x.into(),
                y0: bounds.min.y.into(),
                x1: bounds.max.x.into(),
                y1: bounds.max.y.into(),
            });
            node.remove_action(Action::Click);
            node.set_size_of_set(len);
            node.set_multiselectable();
            match lead_tile.filter(|_| focused) {
                Some(tile) => node.set_active_descendant(tile.accesskit_id()),
                None => node.clear_active_descendant(),
            }
        });
        FocusRing::paint_if(ui, response.rect, CornerRadius::ZERO, focused && len == 0);
        if focused {
            // Esc keeps the focus while it has a selection to empty; egui would drop focus on it.
            let escape = self.takes_escape();
            self.ctx.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape }));
        }
        let events = std::mem::take(&mut self.events);
        let ctx = self.ctx.clone();
        self.state.store(&ctx, id);
        TileGridOutput { response: response.clone(), events, drop_target: self.drop_target.take(), file_gap: self.file_gap }
    }
}

/// A command from the keys: the grid's, or a stacking key with the modifiers held.
enum Command {
    Grid(GridKeyAction),
    Stack(StackKey, Modifiers),
}

/// The tooltip of a tile: the host's, or the name (with the failure message of its picture).
fn tooltip(content: &TileContent, texture: Option<&TextureState>) -> Option<String> {
    match (&content.tooltip, texture) {
        (Some(tip), _) => Some(tip.clone()),
        (None, Some(TextureState::Failed { message })) if !message.is_empty() => Some(format!("{}\n{message}", content.name)),
        (None, _) if !content.name.is_empty() => Some(content.name.clone()),
        _ => None,
    }
}

/// The tiles whose rect `area` (on the screen) touches.
fn tiles_touching(arr: &TileArrangement, mapper: &Mapper, area: Rect) -> Vec<usize> {
    let rows = arr.rows_between(area.top() - mapper.origin.y, area.bottom() - mapper.origin.y);
    rows.flat_map(|r| arr.rows[r].tiles.clone()).filter(|&i| mapper.rect(arr.tiles[i]).intersects(area)).collect()
}

/// Check mode: a click on tile `at` toggles its check; with `range` and a tile `from` clicked
/// before, every enabled tile from there to `at` is set to the state `at` takes.
fn check_toggle<K: Clone + PartialEq>(
    selection: &Selection<K>,
    order: &(impl DisplayOrder<K> + ?Sized),
    at: usize,
    range: bool,
    from: Option<&K>,
) -> Option<SelectionChange<K>> {
    let key = order.key(at).clone();
    let checked = !selection.contains(&key);
    let mut items = selection.items.clone();
    let start = from.filter(|_| range).and_then(|k| order.position(k)).unwrap_or(at);
    let (first, last) = (start.min(at), start.max(at));
    for position in first..=last {
        if order.is_section(position) {
            continue;
        }
        let tile = order.key(position);
        if checked {
            if !items.contains(tile) {
                items.push(tile.clone());
            }
        } else {
            items.retain(|k| k != tile);
        }
    }
    let pick = if first == last { SelectionPick::Toggle } else { SelectionPick::Range };
    let new = Selection { items, lead: Some(key.clone()), anchor: Some(key.clone()) };
    change_of(selection, pick, Some(key), new)
}
