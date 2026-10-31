//! [`CommandForm`]: a plain dialog for a command with parameters: labelled fields, OK and Cancel,
//! Reset on Alt+Cancel. The same [`Param`] list as [`ParamDialog`](super::ParamDialog), without
//! preview, sliders or remembered values.

use craft_theme::Srgba;
use egui::{Context, Ui};

use super::color_request::ColorRequest;
use super::dialog::{Dialog, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::fields::DialogFields;
use super::footer::{DialogLabels, Footer, Reset};
use super::params::{
    FooterExtra, Param, ParamResult, PathRequest, draw_rows, first_field_focused, open_fields, set_color_field, take_color_press, take_path_press,
};
use super::size::{Body, Width};
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::composites::FormGrid;
use crate::localized::Labels as _;

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
/// - Result: a [`ParamResult`], the form's [`key`](CommandForm::key) with the [`DialogFields`] at OK. Nothing is remembered across runs; a form always opens
///   with the values the host gave.
pub struct CommandForm {
    id: DialogId,
    key: String,
    title: String,
    params: Vec<Param>,
    fields: DialogFields,
    ok_label: Option<String>,
    ok_enabled: bool,
    width: Width,
    body: Body,
    picked_color: Option<ColorRequest>,
    picked_path: Option<PathRequest>,
    below: Option<Below>,
    history: bool,
    close_label: Option<Option<String>>,
    footer_extra: Option<FooterExtra>,
    state: DialogState<DialogFields>,
}

/// A section drawn under the rows ([`CommandForm::below`]).
type Below = Box<dyn Fn(&mut Ui, &DialogFields) + Send>;

impl CommandForm {
    /// A form for `params`, opening with their defaults.
    pub fn new(key: impl Into<String>, title: impl Into<String>, params: Vec<Param>) -> Self {
        let key = key.into();
        Self {
            id: dialog_id(key.as_str()),
            key,
            title: title.into(),
            params,
            fields: DialogFields::new(),
            ok_label: None,
            ok_enabled: true,
            width: Width::Standard,
            body: Body::Scroll,
            picked_color: None,
            picked_path: None,
            below: None,
            history: false,
            close_label: None,
            footer_extra: None,
            state: DialogState::new(),
        }
    }

    /// The key the form was made with: which command it answers for. The result carries it.
    pub fn key(&self) -> &str {
        &self.key
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

    /// [`ok_enabled`](CommandForm::ok_enabled) for a form already open.
    pub fn set_ok_enabled(&mut self, enabled: bool) {
        self.ok_enabled = enabled;
    }

    /// A section under the rows that only reads the values (File Info's Camera Data).
    pub fn below(mut self, draw: impl Fn(&mut Ui, &DialogFields) + Send + 'static) -> Self {
        self.below = Some(Box::new(draw));
        self
    }

    /// Cmd/Ctrl+Z steps the form's own settings ([`Dialog::history`]).
    pub fn history(mut self, history: bool) -> Self {
        self.history = history;
        self
    }

    /// The close box's accessible name; `None` removes the box ([`Dialog::close_label`]).
    pub fn close_label(mut self, label: Option<String>) -> Self {
        self.close_label = Some(label);
        self
    }

    /// Controls before the buttons in the footer row.
    pub fn footer_extra(mut self, add: impl FnMut(&mut Ui) + Send + 'static) -> Self {
        self.footer_extra = Some(Box::new(add));
        self
    }

    /// How the body fits the window: [`Body::Scroll`] (the default) or [`Body::Fit`], where the
    /// dialog takes the height its rows need and shrinks to their width, scrolling only when the
    /// window is too short ([`Dialog::body`]).
    pub fn body(mut self, body: Body) -> Self {
        self.body = body;
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

    /// The colour parameter whose swatch the user pressed, once: the host opens its colour picker
    /// for the request (key = the parameter's key, title = its label, colour = the value shown) and
    /// answers with [`set_color`](Self::set_color).
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.picked_color.take()
    }

    /// The colour picker's answer for the parameter `key`; other keys are ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        set_color_field(&self.params, &mut self.fields, key, color);
    }

    /// The Browse button of a [`Param::browse`] row the user pressed, once: the host opens its
    /// file dialog and writes the path with `fields_mut().set(key, FieldValue::Text(..))`.
    pub fn pick_path(&mut self) -> Option<PathRequest> {
        self.picked_path.take()
    }

    fn answer(&self) -> ParamResult {
        ParamResult { key: self.key.clone(), fields: self.fields.clone() }
    }
}

impl StackDialog for CommandForm {
    type Result = ParamResult;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/menus/view_cmds/dialogs.rs:60 `form` and the `__form` body in
    // crates/ui-egui/src/dialogs.rs:177 `command_body_more`; the rows through `Param::row`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<ParamResult>> {
        if !self.state.is_open() {
            open_fields(ctx, self.id, &self.params, &mut self.fields);
        }
        let labels = DialogLabels::of_ctx(ctx);
        let mut footer = Footer::standard().reset(Reset::AltCancel(labels.reset)).ok_enabled(self.ok_enabled);
        if let Some(ok) = &self.ok_label {
            footer = footer.ok(ok.as_str());
        }
        if let Some(extra) = &mut self.footer_extra {
            footer = footer.trailing_extra(|ui| extra(ui));
        }
        let (params, id, below) = (&self.params, self.id, &self.below);
        let mut dialog =
            Dialog::with_id(self.id, self.title.as_str()).stack(stack).width(self.width).body(self.body).footer(footer).history(self.history);
        if let Some(label) = &self.close_label {
            dialog = dialog.close_label(label.clone());
        }
        let shown = dialog.show(ctx, &mut self.state, &mut self.fields, |ui, fields| {
            first_field_focused(ui, id, |ui| {
                FormGrid::new("form").show(ui, |form| draw_rows(params, id, form, fields, None));
            });
            if let Some(below) = below {
                below(ui, fields);
            }
        });
        self.picked_color = take_color_press(ctx, self.id).or(self.picked_color.take());
        self.picked_path = take_path_press(ctx, self.id).or(self.picked_path.take());
        match shown.outcome? {
            Outcome::Ok => Some(DialogResult::Ok(self.answer())),
            Outcome::Cancel | Outcome::Close => Some(DialogResult::Cancel),
            Outcome::Reset => {
                self.fields.reset();
                None
            }
            _ => None,
        }
    }

    fn key(&self) -> Option<&str> {
        Some(&self.key)
    }

    fn confirm(&mut self, ctx: &Context) -> Option<ParamResult> {
        if !self.ok_enabled {
            return None;
        }
        if !self.state.is_open() {
            open_fields(ctx, self.id, &self.params, &mut self.fields);
        }
        Some(self.answer())
    }
}
