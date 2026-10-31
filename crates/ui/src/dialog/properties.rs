//! [`PropertiesDialog`]: tabs of label and value rows (Document Properties, file info), editable
//! or read-only.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Context;

use super::dialog_host::{DialogResult, StackDialog};
use super::fields::DialogFields;
use super::params::Param;
use super::size::Width;
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;

/// One row of a properties tab.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PropertyRow {
    /// A label and a read-only value ("Page size:", "210 × 297 mm"); the value is selectable
    /// text.
    Value { label: String, text: String },
    /// An editable field, drawn from the [`Param`] like a [`CommandForm`](super::CommandForm)
    /// row; read-only mode shows its value as text.
    Field(Param),
    /// A heading over the rows that follow.
    Heading(String),
    /// Empty space.
    Space,
}

/// One tab.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertiesTab {
    pub title: String,
    pub rows: Vec<PropertyRow>,
}

impl PropertiesTab {
    pub fn new(title: impl Into<String>, rows: Vec<PropertyRow>) -> Self {
        Self { title: title.into(), rows }
    }
}

/// A dialog of tabs, each a [`FormGrid`](crate::composites::FormGrid) of label/value rows. The
/// user sees a [`TabStrip`](crate::tab_strip) under the title (one Tab stop; ←/→ switch), the
/// current tab's rows, and OK / Cancel; in [`read_only`](PropertiesDialog::read_only) mode a
/// single OK. Tab order inside a tab walks the editable fields only. The dialog remembers the
/// tab the user left open.
///
/// Result: the [`DialogFields`] of every editable row (all tabs) at OK; `()`-like for read-only
/// (the fields are empty).
pub struct PropertiesDialog {
    id: DialogId,
    title: String,
    tabs: Vec<PropertiesTab>,
    fields: DialogFields,
    read_only: bool,
    width: Width,
    current_tab: usize,
    state: DialogState<DialogFields>,
}

impl PropertiesDialog {
    pub fn new(key: impl std::hash::Hash + std::fmt::Debug, title: impl Into<String>, tabs: Vec<PropertiesTab>) -> Self {
        Self {
            id: dialog_id(key),
            title: title.into(),
            tabs,
            fields: DialogFields::new(),
            read_only: false,
            width: Width::Wide,
            current_tab: 0,
            state: DialogState::new(),
        }
    }

    /// Opening values of the editable rows.
    pub fn values(mut self, values: DialogFields) -> Self {
        self.fields = values;
        self
    }

    /// Every row shows as text; the footer is one OK.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn width(mut self, width: Width) -> Self {
        self.width = width;
        self
    }

    pub fn fields(&self) -> &DialogFields {
        &self.fields
    }

    pub fn fields_mut(&mut self) -> &mut DialogFields {
        &mut self.fields
    }
}

impl StackDialog for PropertiesDialog {
    type Result = DialogFields;

    fn id(&self) -> DialogId {
        self.id
    }

    // new: the step 2 source is PdfCraft's Document Properties
    // (pdfcraft crates/ui-egui/src/dialogs.rs, `dialogs` from line 18: tabs, label column, the
    // read-only file info tab); built on `Dialog`, `TabStrip` and `FormGrid::value` / `Param::row`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<DialogFields>> {
        todo!()
    }
}
