//! `CommandForm`: a plain dialog for a command with parameters, and `HostedCommandForm`, the same form owned by a
//! `DialogHost`. A command form is a `ParamDialog` without a preview and without a remember key.

use super::dialog::{HostedParamDialog, ParamDialog, ParamResult};
use super::fields::DialogFields;
use super::param::{ColorRequest, EyedropperRequest, Param, PathRequest};
use crate::contract::Bind;
use crate::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
use crate::runtime::env::Label;
use egui::Ui;
use std::borrow::Cow;

#[cfg(feature = "testing")]
mod stories;

/// What a command form reports: the parameter dialog's output.
pub type CommandFormOutput = super::dialog::ParamDialogOutput;

/// A form for a command with parameters: one row per [`Param`], OK and Cancel, and Reset while Alt is held. Batch,
/// Image Processor, Open EXR Parts and Fit Image are command forms.
///
/// The form is shown as the [`ParamDialog`] it builds with [`CommandForm::dialog`]: keys, values and answer are the
/// parameter dialog's. Nothing is remembered across runs; a form opens with the values the app gives, else the
/// parameters' defaults.
#[must_use = "show the form with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct CommandForm<'a> {
    key: String,
    title: Label,
    params: Cow<'a, [Param]>,
    /// Binds the values to `values`.
    #[prop(name = "values")]
    bound: Option<Bind<'a, DialogFields>>,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    /// Whether OK can be pressed; Enter does nothing while it cannot.
    #[prop]
    ok_enabled: bool,
    /// Why OK is disabled: the tooltip of the disabled button.
    #[prop]
    ok_reason: Option<Label>,
    /// The accepting button's label when it is not OK ("Fit", "Run").
    #[prop]
    ok_label: Option<Label>,
    /// The second button's label when it is not Cancel (Close, say).
    #[prop]
    cancel_label: Option<Label>,
    /// Whether the title row has a close action.
    #[prop]
    closable: bool,
    /// Whether the form holds one outer gesture over its fields, so Ctrl/Cmd+Z steps the form's own settings.
    #[prop]
    outer_gesture: bool,
    /// Controls before the buttons in the footer.
    #[prop(boxed, name = "footer_extra")]
    extra: Option<Box<dyn FnMut(&mut Ui) + 'a>>,
    /// A section under the rows that only reads the values (File Info's camera data).
    #[prop(boxed, name = "below")]
    rows_below: Option<Box<dyn Fn(&mut Ui, &DialogFields) + 'a>>,
}

impl<'a> CommandForm<'a> {
    /// A form for `params`, titled `title`, answering for the command `key`.
    pub fn new(key: impl Into<String>, title: impl Into<Label>, params: impl Into<Cow<'a, [Param]>>) -> Self {
        CommandForm {
            key: key.into(),
            title: title.into(),
            params: params.into(),
            bound: None,
            open: None,
            ok_enabled: true,
            ok_reason: None,
            ok_label: None,
            cancel_label: None,
            closable: true,
            outer_gesture: false,
            extra: None,
            rows_below: None,
        }
    }

    /// The parameter dialog the form is: the one dialog [`show`](CommandForm::show) shows and a host's
    /// [`HostedParamDialog`] builds.
    pub fn dialog(self) -> ParamDialog<'a> {
        let CommandForm {
            key,
            title,
            params,
            bound,
            open,
            ok_enabled,
            ok_reason,
            ok_label,
            cancel_label,
            closable,
            outer_gesture,
            extra,
            rows_below,
        } = self;
        let mut dialog = ParamDialog::new(key, title, params).ok_enabled(ok_enabled).closable(closable).outer_gesture(outer_gesture);
        if let Some(bound) = bound {
            dialog = dialog.values(bound);
        }
        if let Some(open) = open {
            dialog = dialog.opened(open);
        }
        if let Some(reason) = ok_reason {
            dialog = dialog.ok_reason(reason);
        }
        if let Some(label) = ok_label {
            dialog = dialog.ok_label(label);
        }
        if let Some(label) = cancel_label {
            dialog = dialog.cancel_label(label);
        }
        if let Some(extra) = extra {
            dialog = dialog.footer_extra(extra);
        }
        if let Some(draw) = rows_below {
            dialog = dialog.below(draw);
        }
        dialog
    }

    /// Shows the form.
    pub fn show(self, ui: &mut Ui) -> CommandFormOutput {
        self.dialog().show(ui)
    }
}

/// A [`CommandForm`] a `DialogHost` owns: the host keeps the values, and `build` makes the form over them each pass.
/// It is a [`HostedParamDialog`] over the form's dialog, so the host's colour, path and eyedropper requests have one
/// path.
///
/// ```ignore
/// host.open(HostedCommandForm::new("fit-image", DialogFields::new(), |fields| {
///     CommandForm::new("fit-image", "Fit Image", params.clone()).values(Bind::bound(fields)).ok_label("Fit".into())
/// }));
/// ```
pub struct HostedCommandForm {
    inner: HostedParamDialog,
}

impl HostedCommandForm {
    /// A hosted form for the command `key`, opening with `fields`; `build` makes the form over the values.
    pub fn new(
        key: impl Into<String>,
        fields: DialogFields,
        build: impl for<'b> Fn(&'b mut DialogFields) -> CommandForm<'b> + Send + 'static,
    ) -> Self {
        HostedCommandForm { inner: HostedParamDialog::new(key, fields, move |fields| build(fields).dialog()) }
    }

    /// Names this form among others made with the same key, so they keep separate state.
    #[must_use]
    pub fn instance(mut self, instance: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.inner = self.inner.instance(instance);
        self
    }

    /// The values the form shows.
    pub fn fields(&self) -> &DialogFields {
        self.inner.fields()
    }

    /// The values the form shows, for the app to set (a script, a colour picker's answer).
    pub fn fields_mut(&mut self) -> &mut DialogFields {
        self.inner.fields_mut()
    }

    /// The colour parameter whose button was pressed, once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.inner.pick_color()
    }

    /// The Browse button that was pressed, once.
    pub fn pick_path(&mut self) -> Option<PathRequest> {
        self.inner.pick_path()
    }

    /// The eyedropper toggle that was pressed, once.
    pub fn pick_eyedropper(&mut self) -> Option<EyedropperRequest> {
        self.inner.pick_eyedropper()
    }
}

impl StackDialog for HostedCommandForm {
    type Result = ParamResult;

    fn id(&self) -> DialogId {
        self.inner.id()
    }

    fn show(&mut self, ui: &mut Ui, stack: &DialogStack) -> Option<DialogResult<ParamResult>> {
        self.inner.show(ui, stack)
    }

    fn key(&self) -> Option<&str> {
        self.inner.key()
    }

    fn instance(&self) -> Option<DialogId> {
        self.inner.instance()
    }
}
