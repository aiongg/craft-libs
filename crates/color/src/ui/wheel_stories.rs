//! Stories of [`ColorWheel`]: the hue ring round the saturation and brightness triangle.

use craft_theme::{ColorModel, HsbModel, ModelValues, kept_or_converted};
use egui::Ui;

use super::ColorWheel;
use craft_ui::controls::slider;
use craft_ui::edit;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice};
use craft_ui::typed_story;

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/color/wheel",
        Level::Composites,
        "ColorWheel",
        "Wheel",
        WheelArgs { side: 220.0, hue_on: HueOn::Colour, disabled: false, width: 260.0 },
        show
    )]
}

/// The colour the wheel starts on, and returns to when this choice changes.
#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum HueOn {
    #[choice(name = "colour")]
    Colour,
    #[choice(name = "black")]
    Black,
    #[choice(name = "grey")]
    Grey,
}

impl HueOn {
    fn values(self) -> ModelValues {
        match self {
            HueOn::Colour => ModelValues::new("hsb", &[210.0, 70.0, 80.0]),
            HueOn::Black => ModelValues::new("hsb", &[120.0, 60.0, 0.0]),
            HueOn::Grey => ModelValues::new("hsb", &[30.0, 0.0, 50.0]),
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct WheelArgs {
    #[arg(range = 28.0..=400.0)]
    side: f32,
    #[arg(name = "hue on")]
    hue_on: HueOn,
    disabled: bool,
    #[arg(range = 60.0..=480.0)]
    width: f32,
}

/// A wheel editing one colour as a panel does, keeping the values last edited so a drag to black
/// or grey keeps the hue; a cancel puts the colour from before the gesture back.
fn show(ui: &mut Ui, args: &mut WheelArgs) {
    let start = HsbModel.to_srgb(&args.hue_on.values().values);
    let state = (start, None::<ModelValues>, start, args.hue_on, String::new());
    slider::with_values(ui, "color-wheel", [state], |ui, state| {
        let (color, kept, before, on, last) = &mut state[0];
        if *on != args.hue_on {
            *on = args.hue_on;
            *color = HsbModel.to_srgb(&on.values().values);
            *kept = Some(on.values());
        }
        let values = kept_or_converted(&HsbModel, *color, kept.as_ref());
        ui.set_width(args.width.min(ui.available_width()));
        let shown = ColorWheel::new("color-wheel", "Color wheel", values).side(args.side).enabled(!args.disabled).show(ui);
        if let Some(event) = &shown.edit.event {
            match event {
                edit::EditEvent::Begin { .. } => *before = *color,
                edit::EditEvent::Cancel { .. } => *color = *before,
                _ => {}
            }
            if let Some(v) = event.value() {
                *color = HsbModel.to_srgb(&v.values);
                *kept = Some(*v);
            }
            *last = edit::describe(&event.clone().map(|v| v.values.map(|c| c as i32)[..3].to_vec()));
        }
        ui.add(Text::new(format!("Last event: {last}")).content(Content::Muted));
    });
}
