#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`MetadataTable`]: name and value rows in groups, with a filter field and an optional check
//! per field for the app's column picker.

use egui::{AsIdSalt, IdSalt, Response, Ui};

crate::words! {
    /// The table's words, English by default.
    #[non_exhaustive]
    pub struct MetadataLabels {
        /// The filter field's placeholder and accessible name (LightCraft's All Metadata).
        filter: "metadata.filter" = "Filter fields";
        /// The body with no rows at all.
        none: "metadata.none" = "No metadata found";
        /// The body when the filter hides every row.
        no_match: "metadata.noMatch" = "No fields match";
        /// The accessible name of a field's check in the column picker: `{name}` the field.
        show_field: "metadata.showField" = "Show {name}";
    }
}

/// One field: its name and its value, as the app formats them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataRow {
    /// "Exposure Time", "dc:creator".
    pub name: String,
    /// "1/250 s"; may run to several lines, which wrap.
    pub value: String,
}

/// A titled group of fields ("Camera", "EXIF", "XMP"); a group with an empty name has no title
/// line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataGroup {
    pub name: String,
    pub rows: Vec<MetadataRow>,
}

/// What the table reported this pass.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MetadataResponse {
    pub response: Response,
    /// A check of the column picker changed: the chosen names have changed.
    pub chosen_changed: bool,
}

/// A key and value table, after LightCraft's All Metadata dialog, with FilmCraft's Metadata
/// Display column picker.
///
/// ```ignore
/// let out = MetadataTable::new("all-metadata", &groups).filter(&mut dialog.search).show(ui);
/// ```
///
/// - **Rows.** Each group's name on a line of its own in the `BodyStrong` role, its fields under
///   it in two columns: the name in the `Label` role and `text.muted`, the value in the `Body`
///   role, wrapping; striped rows (the proposed `list.stripe`), 16 points between the columns and 3 between
///   rows (LightCraft's grid spacing). The name column is as wide as the widest name, at most a
///   third of the table. Values are selectable text, so the user copies part of one with the
///   platform's keys.
/// - **Filter** ([`filter`](Self::filter)): a [`SearchField`](crate::controls::SearchField) over
///   the rows keeps the fields whose name or value holds the typed text, ignoring case
///   ([`filter_metadata`]); a group with none left is hidden. The field keeps the focus while
///   the user types; ↓ moves into the rows.
/// - **Column picker** ([`choose`](Self::choose)): a [`Checkbox`](crate::controls::Checkbox)
///   before each field, checked while its name is in `chosen`; checking adds the name at the end,
///   unchecking removes it. The app shows the chosen fields as columns (FilmCraft's Project
///   panel Metadata Display).
/// - **Empty.** [`MetadataLabels::none`] with no rows, [`MetadataLabels::no_match`] when the
///   filter hides them all, in `text.muted`.
/// - **Keys and AccessKit.** The rows are one Tab stop, a `Table` with a `Row` per field and
///   `Cell`s for name and value; ↑ and ↓ move the focused row and scroll it into view. With
///   the column picker, Space toggles the focused row's check.
pub struct MetadataTable<'a> {
    id_salt: IdSalt,
    groups: &'a [MetadataGroup],
    filter: Option<&'a mut String>,
    chosen: Option<&'a mut Vec<String>>,
    max_height: Option<f32>,
}

impl<'a> MetadataTable<'a> {
    pub fn new(id_salt: impl AsIdSalt, groups: &'a [MetadataGroup]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), groups, filter: None, chosen: None, max_height: None }
    }

    /// A filter field over the rows, editing `text` (the host keeps it, so it survives the
    /// dialog being redrawn).
    pub fn filter(mut self, text: &'a mut String) -> Self {
        self.filter = Some(text);
        self
    }

    /// A check before each field, for the names in `chosen`.
    pub fn choose(mut self, chosen: &'a mut Vec<String>) -> Self {
        self.chosen = Some(chosen);
        self
    }

    /// The most the rows take before they scroll (LightCraft's dialog: 440).
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    // lightcraft: crates/ui-egui/src/dialogs.rs:662 all_metadata_body
    // filmcraft: crates/ui-egui/src/panels/project_dialogs.rs:124 metadata_display
    pub fn show(self, ui: &mut Ui) -> MetadataResponse {
        todo!()
    }
}

/// The groups with only the fields whose name or value holds `query` (trimmed, any case); groups
/// left empty are dropped. An empty query keeps everything.
// lightcraft: crates/ui-egui/src/dialogs.rs:670 (keep)
pub fn filter_metadata(groups: &[MetadataGroup], query: &str) -> Vec<MetadataGroup> {
    todo!()
}
