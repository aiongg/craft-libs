//! [`CommandForm`]: a plain dialog for a command with parameters: labelled fields, OK and Cancel,
//! Reset on Alt+Cancel. The same [`Param`] list as [`ParamDialog`](super::ParamDialog), without
//! preview, sliders or remembered values.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Context;

use super::dialog_host::{DialogResult, StackDialog};
use super::fields::DialogFields;
use super::params::Param;
use super::size::Width;
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;

/// A form dialog: one [`FormGrid`](crate::composites::FormGrid) row per [`Param`] (text fields,
/// number fields, check boxes, selects), a bottom footer with OK and Cancel, Reset on Alt+Cancel
/// restoring the values from when it opened. Batch, Image Processor, Open EXR Parts, Fit Image.
///
/// ```ignore
/// dialogs.open(CommandForm::new("fit-image", "Fit Image", params).ok_label("Fit"));
/// if let Some(DialogResult::Ok(fields)) = dialogs.take::<CommandForm>() { .. }
/// ```
///
/// - The first field opens selected; Tab walks the fields; Enter is OK unless a multi-line field
///   owns it; Esc cancels. OK is disabled while [`ok_enabled`](CommandForm::ok_enabled) says so
///   (a required list is empty).
/// - Result: the [`DialogFields`] at OK. Nothing is remembered across runs; a form always opens
///   with the values the host gave.
pub struct CommandForm {
    id: DialogId,
    title: String,
    params: Vec<Param>,
    fields: DialogFields,
    ok_label: Option<String>,
    ok_enabled: bool,
    width: Width,
    state: DialogState<DialogFields>,
}

impl CommandForm {
    /// A form for `params`, opening with their defaults.
    pub fn new(key: impl std::hash::Hash + std::fmt::Debug, title: impl Into<String>, params: Vec<Param>) -> Self {
        Self {
            id: dialog_id(key),
            title: title.into(),
            params,
            fields: DialogFields::new(),
            ok_label: None,
            ok_enabled: true,
            width: Width::Standard,
            state: DialogState::new(),
        }
    }

    /// Opening values that override the defaults.
    pub fn values(mut self, values: DialogFields) -> Self {
        self.fields = values;
        self
    }

    /// The default button's label when it is not "OK" ("Fit", "Run").
    pub fn ok_label(mut self, label: impl Into<String>) -> Self {
        self.ok_label = Some(label.into());
        self
    }

    /// Whether OK can be pressed; the host updates it through
    /// [`DialogHost::get_mut`](super::DialogHost::get_mut) when it depends on the values.
    pub fn ok_enabled(mut self, enabled: bool) -> Self {
        self.ok_enabled = enabled;
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

impl StackDialog for CommandForm {
    type Result = DialogFields;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/menus/view_cmds/dialogs.rs:60 `form` and the `__form` body in
    // crates/ui-egui/src/dialogs.rs:177 `command_body_more`; the rows through `Param::row`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<DialogFields>> {
        todo!()
    }
}
