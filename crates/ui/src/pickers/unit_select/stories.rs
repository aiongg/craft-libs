//! Stories of `UnitSelect`.

use super::UnitSelect;
use crate::contract::{Bind, Entry, Show as _};
use crate::runtime::env::UnitKind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(UnitSelect<'static>, covers = [Default], drive = Drive::Type("42"))]
fn unit_select(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let units = [Entry::new(UnitKind::Millimetres, "mm"), Entry::new(UnitKind::Inches, "in"), Entry::new(UnitKind::Points, "pt")];
    let width = data.value("width_and_unit", (Some(210.0), UnitKind::Millimetres));
    let (width, unit) = (&mut width.0, &mut width.1);
    UnitSelect::new("Width", Bind::bound(width), Bind::bound(unit), &units).show(ui).into()
}
