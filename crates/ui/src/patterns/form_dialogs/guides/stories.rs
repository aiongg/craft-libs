//! Stories of the guide dialogs.

use super::{GuideColor, GuideLayoutDialog, GuideLayoutValues, GuideRect, GuideUnits, NewGuideDialog, NewGuideValues};
use crate::canvas::{Guide, GuideKey};
use crate::contract::Bind;
use crate::runtime::env::UnitKind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::Axis;
use craft_theme::Rgba;
use egui::Ui;

/// A 3000 by 2000 pixel document at 300 pixels per inch.
const WIDTH: f64 = 3000.0;
const HEIGHT: f64 = 2000.0;
const RESOLUTION: f64 = 300.0;
/// Where the edited guide stands, in pixels.
const EDITED_AT: f64 = 750.0;
/// The guide colour preference the edited guide is drawn in.
const PREFERENCE: Rgba = Rgba::rgb(0x4a, 0xff, 0xff);

fn units() -> GuideUnits {
    GuideUnits::new(UnitKind::Centimetres, WIDTH, HEIGHT, RESOLUTION)
}

#[story(NewGuideDialog<'static>, covers = [], drive = Drive::Never)]
fn new_guide_dialog(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", NewGuideValues::default());
    NewGuideDialog::new(units(), Bind::bound(values)).opened(Bind::given(true)).show(ui).into()
}

#[story(NewGuideDialog<'static>, covers = [], drive = Drive::Never)]
fn new_guide_dialog_edit(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut guide = Guide::new(GuideKey::new(1), Axis::Y, EDITED_AT);
    guide.color = Some(GuideColor::Magenta.rgba());
    let values = data.value("values", NewGuideValues::of_guide(&guide, PREFERENCE));
    NewGuideDialog::edit(units(), guide, PREFERENCE, Bind::bound(values)).opened(Bind::given(true)).show(ui).into()
}

#[story(GuideLayoutDialog<'static>, covers = [], drive = Drive::Never)]
fn guide_layout_dialog(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = data.value("values", GuideLayoutValues::default());
    let document = GuideRect::new(0.0, 0.0, WIDTH, HEIGHT);
    GuideLayoutDialog::new(units(), document, Bind::bound(values)).opened(Bind::given(true)).show(ui).into()
}
