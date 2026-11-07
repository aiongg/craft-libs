//! Stories of `Field`.

use super::{Field, FieldGroup};
use super::Help;
use crate::contract::{Bind, FieldLayout, LabelKind, Show as _, ShowContainer as _, SideLabel, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use crate::text::NumberField;
use craft_behavior::IconId;
use craft_theme::variant::Tone;
use egui::Ui;

const SIZE: f64 = 12.0;

#[story(Field<'static>, covers = [Default], drive = Drive::Never)]
fn field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Field::new("Size")
        .help(Help::Text("In points"))
        .show(ui, |ui, _| NumberField::new("Size", Bind::bound(data.value("size", Some(SIZE)))).show(ui))
        .into()
}

#[story(Field<'static>, covers = [], drive = Drive::Never)]
fn field_above_with_status(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Field::new("Size")
        .layout(FieldLayout::Above)
        .help(Help::Status(Tone::Warn, "Large sizes slow the preview"))
        .show(ui, |ui, _| NumberField::new("Size", Bind::bound(data.value("above", Some(SIZE)))).show(ui))
        .into()
}

#[story(Field<'static>, covers = [LongText], drive = Drive::Never)]
fn field_long_text(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Field::new("A label much longer than the column a form usually gives it")
        .help(Help::Text("A help text much longer than the room under the control, so it wraps onto more than one line"))
        .show(ui, |ui, _| NumberField::new("Size", Bind::bound(data.value("long", Some(SIZE)))).show(ui))
        .into()
}

/// A glyph label: the icon shows, the text only names the control.
#[story(Field<'static>, covers = [], drive = Drive::Never)]
fn field_icon_label(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Field::new("Width")
        .side(SideLabel::new().kind(LabelKind::Hidden).icon(IconId::new("list")))
        .show(ui, |ui, _| NumberField::new("Width", Bind::bound(data.value("width", Some(SIZE)))).show(ui))
        .into()
}

#[story(Field<'static>, covers = [], drive = Drive::Never)]
fn field_with_refused_value(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_negative = |size: &Option<f64>| match size {
        Some(size) if *size < 0.0 => Validity::Refuse(Message::new("Size cannot be negative")),
        _ => Validity::Ok,
    };
    Field::new("Size")
        .help(Help::Text("In points"))
        .show(ui, |ui, _| NumberField::new("Size", Bind::bound(data.value("refused", Some(-SIZE))).check(refuse_negative)).show(ui))
        .into()
}

#[story(FieldGroup, covers = [Default], drive = Drive::Never)]
fn field_group(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let rows = [("Width", "width"), ("Corner radius", "radius"), ("Maximum number of layers", "layers")];
    FieldGroup::new("Shape")
        .show(ui, |ui, _| {
            for (label, name) in rows {
                Field::new(label).show(ui, |ui, _| NumberField::new(label, Bind::bound(data.value(name, Some(SIZE)))).show(ui));
            }
        })
        .into()
}
