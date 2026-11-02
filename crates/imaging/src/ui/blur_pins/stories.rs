//! Stories of the Blur Gallery pins (task ST-WS2): not registered. Registering them (task RG2)
//! adds `blur_pins::stories` to the crate's `register!` list and `BlurPinsOverlay` to
//! `COMPONENTS`, and regenerates the registry dump.
//!
//! The stories draw `MockImage` on a canvas with the overlay and the panel beside it; no blur is
//! computed. Every string goes through story words. The pins are the story's own state: an arg
//! change starts them again, and dragging, adding and removing pins work as in the filter dialog.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/blur-pins/field` | Three Field Blur pins, one selected, its amount on the ring |
//! | `composites/blur-pins/iris` | An Iris pin with its outline, handles, rounding square and feather dots |
//! | `composites/blur-pins/spin` | A Spin pin; the ring sets the blur angle |
//! | `composites/blur-pins/tilt` | Tilt-Shift's band with its solid and dashed lines and turn dots |

use craft_ui::canvas::RasterSource as _;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, Surface};
use craft_ui::typed_story;
use egui::{Sense, Ui};

use super::{BlurKind, BlurPin, BlurPins, BlurPinsLabels, BlurPinsOverlay, BlurPinsPanel, IrisShape, TiltBand};
use crate::ui::mock_image::MockImage;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["BlurPinsOverlay"];

craft_ui::words! {
    /// The captions of the blur pins stories, English by default.
    #[non_exhaustive]
    pub(crate) struct BlurPinsStoryLabels {
        edit: "blurPinsStory.edit" = "Edit last: {what}";
        none: "blurPinsStory.none" = "none";
        set: "blurPinsStory.set" = "pins changed";
        begin: "blurPinsStory.begin" = "drag started";
        update: "blurPinsStory.update" = "dragging";
        commit: "blurPinsStory.commit" = "drag finished";
        cancel: "blurPinsStory.cancel" = "drag cancelled";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/blur-pins/field", Level::Composites, "BlurPinsOverlay", "Field", FieldArgs { amount: 30.0 }, show_field)
            .on(Surface::Canvas),
        typed_story!(
            "composites/blur-pins/iris",
            Level::Composites,
            "BlurPinsOverlay",
            "Iris",
            IrisArgs { roundness: 40.0, feather: 50.0, angle: 0.0 },
            show_iris
        )
        .on(Surface::Canvas),
        typed_story!("composites/blur-pins/spin", Level::Composites, "BlurPinsOverlay", "Spin", SpinArgs { angle: 90.0 }, show_spin)
            .on(Surface::Canvas),
        typed_story!(
            "composites/blur-pins/tilt",
            Level::Composites,
            "BlurPinsOverlay",
            "Tilt-Shift",
            TiltArgs { angle: 15.0, focus: 10.0, transition: 15.0 },
            show_tilt
        )
        .on(Surface::Canvas),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FieldArgs {
    /// The selected pin's blur, in pixels.
    #[arg(range = 0.0..=500.0)]
    amount: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct IrisArgs {
    /// The corner rounding, in percent.
    #[arg(range = 0.0..=100.0)]
    roundness: f64,
    /// Where the blur starts, in percent of the shape.
    #[arg(range = 0.0..=99.0)]
    feather: f64,
    /// The outline's turn, in degrees.
    #[arg(range = -180.0..=180.0)]
    angle: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SpinArgs {
    /// The blur angle, in degrees.
    #[arg(range = 0.0..=360.0)]
    angle: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct TiltArgs {
    /// The band's turn, in degrees.
    #[arg(range = -90.0..=90.0)]
    angle: f64,
    /// The sharp band's half width, in percent of the shorter side.
    #[arg(range = 0.0..=100.0)]
    focus: f64,
    /// The transition's width, in percent of the shorter side.
    #[arg(range = 1.0..=100.0)]
    transition: f64,
}

/// The pins' state and the last edit's kind, kept in egui memory with the args they were built
/// from.
#[derive(Clone)]
struct Kept {
    pins: BlurPins,
    signature: Vec<f64>,
    edit: Option<&'static str>,
}

fn bounds(image: &MockImage) -> [f64; 4] {
    let [w, h] = image.size();
    [0.0, 0.0, f64::from(w), f64::from(h)]
}

fn pin(x: f64, y: f64, amount: f64) -> BlurPin {
    BlurPin { x, y, amount, iris: None, tilt: None }
}

/// The picture with the pins over it on the left, the panel on the right.
fn show(ui: &mut Ui, salt: &'static str, signature: Vec<f64>, build: impl FnOnce(&MockImage) -> BlurPins) {
    let labels = BlurPinsLabels::of(ui);
    let words = BlurPinsStoryLabels::of(ui);
    let image = MockImage::default();
    let key = ui.make_persistent_id(("blur-pins-story", salt));
    let mut kept = ui.data(|d| d.get_temp::<Kept>(key)).filter(|kept| kept.signature == signature).unwrap_or_else(|| Kept {
        pins: build(&image),
        signature,
        edit: None,
    });
    ui.columns(2, |columns| {
        let ui = &mut columns[0];
        let width = ui.available_width();
        let (area, view) = image.canvas(ui, ("blur-pins-story", salt), width, None, None);
        let canvas = ui.interact(area.rect, ui.make_persistent_id(("blur-pins-story-canvas", salt)), Sense::click_and_drag() - Sense::FOCUSABLE);
        let out = BlurPinsOverlay::new(("pins", salt), &mut kept.pins).labels(labels.clone()).show(ui, &canvas, &view);
        if let Some(edit) = &out.edit {
            kept.edit = Some(match edit {
                craft_ui::edit::EditEvent::Begin { .. } => "begin",
                craft_ui::edit::EditEvent::Update { .. } => "update",
                craft_ui::edit::EditEvent::Commit { .. } => "commit",
                craft_ui::edit::EditEvent::Cancel { .. } => "cancel",
                _ => "set",
            });
        }

        let ui = &mut columns[1];
        BlurPinsPanel::new(("pin-rows", salt), &mut kept.pins).labels(labels).show(ui);
        let what = match kept.edit {
            Some("begin") => &words.begin,
            Some("update") => &words.update,
            Some("commit") => &words.commit,
            Some("cancel") => &words.cancel,
            Some(_) => &words.set,
            None => &words.none,
        };
        ui.add(Text::new(craft_i18n::fmt(&words.edit, &[("what", what.as_str())])).content(Content::Muted));
    });
    ui.data_mut(|d| d.insert_temp(key, kept));
}

fn show_field(ui: &mut Ui, args: &mut FieldArgs) {
    let amount = args.amount;
    show(ui, "field", vec![amount], |image| {
        let mut pins = BlurPins::new(BlurKind::Field, bounds(image), pin(0.5, 0.5, amount));
        pins.pins = vec![pin(0.22, 0.55, 8.0), pin(0.5, 0.4, amount), pin(0.78, 0.62, 30.0)];
        pins.selected = Some(1);
        pins.clean();
        pins
    });
}

fn show_iris(ui: &mut Ui, args: &mut IrisArgs) {
    let shape = IrisShape { roundness: args.roundness, feather: args.feather / 100.0, angle: args.angle, ..IrisShape::iris() };
    show(ui, "iris", vec![args.roundness, args.feather, args.angle], |image| {
        BlurPins::new(BlurKind::Iris, bounds(image), BlurPin { iris: Some(shape), ..pin(0.5, 0.5, 15.0) })
    });
}

fn show_spin(ui: &mut Ui, args: &mut SpinArgs) {
    let angle = args.angle;
    show(ui, "spin", vec![angle], |image| BlurPins::new(BlurKind::Spin, bounds(image), pin(0.5, 0.5, angle)));
}

fn show_tilt(ui: &mut Ui, args: &mut TiltArgs) {
    let band = TiltBand { angle: args.angle, focus: args.focus / 100.0, transition: args.transition / 100.0 };
    show(ui, "tilt", vec![args.angle, args.focus, args.transition], |image| {
        BlurPins::new(BlurKind::TiltShift, bounds(image), BlurPin { tilt: Some(band), ..pin(0.5, 0.55, 15.0) })
    });
}
