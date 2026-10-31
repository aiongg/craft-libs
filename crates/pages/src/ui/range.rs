//! [`PageRangeField`] (N3.13): Acrobat's Pages selected field: a text field taking "1-3, 5"
//! with a chevron menu of page choices, showing the selection back in the same format; and
//! [`PageRangeOptions`], the All pages / Pages from–to / Subset row of Acrobat's Page Range
//! Options.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Response, Ui};

use crate::range::{PageRange, PageRangeError};

craft_ui::words! {
    /// The field's and row's words (PdfCraft's strings).
    #[non_exhaustive]
    pub struct PageRangeLabels {
        selected_pages: "pageRange.selectedPages" = "Selected pages";
        select_pages: "pageRange.selectPages" = "Select pages";
        not_a_page: "pageRange.notAPage" = "{page} is not a page number or label";
        out_of_range: "pageRange.outOfRange" = "page {n} is out of range (1–{count})";
        too_long: "pageRange.tooLong" = "Too many pages";
        all_pages: "pageRange.allPages" = "All pages";
        pages_from: "pageRange.pagesFrom" = "Pages from";
        to: "pageRange.to" = "to";
        subset: "pageRange.subset" = "Subset";
        all_in_range: "pageRange.allInRange" = "All pages in range";
        even_only: "pageRange.evenOnly" = "Even pages only";
        odd_only: "pageRange.oddOnly" = "Odd pages only";
    }
}

impl PageRangeLabels {
    /// The error in these words.
    pub fn error(&self, error: &PageRangeError) -> String {
        todo!()
    }
}

/// The field's text and error between passes, kept by the app per field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageRangeFieldState {
    text: String,
    shown: Vec<usize>,
    error: Option<PageRangeError>,
}

/// The app's page choices menu: its pick, if any.
type RangeMenu<'a> = Box<dyn FnOnce(&mut Ui) -> Option<Vec<usize>> + 'a>;

/// The field, built per pass.
///
/// - Shows `selected` formatted ([`format_pages`](crate::range::format_pages)); with nothing
///   selected, the current page as its placeholder (the page actions apply to).
/// - Typed text applies on Enter or when the field loses focus: parsed text returns the pages and
///   shows them back formatted; text that doesn't parse stays, with the error under the field.
///   Esc puts the selection's text back.
/// - The chevron opens the app's menu (pages by kind: Odd, Even, Portrait, Landscape, All); its
///   pick returns like typed pages.
/// - The field is one Tab stop (AccessKit TextInput "Selected pages"), the chevron another.
pub struct PageRangeField<'a> {
    id_salt: egui::IdSalt,
    state: &'a mut PageRangeFieldState,
    selected: &'a [usize],
    count: usize,
    current: usize,
    menu: Option<RangeMenu<'a>>,
    labels: Option<PageRangeLabels>,
}

impl<'a> PageRangeField<'a> {
    /// The field over `selected` (0-based) in a document of `count` pages; `current` is 0-based.
    pub fn new(id_salt: impl egui::AsIdSalt, state: &'a mut PageRangeFieldState, selected: &'a [usize], count: usize, current: usize) -> Self {
        PageRangeField { id_salt: egui::IdSalt::new(id_salt), state, selected, count, current, menu: None, labels: None }
    }

    pub fn menu(mut self, menu: impl FnOnce(&mut Ui) -> Option<Vec<usize>> + 'a) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }

    pub fn labels(mut self, labels: PageRangeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The pages to select, on the pass they were typed or picked.
    // pdfcraft: crates/ui-egui/src/panels/page_selection_ui.rs:64 `PageSelectionInput::show`.
    pub fn show(self, ui: &mut Ui) -> Option<Vec<usize>> {
        todo!()
    }
}

/// The All pages / Pages from [ ] to [ ] / Subset [ ] row, editing a [`PageRange`]; From and To
/// are 1 to `count` and disabled with All pages. Returns whether the range changed.
pub struct PageRangeOptions<'a> {
    range: &'a mut PageRange,
    count: usize,
    labels: Option<PageRangeLabels>,
}

impl<'a> PageRangeOptions<'a> {
    pub fn new(range: &'a mut PageRange, count: usize) -> Self {
        PageRangeOptions { range, count, labels: None }
    }

    pub fn labels(mut self, labels: PageRangeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/dialogs/marks_ui.rs:58 `PageRange::ui`.
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
