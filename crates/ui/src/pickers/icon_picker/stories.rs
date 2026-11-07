//! Stories of `IconPicker`.

use super::{IconPicker, IconPickerGroup, IconPickerItem};
use crate::contract::Bind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

fn groups() -> Vec<IconPickerGroup<u8>> {
    vec![
        IconPickerGroup::new(
            "Selection",
            vec![IconPickerItem::new(1, "Move"), IconPickerItem::new(2, "Marquee").keywords(vec!["select".to_owned()])],
        ),
        IconPickerGroup::new("Drawing", vec![IconPickerItem::new(3, "Brush"), IconPickerItem::new(4, "Eraser"), IconPickerItem::new(5, "Pen")]),
    ]
}

#[story(IconPicker<'static, u8>, covers = [Default], drive = Drive::Never)]
fn icon_picker(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    IconPicker::new("Tool", Bind::bound(data.value("tool", Some(3u8))), &groups()).paint(|_, _, _| {}).show(ui).into()
}

#[story(IconPicker<'static, u8>, covers = [Empty], drive = Drive::Type("zzz"))]
fn icon_picker_no_match(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    IconPicker::new("Tool", Bind::bound(data.value("none", None)), &groups()).show(ui).into()
}
