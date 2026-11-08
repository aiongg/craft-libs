//! Stories of the Brush Settings panel and its rows.

use craft_behavior::Mixed;
use craft_ui::contract::{Bind, Props as _, Show as _, Validity};
use craft_ui::primitives::Revision;
use craft_ui::runtime::env::Message;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Color32, ColorImage, Ui};

use super::{
    BrushSection, BrushSectionList, BrushSettingsPanel, BrushSizeField, BrushStrokeStrip, DynamicControl, DynamicControlRow, StripImage, TipAngle,
    TipEllipse,
};

const ANGLE: TipAngle = TipAngle::new(30.0, 0.6);
const SIZE: f64 = 30.0;
const MAX_SIZE: f64 = 5000.0;
const ORIGINAL: f64 = 64.0;
const FADE: f64 = 25.0;
/// The sections of the Brush Settings list, by key.
const TIP: u8 = 0;
const DYNAMICS: u8 = 1;
const SCATTER: u8 = 2;
const NOISE: u8 = 10;
const SMOOTHING: u8 = 11;

fn sections(on: bool) -> Vec<BrushSection<u8>> {
    vec![
        BrushSection::pane(TIP, "Brush Tip Shape"),
        BrushSection::pane(DYNAMICS, "Shape Dynamics").enabled(on).locked(false),
        BrushSection::pane(SCATTER, "Scattering").enabled(on).locked(true),
        BrushSection::switch(NOISE, "Noise", on),
        BrushSection::switch(SMOOTHING, "Smoothing", true),
    ]
}

fn strip<'a>() -> BrushStrokeStrip<'a> {
    BrushStrokeStrip::new(
        "Stroke preview",
        Revision::new(1),
        StripImage::new(|[width, height]| ColorImage::filled([width, height], Color32::TRANSPARENT)),
    )
}

#[story(BrushSettingsPanel<'static, u8>, covers = [Default], drive = Drive::Never)]
fn panel(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let sections = sections(true);
    let shown = *data.value("shown", TIP);
    BrushSettingsPanel::new("Brush Settings", &sections, Bind::given(shown))
        .link_row()
        .can_update()
        .strip(strip())
        .show(ui, |ui, _, _| {
            let _ = BrushSizeField::new("Size", Bind::given(SIZE), MAX_SIZE).show(ui);
        })
        .into()
}

#[story(BrushSettingsPanel<'static, u8>, covers = [], drive = Drive::Never)]
fn panel_off(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let sections = sections(false);
    let shown = *data.value("shown", DYNAMICS);
    BrushSettingsPanel::new("Brush Settings", &sections, Bind::given(shown))
        .strip(strip())
        .show(ui, |ui, _, _| {
            let _ = BrushSizeField::new("Size", Bind::given(SIZE), MAX_SIZE).show(ui);
        })
        .into()
}

#[story(BrushSectionList<'static, u8>, covers = [Default], drive = Drive::Never)]
fn section_list(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let sections = sections(true);
    let shown = *data.value("shown", TIP);
    BrushSectionList::new("Brush settings sections", &sections, Bind::given(shown)).link_row().show(ui).into()
}

#[story(TipEllipse<'static>, covers = [Default], drive = Drive::Never)]
fn tip_ellipse(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TipEllipse::new("Tip angle and roundness", Bind::bound(data.value("angle", ANGLE))).show(ui).into()
}

#[story(TipEllipse<'static>, covers = [Mixed], drive = Drive::Never)]
fn tip_ellipse_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TipEllipse::new("Tip angle and roundness", Bind::bound_mixed(data.value("mixed", Mixed::<TipAngle>::Mixed))).show(ui).into()
}

#[story(TipEllipse<'static>, covers = [Invalid], drive = Drive::Never)]
fn tip_ellipse_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_flat = |angle: &TipAngle| if angle.roundness < 0.5 { Validity::Refuse(Message::new("Too flat")) } else { Validity::Ok };
    TipEllipse::new("Tip angle and roundness", Bind::bound(data.value("flat", ANGLE)).check(refuse_flat)).show(ui).into()
}

#[story(DynamicControlRow<'static, u8>, covers = [Default], drive = Drive::Never)]
fn dynamic_control_row(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    const OFF: u8 = 0;
    const FADE_CONTROL: u8 = 1;
    let controls = [DynamicControl::new(OFF, "Off"), DynamicControl::new(FADE_CONTROL, "Fade").fade().curve()];
    let pair = data.value("control", (FADE_CONTROL, FADE));
    let (control, steps) = (&mut pair.0, &mut pair.1);
    DynamicControlRow::new(&controls, Bind::bound(control), Bind::bound(steps)).curve(false, true).show(ui).into()
}

#[story(BrushStrokeStrip<'static>, covers = [Default], drive = Drive::Never)]
fn stroke_strip(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    strip().show(ui).into()
}

#[story(BrushSizeField<'static>, covers = [Default], drive = Drive::Never)]
fn size_field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    BrushSizeField::new("Size", Bind::bound(data.value("size", SIZE)), MAX_SIZE).restore(ORIGINAL).show(ui).into()
}
