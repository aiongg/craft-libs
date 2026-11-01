//! Export As and Save for Web: the widths of the dialogs and their settings columns. Sizes only (no colours): the colours are the surface, text and state tokens of the controls
//! inside, and no theme overrides these. The values are PhotoCraft's.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Export tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ExportTokens {
    /// Content width of Save for Web: `export.web_width`.
    pub web_width: f32,
    /// Width of Save for Web's Matte field: `export.web_field_width`.
    pub web_field_width: f32,
    /// Width of the settings column of Export As and Save for Web: `export.settings_width`.
    pub settings_width: f32,
    /// Between Export As's settings and its preview: `export.pane_gap`.
    pub pane_gap: f32,
    /// Width of the Save PDF dialog: `export.pdf_width`.
    pub pdf_width: f32,
    /// Width of Save PDF's section list: `export.pdf_list_width`.
    pub pdf_list_width: f32,
    /// Width of Save PDF's section content: `export.pdf_content_width`.
    pub pdf_content_width: f32,
    /// Height of Save PDF's section content before it scrolls: `export.pdf_content_height`.
    pub pdf_content_height: f32,
    /// Between Save PDF's section list and its content: `export.pdf_gap`.
    pub pdf_gap: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExportField {}

impl Family for ExportTokens {
    type Field = ExportField;
    const FIELDS: &'static [ExportField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ExportTokens;
        "web_width" 0.0..=4096.0 => web_width;
        "web_field_width" 0.0..=1024.0 => web_field_width;
        "settings_width" 0.0..=1024.0 => settings_width;
        "pane_gap" 0.0..=64.0 => pane_gap;
        "pdf_width" 0.0..=4096.0 => pdf_width;
        "pdf_list_width" 0.0..=1024.0 => pdf_list_width;
        "pdf_content_width" 0.0..=2048.0 => pdf_content_width;
        "pdf_content_height" 0.0..=2048.0 => pdf_content_height;
        "pdf_gap" 0.0..=64.0 => pdf_gap;
    };

    fn defaults(_: &Semantic<'_>) -> ExportTokens {
        ExportTokens {
            web_width: values::EXPORT_WEB_WIDTH,
            web_field_width: values::EXPORT_WEB_FIELD_WIDTH,
            settings_width: values::EXPORT_SETTINGS_WIDTH,
            pane_gap: values::EXPORT_PANE_GAP,
            pdf_width: values::EXPORT_PDF_WIDTH,
            pdf_list_width: values::EXPORT_PDF_LIST_WIDTH,
            pdf_content_width: values::EXPORT_PDF_CONTENT_WIDTH,
            pdf_content_height: values::EXPORT_PDF_CONTENT_HEIGHT,
            pdf_gap: values::EXPORT_PDF_GAP,
        }
    }

    fn path(f: ExportField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ExportField> {
        None
    }

    fn kind(f: ExportField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ExportField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ExportField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
