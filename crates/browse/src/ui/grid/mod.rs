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

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod grid_tests;
mod groups;
mod keys;
mod layout;
#[cfg(test)]
mod layout_tests;
mod paint;
pub(crate) mod stories;
mod style;
mod zoom;

use craft_theme::Rgba;
use craft_ui::behavior::{DisplayOrder, ListDrag, Move, Reorder, Selection, SelectionChange};
use craft_ui::canvas::textures::{TextureKey, ThumbSource, Thumbnails};
use craft_ui::controls::BadgeTone;
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::ContextMenuRequest;
use egui::{AsIdSalt, Modifiers, Rect, Response, Ui};

pub use layout::{ArrangeSpacing, TileArrangement, TileGroup, TileLayout, TileRow, arrange, card_size, fit_name};
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
    /// Replaces the accessible name the grid builds from `TileLabels::tile_name`.
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
}

impl Default for TileContent {
    fn default() -> Self {
        // new: enabled, everything else empty.
        todo!()
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
    Rate { keys: Vec<K>, rating: u8 },
    /// Delete or Backspace on the selection.
    Remove(Vec<K>),
    /// A drag of the selection began (once); the payload is also egui's drag payload.
    Drag(ListDrag<K>),
    /// A reorder gesture (with `.reorder(..)`): Begin, Commit, Cancel, or Set for a key move.
    Reorder(EditEvent<Move<K>>),
    /// The pointer is over a picture (with `.hover_scrub(true)`): `fraction` across it from the
    /// reading start, 0..1, every pass while it hovers.
    Scrub { key: K, fraction: f32 },
    /// A click on a group header (its tiles were selected; Shift or Cmd added them).
    GroupPicked { group: usize, modifiers: Modifiers },
    /// Cmd/Ctrl + / − / 0, or a pinch.
    Zoom(ZoomStep),
    /// A context menu was asked for, on a tile or the background (`row: None`).
    ContextMenu(ContextMenuRequest<K>),
    /// A double click on the background (FilmCraft: import).
    EmptyActivated,
}

/// What [`TileGrid::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the grid's events"]
pub struct TileGridOutput<K> {
    /// The grid: its focus and its rect.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<TileEvent<K>>,
}

craft_ui::words! {
    /// The grid's words, English by default.
    #[non_exhaustive]
    pub struct TileLabels {
        /// A tile's accessible name: `{name}`, then `{details}` (the parts below that apply,
        /// joined with ", ").
        tile_name: "browse.tileName" = "{name}, {details}";
        /// `{n}` stars.
        stars: "browse.stars" = "{n} stars";
        one_star: "browse.oneStar" = "1 star";
        picked: "browse.picked" = "picked";
        rejected: "browse.rejected" = "rejected";
        /// `{label}` the app's label name.
        label: "browse.label" = "{label} label";
        offline: "browse.offline" = "Offline";
        unreadable: "browse.unreadable" = "Unreadable";
        failed: "browse.failed" = "Preview failed";
        /// The stack badge: `{n}` tiles.
        stack: "browse.stack" = "×{n}";
        /// The tile size slider's accessible name.
        thumbnail_size: "browse.thumbnailSize" = "Thumbnail Size";
        smaller: "browse.smaller" = "Smaller Thumbnails";
        larger: "browse.larger" = "Larger Thumbnails";
    }
}

/// The thumbnails a widget draws with: the cache and the app's source, keyed by the widget's key.
type ThumbPair<'a, K> = (&'a Thumbnails<K>, &'a dyn ThumbSource<K>);

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
///   toggles the lead. Delete and Backspace: `Remove(selection)`. Rating keys per [`RatingKeys`].
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
///   insert line is `list::drop_line` in the gap; `Reorder(Begin)`, then `Commit` on release or
///   `Cancel` (Esc, focus lost, released outside); Alt (Option) copies
///   (`Reorder::copy_with_alt`); edge auto-scroll (`Reorder::auto_scroll`).
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
///   state; disabled tiles are skipped and can't be checked; the check box is drawn in the
///   start-top corner (`tile.checkbox`, at `tile.inset + 2`, on `surface.scrim` at 60 %).
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
    id_salt: egui::Id,
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
}

impl<'a, K, S> TileGrid<'a, K, S>
where
    K: TextureKey + std::fmt::Debug,
    S: TileSource<K> + ?Sized,
{
    /// A grid over `source`, `Cards { side: 128, aspect: 1, caption: true }` until `.layout`.
    pub fn new(id_salt: impl AsIdSalt, source: &'a S) -> Self {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:286 (`show_inner`).
        todo!()
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

    /// The words (default: the installed `Words`).
    #[must_use]
    pub fn labels(mut self, labels: TileLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the grid in the space given and reports what happened.
    pub fn show(self, ui: &mut Ui) -> TileGridOutput<K> {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:286 (`show_inner`: layout, rows in view ± one viewport, cells, groups).
        todo!()
    }
}
