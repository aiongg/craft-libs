//! Page ranges (N3.13): pages typed as "1-3, 5" (Acrobat's Pages selected and Print's Pages
//! field), and Acrobat's Page Range Options (All pages or From/To, with an even/odd subset).
//! Page numbers are 1-based in text and 0-based in the API. [`crate::ui::range`] has the field.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

/// The longest range text read, in bytes.
pub const MAX_RANGE_TEXT: usize = 16_384;

/// The most pages one range text may expand to, counting overlaps again ("1-100000, 1-100000").
pub const MAX_EXPANDED: usize = 1_000_000;

/// Why a range text can't be read. `Display` gives PdfCraft's English sentence; the field shows
/// its translation ([`crate::ui::range::PageRangeLabels`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageRangeError {
    /// `"{page}" is not a page number or label` (the token as typed).
    NotAPage(String),
    /// `page {n} is out of range (1–{count})`.
    OutOfRange { page: usize, count: usize },
    /// Longer than [`MAX_RANGE_TEXT`], or expanding past [`MAX_EXPANDED`].
    TooLong,
}

impl std::fmt::Display for PageRangeError {
    // pdfcraft: crates/print/src/error.rs:46 `PrintError::said`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for PageRangeError {}

/// The pages (0-based, sorted, each once) `text` names in a document of `count` pages: numbers and
/// `a-b` ranges separated by `,` or `;`, spaces anywhere, a reversed range read forwards. Blank
/// text is no pages (an explicit selection cleared).
// pdfcraft: crates/ui-egui/src/panels/page_selection_ui.rs:15 `parse`.
pub fn parse_pages(text: &str, count: usize) -> Result<Vec<usize>, PageRangeError> {
    todo!()
}

/// `pages` (0-based, any order) as range text: runs joined, "1-3, 5, 8-9".
// pdfcraft: crates/ui-egui/src/panels/page_selection_ui.rs:43 `formatted`; vectorcraft:
// crates/ui-egui/src/dialogs/export_for_screens.rs:262 `range_text`.
pub fn format_pages(pages: &[usize]) -> String {
    todo!()
}

/// Which pages of a From/To range count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Subset {
    /// All pages in range.
    #[default]
    All,
    /// Even pages only (2, 4, … by page number).
    Even,
    /// Odd pages only.
    Odd,
}

/// Acrobat's Page Range Options: All pages, or Pages from `from` to `to` (1-based), and a subset.
/// Shared by the Header & Footer, Watermark and Background dialogs in PdfCraft.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PageRange {
    pub all: bool,
    pub from: usize,
    /// Starts past any page count, so the field shows (and clamps it to) the last page.
    pub to: usize,
    pub subset: Subset,
}

impl Default for PageRange {
    fn default() -> Self {
        PageRange { all: true, from: 1, to: usize::MAX, subset: Subset::All }
    }
}

impl PageRange {
    /// The pages (0-based) the range selects in a document of `count` pages.
    // pdfcraft: crates/ui-egui/src/dialogs/marks_ui.rs:46 `PageRange::pages`.
    pub fn pages(&self, count: usize) -> Vec<usize> {
        todo!()
    }
}
