//! Document fonts and missing fonts (F14.08): which fonts a document's text uses, which of them
//! are not installed, and the dialog that lists them with a replacement for each (Photoshop's
//! "missing fonts" warning on open; VectorCraft's and FigCraft's Document Fonts lists).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::sync::Arc;

use craft_font_catalog::Catalog;
use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
use egui::Context;

craft_ui::words! {
    /// The dialog's words, English by default.
    #[non_exhaustive]
    pub struct MissingFontsLabels {
        title: "missingFonts.title" = "Missing Fonts";
        message: "missingFonts.message" = "{count} fonts used in this document are not installed.";
        font: "missingFonts.font" = "Font";
        replace_with: "missingFonts.replaceWith" = "Replace With";
        default_replacement: "missingFonts.default" = "Default font";
        replace: "missingFonts.replace" = "Replace Fonts";
        dont_replace: "missingFonts.dontReplace" = "Don't Replace";
        used_fonts: "missingFonts.usedFonts" = "Document Fonts";
        installed: "missingFonts.installed" = "Installed";
        missing: "missingFonts.missing" = "Missing";
    }
}

/// A font a document names: family and style as stored in the document.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocumentFont {
    pub family: String,
    pub style: String,
    /// The PostScript name, when the document stores one (PDF, PSD).
    pub postscript_name: Option<String>,
}

/// Whether a document font resolves on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontStatus {
    /// Installed as named.
    Installed,
    /// Another style of the family stands in (`Match::Style` in the catalog).
    OtherStyle(String),
    /// Nothing matches; text shapes with the default font until replaced.
    Missing,
}

/// A document font with its status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentFontRow {
    pub font: DocumentFont,
    pub status: FontStatus,
}

/// Resolve each of `fonts` against the installed catalog, document order kept, duplicates merged.
// pc: crates/engine/src/cmd/type_extra_cmds.rs:617 `missing_fonts` (PostScript name first, then
// family + style through `Catalog::resolve`).
pub fn document_fonts(catalog: &Catalog, fonts: &[DocumentFont]) -> Vec<DocumentFontRow> {
    todo!()
}

/// Photoshop's one-line warnings for a document's missing fonts ("The font ‘X’ is missing…"), for
/// the open job's warning list (gap: PhotoCraft `file_open::with_missing_fonts`).
// pc: crates/engine/src/cmd/type_extra_cmds.rs:647 `missing_font_warnings`.
pub fn missing_font_warnings(rows: &[DocumentFontRow]) -> Vec<String> {
    todo!()
}

/// What the user chose for the missing fonts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MissingFontsResult {
    /// (missing font, replacement family) for every row the user set; the default font where unset.
    pub replacements: Vec<(DocumentFont, String)>,
}

/// The Missing Fonts dialog on the dialog host, opened by the app when a document with missing
/// fonts opens or when the user picks Type › Manage Missing Fonts.
///
/// A message with the count, then a table: Font (family and style, marked missing) and Replace
/// With (a [`FontPicker`](crate::picker::FontPicker) per row when `font-picker` is on, else a
/// `Select` of installed families), a Document Fonts expander listing every font with its status,
/// and the footer Replace Fonts (default) / Don't Replace (Cancel). Tab walks the pickers; Enter
/// is Replace Fonts. Result: [`MissingFontsResult`] on Replace; Cancel leaves the text on the
/// default font with the names kept.
pub struct MissingFontsDialog {
    rows: Vec<DocumentFontRow>,
    catalog: Arc<Catalog>,
    replacements: Vec<Option<String>>,
    labels: Option<MissingFontsLabels>,
}

impl MissingFontsDialog {
    /// Over `rows` from [`document_fonts`]; only `Missing` rows get a replacement picker.
    pub fn new(catalog: Arc<Catalog>, rows: Vec<DocumentFontRow>) -> Self {
        let n = rows.len();
        MissingFontsDialog { rows, catalog, replacements: vec![None; n], labels: None }
    }

    pub fn labels(mut self, labels: MissingFontsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// How many rows are missing.
    pub fn missing_count(&self) -> usize {
        self.rows.iter().filter(|r| r.status == FontStatus::Missing).count()
    }
}

impl StackDialog for MissingFontsDialog {
    type Result = MissingFontsResult;

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("missing-fonts")
    }

    // new: no PhotoCraft dialog (it only warns); layout as Photoshop's Missing Fonts window.
    // Rows on `Dialog` + `FormGrid`, footer through `Footer`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<MissingFontsResult>> {
        todo!()
    }
}
