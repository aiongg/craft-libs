//! Screen colour picker: the loupe beside the pointer over the captured screen. The numbers are
//! the same at every Size, and no theme overrides them; the loupe's card and text are semantic tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Screen colour picker tokens. Cells and zoom steps are counts; no size path reaches them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScreenPickerTokens {
    /// Screen pixels across and down the loupe: `screen_picker.loupe_cells`.
    pub loupe_cells: u32,
    /// The loupe's starting zoom, in physical pixels per screen pixel: `screen_picker.zoom`.
    pub zoom: u32,
    /// The least zoom: `screen_picker.zoom_min`.
    pub zoom_min: u32,
    /// The greatest zoom: `screen_picker.zoom_max`.
    pub zoom_max: u32,
    /// What one wheel step changes the zoom by: `screen_picker.zoom_step`.
    pub zoom_step: u32,
}

/// Screen pickers have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScreenPickerField {}

impl Family for ScreenPickerTokens {
    type Field = ScreenPickerField;
    const FIELDS: &'static [ScreenPickerField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = &[];

    fn defaults(_: &Semantic<'_>) -> ScreenPickerTokens {
        ScreenPickerTokens {
            loupe_cells: values::SCREEN_PICKER_LOUPE_CELLS,
            zoom: values::SCREEN_PICKER_ZOOM,
            zoom_min: values::SCREEN_PICKER_ZOOM_MIN,
            zoom_max: values::SCREEN_PICKER_ZOOM_MAX,
            zoom_step: values::SCREEN_PICKER_ZOOM_STEP,
        }
    }

    fn path(f: ScreenPickerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ScreenPickerField> {
        None
    }

    fn kind(f: ScreenPickerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ScreenPickerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ScreenPickerField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
