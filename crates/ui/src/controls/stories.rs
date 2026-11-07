//! Stories of the value controls: `RadioGroup`, `Radio` and `Disclosure`.

use super::disclosure::Disclosure;
use super::radio::{Radio, RadioGroup};
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::{Response, Ui};

/// The three members of a group over one value; the story shows the second one's response.
fn spread(ui: &mut Ui, group: RadioGroup<'_, u8>) -> (StoryOutput, Option<Response>) {
    let mut second = None;
    let shown = group.show(ui, |ui, _| {
        Radio::new(1).label("Small").show(ui);
        second = Some(Radio::new(2).label("Medium").show(ui));
        Radio::new(3).label("Large").show(ui);
    });
    (shown.into(), second)
}

fn pick_second(fixture: &mut Fixture) {
    if let Some(second) = fixture.by_label("Medium") {
        fixture.click(&second);
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

#[story(Radio<u8>, covers = [Default])]
fn radio(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    spread(ui, RadioGroup::new("Size", Bind::bound(data.value("size", 1u8)))).1.map(StoryOutput::from).unwrap_or_default()
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
