//! Preferences windows: the page list column and the gap between it and the page. Sizes only (no
//! colours): the look is the list, field and surface families'.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Preferences window tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PreferencesTokens {
    /// Width of the page list column: `preferences.list_width`.
    pub list_width: f32,
    /// Between the page list and the page: `preferences.page_gap`, `pad.panel`.
    pub page_gap: f32,
    /// Width of a choice row's control: `preferences.choice_width`, 220 at Compact, scaled by the
    /// control height like a dialog's sizes.
    pub choice_width: f32,
    /// Width of a text row's control: `preferences.text_width`, 260 at Compact, scaled likewise.
    pub text_width: f32,
    /// Between the divider and the page's contents, and between them and the trailing edge:
    /// `preferences.page_inset`.
    pub page_inset: f32,
    /// Height of a page list row: `preferences.page_row_height`.
    pub page_row_height: f32,
    /// Width of a colour row's hex field: `preferences.hex_width`, 72 at Compact, scaled by the
    /// control height.
    pub hex_width: f32,
    /// Height of a page body laid out in an unbounded rect: `preferences.body_fallback_height`.
    pub body_fallback_height: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PreferencesField {}

impl Family for PreferencesTokens {
    type Field = PreferencesField;
    const FIELDS: &'static [PreferencesField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { PreferencesTokens;
        "list_width" 0.0..=1024.0 => list_width;
        "page_gap" 0.0..=64.0 => page_gap;
        "choice_width" 0.0..=1024.0 => choice_width;
        "text_width" 0.0..=1024.0 => text_width;
        "page_inset" 0.0..=64.0 => page_inset;
        "page_row_height" 0.0..=1024.0 => page_row_height;
        "hex_width" 0.0..=1024.0 => hex_width;
        "body_fallback_height" 0.0..=4096.0 => body_fallback_height;
    };

    fn defaults(s: &Semantic<'_>) -> PreferencesTokens {
        let k = s.size.control.md / values::DIALOG_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        PreferencesTokens {
            list_width: values::PREFERENCES_LIST_WIDTH,
            page_gap: s.space.pad.panel,
            choice_width: scale(values::PREFERENCES_CHOICE_WIDTH),
            text_width: scale(values::PREFERENCES_TEXT_WIDTH),
            page_inset: values::PREFERENCES_PAGE_INSET,
            page_row_height: values::PREFERENCES_PAGE_ROW_HEIGHT,
            hex_width: scale(values::PREFERENCES_HEX_WIDTH),
            body_fallback_height: values::PREFERENCES_BODY_FALLBACK_HEIGHT,
        }
    }

    fn path(f: PreferencesField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<PreferencesField> {
        None
    }

    fn kind(f: PreferencesField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: PreferencesField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: PreferencesField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
