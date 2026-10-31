//! [`PropertiesDialog`]: tabs of label and value rows (Document Properties, file info), editable
//! or read-only.

use craft_host::Host;
use egui::{Context, Label, Ui};

use super::dialog::{Dialog, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::fields::{DialogFields, FieldValue};
use super::footer::Footer;
use super::params::{Param, ParamKind, open_fields};
use super::size::Width;
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::composites::{Form, FormGrid, SectionLabel};
use crate::controls::Checkbox;
use crate::localized::LocaleNumbers;
use crate::primitives::Content;
use crate::tab_strip::{Tab, TabEvent, TabKind, TabStrip};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};

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
        let Self { id, title, tabs, fields, read_only, width, current_tab, state } = self;
        let (id, read_only) = (*id, *read_only);
        let memory = id.with("open-tab");
        if !state.is_open() {
            let params: Vec<Param> = tabs
                .iter()
                .flat_map(|tab| &tab.rows)
                .filter_map(|row| if let PropertyRow::Field(param) = row { Some(param.clone()) } else { None })
                .collect();
            open_fields(ctx, id, &params, fields);
            *current_tab = ctx.data_mut(|d| d.get_persisted::<usize>(memory)).unwrap_or(*current_tab);
        }
        *current_tab = (*current_tab).min(tabs.len().saturating_sub(1));
        let numbers = LocaleNumbers::of_ctx(ctx);
        let gap = FormGrid::style(&crate::theme::current(ctx), false).row_gap * 2.0;
        let footer = if read_only { Footer::standard().without_cancel() } else { Footer::standard() };
        let tab_title = title.clone();
        let mut dialog = Dialog::with_id(id, title.as_str()).stack(stack).width(*width).footer(footer);
        if read_only {
            dialog = dialog.close_label(None);
        }
        let shown_tab = current_tab;
        let shown = dialog.show(ctx, state, fields, |ui, fields| {
            let strips: Vec<Tab<usize>> = tabs.iter().enumerate().map(|(i, tab)| Tab::new(i, tab.title.as_str())).collect();
            if strips.len() > 1 {
                let out = TabStrip::new(id.with("tabs"), tab_title.as_str(), TabKind::Text).selected(shown_tab).show(ui, &strips);
                for event in out.events {
                    if let TabEvent::Activate(tab) = event {
                        *shown_tab = tab;
                    }
                }
                ui.add_space(Tokens::of(ui).component.dialog.pad);
            }
            let Some(tab) = tabs.get(*shown_tab) else { return };
            FormGrid::new(id.with(("rows", *shown_tab))).show(ui, |form| {
                for row in &tab.rows {
                    show_row(form, row, fields, read_only, &numbers, gap);
                }
            });
        });
        let tab = *shown_tab;
        ctx.data_mut(|d| d.insert_persisted(memory, tab));
        match shown.outcome {
            Some(Outcome::Ok) if read_only => Some(DialogResult::Ok(DialogFields::new())),
            Some(Outcome::Ok) => Some(DialogResult::Ok(fields.clone())),
            Some(Outcome::Cancel | Outcome::Close) if read_only => Some(DialogResult::Ok(DialogFields::new())),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}

fn show_row(form: &mut Form<'_>, row: &PropertyRow, fields: &mut DialogFields, read_only: bool, numbers: &LocaleNumbers, gap: f32) {
    match row {
        PropertyRow::Value { label, text } => {
            form.row(label.as_str(), |ui, _| selectable_text(ui, text));
        }
        PropertyRow::Field(param) if read_only => read_only_field(form, param, fields, numbers),
        PropertyRow::Field(param) => param.row(form, fields, None),
        PropertyRow::Heading(text) => {
            form.spanning(|ui| SectionLabel::new(text.as_str()).show(ui));
        }
        PropertyRow::Space => form.space(gap),
    }
}

fn selectable_text(ui: &mut Ui, text: &str) -> egui::Response {
    let tokens = Tokens::of(ui);
    let format = role_format(&tokens, craft_theme::TypeRole::Body, Content::Normal.color(&tokens).to_egui());
    ui.add(Label::new(egui::text::LayoutJob::single_section(text.to_owned(), format)).selectable(true))
}

fn read_only_field(form: &mut Form<'_>, param: &Param, fields: &DialogFields, numbers: &LocaleNumbers) {
    let ctx = form.ctx().clone();
    let label = format!("{}:", translated(&ctx, param.label.trim_end_matches([':', '：'])));
    let Some(value) = fields.get(&param.key) else {
        form.row(label.as_str(), |ui, _| selectable_text(ui, ""));
        return;
    };
    if let FieldValue::Bool(on) = value {
        let mut on = *on;
        form.row(label.as_str(), |ui, _| Checkbox::bool("", &mut on).enabled(false).show(ui));
        return;
    }
    let text = value_text(&ctx, param, value, numbers);
    form.row(label.as_str(), |ui, _| selectable_text(ui, &text));
}

/// A field's value as the read-only view writes it.
fn value_text(ctx: &egui::Context, param: &Param, value: &FieldValue, numbers: &LocaleNumbers) -> String {
    let number = |n: f64| match &param.unit {
        Some(unit) => format!("{}{}{}", unit.prefix(), numbers.format(n, 0..=6), unit.suffix()),
        None => numbers.format(n, 0..=6),
    };
    match (value, &param.kind) {
        (FieldValue::Number(n), _) => number(*n),
        (FieldValue::Int(i), ParamKind::Channel { names, .. }) => usize::try_from(*i).ok().and_then(|i| names.get(i)).cloned().unwrap_or_default(),
        (FieldValue::Int(i), _) => number(*i as f64),
        (FieldValue::Text(id), ParamKind::Choice { options, .. }) => {
            options.iter().find(|(option, _)| option == id).map_or_else(|| id.clone(), |(_, label)| translated(ctx, label))
        }
        (FieldValue::Text(text), _) => text.clone(),
        (FieldValue::Point([x, y]), _) => format!("{}, {}", numbers.format(*x, 0..=2), numbers.format(*y, 0..=2)),
        (FieldValue::Grid(cells), _) => cells.iter().map(ToString::to_string).collect::<Vec<_>>().join(" "),
        (FieldValue::Color([r, g, b]), _) => {
            format!("#{:02X}{:02X}{:02X}", encode(*r), encode(*g), encode(*b))
        }
        _ => String::new(),
    }
}

/// An sRGB component in 0..=1 as a byte.
fn encode(component: f32) -> u8 {
    (component.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// `text` in the installed Host's language: the catalog row of exactly this text, else the text
/// (the same lookup `Param::row` makes for its labels).
fn translated(ctx: &egui::Context, text: &str) -> String {
    match Host::try_of_ctx(ctx) {
        Some(host) => host.i18n().map_or(text, |i18n| i18n.t(host.lang(), text)).to_owned(),
        None => text.to_owned(),
    }
}
