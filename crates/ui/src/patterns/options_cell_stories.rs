//! Stories of the step 3 option cells.

use egui::{Ui, vec2};

use super::options_bar::OptionsBar;
use super::options_cell::CellWidth;
use crate::behavior::DigitKeys;
use crate::controls::NumberField;
use crate::localized::units::{LengthUnit, UnitContext};
use crate::localized::{LocaleNumbers, Unit};
use crate::story::{Level, Story, StoryArgs, Surface};
use crate::typed_story;
use craft_theme::ControlSize;

#[derive(StoryArgs, Debug, PartialEq)]
struct CellsArgs {
    narrow: bool,
    expressions: bool,
    #[arg(name = "digit keys")]
    digit_keys: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/options-bar/cells",
            Level::Patterns,
            "OptionsBar",
            "Value cells",
            CellsArgs { narrow: false, expressions: true, digit_keys: true },
            show_cells
        )
        .on(Surface::Chrome),
    ]
}

/// The values the cells edit, kept between passes.
#[derive(Clone, Copy, Debug)]
struct Values {
    feather: f64,
    opacity: f64,
    flow: f64,
    width: f64,
    height: f64,
    unit: LengthUnit,
    digits: DigitKeys,
}

impl Default for Values {
    fn default() -> Self {
        Self { feather: 0.0, opacity: 100.0, flow: 100.0, width: 640.0, height: 400.0, unit: LengthUnit::Pixels, digits: DigitKeys::default() }
    }
}

/// A marquee-like bar: Feather (number cell), Opacity and Flow (percent cells, digit keys when
/// the story has the keyboard), Fixed Size Width and Height (unit cells with the units menu), and
/// `reveal_focused_control` in a narrow bar.
fn show_cells(ui: &mut Ui, args: &mut CellsArgs) {
    let key = ui.scope_id().with("cells-story");
    let mut values: Values = ui.data(|d| d.get_temp(key)).unwrap_or_default();
    let numbers = LocaleNumbers::of(ui);
    let width = if args.narrow { 320.0 } else { ui.available_width() };
    let expressions = args.expressions;
    let bar = ui
        .allocate_ui(vec2(width, 0.0), |ui| {
            ui.set_max_width(width);
            OptionsBar::new("cells-story-bar").narrow(args.narrow).show(ui, |bar| {
                bar.body(|ui| {
                    let feather = OptionsBar::number_cell(
                        ui,
                        "Feather",
                        CellWidth::Md,
                        NumberField::new("cells-feather", "Feather", values.feather, &numbers)
                            .range(0.0..=1000.0)
                            .unit(Unit::new("{n} px"))
                            .expressions(expressions)
                            .size(ControlSize::Sm),
                    );
                    if let Some(v) = feather.event.as_ref().and_then(|e| e.value()) {
                        values.feather = *v;
                    }
                    if let Some(v) =
                        OptionsBar::percent_cell(ui, "cells-opacity", "Opacity", values.opacity, &numbers).event.as_ref().and_then(|e| e.value())
                    {
                        values.opacity = *v;
                    }
                    if let Some(v) = OptionsBar::percent_cell(ui, "cells-flow", "Flow", values.flow, &numbers).event.as_ref().and_then(|e| e.value())
                    {
                        values.flow = *v;
                    }
                    OptionsBar::divider(ui);
                    let context = UnitContext::default();
                    for (salt, caption, pixels) in [("cells-width", "Width", &mut values.width), ("cells-height", "Height", &mut values.height)] {
                        let cell = OptionsBar::unit_cell(ui, salt, caption, *pixels, 1.0..=30000.0, values.unit, &LengthUnit::ALL, context);
                        if let Some(v) = cell.edit.event.as_ref().and_then(|e| e.value()) {
                            *pixels = *v;
                        }
                        if let Some(unit) = cell.unit_picked {
                            values.unit = unit;
                        }
                    }
                    OptionsBar::reveal_focused_control(ui);
                });
            })
        })
        .inner;
    if let Some(entry) = values.digits.handle(ui.ctx(), args.digit_keys && bar.contains_pointer()) {
        *if entry.shift { &mut values.flow } else { &mut values.opacity } = entry.percent;
    }
    ui.data_mut(|d| d.insert_temp(key, values));
}
