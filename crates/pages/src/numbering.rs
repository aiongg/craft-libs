//! Page numbering (N3.29): a page's number in its section's style, start and prefix (InDesign's
//! Numbering & Section Options, Word's Page Number Format). The styles are craft-text's list
//! number styles, so a page number and a list number written "iv" look the same.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

pub use craft_text::lists::NumberStyle;

/// The styles a page numbering Style menu offers, in DesignCraft's order; each item reads its
/// [`NumberStyle::sample`] ("1, 2, 3, 4…").
// designcraft: crates/ui-egui/src/section_options.rs:14 `STYLES`.
pub const PAGE_NUMBER_STYLES: [NumberStyle; 8] = [
    NumberStyle::Arabic,
    NumberStyle::UpperRoman,
    NumberStyle::LowerRoman,
    NumberStyle::UpperLetters,
    NumberStyle::LowerLetters,
    NumberStyle::ArabicLeadingZero,
    NumberStyle::ArabicThreeDigits,
    NumberStyle::ArabicFourDigits,
];

/// A section's page numbering: the number its first page gets, the style, and a prefix shown with
/// the number where `include_prefix` says (InDesign's Section Prefix, "Include Prefix when
/// Numbering Pages").
#[derive(Clone, Debug, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PageNumbering {
    pub style: NumberStyle,
    /// The first page's number, 1 or more.
    pub start: u32,
    pub prefix: String,
    pub include_prefix: bool,
}

impl Default for PageNumbering {
    fn default() -> Self {
        PageNumbering { style: NumberStyle::Arabic, start: 1, prefix: String::new(), include_prefix: false }
    }
}

impl PageNumbering {
    /// The label of the section's page `index` (0-based within the section): the prefix when
    /// included, then `start + index` in the style ("A-iv"). Saturates at `u32::MAX`.
    // designcraft: crates/ui-egui/src/section_options.rs:32 `open` (start number and prefix fields);
    // crates/doc/src/attrs/rules_lists.rs:144 `NumberStyle::format`.
    pub fn label(&self, index: usize) -> String {
        todo!()
    }
}
