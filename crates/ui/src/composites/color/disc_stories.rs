//! Stories of [`ColorDisc`]: the hue and saturation disc with a marker per colour.

use craft_theme::{ColorModel, HsbModel, ModelValues, Srgba};
use egui::Ui;

use super::ColorDisc;
use crate::controls::slider;
use crate::edit;
use crate::primitives::{Content, Text};
use crate::story::{Level, Story, StoryArgs};
use crate::typed_story;

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/color/disc",
        Level::Composites,
        "ColorDisc",
        "Disc",
        DiscArgs { selected: 0.0, brightness: 1.0, side: 150.0, disabled: false },
        show
    )]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DiscArgs {
    /// The selected marker, 1 to 3 (0 for none).
    #[arg(range = 0.0..=3.0)]
    selected: f32,
    #[arg(range = 0.0..=1.0)]
    brightness: f32,
    #[arg(range = 28.0..=400.0)]
    side: f32,
    disabled: bool,
}

fn start() -> [Srgba; 3] {
    [[0.0, 80.0, 90.0], [120.0, 60.0, 90.0], [250.0, 70.0, 90.0]].map(|v| HsbModel.to_srgb(&ModelValues::new("hsb", &v).values))
}

/// Three markers the host moves where they are dragged; a press selects the marker it grabbed.
fn show(ui: &mut Ui, args: &mut DiscArgs) {
    slider::with_values(ui, "color-disc", [(start(), None::<usize>, String::new())], |ui, state| {
        let (colors, picked, last) = &mut state[0];
        let selected = if args.selected >= 0.5 { Some((args.selected.round() as usize - 1).min(2)) } else { *picked };
        let width = ui.available_width().min(args.side);
        ui.set_width(width);
        let shown = ColorDisc::new("color-disc", "Harmony", &colors[..])
            .selected(selected)
            .brightness(args.brightness)
            .side(width.min(args.side))
            .enabled(!args.disabled)
            .show(ui);
        if let Some(index) = shown.pressed {
            *picked = Some(index);
        }
        if let Some(event) = &shown.edit.event {
            if let Some(v) = event.value()
                && let Some(color) = colors.get_mut(v.marker)
            {
                let brightness = HsbModel.from_srgb(*color).values[2];
                *color = HsbModel.to_srgb(&ModelValues::new("hsb", &[v.hue, v.saturation * 100.0, brightness]).values);
            }
            *last = edit::describe(&event.clone().map(|v| vec![v.marker as i32, v.hue as i32, (v.saturation * 100.0) as i32]));
        }
        ui.add(Text::new(format!("Last event: {last}")).content(Content::Muted));
    });
}
