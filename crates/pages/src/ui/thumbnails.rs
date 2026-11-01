//! [`PageThumbnails`] (N3.01): the Pages panel of Acrobat and the slide strip of PowerPoint:
//! page thumbnails with the current page marked, several picked with ⌘/Ctrl- and ⇧-click,
//! dragged to reorder, a context menu the app fills, and a thumbnail size slider.
//!
//! Built on craft-ui's shared pieces, never copies of them: picks through
//! [`ListSelection`](craft_ui::behavior::ListSelection) on a host-owned
//! [`Selection`], arrow keys through
//! [`GridNav`](craft_ui::behavior::GridNav), drags through
//! [`Reorder`](craft_ui::behavior::Reorder) with a [`ListDrag`](craft_ui::behavior::ListDrag)
//! payload reported as `EditEvent<Move<usize>>`, pictures through
//! [`Thumbnails`](craft_ui::canvas::textures::Thumbnails) over the app's [`PageSource`], context
//! menus as [`ContextMenuRequest`]s the app answers with `craft_ui::menu`.
//!
//! ```ignore
//! let out = PageThumbnails::new("pages", &doc_pages, &mut picked, view.current)
//!     .editable(!doc.read_only)
//!     .size(&mut settings.thumbnail_zoom)
//!     .show(ui);
//! for event in out.events {
//!     match event {
//!         ThumbnailEvent::Go(page) => view.go_to(page),
//!         ThumbnailEvent::Reorder(EditEvent::Commit { value, .. }) => doc.apply(Edit::MovePages(value)),
//!         ThumbnailEvent::Delete(pages) => doc.delete_pages(&pages),
//!         ThumbnailEvent::ContextMenu(request) => page_menu.open(request),
//!         _ => {}
//!     }
//! }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::RangeInclusive;

use craft_ui::behavior::{Move, Selection, SelectionChange};
use craft_ui::edit::EditEvent;
use craft_ui::list::ContextMenuRequest;
use craft_ui::localized::words::Counted;
use craft_ui::theme::UiTokens;
use egui::{Pos2, Rect, Response, Ui};

use super::style::PageThumbnailsStyle;
use super::view::PageSource;

/// The thumbnail zooms the size slider covers (PdfCraft's `THUMBNAIL_ZOOM_RANGE`), in steps of
/// [`ZOOM_STEP`] for the − and + buttons.
pub const THUMBNAIL_ZOOM_RANGE: RangeInclusive<f32> = 0.5..=1.5;

/// The step of the − and + buttons (PdfCraft).
pub const ZOOM_STEP: f32 = 0.25;

/// The opacity of a hidden slide's thumbnail (PowerPoint, DeckCraft's `0.55`): a proportion, not
/// a size (lead call 38).
pub const HIDDEN_OPACITY: f32 = 0.55;

/// "1 page" / "{n} pages": the count by the pointer while dragging pages.
pub const PAGES: Counted = Counted::new("pages.count", "{n} page", "{n} pages");

craft_ui::words! {
    /// The panel's words, English by default (PdfCraft's and DeckCraft's strings).
    #[non_exhaustive]
    pub struct ThumbnailLabels {
        pages: "pages.thumbnails" = "Pages";
        page: "pages.thumbnail" = "Page {label}";
        thumbnail_size: "pages.thumbnailSize" = "Thumbnail size";
        smaller: "pages.smallerThumbnails" = "Smaller thumbnails";
        larger: "pages.largerThumbnails" = "Larger thumbnails";
        hidden: "pages.hiddenPage" = "Hidden";
        animated: "pages.animatedPage" = "Has animations";
        new_page: "pages.newPage" = "New Slide";
    }
}

/// Where the page label goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelPlace {
    /// Under the thumbnail, centred (Acrobat).
    #[default]
    Below,
    /// The number left of the thumbnail, with hidden and animation marks under it (PowerPoint).
    Left,
}

/// A named run of pages (PowerPoint's sections): a header row with a disclosure chevron, the name
/// and the count; its context menu is the app's ([`ThumbnailEvent::SectionMenu`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThumbnailSection {
    pub name: String,
    /// The section's first page.
    pub first: usize,
    pub count: usize,
    /// Open (its pages shown); the app keeps it and applies [`ThumbnailEvent::SectionOpen`].
    pub open: bool,
}

/// Marks on one thumbnail.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ThumbnailMarks {
    /// Drawn at [`HIDDEN_OPACITY`] with a slash through the number (a hidden slide).
    pub hidden: bool,
    /// A small star under the number (the slide has animations or a transition).
    pub animated: bool,
}

/// Where page `p` (0-based) is after `mv` (craft-ui's [`Move`] over page indices: the pages
/// `items` go before or after page `target`) in a document of `count` pages.
// pdfcraft: crates/ui-egui/src/panels/pages.rs:283 `moved_index`.
pub fn moved_page(mv: &Move<usize>, p: usize, count: usize) -> usize {
    todo!()
}

/// What the panel reports, in the order it happened this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ThumbnailEvent {
    /// The picked set changed (⌘/Ctrl-click, ⇧-click, ⇧-arrows, Esc); the panel has applied it
    /// to the selection it was given.
    Picked(SelectionChange<usize>),
    /// A plain click, or an arrow key moving the lead: go to the page. The picked set became just
    /// this page (reported first as [`Picked`](Self::Picked)).
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
    Open(usize),
    /// A click on the New Slide row ([`PageThumbnails::new_page_row`]).
    NewPage,
    /// The app's menu for the picked pages is asked for (secondary click, Menu key, ⇧F10). A
    /// secondary click on an unpicked page makes it the picked set first ([`Picked`](Self::Picked)).
    ContextMenu(ContextMenuRequest<usize>),
    /// The app's menu for section `section` is asked for (secondary click on its header).
    SectionMenu { section: usize, anchor: Rect, pointer: Option<Pos2> },
    /// A section header's chevron (or ← / → on it) asks to open or close it.
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
    /// While pages are dragged, where they would drop this pass (the bar shows it).
    pub drop_target: Option<Move<usize>>,
    /// Files dragged in from outside are over the gap before this page (the app takes the drop).
    pub file_gap: Option<usize>,
}

/// The thumbnails panel body, built per pass. Sizes come from [`PageThumbnailsStyle`] (the
/// `thumbnails.*` tokens proposed in the spec).
///
/// - Look (Acrobat; PowerPoint with [`LabelPlace::Left`]): each thumbnail is the style's usual
///   width × the zoom (at most the panel less its inset), in a cell padded by the style, the
///   page's aspect kept; cells wrap into as many columns as fit
///   ([`GridNav::columns`](craft_ui::behavior::GridNav::columns)), one at the usual panel width.
///   Pictures come from `Thumbnails<usize>` over [`PageSource`] (asked at `Priority::VISIBLE` in
///   view and `Priority::NEAR` one row beyond), so placeholders and failures look as everywhere
///   else. Page labels wider than the cell are elided at the end (`craft_ui::text::elided_galley`).
/// - The current page has the style's current border in the accent colour (the plain border
///   otherwise) and the selected-row wash; picked pages share the wash with the picked outline
///   (Acrobat). With [`LabelPlace::Left`] the current and picked pages get the strip outline,
///   offset outward (PowerPoint).
/// - Pointer (behaviour: Photoshop's panels, then PdfCraft): click goes to the page;
///   ⌘/Ctrl-click toggles a page in the picked set and ⇧-click extends it, neither moving the
///   document. A new current page scrolls into view as little as it takes, not while dragging.
/// - Drag (when editable): `Reorder` along the cells; an unpicked page grabbed drags alone; the
///   drop bar (accent, the style's width) shows the gap, the [`PAGES`] count by the pointer, the
///   panel scrolls near its edges (`edge_scroll_speed`); Esc cancels. The payload is a
///   `ListDrag<usize>` named by this panel's id, so a footer button or another widget can take it.
/// - Keys (one Tab stop; arrows move inside, `GridNav` over the cells): arrows go to the next
///   page, ⇧ extends, Home and End, Esc clears the picked set, Enter opens, Delete and Backspace
///   delete, ⌘/Ctrl+] and [ move the picked pages. The Menu key and ⇧F10 ask for the menu.
/// - AccessKit: a `ListBox` named "Pages" with one `ListBoxOption` per page, "Page {label}",
///   selected when picked; the size row's controls are their own Tab stops.
pub struct PageThumbnails<'a> {
    id_salt: egui::IdSalt,
    source: &'a dyn PageSource,
    picked: &'a mut Selection<usize>,
    current: usize,
    editable: bool,
    zoom: Option<&'a mut f32>,
    label_place: LabelPlace,
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
            label_place: LabelPlace::Below,
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

    /// The panel's sizes and colours, from the tokens.
    pub fn style(tokens: &UiTokens) -> PageThumbnailsStyle {
        PageThumbnailsStyle::of(tokens)
    }

    /// Off: no drag, no stacking keys, no Delete (a read-only document).
    pub fn editable(mut self, on: bool) -> Self {
        self.editable = on;
        self
    }

    /// Show the Thumbnail size row above the list (label, percentage, − and + `IconButton`s
    /// stepping [`ZOOM_STEP`], a craft-ui `Slider` between), editing `zoom` (a view preference,
    /// not the document) within [`THUMBNAIL_ZOOM_RANGE`]; the row stays above the scroll area.
    pub fn size(mut self, zoom: &'a mut f32) -> Self {
        self.zoom = Some(zoom);
        self
    }

    pub fn label_place(mut self, place: LabelPlace) -> Self {
        self.label_place = place;
        self
    }

    pub fn sections(mut self, sections: &'a [ThumbnailSection]) -> Self {
        self.sections = sections;
        self
    }

    pub fn marks(mut self, marks: &'a dyn Fn(usize) -> ThumbnailMarks) -> Self {
        self.marks = Some(marks);
        self
    }

    /// The New Slide hint row under the last thumbnail (PowerPoint).
    pub fn new_page_row(mut self, on: bool) -> Self {
        self.new_page_row = on;
        self
    }

    pub fn labels(mut self, labels: ThumbnailLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/panels/pages.rs:90 `pages`, :36 `thumbnail_size`, :295
    // `page_drag`, :277 `panel_gap`; deckcraft: crates/ui-egui/src/panels/thumbs.rs:116 `list`
    // (sections, number at the left, hidden and animation marks, New Slide). Structure:
    // craft-browse's `TileGrid` (`browse/src/ui/grid/mod.rs`) composes the same craft-ui pieces.
    pub fn show(self, ui: &mut Ui) -> PageThumbnailsOutput {
        todo!()
    }
}
