//! The story of [`ColorRamp`].

use craft_theme::Srgba;
use egui::Ui;

use super::{ColorRamp, RampKind};
use craft_ui::controls::slider;
use craft_ui::edit;
use craft_ui::primitives::{Content, Gap, Stack, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice};
use craft_ui::typed_story;

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/color/ramp",
        Level::Composites,
        "ColorRamp",
        "Ramp",
        RampArgs { kind: KindArg::Spectrum, web_safe: false, boxes: true, gamut: false, disabled: false },
        show
    )]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum KindArg {
    Spectrum,
    Grayscale,
    Between,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct RampArgs {
    kind: KindArg,
    web_safe: bool,
    boxes: bool,
    /// A stand-in for a CMYK gamut: pulls every channel a tenth towards grey.
    gamut: bool,
    disabled: bool,
}

/// The ramp as a panel shows it: a pick is the "colour" under the bar, a Shift-click counts a
/// cycle.
fn show(ui: &mut Ui, args: &mut RampArgs) {
    let start = (Srgba::rgb(0.24, 0.39, 0.87), 0_usize, String::new());
    slider::with_values(ui, "color-ramp", [start], |ui, state| {
        let (color, cycles, last) = &mut state[0];
        let kind = match args.kind {
            KindArg::Spectrum => RampKind::Spectrum,
            KindArg::Grayscale => RampKind::Grayscale,
            KindArg::Between => RampKind::Between,
        };
        let muted = |c: Srgba| Srgba::new(c.r * 0.9 + 0.05, c.g * 0.9 + 0.05, c.b * 0.9 + 0.05, c.a);
        let mut ramp = ColorRamp::new("color-ramp", "Color ramp", kind)
            .ends(Srgba::rgb(0.9, 0.2, 0.2), Srgba::rgb(0.2, 0.3, 0.9))
            .web_safe(args.web_safe)
            .boxes(args.boxes)
            .enabled(!args.disabled)
            .kind_name(format!("{:?}", args.kind));
        if args.gamut {
            ramp = ramp.gamut(&muted).gamut_revision(1);
        }
        let shown = ramp.show(ui);
        if let Some(event) = &shown.edit.event {
            if let Some(pick) = event.value() {
                *color = pick.color;
            }
            *last = edit::describe(&event.clone().map(|p| format!("{}{}", p.color.to_rgba().to_hex(), if p.other { " (other)" } else { "" })));
        }
        *cycles += usize::from(shown.cycle);
        Stack::new(Gap::Stack).show(ui, |ui| {
            ui.add(Text::new(format!("Colour: {}", color.to_rgba().to_hex())).content(Content::Muted));
            ui.add(Text::new(format!("Last event: {last}")).content(Content::Muted));
            ui.add(Text::new(format!("Cycles: {cycles}")).content(Content::Muted));
        });
    });
}
