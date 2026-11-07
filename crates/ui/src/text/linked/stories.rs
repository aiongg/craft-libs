//! Stories of `LinkedFields`.

use super::{LinkMode, LinkedFields, LinkedLayout, LinkedOutput};
use crate::contract::{Bind, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::Ui;

impl<const N: usize> From<LinkedOutput<N>> for StoryOutput {
    fn from(output: LinkedOutput<N>) -> Self {
        output.edit.into()
    }
}

#[story(LinkedFields<'static, 2>, covers = [Default], drive = Drive::Type("120"))]
fn linked_fields(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let linked = *data.value("linked", true);
    LinkedFields::new("Size", Bind::bound(data.value("size", [Some(100.0), Some(50.0)])), Bind::given(linked)).captions(["W", "H"]).range(1.0..=1000.0).show(ui).into()
}

#[story(LinkedFields<'static, 4>, covers = [], drive = Drive::Never)]
fn linked_fields_equal_columns(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let linked = *data.value("linked", true);
    LinkedFields::new("Margins", Bind::bound(data.value("margins", [Some(12.0); 4])), Bind::given(linked))
        .captions(["Top", "Bottom", "Inside", "Outside"])
        .mode(LinkMode::Equal)
        .layout(LinkedLayout::Columns)
        .show(ui)
        .into()
}

#[story(LinkedFields<'static, 2>, covers = [Mixed], drive = Drive::Never)]
fn linked_fields_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    LinkedFields::new("Size", Bind::bound_mixed(data.value("mixed", Mixed::<[Option<f64>; 2]>::Mixed)), Bind::given(false)).show(ui).into()
}

#[story(LinkedFields<'static, 2>, covers = [Invalid], drive = Drive::Never)]
fn linked_fields_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let too_wide = |size: &[Option<f64>; 2]| match size[0] {
        Some(width) if width > 500.0 => Validity::Refuse(Message::new("Too wide")),
        _ => Validity::Ok,
    };
    LinkedFields::new("Size", Bind::bound(data.value("wide", [Some(900.0), Some(50.0)])).check(too_wide), Bind::given(false)).show(ui).into()
}
