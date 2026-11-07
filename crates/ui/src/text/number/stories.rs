//! Stories of `NumberField`.

use super::{NumberField, NumberPick};
use crate::contract::{Bind, Entry, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::{Mixed, NumberCodec, Reader};
use egui::{Key, Modifiers, Ui};

const SIZE: f64 = 12.5;
const HUGE: f64 = 123_456_789_012_345.0;
/// The number the Auto word reads as.
const AUTO: f64 = 0.0;
const RANGE: std::ops::RangeInclusive<f64> = 0.0..=100.0;

#[story(NumberField<'static>, covers = [Default], drive = Drive::Type("42"))]
fn number_field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    NumberField::new("Size", Bind::bound(data.value("size", Some(SIZE)))).range(RANGE).step(0.5).unit("pt").show(ui).edit.into()
}

#[story(NumberField<'static>, covers = [LongText], drive = Drive::Never)]
fn number_field_long(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    NumberField::new("Size", Bind::bound(data.value("long", Some(HUGE)))).show(ui).edit.into()
}

#[story(NumberField<'static>, covers = [Mixed], drive = Drive::Never)]
fn number_field_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    NumberField::new("Size", Bind::bound_mixed(data.value("mixed", Mixed::<Option<f64>>::Mixed))).show(ui).edit.into()
}

#[story(NumberField<'static>, covers = [Invalid], drive = Drive::Never)]
fn number_field_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_negative = |size: &Option<f64>| match size {
        Some(size) if *size < 0.0 => Validity::Refuse(Message::new("Size cannot be negative")),
        _ => Validity::Ok,
    };
    NumberField::new("Size", Bind::bound(data.value("negative", Some(-SIZE))).check(refuse_negative)).show(ui).edit.into()
}

/// Focuses the field and opens its list or slider with Alt+Down.
fn open_popup(fixture: &mut Fixture) {
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(4);
}

#[story(NumberField<'static>, covers = [Open], drive = Drive::Custom(open_popup))]
fn number_field_presets(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let presets = [
        Entry::new(NumberPick::Choice(0), "Auto"),
        Entry::separator(),
        Entry::new(NumberPick::Number(8.0), "Small"),
        Entry::new(NumberPick::Number(24.0), "Large"),
    ];
    NumberField::new("Size", Bind::bound(data.value("presets", Some(SIZE)))).presets(&presets).show(ui).edit.into()
}

#[story(NumberField<'static>, covers = [], drive = Drive::Custom(open_popup))]
fn number_field_popup_slider(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    NumberField::new("Size", Bind::bound(data.value("slider", Some(SIZE)))).range(RANGE).popup_slider(RANGE).show(ui).edit.into()
}

#[story(NumberField<'static>, covers = [], drive = Drive::Type("Auto"))]
fn number_field_auto(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let codec = NumberCodec::new(Reader::new(), 1).or_parse(|text| text.trim().eq_ignore_ascii_case("auto").then_some(AUTO));
    NumberField::new("Size", Bind::bound(data.value("auto", Some(SIZE)))).codec(codec).range(RANGE).show(ui).edit.into()
}

#[story(NumberField<'static>, covers = [], drive = Drive::Type("7"))]
fn number_field_snapped(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    NumberField::new("Size", Bind::bound(data.value("snapped", Some(SIZE)))).range(RANGE).step(5.0).snap_typed(true).show(ui).edit.into()
}
