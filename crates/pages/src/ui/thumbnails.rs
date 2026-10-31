//! [`PageThumbnails`] (N3.01): the Pages panel of Acrobat and the slide strip of PowerPoint: a
//! column of page thumbnails with the current page marked, several picked with ⌘/Ctrl- and
//! ⇧-click, dragged to reorder, a context menu the app fills, and a thumbnail size slider.
//!
//! ```ignore
//! let r = PageThumbnails::new("pages", &mut doc_pages, &mut picked, view.current)
//!     .editable(!doc.read_only)
//!     .size(&mut settings.thumbnail_zoom)
//!     .context_menu(|ui, pages| page_menu(ui, pages))
//!     .show(ui);
//! match r.action {
//!     Some(ThumbnailAction::Go(page)) => view.go_to(page),
//!     Some(ThumbnailAction::Move(mv)) => doc.apply(Edit::MovePages { pages: mv.pages, to: mv.to }),
//!     Some(ThumbnailAction::Delete(pages)) => doc.delete_pages(&pages),
//!     _ => {}
//! }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::RangeInclusive;

use craft_ui::behavior::Selection;
use egui::{Response, Ui};

use super::view::PageSource;

/// The thumbnail zooms the size slider covers (PdfCraft's `THUMBNAIL_ZOOM_RANGE`), in steps of
/// 0.25 for the − and + buttons.
pub const THUMBNAIL_ZOOM_RANGE: RangeInclusive<f32> = 0.5..=1.5;

/// The usual thumbnail width at zoom 1: the panel width less 40, at most 150 pt (PdfCraft).
pub const USUAL_WIDTH: f32 = 150.0;

craft_ui::words! {
    /// The panel's words, English by default (PdfCraft's and DeckCraft's strings).
    #[non_exhaustive]
    pub struct ThumbnailLabels {
        pages: "pages.thumbnails" = "Pages";
        page: "pages.thumbnail" = "Page {label}";
        thumbnail_size: "pages.thumbnailSize" = "Thumbnail size";
        smaller: "pages.smallerThumbnails" = "Smaller thumbnails";
        larger: "pages.largerThumbnails" = "Larger thumbnails";
        one_page: "pages.onePage" = "1 page";
        n_pages: "pages.nPages" = "{n} pages";
        hidden: "pages.hiddenPage" = "Hidden";
        new_page: "pages.newPage" = "+ New Slide";
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
/// and the count; its context menu is the app's ([`PageThumbnails::section_menu`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThumbnailSection {
    pub name: String,
    /// The section's first page.
    pub first: usize,
    pub count: usize,
}

/// Marks on one thumbnail.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ThumbnailMarks {
    /// Drawn at 55 % opacity with a slash through the number (a hidden slide).
    pub hidden: bool,
    /// A small star under the number (the slide has animations or a transition).
    pub animated: bool,
}

/// Pages moved by a drag: `pages` (sorted, 0-based) go to `to`, counted without them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageMove {
    pub pages: Vec<usize>,
    pub to: usize,
}

impl PageMove {
    /// Where page `p` is after the move.
    // pdfcraft: crates/ui-egui/src/panels/pages.rs:283 `moved_index`.
    pub fn moved_index(&self, p: usize) -> usize {
        todo!()
    }
}

/// What the user asked for this pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThumbnailAction {
    /// A plain click: go to the page; the picked pages become just this one.
    Go(usize),
    /// A drag dropped on a gap that moves something.
    Move(PageMove),
    /// Delete or Backspace with the panel focused or under the pointer: the picked pages, or the
    /// current page when none are (never every page: a document keeps one).
    Delete(Vec<usize>),
    /// A double-click (PowerPoint starts the slide show from it).
    Open(usize),
    /// A click on the empty space under the last thumbnail ([`PageThumbnails::new_page_row`]).
    NewPage,
}

/// The panel's result.
#[derive(Debug)]
pub struct ThumbnailResponse {
    pub action: Option<ThumbnailAction>,
    /// The picked pages changed (⌘/Ctrl- and ⇧-click, Esc); the selection was updated in place.
    pub picked_changed: bool,
    /// Files dragged in from outside are over the gap before this page (the app takes the drop).
    pub file_gap: Option<usize>,
    pub response: Response,
}

/// The app's menu for the picked pages.
type PageMenu<'a> = Box<dyn FnMut(&mut Ui, &[usize]) + 'a>;
/// The app's menu for a section header.
type SectionMenu<'a> = Box<dyn FnMut(&mut Ui, usize) + 'a>;

/// The thumbnails panel body, built per pass.
///
/// - Each thumbnail is `w = min(USUAL_WIDTH, panel - 40) × zoom` wide (at most the panel less
///   16), in a cell 16 pt larger, the page's aspect kept; rows 4 pt apart. Only rows within
///   200 pt of the view ask [`PageSource::texture`] (DeckCraft: two rows) so a long document
///   never evicts its own thumbnails.
/// - The current page has a 2 pt accent border (1 pt border otherwise) and the selected-row
///   wash; picked pages share the wash with a 1.5 pt accent outline (Acrobat). With
///   [`LabelPlace::Left`] the current and picked pages get a 3 pt accent outline 2 pt out
///   (PowerPoint).
/// - Click goes to the page; ⌘/Ctrl-click toggles a page in the picked set and ⇧-click extends
///   it, neither moving the document. A new current page scrolls its thumbnail into view as
///   little as it takes, not while dragging.
/// - Drag with the primary button: an unpicked page grabbed becomes the picked set; a 3 pt accent
///   bar shows the gap (above a thumbnail's middle is the gap before it), "1 page" / "{n} pages"
///   by the pointer, the list scrolls within 40 pt of its edges; release reports
///   [`ThumbnailAction::Move`], or nothing when the pages wouldn't move. Esc cancels the drag.
/// - Keys: the panel is one Tab stop (an AccessKit Group named "Pages"); ↑/↓ move the current
///   page, ⇧ extends, Esc clears the picked set, Delete and Backspace delete (when editable).
///   Each thumbnail is a Button node "Page {label}", selected when picked.
/// - Right-click: an unpicked page becomes the picked set, then the app's menu opens for it.
pub struct PageThumbnails<'a> {
    id_salt: egui::IdSalt,
    source: &'a mut dyn PageSource,
    picked: &'a mut Selection<usize>,
    current: usize,
    editable: bool,
    zoom: Option<&'a mut f32>,
    label_place: LabelPlace,
    sections: &'a [ThumbnailSection],
    marks: Option<&'a dyn Fn(usize) -> ThumbnailMarks>,
    new_page_row: bool,
    context_menu: Option<PageMenu<'a>>,
    section_menu: Option<SectionMenu<'a>>,
    labels: Option<ThumbnailLabels>,
}

impl<'a> PageThumbnails<'a> {
    /// Thumbnails of `source`'s pages; `picked` is the set page commands act on, kept by the app.
    pub fn new(id_salt: impl egui::AsIdSalt, source: &'a mut dyn PageSource, picked: &'a mut Selection<usize>, current: usize) -> Self {
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
            context_menu: None,
            section_menu: None,
            labels: None,
        }
    }

    /// Off: no drag, no Delete (a read-only document).
    pub fn editable(mut self, on: bool) -> Self {
        self.editable = on;
        self
    }

    /// Show the Thumbnail size row (label, percentage, −, slider, +) above the list, editing
    /// `zoom` within [`THUMBNAIL_ZOOM_RANGE`]; the row stays above the scroll area.
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

    /// The "+ New Slide" hint row under the last thumbnail (PowerPoint).
    pub fn new_page_row(mut self, on: bool) -> Self {
        self.new_page_row = on;
        self
    }

    /// The menu for the picked pages (Extract, Cut, Copy, Paste after, Delete…: the app's
    /// commands).
    pub fn context_menu(mut self, menu: impl FnMut(&mut Ui, &[usize]) + 'a) -> Self {
        self.context_menu = Some(Box::new(menu));
        self
    }

    /// The menu of section header `index` (Rename, Remove, Move Up/Down).
    pub fn section_menu(mut self, menu: impl FnMut(&mut Ui, usize) + 'a) -> Self {
        self.section_menu = Some(Box::new(menu));
        self
    }

    pub fn labels(mut self, labels: ThumbnailLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/panels/pages.rs:90 `pages`, :36 `thumbnail_size`, :295
    // `page_drag`, :277 `panel_gap`; deckcraft: crates/ui-egui/src/panels/thumbs.rs:116 `list`
    // (sections, number at the left, hidden and animation marks, + New Slide).
    pub fn show(self, ui: &mut Ui) -> ThumbnailResponse {
        todo!()
    }
}
