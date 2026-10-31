//! [`HeaderFooterZones`] (N3.10): the header and footer bands of the pages on screen, each in three
//! sections (left, centre, right), as Excel's Page Layout view shows them: the hovered section
//! says "Add header" / "Add footer" when empty, a click starts editing it, and the app's editor
//! takes the section's rectangle. What each section holds, its fields and the editor are the
//! app's.
//!
//! ```ignore
//! let r = HeaderFooterZones::new("hf", &view.pages, &bands).text(&|page, zone| doc.hf_text(page, zone)).editing(edit.map(|e| e.at)).show(ui);
//! if let Some(at) = r.begin { edit = Some(ZoneEditor::new(at, doc.hf_source(at))); }
//! if let Some(editor) = &mut edit { editor.show(ui, r.rect(at)); }   // commit on a click elsewhere
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Rect, Response, Ui};

use super::view::PlacedPage;

craft_ui::words! {
    /// The zones' words (GridCraft's strings).
    #[non_exhaustive]
    pub struct HeaderFooterLabels {
        add_header: "pages.addHeader" = "Add header";
        add_footer: "pages.addFooter" = "Add footer";
        header: "pages.header" = "Header";
        footer: "pages.footer" = "Footer";
        first_page: "pages.firstPageHeader" = "First Page Header";
        even_page: "pages.evenPageHeader" = "Even Page Header";
        odd_page: "pages.oddPageHeader" = "Odd Page Header";
    }
}

/// One of the six sections of a page's header and footer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Zone {
    pub footer: bool,
    /// 0 left, 1 centre, 2 right (in reading order: mirrored for a right-to-left page).
    pub section: u8,
}

/// A section of one page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ZoneAt {
    pub page: usize,
    pub zone: Zone,
}

/// Which header and footer a page uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HeaderFooterPage {
    First,
    Odd,
    Even,
}

impl HeaderFooterPage {
    /// The kind for 1-based page `number` with Different First Page and Different Odd & Even.
    // gridcraft: crates/model/src/features/print.rs:86 `hf_page`; crates/ui-egui/src/paper_view.rs:354
    // `page_kind`.
    pub fn of(number: usize, different_first: bool, different_odd_even: bool) -> Self {
        todo!()
    }
}

/// Where the bands are on each page, page points: the header band from the top edge down to the
/// top margin and the footer band from the bottom margin to the bottom edge, inset left and right
/// by `sides` (GridCraft's `hf_sides`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoneBands {
    pub top: f64,
    pub bottom: f64,
    pub sides: [f64; 2],
}

/// What the zones did this pass.
#[derive(Debug)]
pub struct HeaderFooterResponse {
    /// A click on a section: start (or move) editing there.
    pub begin: Option<ZoneAt>,
    /// A click on a page outside the zones while one was being edited: commit it.
    pub end: bool,
    pub response: Response,
}

/// The zones over the pages on screen.
///
/// - Sections split the band's inner width in thirds, 3 screen points inset top and bottom.
/// - A hovered empty section shows a dashed outline and "Add header" / "Add footer"; the section
///   being edited ([`editing`](Self::editing)) is filled white with an accent outline, its
///   neighbours outlined.
/// - Text the app gives is drawn left, centred and right aligned in its section, at 10 pt × the
///   page scale, clamped 4–120 screen points (GridCraft's `hf_font`).
/// - Each section is an AccessKit Button ("Add header" or its text) in the view's tree; Tab
///   doesn't stop on them (the app's Insert › Header & Footer starts editing from the keyboard).
pub struct HeaderFooterZones<'a> {
    id_salt: egui::IdSalt,
    placed: &'a [PlacedPage],
    bands: &'a dyn Fn(usize) -> ZoneBands,
    text: Option<&'a dyn Fn(ZoneAt) -> String>,
    editing: Option<ZoneAt>,
    labels: Option<HeaderFooterLabels>,
}

impl<'a> HeaderFooterZones<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, placed: &'a [PlacedPage], bands: &'a dyn Fn(usize) -> ZoneBands) -> Self {
        HeaderFooterZones { id_salt: egui::IdSalt::new(id_salt), placed, bands, text: None, editing: None, labels: None }
    }

    /// Each section's text as it prints (fields expanded by the app).
    pub fn text(mut self, text: &'a dyn Fn(ZoneAt) -> String) -> Self {
        self.text = Some(text);
        self
    }

    pub fn editing(mut self, at: Option<ZoneAt>) -> Self {
        self.editing = at;
        self
    }

    pub fn labels(mut self, labels: HeaderFooterLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The section's rectangle on screen, for the app's editor; `None` when its page isn't
    /// placed.
    // gridcraft: crates/ui-egui/src/paper_view.rs:194 `section_rects`.
    pub fn rect(&self, at: ZoneAt) -> Option<Rect> {
        todo!()
    }

    // gridcraft: crates/ui-egui/src/paper_view.rs:217 `paint_headers_footers`, :291 `interact`.
    pub fn show(self, ui: &mut Ui) -> HeaderFooterResponse {
        todo!()
    }
}
