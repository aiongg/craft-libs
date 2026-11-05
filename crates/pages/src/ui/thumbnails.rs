//! [`PageThumbnails`] (N3.01): the Pages panel of Acrobat and the slide strip of PowerPoint, on
//! craft-ui's [`TileGrid`](craft_ui::list::TileGrid): page thumbnails with the current page marked, several picked with
//! ⌘/Ctrl- and ⇧-click, dragged to reorder, a context menu the app fills, and the
//! [`ThumbnailSlider`](craft_ui::list::ThumbnailSlider) for their size.
//!
//! The grid does the work: layout, `ListSelection` picks on the app's [`Selection`], `GridNav`
//! keys, `Reorder` drags with a `ListDrag` payload reported as `EditEvent<Move<usize>>`, pictures
//! through `Thumbnails` over the app's [`PageSource`], context menus as [`ContextMenuRequest`]s,
//! the current ring, captions at the start, sections, the New Slide row and file drops (the
//! grid's strip builders). This module maps pages onto tiles and the grid's events onto page
//! events.
//!
//! ```ignore
//! let out = PageThumbnails::new("pages", &doc_pages, &mut picked, view.current)
//!     .editable(!doc.read_only)
//!     .size(&mut settings.thumbnail_zoom)
//!     .show(ui);
//! for event in out.events {
//!     match event {
//!         ThumbnailEvent::Go(page) => view.go_to(page),
//!         ThumbnailEvent::Activate(page) => start_show(page),
//!         ThumbnailEvent::Reorder(EditEvent::Commit { value, .. }) => doc.apply(Edit::MovePages(value)),
//!         ThumbnailEvent::Delete(pages) => doc.delete_pages(&pages),
//!         ThumbnailEvent::ContextMenu(request) => page_menu.open(request),
//!         _ => {}
//!     }
//! }
//! ```

use craft_ui::behavior::{Axis, DisplayOrder, DragBadge, DropPosition, ListDrag, Move, Reorder, Selection, SelectionChange, SelectionPick};
use craft_ui::canvas::textures::{ThumbnailOptions, Thumbnails};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{
    CaptionPlace, ContextMenuRequest, ThumbnailSlider, TileContent, TileEvent, TileGrid, TileGroup, TileLayout, TileMark, TileSource, TileZoom,
};
use craft_ui::localized::words::Counted;
use craft_ui::localized::{Labels as _, Words};
use craft_ui::theme::{Tokens, TokensOf as _, UiTokens};
use egui::{DragAndDrop, Pos2, Rect, Response, Ui};

use super::style::PageThumbnailsStyle;
use super::view::PageSource;

/// The thumbnail zoom: 0.5 to 1.5 of the usual width in steps of 0.25, 1 by default (PdfCraft's
/// `THUMBNAIL_ZOOM_RANGE` and its − and + step). The size row's [`ThumbnailSlider`](craft_ui::list::ThumbnailSlider) and the grid's
/// ⌘/Ctrl + / − / 0 step on it.
pub const THUMBNAIL_ZOOM: TileZoom = TileZoom::Range { min: 0.5, max: 1.5, step: 0.25, default: 1.0 };

/// "1 page" / "{n} pages": the count by the pointer while dragging pages.
pub const PAGES: Counted = Counted::new("pages.count", "{n} page", "{n} pages");

craft_ui::words! {
    /// The panel's words, English by default (PdfCraft's and DeckCraft's strings). The size row's
    /// words are the grid's (`TileLabels`).
    #[non_exhaustive]
    pub struct ThumbnailLabels {
        pages: "pages.thumbnails" = "Pages";
        page: "pages.thumbnail" = "Page {label}";
        hidden: "pages.hiddenPage" = "Hidden";
        animated: "pages.animatedPage" = "Has animations";
        new_page: "pages.newPage" = "New Slide";
    }
}

/// A named run of pages (PowerPoint's sections): a grid group header with a disclosure chevron,
/// the name and the count; its context menu is the app's ([`ThumbnailEvent::SectionMenu`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThumbnailSection {
    pub name: String,
    /// The section's first page.
    pub first: usize,
    pub count: usize,
    /// Open (its pages shown); the app keeps it and applies [`ThumbnailEvent::SectionOpen`].
    pub open: bool,
}

/// Marks on one thumbnail (PowerPoint), drawn by the grid with [`CaptionPlace::Start`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ThumbnailMarks {
    /// A hidden slide: the tile's `hidden` (the picture under the veil, a slash across the
    /// number), and [`ThumbnailLabels::hidden`] in its accessible name.
    pub hidden: bool,
    /// The slide has animations or a transition: a `TileMark` with `Icon::Star` and
    /// [`ThumbnailLabels::animated`] under the number.
    pub animated: bool,
}

/// Where page `p` (0-based) is after `mv` (craft-ui's [`Move`] over page indices: the pages
/// `items` go before or after page `target`) in a document of `count` pages. A
/// [`DropPosition::Into`] move (a drop onto a page, which pages don't take) places the pages
/// before `target`, as `Before` does.
// pdfcraft: crates/ui-egui/src/panels/pages.rs:283 `moved_index`.
pub fn moved_page(mv: &Move<usize>, p: usize, count: usize) -> usize {
    if p >= count {
        return p;
    }
    let mut items = mv.items.clone();
    items.sort_unstable();
    items.dedup();
    let before = |limit: usize| (0..limit).filter(|q| !items.contains(q)).count();
    // Where the moved pages start among the others; a target among the moved pages leaves the
    // slot at its own edge.
    let slot = match mv.position {
        DropPosition::After if !items.contains(&mv.target) => before(mv.target + 1),
        _ => before(mv.target),
    };
    let moved = match items.iter().position(|item| *item == p) {
        Some(k) => slot + k,
        None => {
            let rest = before(p);
            if rest >= slot { rest + items.len() } else { rest }
        }
    };
    moved.min(count - 1)
}

/// What the panel reports, in the order it happened this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ThumbnailEvent {
    /// The picked set changed (⌘/Ctrl-click, ⇧-click, ⇧-arrows, Esc, a secondary click on an
    /// unpicked page); the panel has applied it to the selection it was given.
    Selection(SelectionChange<usize>),
    /// A plain click, or an arrow key moving the lead: this page becomes the current page. The
    /// picked set became just this page (reported first as [`Selection`](Self::Selection)).
    Go(usize),
    /// A drag of pages as one edit gesture (`Begin`; `Commit` with the move on a drop that moves
    /// something; `Cancel` on Esc, on focus loss or when dropped where nothing moves), or a
    /// stacking key (⌘/Ctrl+] and [, ⇧ to the ends) as a `Set`, as craft-ui's `List` reports
    /// reorders. Only when [`editable`](PageThumbnails::editable).
    Reorder(EditEvent<Move<usize>>),
    /// Delete or Backspace with the panel focused: the picked pages, or the current page when none
    /// are; never when every page is picked (a document keeps one page), never read-only.
    Delete(Vec<usize>),
    /// A double-click or Enter on a thumbnail (PowerPoint starts the slide show from it).
    Activate(usize),
    /// A click on the New Slide row ([`PageThumbnails::new_page_row`]).
    NewPage,
    /// The app's menu for the picked pages is asked for (secondary click, Menu key, ⇧F10). A
    /// secondary click on an unpicked page makes it the picked set first
    /// ([`Selection`](Self::Selection)), without going to it.
    ContextMenu(ContextMenuRequest<usize>),
    /// The app's menu for section `section` is asked for (secondary click on its header; the
    /// grid's `GroupContextMenu`).
    SectionMenu { section: usize, anchor: Rect, pointer: Option<Pos2> },
    /// A section header's chevron (or AccessKit's Expand or Collapse on it) asks to open or close
    /// it (the grid's `GroupOpen`).
    SectionOpen { section: usize, open: bool },
}

/// The panel's result.
#[derive(Debug)]
#[must_use = "the app applies the panel's events"]
pub struct PageThumbnailsOutput {
    /// The panel: its focus and rect.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<ThumbnailEvent>,
    /// While pages are dragged, where they would drop this pass (the grid's `drop_target`).
    pub drop_target: Option<Move<usize>>,
    /// Files dragged in from outside are over the gap before this page (the grid's `file_gap`;
    /// the app takes the drop).
    pub file_gap: Option<usize>,
}

/// The thumbnails panel body, built per pass: a [`ThumbnailSlider`](craft_ui::list::ThumbnailSlider) row (with
/// [`size`](Self::size)) over a craft-ui [`TileGrid`](craft_ui::list::TileGrid) of the pages. Sizes come from
/// [`PageThumbnailsStyle`] (the `thumbnails.*` tokens proposed in the spec) and the grid's
/// `tile.*` tokens.
///
/// - Look (Acrobat; PowerPoint with [`CaptionPlace::Start`]): `TileLayout::Cards` whose side is
///   the style's usual width × the zoom, at most the panel's width less its inset, the aspect
///   page 0's (`PageSource::page_size`; other pages are fitted into the card by `Thumbnails`), a
///   caption under it with the page label (`PageSource::page_label`, the grid's `fit_name`), or
///   the number at the start with [`CaptionPlace::Start`]. The current page is the grid's
///   [`current`](craft_ui::list::TileGrid::current) tile (its ring); picked pages are the grid's selected tiles.
///   Pictures come from `Thumbnails<usize>` over [`PageSource`], so placeholders and failures
///   look as everywhere else.
/// - Pointer (behaviour: Photoshop's panels, then PdfCraft, as the grid): a plain click goes to
///   the page ([`ThumbnailEvent::Go`] after the grid's `Selection` or `Clicked`); ⌘/Ctrl-click
///   toggles a page in the picked set and ⇧-click extends it, neither moving the document. A new
///   current page scrolls into view as little as it takes, not while dragging (the grid's
///   `current`).
/// - Drag (when editable): the grid's `.reorder(Reorder::new(Axis::Horizontal))`; an unpicked
///   page grabbed drags alone; the grid draws the insert line and scrolls near its edges; this
///   panel shows craft-ui's `DragBadge` with the [`PAGES`] count by the pointer while the grid's
///   `ListDrag` payload is its own; Esc cancels. Files dragged in from outside report
///   [`PageThumbnailsOutput::file_gap`] (the grid's `file_drops`).
/// - Keys (one Tab stop, the grid's): arrows go to the next page (`Go`), ⇧ extends, Home and End,
///   Enter activates (`Activate` → [`ThumbnailEvent::Activate`]), Delete and Backspace delete
///   (`Remove`, the current page when nothing is picked → [`ThumbnailEvent::Delete`], with this
///   panel's rules), ⌘/Ctrl+] and [ move the picked
///   pages, ⌘/Ctrl + / − / 0 step [`THUMBNAIL_ZOOM`] (`Zoom`), Space toggles the lead
///   (`SpaceAction::Toggle`), no rating keys (`RatingKeys::None`). Esc with the grid focused and
///   no drag clears the picked set (`Selection`). The Menu key and ⇧F10 ask for the menu.
/// - AccessKit: the grid's `ListBox` named [`ThumbnailLabels::pages`], one `ListBoxOption` per
///   page named [`ThumbnailLabels::page`] (`TileContent::accessible_name`), selected when picked,
///   the current page `aria_current`; the size row's controls are their own Tab stops.
pub struct PageThumbnails<'a> {
    id_salt: egui::IdSalt,
    source: &'a dyn PageSource,
    picked: &'a mut Selection<usize>,
    current: usize,
    editable: bool,
    zoom: Option<&'a mut f32>,
    caption_place: CaptionPlace,
    sections: &'a [ThumbnailSection],
    marks: Option<&'a dyn Fn(usize) -> ThumbnailMarks>,
    new_page_row: bool,
    labels: Option<ThumbnailLabels>,
}

impl<'a> PageThumbnails<'a> {
    /// Thumbnails of `source`'s pages; `picked` is the set page commands act on, kept by the app.
    pub fn new(id_salt: impl egui::AsIdSalt, source: &'a dyn PageSource, picked: &'a mut Selection<usize>, current: usize) -> Self {
        PageThumbnails {
            id_salt: egui::IdSalt::new(id_salt),
            source,
            picked,
            current,
            editable: true,
            zoom: None,
            caption_place: CaptionPlace::Below,
            sections: &[],
            marks: None,
            new_page_row: false,
            labels: None,
        }
    }

    /// The id the panel keeps its memory under (and names its drag payload with), by craft-ui's
    /// id rule (`Interactive::id_in`).
    pub fn id_in(ui: &Ui, id_salt: impl egui::AsIdSalt) -> egui::Id {
        ui.make_persistent_id(egui::IdSalt::new(id_salt))
    }

    /// The panel's sizes, from the tokens.
    pub fn style(tokens: &UiTokens) -> PageThumbnailsStyle {
        let t = &tokens.component.pages;
        PageThumbnailsStyle { usual_width: t.thumbnail_usual_width, panel_inset: t.thumbnail_panel_inset }
    }

    /// Off: no drag, no stacking keys, no Delete, no file drops (a read-only document).
    pub fn editable(mut self, on: bool) -> Self {
        self.editable = on;
        self
    }

    /// Show the size row above the grid: craft-ui's [`ThumbnailSlider`](craft_ui::list::ThumbnailSlider) over [`THUMBNAIL_ZOOM`]
    /// (its − and + glyph buttons step 0.25), editing `zoom` (a view preference, not the
    /// document): `Update`, `Commit` and `Set` write it, `Cancel` restores the value the gesture
    /// began with. The row stays above the scroll area. The grid's `Zoom` keys step it too.
    pub fn size(mut self, zoom: &'a mut f32) -> Self {
        self.zoom = Some(zoom);
        self
    }

    /// Where page labels go (default [`CaptionPlace::Below`], Acrobat;
    /// [`CaptionPlace::Start`] is PowerPoint's number at the left).
    pub fn caption_place(mut self, place: CaptionPlace) -> Self {
        self.caption_place = place;
        self
    }

    /// PowerPoint's sections, as the grid's groups with [`TileGrid::group_disclosure`](craft_ui::list::TileGrid::group_disclosure) and
    /// [`TileGrid::group_menus`](craft_ui::list::TileGrid::group_menus): title the name, detail the count.
    pub fn sections(mut self, sections: &'a [ThumbnailSection]) -> Self {
        self.sections = sections;
        self
    }

    /// Each page's [`ThumbnailMarks`].
    pub fn marks(mut self, marks: &'a dyn Fn(usize) -> ThumbnailMarks) -> Self {
        self.marks = Some(marks);
        self
    }

    /// The New Slide row under the last thumbnail (DeckCraft; the grid's
    /// [`new_item_row`](craft_ui::list::TileGrid::new_item_row) with [`ThumbnailLabels::new_page`]).
    pub fn new_page_row(mut self, on: bool) -> Self {
        self.new_page_row = on;
        self
    }

    pub fn labels(mut self, labels: ThumbnailLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the size row and the grid. The grid gets the pages as `PageTiles`, the picked set,
    /// `Thumbnails::in_context` over the source, `current`, the caption place, the sections as
    /// groups, `reorder` and `file_drops` when editable, `new_item_row` when asked, and
    /// [`ThumbnailLabels::pages`] as its label; its events become [`ThumbnailEvent`]s through
    /// `page_events`, in order, and its `drop_target` and `file_gap` the output's.
    // pdfcraft: crates/ui-egui/src/panels/pages.rs:90 `pages`, :36 `thumbnail_size`, :295
    // `page_drag`, :277 `panel_gap`; deckcraft: crates/ui-egui/src/panels/thumbs.rs:116 `list`
    // (sections, number at the left, hidden and animation marks, New Slide).
    pub fn show(self, ui: &mut Ui) -> PageThumbnailsOutput {
        let Self { id_salt, source, picked, current, editable, mut zoom, caption_place, sections, marks, new_page_row, labels } = self;
        let labels = labels.unwrap_or_else(|| ThumbnailLabels::of(ui));
        let style = Self::style(&Tokens::of(ui));
        let id = Self::id_in(ui, id_salt);
        let ctx = ui.ctx().clone();
        let count = source.page_count();

        if let Some(zoom) = zoom.as_deref_mut() {
            let began = id.with("zoom began");
            let width = ui.available_width().clamp(ThumbnailSlider::extent(ui).1, ThumbnailSlider::extent(ui).0);
            let edit = ThumbnailSlider::new((id, "size"), *zoom, THUMBNAIL_ZOOM).width(width).show(ui);
            match edit.event {
                Some(EditEvent::Begin { value, .. }) => {
                    ctx.data_mut(|d| d.insert_temp(began, *zoom));
                    if let Some(value) = value {
                        *zoom = value as f32;
                    }
                }
                Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => *zoom = value as f32,
                Some(EditEvent::Cancel { .. }) => {
                    if let Some(start) = ctx.data(|d| d.get_temp::<f32>(began)) {
                        *zoom = start;
                    }
                }
                _ => {}
            }
        }
        let scale = zoom.as_deref().copied().unwrap_or(1.0);

        let aspect = if count == 0 { 1.0 } else { aspect_of(source, 0) };
        let side = (style.usual_width * scale).min((ui.available_width() - style.panel_inset).max(0.0));
        let tiles = PageTiles { source, keys: (0..count).collect(), marks, labels: &labels };
        let thumbnails = Thumbnails::<usize>::in_context(&ctx, (id_salt, "pictures"), ThumbnailOptions::default());

        let words = Words::of(ui);
        let pages_text = words.get(PAGES.word);
        let groups: Vec<TileGroup> = sections
            .iter()
            .map(|section| TileGroup {
                start: section.first,
                count: section.count,
                title: section.name.clone(),
                detail: words.counted(&ctx, PAGES, &pages_text, section.count as u64),
            })
            .collect();
        let open: Vec<bool> = sections.iter().map(|section| section.open).collect();

        let mut grid = TileGrid::new(id_salt, &tiles)
            .layout(TileLayout::Cards { side, aspect, caption: true })
            .selection(&mut *picked)
            .thumbnails(&thumbnails, source)
            .rating_keys(craft_ui::list::RatingKeys::None)
            .space(craft_ui::list::SpaceAction::Toggle)
            .label(labels.pages.clone())
            .current(current)
            .caption_place(caption_place);
        if !groups.is_empty() {
            grid = grid.groups(&groups).group_disclosure(&open).group_menus(true);
        }
        if editable {
            grid = grid.reorder(Reorder::new(Axis::Horizontal)).file_drops(true);
        }
        if new_page_row {
            grid = grid.new_item_row(labels.new_page.clone());
        }
        let out = grid.show(ui);

        if let Some(drag) = DragAndDrop::payload::<ListDrag<usize>>(&ctx).filter(|drag| drag.list == id) {
            let text = words.counted(&ctx, PAGES, &pages_text, drag.items.len() as u64);
            DragBadge::new((id, "drag"), text).show(&ctx);
        }

        let events = page_events(out.events, count, editable, zoom);
        PageThumbnailsOutput { response: out.response, events, drop_target: out.drop_target, file_gap: out.file_gap }
    }
}

/// The pages as the grid's tiles: keys `0..page_count` in order (`keys`, built once per pass so
/// `DisplayOrder::key` can lend them), the aspect from `page_size`, the content from
/// `page_label`, the marks and the labels.
struct PageTiles<'a> {
    source: &'a dyn PageSource,
    keys: Vec<usize>,
    marks: Option<&'a dyn Fn(usize) -> ThumbnailMarks>,
    labels: &'a ThumbnailLabels,
}

impl DisplayOrder<usize> for PageTiles<'_> {
    fn len(&self) -> usize {
        self.keys.len()
    }

    fn key(&self, at: usize) -> &usize {
        &self.keys[at]
    }
}

impl TileSource<usize> for PageTiles<'_> {
    fn aspect(&self, at: usize) -> f32 {
        // pdfcraft: crates/ui-egui/src/panels/pages.rs:90 (each thumbnail keeps its page's aspect).
        aspect_of(self.source, at)
    }

    /// The name is the page label; the accessible name [`ThumbnailLabels::page`] with it, then
    /// [`ThumbnailLabels::hidden`] for a hidden slide, and the grid joins the tips of the marks
    /// (an `Icon::Star` tipped [`ThumbnailLabels::animated`]) after it.
    fn content(&self, at: usize, content: &mut TileContent) {
        // deckcraft: crates/ui-egui/src/panels/thumbs.rs:116 `list` (number, hidden and animation marks).
        let label = self.source.page_label(at);
        let marks = self.marks.map(|marks| marks(at)).unwrap_or_default();
        let mut accessible = vec![self.labels.page.replace("{label}", &label)];
        if marks.hidden {
            accessible.push(self.labels.hidden.clone());
        }
        if marks.animated {
            content.marks.push(TileMark { icon: Icon::Star, tip: self.labels.animated.clone() });
        }
        content.name = label;
        content.hidden = marks.hidden;
        content.accessible_name = Some(accessible.join(", "));
    }
}

/// Page `at`'s width over its height; 1 for a page without a size.
fn aspect_of(source: &dyn PageSource, at: usize) -> f32 {
    let size = source.page_size(at);
    if size.width > 0.0 && size.height > 0.0 { (size.width / size.height) as f32 } else { 1.0 }
}

/// The grid's events as the panel's, in order: `Selection` and `ContextPicked` →
/// [`ThumbnailEvent::Selection`], then [`ThumbnailEvent::Go`] for a plain pick of one page
/// (`SelectionPick::Replace` by a click or an arrow; a context pick never goes); `Clicked` →
/// `Go`; `Activate` → `Activate`; `Remove` → `Delete` unless read-only or every page is picked
/// (the grid reports the current page when nothing is picked); `Reorder` as is (only when
/// editable); `ContextMenu` as is; `GroupOpen` → `SectionOpen`; `GroupContextMenu` →
/// `SectionMenu`; `NewItem` → `NewPage`; `Zoom` steps `zoom` on [`THUMBNAIL_ZOOM`]. Everything
/// else is dropped.
fn page_events(events: Vec<TileEvent<usize>>, page_count: usize, editable: bool, zoom: Option<&mut f32>) -> Vec<ThumbnailEvent> {
    // pdfcraft: crates/ui-egui/src/panels/pages.rs:90 `pages` (click goes, Delete keeps a page).
    let mut out = Vec::new();
    let mut zoom = zoom;
    for event in events {
        match event {
            TileEvent::Selection(change) => {
                let go = (change.pick == SelectionPick::Replace).then_some(change.row).flatten();
                out.push(ThumbnailEvent::Selection(change));
                out.extend(go.map(ThumbnailEvent::Go));
            }
            TileEvent::ContextPicked(change) => out.push(ThumbnailEvent::Selection(change)),
            TileEvent::Clicked(page) => out.push(ThumbnailEvent::Go(page)),
            TileEvent::Activate(page) => out.push(ThumbnailEvent::Activate(page)),
            TileEvent::Remove(pages) => {
                if editable && !pages.is_empty() && pages.len() < page_count {
                    out.push(ThumbnailEvent::Delete(pages));
                }
            }
            TileEvent::Reorder(edit) if editable => out.push(ThumbnailEvent::Reorder(edit)),
            TileEvent::ContextMenu(request) => out.push(ThumbnailEvent::ContextMenu(request)),
            TileEvent::GroupOpen { group, open } => out.push(ThumbnailEvent::SectionOpen { section: group, open }),
            TileEvent::GroupContextMenu { group, anchor, pointer } => out.push(ThumbnailEvent::SectionMenu { section: group, anchor, pointer }),
            TileEvent::NewItem => out.push(ThumbnailEvent::NewPage),
            TileEvent::Zoom(step) => {
                if let Some(zoom) = zoom.as_deref_mut() {
                    *zoom = THUMBNAIL_ZOOM.step(*zoom, step);
                }
            }
            _ => {}
        }
    }
    out
}
