//! Stories of the value controls: `Checkbox`, `Slider`, `RadioGroup` and `Radio`.

use super::check::Checkbox;
use super::disclosure::Disclosure;
use super::radio::{Radio, RadioGroup};
use super::slider::{Slider, THUMB};
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _, Validity};
use crate::layout_components::{Row, Stack};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::{Response, Ui};

#[story(Checkbox<'static>, covers = [Default])]
fn checkbox(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Checkbox::new(Bind::bound(data.value("checked", false))).label("Show grid").show(ui).into()
}

#[story(Checkbox<'static>, covers = [Mixed], drive = Drive::Never)]
fn checkbox_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Checkbox::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed))).label("Show grid").show(ui).into()
}

#[story(Checkbox<'static>, covers = [Invalid], drive = Drive::Never)]
fn checkbox_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_on = |on: &bool| if *on { Validity::Refuse(Message::new("This cannot be on")) } else { Validity::Ok };
    Checkbox::new(Bind::bound(data.value("invalid", true)).check(refuse_on)).label("Lock aspect").show(ui).into()
}

#[story(Checkbox<'static>, covers = [], drive = Drive::Never)]
fn checkbox_disabled(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Checkbox::new(Bind::bound(data.value("disabled", true))).label("Show grid").enabled(false).show(ui).into()
}

#[story(Checkbox<'static>, covers = [], disabled_text = "Turn on the grid first")]
fn checkbox_with_reason(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let tooltip = data.tooltip("Snap to the grid");
    Checkbox::new(Bind::bound(data.value("on", false))).label("Snap").tooltip(tooltip).show(ui).into()
}

const RANGE: std::ops::RangeInclusive<f64> = 0.0..=100.0;
const CAP: f64 = 80.0;

#[story(Slider<'static>, covers = [Default], drive = Drive::Step)]
fn slider(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Drag(THUMB))]
fn slider_drag(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [Mixed], drive = Drive::Never)]
fn slider_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed)), RANGE).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [Invalid], drive = Drive::Never)]
fn slider_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_over = |value: &f64| if *value > CAP { Validity::Refuse(Message::new("Above the limit")) } else { Validity::Ok };
    Slider::new(Bind::bound(data.value("over", 95.0)).check(refuse_over), RANGE).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Never)]
fn slider_disabled(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).label("Opacity").enabled(false).show(ui).into()
}

/// Members spread over nested layouts, with the group's body between them: one group, one value.
fn spread(ui: &mut Ui, group: RadioGroup<'_, u8>) -> (StoryOutput, Option<Response>) {
    let mut second = None;
    let shown = group.show(ui, |ui, _| {
        Stack::new().show(ui, |ui, _| {
            Radio::new(1u8).label("Small").show(ui);
            Row::new().show(ui, |ui, _| {
                second = Some(Radio::new(2u8).label("Medium").show(ui));
                Radio::new(3u8).label("Large").show(ui);
            });
        });
    });
    (shown.into(), second)
}

fn pick_second(fixture: &mut Fixture) {
    if let Some(medium) = fixture.by_label("Medium") {
        fixture.click(&medium);
    }
}

#[story(RadioGroup<'static, u8>, covers = [Default], drive = Drive::Custom(pick_second))]
fn radio_group(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    spread(ui, RadioGroup::new("Size", Bind::bound(data.value("size", 1u8)))).0
}

#[story(RadioGroup<'static, u8>, covers = [Mixed], drive = Drive::Never)]
fn radio_group_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    spread(ui, RadioGroup::new("Size", Bind::bound_mixed(data.value("mixed", Mixed::Mixed)))).0
}

#[story(RadioGroup<'static, u8>, covers = [Invalid], drive = Drive::Never)]
fn radio_group_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_large = |size: &u8| if *size == 3 { Validity::Refuse(Message::new("Too large")) } else { Validity::Ok };
    spread(ui, RadioGroup::new("Size", Bind::bound(data.value("large", 3u8)).check(refuse_large))).0
}

// The shown member is the selected one, where Tab arrives, so the key drives reach it.
#[story(Radio<u8>, covers = [Default])]
fn radio(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    spread(ui, RadioGroup::new("Size", Bind::bound(data.value("size", 2u8)))).1.map(StoryOutput::from).unwrap_or_default()
}

#[story(Disclosure<'static>, covers = [Default])]
fn disclosure(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Disclosure::new(Bind::bound(data.value("open", false))).label("Details").show(ui).into()
}

#[story(Disclosure<'static>, covers = [Mixed], drive = Drive::Never)]
fn disclosure_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Disclosure::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed))).label("Details").show(ui).into()
}

#[story(Disclosure<'static>, covers = [Invalid], drive = Drive::Never)]
fn disclosure_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_open = |open: &bool| if *open { Validity::Refuse(Message::new("Nothing to show")) } else { Validity::Ok };
    Disclosure::new(Bind::bound(data.value("open", true)).check(refuse_open)).label("Details").show(ui).into()
}
