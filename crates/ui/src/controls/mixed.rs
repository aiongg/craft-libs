#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Mixed values: what a field shows while a multi-selection's values differ, and the apply
//! toggle of a bulk-edit dialog.
//!
//! The fields themselves already show mixed values: [`NumberField::mixed`](super::NumberField::mixed)
//! is blank with a `—` placeholder, [`Swatch::mixed`](super::Swatch::mixed) draws the mixed
//! well, [`CheckState::Mixed`](super::CheckState::Mixed) the dash, and a
//! [`Select`](super::Select) whose value is none of its options shows its placeholder (give it
//! [`MixedLabels::mixed`]). This module gives the host the value to feed them and PdfCraft's
//! apply rows.

use egui::{AsIdSalt, IdSalt, Response, Ui};

crate::words! {
    /// The words of mixed values, English by default.
    #[non_exhaustive]
    pub struct MixedLabels {
        /// A select's or a text's placeholder while the values differ (PdfCraft's).
        mixed: "mixed.mixed" = "Mixed";
    }
}

/// The value a multi-selection shares, or the fact that it doesn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MixedValue<T> {
    /// Every item has this value.
    Same(T),
    /// The items differ.
    Mixed,
}

impl<T: PartialEq> MixedValue<T> {
    /// The shared value of `values`, `Mixed` when two differ, `None` when there are none
    /// (VectorCraft's `same` over the selected type objects, PdfCraft's `Change::from_values`).
    // vectorcraft: crates/ui-egui/src/panels/character.rs:60 (value, mixed, saw)
    pub fn of(values: impl IntoIterator<Item = T>) -> Option<Self> {
        todo!()
    }

    pub fn is_mixed(&self) -> bool {
        matches!(self, MixedValue::Mixed)
    }

    /// The shared value, if any.
    pub fn same(self) -> Option<T> {
        match self {
            MixedValue::Same(value) => Some(value),
            MixedValue::Mixed => None,
        }
    }
}

/// One property of a bulk-edit dialog (PdfCraft's Edit Fields of several form fields): a value,
/// whether the selection's values differed, and whether the user chose to apply it.
#[derive(Clone, Debug, PartialEq)]
pub struct Apply<T> {
    pub value: T,
    pub mixed: bool,
    /// Off until the user turns it on: opening the dialog or visiting a tab never turns mixed
    /// values into the first item's value.
    pub apply: bool,
}

impl<T: Clone + PartialEq> Apply<T> {
    /// The first item's value, `mixed` when another differs, not applied; `None` for no items.
    // pdfcraft: crates/ui-egui/src/tools/bulk_fields.rs:18 from_values
    pub fn of(values: impl IntoIterator<Item = T>) -> Option<Self> {
        todo!()
    }

    /// The value to write when applied and different from `current`.
    // pdfcraft: crates/ui-egui/src/tools/bulk_fields.rs:25 changed
    pub fn change(&self, current: &T) -> Option<T> {
        todo!()
    }
}

/// A bulk-edit row: a [`Checkbox`](super::Checkbox) labelled with the property that turns
/// [`Apply::apply`] on and off, the property's editor (enabled only while applied, in its own id
/// scope so two editors of one kind don't share state), and [`MixedLabels::mixed`] in
/// `text.muted` after it while the values differ. Lay rows out in a grid of three columns
/// (PdfCraft's). Toggling the check is a `Set` of the whole [`Apply`]; the editor's edits go to
/// `value` and report through it.
pub struct ApplyRow<'a, T> {
    id_salt: IdSalt,
    label: String,
    apply: &'a mut Apply<T>,
}

impl<'a, T: Clone + PartialEq> ApplyRow<'a, T> {
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, apply: &'a mut Apply<T>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), apply }
    }

    /// Draws the row; `edit` draws the property's editor on the value. The response is the
    /// check's.
    // pdfcraft: crates/ui-egui/src/tools/bulk_fields.rs:205 row
    pub fn show(self, ui: &mut Ui, edit: impl FnOnce(&mut Ui, &mut T)) -> Response {
        todo!()
    }
}
