//! [`Field`] and [`FieldGroup`]: a form row of a label, a control, and help or error text.

use std::sync::Arc;

use craft_theme::TypeRole;
use egui::accesskit::{Invalid, Role};
use egui::text::{LayoutJob, TextFormat};
use egui::{AsIdSalt, Color32, Galley, IdSalt, InnerResponse, Label, Rect, Response, Sense, Ui, UiBuilder, pos2, vec2};

use super::measure;
use crate::behavior::FieldTab;
use crate::controls::{Checkbox, NumberField, Select, TextField};
use crate::direction;
use crate::edit::{EditEvent, EditResponse};
use crate::localized::LocaleNumbers;
use crate::primitives::Content;
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};
use crate::typed_story;

/// A field's gaps, type roles and colours, from [`Field::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FieldStyle {
    /// Between the label column and the control: `field.label_gap`.
    pub label_gap: f32,
    /// Between the control and the help or error text, and between a label above and the
    /// control: `field.help_gap`.
    pub help_gap: f32,
    /// A label beside the control is centred on the control's first `field.height.md`, so it
    /// lines up with the first line of a taller control.
    pub label_band: f32,
    /// The label's type role: `label`.
    pub label_role: TypeRole,
    /// The help and error text's type role: `caption`.
    pub message_role: TypeRole,
    /// The label: `text.muted`.
    pub label: Color32,
    /// The help text: `text.muted`.
    pub help: Color32,
    /// The error text: `field.invalid`, the colour of the invalid field's edge (`status.bad.fg`).
    pub error: Color32,
}

/// What [`Field::show`] needs from the control it wraps: the response of the control's own
/// AccessKit node. Every control returns one: a [`Response`] from `ui.add`, or an
/// [`EditResponse`] from `show`.
pub trait FieldControl {
    /// The response whose AccessKit node is the control.
    fn control(&self) -> &Response;
}

/// Names the control of a row by its visible `label` (`labelled_by`). A cell of several controls
/// has no node of a widget but its layout's container: that becomes a group, so the label names
/// the cluster.
pub(super) fn name_control(ctx: &egui::Context, control: &Response, label: &Response) {
    if !ctx.has_accesskit_node(control.id) {
        return;
    }
    ctx.accesskit_node_builder(control.id, |node| {
        if node.role() == Role::GenericContainer {
            node.set_role(Role::Group);
        }
        // The node's own label would win over `labelled_by`.
        node.clear_label();
        node.push_labelled_by(label.id.accesskit_id());
    });
}

impl FieldControl for Response {
    fn control(&self) -> &Response {
        self
    }
}

impl<T> FieldControl for EditResponse<T> {
    fn control(&self) -> &Response {
        &self.response
    }
}

/// A control in a form, with its label and its help or error text:
///
/// ```
/// use craft_ui::composites::Field;
/// use craft_ui::controls::TextField;
/// # egui::__run_test_ui(|ui| { let name = String::new();
/// let error = name.is_empty().then_some("Enter a name.");
/// let edit = Field::new("Name:")
///     .help("Shown in the Layers panel.")
///     .error(error)
///     .show(ui, |ui, invalid| TextField::new("name", "Name", &name).invalid(invalid).show(ui))
///     .inner;
/// # });
/// ```
///
/// - **The label** sits beside the control at the reading start, against the control in a label
///   column (the right edge left to right), centred on the control's first line; or over it with
///   [`above`](Field::above). In a [`FieldGroup`] the column is as wide as the group's widest
///   label, so the controls line up; alone, it is the label's own width. The text is the app's
///   string as given: a language that writes a colon after a label has it in its translation.
/// - **A row without a label** ([`unlabelled`](Field::unlabelled): a check box that names
///   itself) starts at the group's control column.
/// - **Help text** goes under the control in `caption`, `text.muted`. **Error text** takes its
///   place while there is one, in `caption` and `field.invalid` (`status.bad.fg`); the help text
///   comes back when the error clears. The control draws its invalid state: `show` passes it
///   `invalid`, for the control's own `invalid(..)`.
/// - **AccessKit:** the control is `labelled_by` the label (whose text becomes its name, in
///   place of the control's own `label` argument), `described_by` the help or error text shown,
///   and while there is an error, `invalid` with that text as its `error_message`. The control's
///   own label stays its name in a row without a label.
///
/// The control is any widget whose response carries its AccessKit node ([`FieldControl`]):
/// `show` adds no per-control code. Tab order is the dialog's ([`FieldTab`]).
#[derive(Clone, Debug)]
#[must_use = "show the field with `show`"]
pub struct Field {
    label: Option<String>,
    help: Option<String>,
    error: Option<String>,
    above: bool,
}

impl Field {
    /// A field labelled `label`, beside the control.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: Some(label.into()), help: None, error: None, above: false }
    }

    /// A row without a label, for a control that names itself (a check box, a switch): it starts
    /// at the control column of its [`FieldGroup`].
    pub fn unlabelled() -> Self {
        Self { label: None, help: None, error: None, above: false }
    }

    /// The label over the control rather than beside it, for narrow dialogs. The row spans the
    /// whole width and doesn't count towards its group's label column.
    pub fn above(mut self) -> Self {
        self.above = true;
        self
    }

    /// Help text under the control, while there is no error.
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into()).filter(|h| !h.is_empty());
        self
    }

    /// The error text, or `None` while the value is valid. While there is one it replaces the
    /// help text and the control shows its invalid state.
    pub fn error<S: Into<String>>(mut self, error: Option<S>) -> Self {
        self.error = error.map(Into::into).filter(|e| !e.is_empty());
        self
    }

    /// The style of a field in `tokens`.
    pub fn style(tokens: &UiTokens) -> FieldStyle {
        let f = &tokens.component.field;
        let muted = Content::Muted.color(tokens).to_egui();
        FieldStyle {
            label_gap: f.label_gap,
            help_gap: f.help_gap,
            label_band: f.height.md,
            label_role: TypeRole::Label,
            message_role: TypeRole::Caption,
            label: muted,
            help: muted,
            error: f.invalid.to_egui(),
        }
    }

    /// Shows the row. `add_control` draws the control in the control column and gets whether
    /// the field is invalid, for the control's own `invalid(..)`. The response covers the row.
    pub fn show<C: FieldControl>(self, ui: &mut Ui, add_control: impl FnOnce(&mut Ui, bool) -> C) -> InnerResponse<C> {
        let tokens = Tokens::of(ui);
        let style = Field::style(&tokens);
        let invalid = self.error.is_some();
        let label_text = self.label.filter(|l| !l.is_empty());
        let label_format = role_format(&tokens, style.label_role, style.label);
        // Beside the control the label is measured for the column and stays on one line.
        let label = if self.above { None } else { label_text.clone().map(|text| one_line(ui, text, label_format.clone())) };
        let message = match (self.error, self.help) {
            (Some(error), _) => Some((error, style.error)),
            (None, Some(help)) => Some((help, style.help)),
            (None, None) => None,
        };

        let rect = ui.available_rect_before_wrap();
        let rtl = direction::is_rtl(ui);
        let column = if self.above { 0.0 } else { column_width(ui, label.as_ref().map_or(0.0, |g| g.size().x)) };
        let indent = if column > 0.0 { column + style.label_gap } else { 0.0 };
        let control_rect = if rtl {
            Rect::from_min_max(rect.min, pos2(rect.right() - indent, rect.bottom()))
        } else {
            Rect::from_min_max(pos2(rect.left() + indent, rect.top()), rect.max)
        };

        // The label beside the control is placed once the control's height is known, in a `Ui`
        // made before the control's so that AccessKit lists the label first.
        let mut beside = label.map(|g| (ui.new_child(UiBuilder::new().max_rect(rect)), g));
        let mut stack = ui.new_child(UiBuilder::new().max_rect(if self.above { rect } else { control_rect }).layout(direction::column(ui)));
        let spacing = ui.spacing().item_spacing;
        stack.spacing_mut().item_spacing.y = style.help_gap;
        let mut label_response = match (self.above, label_text) {
            (true, Some(text)) => Some(stack.add(Label::new(LayoutJob::single_section(text, label_format)).selectable(false))),
            _ => None,
        };
        let control = stack
            .scope(|ui| {
                ui.spacing_mut().item_spacing = spacing;
                add_control(ui, invalid)
            })
            .inner;
        let message = message.map(|(text, color)| {
            stack.add(Label::new(LayoutJob::single_section(text, role_format(&tokens, style.message_role, color))).selectable(false))
        });

        let control_rect = control.control().rect;
        let mut row = stack.min_rect();
        if let Some((label_ui, g)) = beside.as_mut() {
            let x = if rtl { rect.right() - column } else { rect.left() };
            let band = Rect::from_min_size(pos2(x, control_rect.top()), vec2(column, control_rect.height().min(style.label_band)));
            let layout = direction::trailing(ui);
            let added =
                label_ui.scope_builder(UiBuilder::new().max_rect(band).layout(layout), |ui| ui.add(Label::new(Arc::clone(g)).selectable(false)));
            row = row.union(added.inner.rect);
            label_response = Some(added.inner);
        }
        if indent > 0.0 {
            row.extend_with_x(direction::start_x(ui, rect));
        }

        let id = control.control().id;
        let ctx = ui.ctx().clone();
        if ctx.has_accesskit_node(id) {
            ctx.accesskit_node_builder(id, |node| {
                if let Some(label) = &label_response {
                    // The node's own label would win over `labelled_by`.
                    node.clear_label();
                    node.push_labelled_by(label.id.accesskit_id());
                }
                if let Some(message) = &message {
                    node.push_described_by(message.id.accesskit_id());
                    if invalid {
                        node.set_error_message(message.id.accesskit_id());
                    }
                }
                if invalid {
                    node.set_invalid(Invalid::True);
                }
            });
        }
        drop((beside, stack));
        let response = ui.allocate_rect(row, Sense::hover());
        InnerResponse::new(control, response)
    }
}

pub(super) fn one_line(ui: &Ui, text: String, format: TextFormat) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text, format);
    job.wrap.max_width = f32::INFINITY;
    ui.fonts_mut(|f| f.layout_job(job))
}

/// A form's fields with one label column: every [`Field`] in it puts its label in a column as
/// wide as the widest label of the group, so the controls start at one x, as PhotoCraft's dialogs
/// set by hand per dialog:
///
/// ```
/// use craft_ui::composites::{Field, FieldGroup};
/// use craft_ui::controls::{Checkbox, TextField};
/// # egui::__run_test_ui(|ui| { let (name, mut clip) = (String::from("Layer 1"), false);
/// FieldGroup::new("new-layer").show(ui, |ui| {
///     Field::new("Name:").show(ui, |ui, _| TextField::new("name", "Name", &name).show(ui));
///     Field::unlabelled().show(ui, |ui, _| Checkbox::bool("Clip to the layer below", &mut clip).show(ui));
/// });
/// # });
/// ```
///
/// The group measures its labels as its fields draw them and uses the width on the next pass.
/// When the widest label changes (the first pass, a new language) it asks egui to discard the
/// pass and lay it out again, as egui's `Grid` does, so the column never shows at a stale width.
/// Fields reach the group through the `Ui` tree on the group's layer, however deep they sit.
/// Groups don't nest: a field belongs to the innermost one. Two groups under one parent need
/// different ids: sharing one, each would overwrite the other's column and every frame would be
/// laid out twice (a debug build fails an assertion).
#[derive(Clone, Copy, Debug)]
pub struct FieldGroup {
    id_salt: IdSalt,
}

impl FieldGroup {
    /// A group; `id_salt` keeps its measured column when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt) }
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        measure::group(ui, TAG, self.id_salt, add_contents)
    }
}

const TAG: &str = "FieldGroup";

/// The label column of a field whose label is `own` wide: its group's, or its own outside one.
fn column_width(ui: &Ui, own: f32) -> f32 {
    measure::column(ui, TAG, own).0
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Field"];

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/field/dialog",
        Level::Composites,
        "Field",
        "Dialog",
        DialogArgs { above: false, help: true, error: false },
        show_dialog
    )]
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct DialogArgs {
    pub(super) above: bool,
    pub(super) help: bool,
    pub(super) error: bool,
}

const MODES: [&str; 3] = ["Normal", "Multiply", "Screen"];

/// The values of the dialog story.
#[derive(Clone, Debug)]
struct Dialog {
    name: String,
    mode: usize,
    opacity: f64,
    clip: bool,
}

/// A New Layer dialog: name, mode and opacity in one label column, a check box under them in the
/// control column, in a `FieldTab` scope. `error` shows the name's error in place of its help.
pub(super) fn show_dialog(ui: &mut Ui, args: &mut DialogArgs) {
    let (above, help, error) = (args.above, args.help, args.error);
    let state_id = ui.scope_id().with("field-dialog");
    let mut v = ui.data(|d| d.get_temp::<Dialog>(state_id)).unwrap_or(Dialog { name: "Layer 1".into(), mode: 0, opacity: 100.0, clip: false });
    let numbers = LocaleNumbers::from_tag("en");
    let place = |field: Field| if above { field.above() } else { field };
    let _ = FieldTab::new("field-dialog").show(ui, |ui| {
        FieldGroup::new("field-dialog").show(ui, |ui| {
            let name = place(Field::new("Name:"));
            let name = if help { name.help("Shown in the Layers panel.") } else { name };
            let edit = name
                .error((error || v.name.trim().is_empty()).then_some("Enter a name for the layer."))
                .show(ui, |ui, invalid| TextField::new("name", "Name", &v.name).invalid(invalid).show(ui))
                .inner;
            if let Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) = edit.event {
                v.name = value;
            }
            let _ = place(Field::new("Mode:")).show(ui, |ui, _| {
                let mut select = Select::new("mode", "Mode", &mut v.mode);
                for (i, mode) in MODES.into_iter().enumerate() {
                    select = select.option(i, mode);
                }
                select.show(ui)
            });
            let opacity = place(Field::new("Opacity:"));
            let opacity = if help { opacity.help("100% is fully opaque.") } else { opacity };
            let edit = opacity
                .show(ui, |ui, invalid| NumberField::new("opacity", "Opacity", v.opacity, &numbers).range(0.0..=100.0).invalid(invalid).show(ui))
                .inner;
            if let Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) = edit.event {
                v.opacity = value;
            }
            let _ = Field::unlabelled().show(ui, |ui, _| Checkbox::bool("Use previous layer to create clipping mask", &mut v.clip).show(ui));
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, v));
}
