//! Stories of `ConstraintFields`.

use super::ConstraintFields;
use crate::bars::{Bar, BarKind};
use crate::contract::{Bind, Entry, ShowContainer as _};
use crate::runtime::env::UnitKind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::PixelSize;
use craft_behavior::geometry::Constraint;
use egui::Ui;

const DOCUMENT: PixelSize = PixelSize { width: 1920, height: 1080 };
const PIXELS_PER_INCH: f64 = 300.0;

fn bar(ui: &mut Ui, fields: ConstraintFields<'_>) -> StoryOutput {
    Bar::new("Options", BarKind::Options)
        .show(ui, |ui, _| {
            fields.show(ui);
        })
        .output
        .into()
}

#[story(ConstraintFields<'static>, covers = [], drive = Drive::Never)]
fn normal(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    bar(ui, ConstraintFields::new(Bind::bound(data.value("constraint", Constraint::Free))))
}

#[story(ConstraintFields<'static>, covers = [], drive = Drive::Never)]
fn fixed_ratio(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    bar(ui, ConstraintFields::new(Bind::bound(data.value("constraint", Constraint::Ratio(16.0, 9.0)))))
}

#[story(ConstraintFields<'static>, covers = [], drive = Drive::Never)]
fn fixed_size_with_units(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let units = [
        Entry::new(UnitKind::Pixels, "Pixels"),
        Entry::new(UnitKind::Inches, "Inches"),
        Entry::new(UnitKind::Centimetres, "Centimetres"),
        Entry::new(UnitKind::Percent, "Percent"),
    ];
    let fields = ConstraintFields::new(Bind::bound(data.value("constraint", Constraint::Size(600.0, 300.0)))).size_units(
        UnitKind::Inches,
        &units,
        PIXELS_PER_INCH,
        DOCUMENT,
    );
    bar(ui, fields)
}

#[story(ConstraintFields<'static>, covers = [], drive = Drive::Never)]
fn greyed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    bar(ui, ConstraintFields::new(Bind::bound(data.value("constraint", Constraint::Ratio(1.0, 1.0)))).enabled(false))
}
