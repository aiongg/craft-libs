//! Stories of the choice family: `Checkbox`, `Switch`, `RadioGroup` and `Radio`, `Segmented` and `Rating`.

use super::check::Checkbox;
use super::radio::{Radio, RadioGroup};
use super::rating::Rating;
use super::segmented::{Segment, Segmented};
use super::switch::Switch;
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _, SideLabel, Validity};
use crate::layout_components::{Row, Stack};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::{IconId, Mixed};
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

/// Rows that carry an icon and a count through the body's label step.
#[story(RadioGroup<'static, u8>, covers = [], drive = Drive::Never)]
fn radio_group_with_icons_and_counts(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let rows = [(1u8, "Layers", "list", "12"), (2, "Pages", "grid", "4"), (3, "Masks", "eye", "0")];
    RadioGroup::new("Show", Bind::bound(data.value("show", 1u8)))
        .show(ui, |ui, cx| {
            for (value, label, icon, count) in rows {
                cx.labelled(ui, value, label.into(), SideLabel::new().icon(IconId::new(icon)).detail(count), Radio::new(value));
            }
        })
        .into()
}

// The shown member is the selected one, where Tab arrives, so the key drives reach it.
#[story(Radio<u8>, covers = [Default])]
fn radio(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    spread(ui, RadioGroup::new("Size", Bind::bound(data.value("size", 2u8)))).1.map(StoryOutput::from).unwrap_or_default()
}

#[story(Switch<'static>, covers = [Default])]
fn switch(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Switch::new(Bind::bound(data.value("on", false))).label("Show rulers").show(ui).into()
}

#[story(Switch<'static>, covers = [Mixed], drive = Drive::Never)]
fn switch_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Switch::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed))).label("Show rulers").show(ui).into()
}

#[story(Switch<'static>, covers = [Invalid], drive = Drive::Never)]
fn switch_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_on = |on: &bool| if *on { Validity::Refuse(Message::new("This cannot be on")) } else { Validity::Ok };
    Switch::new(Bind::bound(data.value("invalid", true)).check(refuse_on)).label("Lock aspect").show(ui).into()
}

#[story(Switch<'static>, covers = [], drive = Drive::Never)]
fn switch_disabled(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Switch::new(Bind::bound(data.value("disabled", true))).label("Show rulers").enabled(false).show(ui).into()
}

#[story(Switch<'static>, covers = [], disabled_text = "Turn on the grid first")]
fn switch_with_reason(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let tooltip = data.tooltip("Snap to the grid");
    Switch::new(Bind::bound(data.value("on", false))).label("Snap").tooltip(tooltip).show(ui).into()
}

fn sizes() -> [Segment<u8>; 3] {
    [Segment::new(1, "Small"), Segment::new(2, "Medium"), Segment::new(3, "Large")]
}

fn pick_medium(fixture: &mut Fixture) {
    if let Some(medium) = fixture.by_label("Medium") {
        fixture.click(&medium);
    }
}

#[story(Segmented<'static, u8>, covers = [Default], drive = Drive::Custom(pick_medium))]
fn segmented(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Segmented::new(Bind::bound(data.value("size", 1u8)), &sizes()).label("Size").show(ui).into()
}

#[story(Segmented<'static, u8>, covers = [Mixed], drive = Drive::Never)]
fn segmented_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Segmented::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed)), &sizes()).label("Size").show(ui).into()
}

#[story(Segmented<'static, u8>, covers = [Invalid], drive = Drive::Never)]
fn segmented_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_large = |size: &u8| if *size == 3 { Validity::Refuse(Message::new("Too large")) } else { Validity::Ok };
    Segmented::new(Bind::bound(data.value("large", 3u8)).check(refuse_large), &sizes()).label("Size").show(ui).into()
}

#[story(Segmented<'static, u8>, covers = [], drive = Drive::Never)]
fn segmented_disabled(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Segmented::new(Bind::bound(data.value("size", 2u8)), &sizes()).label("Size").enabled(false).show(ui).into()
}

#[story(Segmented<'static, u8>, covers = [], disabled_text = "Pick a layer first")]
fn segmented_with_reason(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let tooltip = data.tooltip("The size of the brush");
    Segmented::new(Bind::bound(data.value("size", 1u8)), &sizes()).label("Size").tooltip(tooltip).show(ui).into()
}

#[story(Segmented<'static, u8>, covers = [], drive = Drive::Never)]
fn segmented_icons(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let views = [
        Segment::new(1u8, "List").icon(IconId::new("list")),
        Segment::new(2, "Grid").icon(IconId::new("grid")),
        Segment::new(3, "Detail").enabled(false),
    ];
    Segmented::new(Bind::bound(data.value("view", 1u8)), &views).label("View").show(ui).into()
}

fn pick_four(fixture: &mut Fixture) {
    if let Some(star) = fixture.by_label("4 stars") {
        fixture.click(&star);
    }
}

#[story(Rating<'static>, covers = [Default], drive = Drive::Custom(pick_four))]
fn rating(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Rating::new(Bind::bound(data.value("stars", 2u8)), 5).label("Rating").show(ui).into()
}

#[story(Rating<'static>, covers = [Mixed], drive = Drive::Never)]
fn rating_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Rating::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed)), 5).label("Rating").show(ui).into()
}

#[story(Rating<'static>, covers = [Invalid], drive = Drive::Never)]
fn rating_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_top = |stars: &u8| if *stars == 5 { Validity::Refuse(Message::new("Five stars needs a reason")) } else { Validity::Ok };
    Rating::new(Bind::bound(data.value("top", 5u8)).check(refuse_top), 5).label("Rating").show(ui).into()
}

#[story(Rating<'static>, covers = [], drive = Drive::Never)]
fn rating_disabled(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Rating::new(Bind::bound(data.value("stars", 3u8)), 5).label("Rating").enabled(false).show(ui).into()
}

#[story(Rating<'static>, covers = [], disabled_text = "Open the photo first")]
fn rating_with_reason(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let tooltip = data.tooltip("The photo's rating");
    Rating::new(Bind::bound(data.value("stars", 0u8)), 5).label("Rating").tooltip(tooltip).show(ui).into()
}
