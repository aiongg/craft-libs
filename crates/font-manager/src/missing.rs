//! Document fonts and missing fonts (F14.08): which fonts a document's text uses, which of them
//! are not installed, and the dialog that lists them with a replacement for each (Photoshop's
//! "missing fonts" warning on open; VectorCraft's and FigCraft's Document Fonts lists).

use std::sync::Arc;

use craft_font_catalog::{Catalog, Match};
use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::composites::{FormGrid, SectionHeader, SectionLabel};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, Outcome, StackDialog, Width};
use craft_ui::list::{List, ListRow};
use craft_ui::localized::words::Counted;
use craft_ui::localized::{Labels as _, Words};
use egui::{Context, Ui};

/// The message's count word: `{n}` fonts, in the plural forms of the language.
const MESSAGE: Counted =
    Counted::new("missingFonts.message", "{n} font used in this document is not installed.", "{n} fonts used in this document are not installed.");

/// The family text falls back to when a missing font gets no replacement (PhotoCraft's default
/// type family).
const DEFAULT_FAMILY: &str = "Inter";

craft_ui::words! {
    /// The dialog's words, English by default.
    #[non_exhaustive]
    pub struct MissingFontsLabels {
        title: "missingFonts.title" = "Missing Fonts";
        message: "missingFonts.message" = "{n} fonts used in this document are not installed.";
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
/// A font with an empty family is `Installed`; one whose PostScript name an installed face has is
/// `Installed` whatever its family says; otherwise family and style go through
/// [`Catalog::resolve`].
// pc: crates/engine/src/cmd/type_extra_cmds.rs:617 `missing_fonts` (PostScript name first, then
// family + style through `Catalog::resolve`).
pub fn document_fonts(catalog: &Catalog, fonts: &[DocumentFont]) -> Vec<DocumentFontRow> {
    let mut rows: Vec<DocumentFontRow> = Vec::new();
    for font in fonts {
        if rows.iter().any(|r| r.font == *font) {
            continue;
        }
        let status = if font.family.is_empty() || font.postscript_name.as_deref().is_some_and(|ps| catalog.by_postscript_name(ps).is_some()) {
            FontStatus::Installed
        } else {
            match catalog.resolve(&font.family, &font.style) {
                Some((_, Match::Exact)) => FontStatus::Installed,
                Some((face, Match::NearestStyle)) => FontStatus::OtherStyle(face.style),
                None => FontStatus::Missing,
            }
        };
        rows.push(DocumentFontRow { font: font.clone(), status });
    }
    rows
}

/// One warning per missing family for the open job's warning list, in document order (the text
/// PhotoCraft's `missing_font_warnings` writes).
// pc: crates/engine/src/cmd/type_extra_cmds.rs:647 `missing_font_warnings`.
pub fn missing_font_warnings(rows: &[DocumentFontRow]) -> Vec<String> {
    let mut families: Vec<&str> = Vec::new();
    for row in rows.iter().filter(|r| r.status == FontStatus::Missing) {
        if !families.contains(&row.font.family.as_str()) {
            families.push(&row.font.family);
        }
    }
    families.into_iter().map(|family| format!("font '{family}' is not installed; text may render using a fallback face")).collect()
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
    default_family: String,
    labels: Option<MissingFontsLabels>,
    state: DialogState<Vec<Option<String>>>,
}

impl MissingFontsDialog {
    /// Over `rows` from [`document_fonts`]; only `Missing` rows get a replacement picker.
    pub fn new(catalog: Arc<Catalog>, rows: Vec<DocumentFontRow>) -> Self {
        let n = rows.len();
        MissingFontsDialog {
            rows,
            catalog,
            replacements: vec![None; n],
            default_family: DEFAULT_FAMILY.to_owned(),
            labels: None,
            state: DialogState::new(),
        }
    }

    /// The family a missing font gets when the user sets none (default `Inter`, PhotoCraft's
    /// default type family).
    pub fn default_family(mut self, family: impl Into<String>) -> Self {
        self.default_family = family.into();
        self
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
        let Self { rows, catalog, replacements, default_family, labels, state } = self;
        let labels = labels.clone().unwrap_or_else(|| MissingFontsLabels::of_ctx(ctx));
        let missing = rows.iter().filter(|r| r.status == FontStatus::Missing).count();
        let message = Words::of_ctx(ctx).counted(ctx, MESSAGE, &labels.message, missing as u64);
        let footer = Footer::standard().ok(labels.replace.clone()).cancel(labels.dont_replace.clone());
        let families = catalog.families();
        let shown = Dialog::new("missing-fonts", labels.title.clone()).stack(stack).width(Width::Wide).footer(footer).show(
            ctx,
            state,
            replacements,
            |ui, replacements| {
                ui.label(message.as_str());
                ui.add_space(ui.spacing().item_spacing.y);
                SectionLabel::new(labels.replace_with.as_str()).show(ui);
                FormGrid::new("missing-fonts-rows").show(ui, |form| {
                    for (i, row) in rows.iter().enumerate().filter(|(_, r)| r.status == FontStatus::Missing) {
                        let name = font_name(&row.font);
                        form.row(format!("{name} ({})", labels.missing).as_str(), |ui, _| {
                            replacement_field(ui, catalog, &families, i, &mut replacements[i], default_family, &labels)
                        });
                    }
                });
                document_fonts_list(ui, rows, &labels);
            },
        );
        match shown.outcome {
            Some(Outcome::Ok) => {
                let replacements = rows
                    .iter()
                    .zip(replacements.iter())
                    .filter(|(r, _)| r.status == FontStatus::Missing)
                    .map(|(r, to)| (r.font.clone(), to.clone().unwrap_or_else(|| default_family.clone())))
                    .collect();
                Some(DialogResult::Ok(MissingFontsResult { replacements }))
            }
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}

fn font_name(font: &DocumentFont) -> String {
    if font.style.is_empty() { font.family.clone() } else { format!("{} {}", font.family, font.style) }
}

/// The Replace With cell of row `i`: a font picker, or a `Select` of installed families without
/// the `font-picker` feature.
fn replacement_field(
    ui: &mut Ui,
    catalog: &Arc<Catalog>,
    families: &Arc<[String]>,
    i: usize,
    chosen: &mut Option<String>,
    default_family: &str,
    labels: &MissingFontsLabels,
) -> egui::Response {
    let mut current = chosen.clone().unwrap_or_else(|| default_family.to_owned());
    #[cfg(feature = "font-picker")]
    let response = {
        let _ = (families, labels);
        let salt = format!("missing-fonts-{i}");
        let out = crate::picker::FontPicker::new(&salt, catalog, &mut current).show(ui);
        if let Some(picked) = out.picked {
            *chosen = Some(picked);
        }
        out.response
    };
    #[cfg(not(feature = "font-picker"))]
    let response = {
        let _ = catalog;
        let list = Arc::clone(families);
        let value = Arc::clone(families);
        let edit = craft_ui::controls::Select::new(("missing-fonts-select", i), labels.replace_with.clone(), &mut current)
            .rows(families.len(), move |k| value[k].clone(), move |k| list[k].clone())
            .placeholder(labels.default_replacement.clone())
            .show(ui);
        if edit.event.is_some() {
            *chosen = Some(current);
        }
        edit.response
    };
    response
}

/// The Document Fonts expander: every font with its status.
fn document_fonts_list(ui: &mut Ui, rows: &[DocumentFontRow], labels: &MissingFontsLabels) {
    let id = ui.scope_id().with("document-fonts-open");
    let mut open = ui.data_mut(|d| d.get_persisted::<bool>(id)).unwrap_or(false);
    SectionHeader::new("missing-fonts-all", labels.used_fonts.clone()).collapsible(true).open(&mut open).show(ui, |ui| {
        let order: Vec<RowInfo<usize>> = (0..rows.len()).map(RowInfo::leaf).collect();
        let mut selection = Selection::default();
        let text: Vec<String> = rows
            .iter()
            .map(|r| {
                let status = match &r.status {
                    FontStatus::Installed => labels.installed.clone(),
                    FontStatus::OtherStyle(style) => format!("{} ({style})", labels.installed),
                    FontStatus::Missing => labels.missing.clone(),
                };
                format!("{}: {status}", font_name(&r.font))
            })
            .collect();
        let _ = List::new(ui.scope_id().with("document-fonts"), labels.font.clone(), order.as_slice())
            .mode(SelectionMode::Single)
            .selection(&mut selection)
            .max_height(160.0)
            .show(ui, |_, &key| ListRow::new(key, text[key].as_str()));
    });
    ui.data_mut(|d| d.insert_persisted(id, open));
}
