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
    /// Width of Save for Web's settings column: `export.web_settings_width`.
    pub web_settings_width: f32,
    /// Width of Save for Web's Matte field: `export.web_field_width`.
    pub web_field_width: f32,
    /// Width of Export As's settings column: `export.settings_width`.
    pub settings_width: f32,
    /// Between Export As's settings and its preview: `export.pane_gap`.
    pub pane_gap: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExportField {}

impl Family for ExportTokens {
    type Field = ExportField;
    const FIELDS: &'static [ExportField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ExportTokens;
        "web_width" 0.0..=4096.0 => web_width;
        "web_settings_width" 0.0..=1024.0 => web_settings_width;
        "web_field_width" 0.0..=1024.0 => web_field_width;
        "settings_width" 0.0..=1024.0 => settings_width;
        "pane_gap" 0.0..=64.0 => pane_gap;
    };

    fn defaults(_: &Semantic<'_>) -> ExportTokens {
        ExportTokens {
            web_width: values::EXPORT_WEB_WIDTH,
            web_settings_width: values::EXPORT_WEB_SETTINGS_WIDTH,
            web_field_width: values::EXPORT_WEB_FIELD_WIDTH,
            settings_width: values::EXPORT_SETTINGS_WIDTH,
            pane_gap: values::EXPORT_PANE_GAP,
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
