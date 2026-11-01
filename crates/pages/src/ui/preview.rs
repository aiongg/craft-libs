//! [`PagePreview`] (N3.16): a page shown large over the window with previous and next, as
//! PdfCraft's Combine Files preview and VectorCraft's Place PDF page choice show it: the page as
//! large as fits, "Page 3 of 12", ← / → (and Page Up / Page Down) to step, Esc or the close
//! button to leave.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Context;

use super::view::PageSource;

craft_ui::words! {
    /// The preview's words (PdfCraft's strings).
    #[non_exhaustive]
    pub struct PagePreviewLabels {
        page_of: "pagePreview.pageOf" = "Page {p} of {n}";
        previous: "pagePreview.previous" = "Previous page";
        next: "pagePreview.next" = "Next page";
        close: "pagePreview.close" = "Close";
        unavailable: "pagePreview.unavailable" = "Preview unavailable";
    }
}

/// What the preview did this pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PagePreviewResponse {
    /// The page shown changed (the app may sync its grid's selection).
    pub stepped: bool,
    /// Esc or the close button.
    pub closed: bool,
}

/// The preview over the window, shown each pass while the app keeps it open.
///
/// - A dimmed backdrop over the window (the dialog framework's backdrop); the page centred, as
///   large as fits, its picture through craft-ui's `Thumbnails<usize>` over the [`PageSource`]
///   (side from `ThumbRequest::for_cell`, at most `MAX_TEXTURE`; the last picture stays as the
///   stand-in until the new one lands). A page that fails to render shows the texture
///   contract's failure look with [`PagePreviewLabels::unavailable`] under it; the failure's
///   reason is in the picture's tooltip, as everywhere else.
/// - Below it, centred: previous, "Page {p} of {n}", next; the buttons are disabled at the ends.
///   The pages stepped through are `pages` (the pages a range selects), in order.
/// - Keys: ← and Page Up previous, → and Page Down next (not while a text field has the
///   keyboard), Esc closes; +, = , − and 0 are taken so the grid underneath doesn't zoom.
/// - Focus moves into the preview when it opens and back when it closes; it is an AccessKit
///   Dialog named "Page {p} of {n}".
pub struct PagePreview<'a> {
    id_salt: egui::IdSalt,
    source: &'a dyn PageSource,
    pages: &'a [usize],
    at: &'a mut usize,
    labels: Option<PagePreviewLabels>,
}

impl<'a> PagePreview<'a> {
    /// Preview `pages` (0-based), showing `pages[*at]`.
    pub fn new(id_salt: impl egui::AsIdSalt, source: &'a dyn PageSource, pages: &'a [usize], at: &'a mut usize) -> Self {
        PagePreview { id_salt: egui::IdSalt::new(id_salt), source, pages, at, labels: None }
    }

    pub fn labels(mut self, labels: PagePreviewLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/workflows/combine_grid/card.rs:326 `preview`; vectorcraft:
    // crates/ui-egui/src/dialogs/import_pdf.rs (page choice with a preview).
    pub fn show(self, ctx: &Context) -> PagePreviewResponse {
        todo!()
    }
}
