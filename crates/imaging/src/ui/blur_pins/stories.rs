//! Stories of the Blur Gallery's pins and the panel of the selected pin.

use craft_ui::canvas::RasterSource as _;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

use super::{BlurKind, BlurPin, BlurPins, BlurPinsOverlay, BlurPinsPanel, IrisShape, TiltBand};
use crate::ui::mock_image::{MockImage, StoryView, story_canvas};

/// The pins' state is the story's value under this name.
const PINS: &str = "pins";

fn bounds(image: &MockImage) -> [f64; 4] {
    let size = image.size();
    [0.0, 0.0, f64::from(size.width), f64::from(size.height)]
}

fn pin(x: f64, y: f64, amount: f64) -> BlurPin {
    BlurPin::new(x, y, amount)
}

/// The picture with the pins over it.
fn on_picture(ui: &mut Ui, data: &mut StoryData, build: impl FnOnce(&MockImage) -> BlurPins) -> StoryOutput {
    let image = MockImage::default();
    let pins = data.shown(PINS, build(&image));
    story_canvas(ui, Some(&image), StoryView::Fit, |ui, at| {
        StoryOutput::from(BlurPinsOverlay::new("Blur pins", Bind::bound(&mut *pins)).show_at(ui, at))
    })
}

#[story(BlurPinsOverlay<'static>, covers = [Default], drive = Drive::Never)]
fn field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_picture(ui, data, |image| {
        let mut pins = BlurPins::new(BlurKind::Field, bounds(image), pin(0.5, 0.5, 30.0));
        pins.pins = vec![pin(0.22, 0.55, 8.0), pin(0.5, 0.4, 30.0), pin(0.78, 0.62, 30.0)];
        pins.selected = Some(1);
        pins.clean();
        pins
    })
}

#[story(BlurPinsOverlay<'static>, covers = [Default], drive = Drive::Never)]
fn field_with_two_pins(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_picture(ui, data, |image| {
        let mut pins = BlurPins::new(BlurKind::Field, bounds(image), pin(0.5, 0.5, 15.0));
        pins.pins = vec![pin(0.3, 0.5, 40.0), pin(0.7, 0.5, 10.0)];
        pins.selected = Some(0);
        pins.clean();
        pins
    })
}

#[story(BlurPinsOverlay<'static>, covers = [Default], drive = Drive::Never)]
fn iris(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_picture(ui, data, |image| {
        let mut shape = IrisShape::iris();
        shape.roundness = 40.0;
        shape.feather = 0.5;
        let mut start = pin(0.5, 0.5, 15.0);
        start.iris = Some(shape);
        BlurPins::new(BlurKind::Iris, bounds(image), start)
    })
}

#[story(BlurPinsOverlay<'static>, covers = [Default], drive = Drive::Never)]
fn spin(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_picture(ui, data, |image| BlurPins::new(BlurKind::Spin, bounds(image), pin(0.5, 0.5, 90.0)))
}

#[story(BlurPinsOverlay<'static>, covers = [Default], drive = Drive::Never)]
fn tilt_shift(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    on_picture(ui, data, |image| {
        let mut start = pin(0.5, 0.55, 15.0);
        start.tilt = Some(TiltBand::new(15.0, 0.1, 0.15));
        BlurPins::new(BlurKind::TiltShift, bounds(image), start)
    })
}

fn panel(ui: &mut Ui, data: &mut StoryData, selected: Option<usize>) -> StoryOutput {
    let pins = data.shown(PINS, {
        let mut pins = BlurPins::new(BlurKind::Iris, [0.0, 0.0, 800.0, 600.0], pin(0.5, 0.5, 15.0));
        pins.selected = selected;
        pins
    });
    StoryOutput::from(BlurPinsPanel::new(pins).show(ui).response)
}

#[story(BlurPinsPanel<'static>, covers = [Default], drive = Drive::Never)]
fn panel_with_a_selected_pin(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    panel(ui, data, Some(0))
}

#[story(BlurPinsPanel<'static>, covers = [Default], drive = Drive::Never)]
fn panel_without_a_selected_pin(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    panel(ui, data, None)
}
