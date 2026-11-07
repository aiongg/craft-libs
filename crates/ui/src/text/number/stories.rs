//! Stories of `NumberField`.

use super::{NumberField, Preset};
use crate::contract::{Bind, Props as _, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::{Key, Modifiers, Ui};

const SIZE: f64 = 12.5;
const HUGE: f64 = 123_456_789_012_345.0;
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
    let presets = [Preset::choice("Auto", false), Preset::Separator, Preset::value("Small", 8.0), Preset::value("Large", 24.0)];
    NumberField::new("Size", Bind::bound(data.value("presets", Some(SIZE)))).presets(&presets).show(ui).edit.into()
}

#[story(NumberField<'static>, covers = [], drive = Drive::Custom(open_popup))]
fn number_field_popup_slider(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    NumberField::new("Size", Bind::bound(data.value("slider", Some(SIZE)))).range(RANGE).popup_slider(RANGE).show(ui).edit.into()
}
