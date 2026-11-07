//! Stories of `UnitSelect`.

use super::UnitSelect;
use crate::contract::{Bind, Entry, Show as _};
use crate::runtime::env::UnitKind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(UnitSelect<'static>, covers = [Default], drive = Drive::Type("42"))]
fn unit_select(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let units = [Entry::new(UnitKind::Millimetres, "mm"), Entry::new(UnitKind::Inches, "in"), Entry::new(UnitKind::Points, "pt")];
    UnitSelect::new("Width", Bind::bound(data.value("width", Some(210.0))), Bind::bound(data.value("unit", UnitKind::Millimetres)), &units)
        .show(ui)
        .into()
}
